//! Hardware shoulder triggers -> virtual touch via /dev/uinput.
//!
//! Low-level trigger handling is kept as close as possible to the user's
//! working `4.zip` project: the trigger side listens to BOTH `EV_ABS/ABS_DISTANCE`
//! and `EV_KEY/KEY_F7|KEY_F8`, with release confirmed by both signals.
//!
//! The only higher-level logic kept from mora is config gating:
//! - active only for the current foreground game
//! - active only when triggers are enabled in that game's config
//! - coordinates come from the game config (screen px -> mapped to raw touch range)

use std::{
    fs,
    io::{self, Read, Write},
    os::unix::fs::OpenOptionsExt,
    os::unix::io::{AsRawFd, RawFd},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use libc::c_int;

// ----------------- Linux input constants -----------------
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;

const SYN_REPORT: u16 = 0;

const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_DISTANCE: u16 = 0x19;

const ABS_MT_SLOT: u16 = 0x2f;
const ABS_MT_TOUCH_MAJOR: u16 = 0x30;
const ABS_MT_POSITION_X: u16 = 0x35;
const ABS_MT_POSITION_Y: u16 = 0x36;
const ABS_MT_TRACKING_ID: u16 = 0x39;

const KEY_F7: u16 = 65;
const KEY_F8: u16 = 66;

const BTN_TOOL_FINGER: u16 = 325;
const BTN_TOUCH: u16 = 330;

const INPUT_PROP_DIRECT: u16 = 0x01;

// ----------------- uinput ioctls -----------------
const IOC_NRBITS: u32 = 8;
const IOC_TYPEBITS: u32 = 8;
const IOC_SIZEBITS: u32 = 14;
const IOC_DIRSHIFT: u32 = IOC_NRBITS + IOC_TYPEBITS + IOC_SIZEBITS;
const IOC_TYPESHIFT: u32 = IOC_NRBITS;
const IOC_SIZESHIFT: u32 = IOC_NRBITS + IOC_TYPEBITS;

const IOC_NONE: u32 = 0;
const IOC_WRITE: u32 = 1;

const fn ioc(dir: u32, ty: u32, nr: u32, size: u32) -> u32 {
    (dir << IOC_DIRSHIFT) | (ty << IOC_TYPESHIFT) | (nr) | (size << IOC_SIZESHIFT)
}
const fn io(ty: u8, nr: u8) -> u32 {
    ioc(IOC_NONE, ty as u32, nr as u32, 0)
}
const fn iow(ty: u8, nr: u8, size: u32) -> u32 {
    ioc(IOC_WRITE, ty as u32, nr as u32, size)
}

const UI_SET_EVBIT: u32 = iow(b'U', 100, 4);
const UI_SET_KEYBIT: u32 = iow(b'U', 101, 4);
const UI_SET_ABSBIT: u32 = iow(b'U', 103, 4);
const UI_SET_PROPBIT: u32 = iow(b'U', 110, 4);
const UI_DEV_CREATE: u32 = io(b'U', 1);
const UI_DEV_DESTROY: u32 = io(b'U', 2);

const BUS_VIRTUAL: u16 = 0x06;

#[repr(C)]
#[derive(Clone, Copy)]
struct InputId {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
}

const ABS_CNT: usize = 0x40;

#[repr(C)]
struct UInputUserDev {
    name: [u8; 80],
    id: InputId,
    ff_effects_max: u32,
    absmax: [i32; ABS_CNT],
    absmin: [i32; ABS_CNT],
    absfuzz: [i32; ABS_CNT],
    absflat: [i32; ABS_CNT],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct TimeVal {
    tv_sec: i64,
    tv_usec: i64,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct InputEvent {
    time: TimeVal,
    type_: u16,
    code: u16,
    value: i32,
}

// ----------------- sysfs helpers -----------------
fn read_to_string(path: &Path) -> io::Result<String> {
    let mut s = String::new();
    fs::File::open(path)?.read_to_string(&mut s)?;
    Ok(s)
}

// sysfs capabilities printed as 64-bit hex chunks
fn parse_caps_hex_words_u64(s: &str) -> Vec<u64> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    let mut words: Vec<u64> = Vec::new();
    for p in parts {
        if let Ok(v) = u64::from_str_radix(p.trim(), 16) {
            words.push(v);
        }
    }
    words.reverse(); // word0=lowest bits
    words
}

fn cap_has(words: &[u64], bit: u32) -> bool {
    let wi = (bit / 64) as usize;
    let bi = (bit % 64) as u64;
    if wi >= words.len() {
        return false;
    }
    (words[wi] & (1u64 << bi)) != 0
}

fn sys_event_dir(ev: &str) -> PathBuf {
    PathBuf::from("/sys/class/input").join(ev).join("device")
}
fn devnode(ev: &str) -> PathBuf {
    PathBuf::from("/dev/input").join(ev)
}

#[derive(Clone, Debug)]
struct EventDevInfo {
    devnode: PathBuf,
    sysdir: PathBuf,
    name: String,
    has_abs_distance: bool,
    has_key_f7: bool,
    has_key_f8: bool,
    is_touch_like: bool,
}

fn scan_input_devices() -> io::Result<Vec<EventDevInfo>> {
    let mut out = Vec::new();
    let dir = Path::new("/sys/class/input");
    for ent in fs::read_dir(dir)? {
        let ent = ent?;
        let ev = ent.file_name().to_string_lossy().to_string();
        if !ev.starts_with("event") {
            continue;
        }
        let sysdir = sys_event_dir(&ev);
        let name = read_to_string(&sysdir.join("name"))
            .unwrap_or_default()
            .trim()
            .to_string();

        let abs_caps = read_to_string(&sysdir.join("capabilities/abs")).unwrap_or_default();
        let key_caps = read_to_string(&sysdir.join("capabilities/key")).unwrap_or_default();
        let prop_caps = read_to_string(&sysdir.join("properties")).unwrap_or_default();

        let abs_words = parse_caps_hex_words_u64(&abs_caps);
        let key_words = parse_caps_hex_words_u64(&key_caps);
        let prop_words = parse_caps_hex_words_u64(&prop_caps);

        let has_abs_distance = cap_has(&abs_words, ABS_DISTANCE as u32);
        let has_key_f7 = cap_has(&key_words, KEY_F7 as u32);
        let has_key_f8 = cap_has(&key_words, KEY_F8 as u32);

        let is_direct = cap_has(&prop_words, INPUT_PROP_DIRECT as u32);
        let has_mt = cap_has(&abs_words, ABS_MT_SLOT as u32)
            && cap_has(&abs_words, ABS_MT_TRACKING_ID as u32)
            && cap_has(&abs_words, ABS_MT_POSITION_X as u32)
            && cap_has(&abs_words, ABS_MT_POSITION_Y as u32);
        let has_btn_touch = cap_has(&key_words, BTN_TOUCH as u32);
        let is_touch_like = is_direct && has_mt && has_btn_touch;

        out.push(EventDevInfo {
            devnode: devnode(&ev),
            sysdir,
            name,
            has_abs_distance,
            has_key_f7,
            has_key_f8,
            is_touch_like,
        });
    }
    Ok(out)
}

fn choose_devices(devs: &[EventDevInfo]) -> io::Result<(EventDevInfo, EventDevInfo, Option<EventDevInfo>)> {
    let left = devs
        .iter()
        .filter(|d| d.has_abs_distance && d.has_key_f7)
        .max_by_key(|d| (d.name.contains("sar0") as i32, d.name.contains("nubia_tgk_aw") as i32))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "left trigger device (ABS_DISTANCE+KEY_F7) not found"))?
        .clone();

    let right = devs
        .iter()
        .filter(|d| d.has_abs_distance && d.has_key_f8)
        .max_by_key(|d| (d.name.contains("sar1") as i32, d.name.contains("nubia_tgk_aw") as i32))
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "right trigger device (ABS_DISTANCE+KEY_F8) not found"))?
        .clone();

    let touch = devs
        .iter()
        .filter(|d| d.is_touch_like)
        .max_by_key(|d| (d.name.contains("goodix") as i32, d.name.contains("ts") as i32))
        .cloned();

    Ok((left, right, touch))
}


// abs sysfs attributes: /sys/class/input/eventX/device/abs/abs_mt_position_x etc.
// Content is usually: "<value> <min> <max> <fuzz> <flat>"
fn read_abs_range(sysdir: &Path, abs_name: &str) -> Option<(i32, i32)> {
    let p = sysdir.join("abs").join(abs_name);
    let s = read_to_string(&p).ok()?;
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    let min = parts[1].parse::<i32>().ok()?;
    let max = parts[2].parse::<i32>().ok()?;
    Some((min, max))
}

fn discover_screen_size() -> (i32, i32) {
    let p = Path::new("/sys/class/graphics/fb0/virtual_size");
    if let Ok(s) = read_to_string(p) {
        let t = s.trim();
        if let Some((a, b)) = t.split_once(',') {
            if let (Ok(w), Ok(h)) = (a.trim().parse::<i32>(), b.trim().parse::<i32>()) {
                if w > 0 && h > 0 {
                    return (w, h);
                }
            }
        }
    }
    // sane fallback (device specific); this will be overwritten on most devices.
    (1116, 2480)
}

#[derive(Clone, Debug)]
struct Ranges {
    x_min: i32,
    x_max: i32,
    y_min: i32,
    y_max: i32,
    touch_major_max: i32,
    slot_max: i32,
}

fn discover_ranges(touch: &Option<EventDevInfo>) -> Ranges {
    if let Some(t) = touch {
        let (x_min, x_max) = read_abs_range(&t.sysdir, "abs_mt_position_x")
            .or_else(|| read_abs_range(&t.sysdir, "abs_x"))
            .unwrap_or((0, 17856));
        let (y_min, y_max) = read_abs_range(&t.sysdir, "abs_mt_position_y")
            .or_else(|| read_abs_range(&t.sysdir, "abs_y"))
            .unwrap_or((0, 39680));
        let (_, touch_major_max) = read_abs_range(&t.sysdir, "abs_mt_touch_major").unwrap_or((0, 4080));
        let (_, slot_max) = read_abs_range(&t.sysdir, "abs_mt_slot").unwrap_or((0, 9));
        return Ranges {
            x_min,
            x_max,
            y_min,
            y_max,
            touch_major_max,
            slot_max,
        };
    }
    Ranges {
        x_min: 0,
        x_max: 17856,
        y_min: 0,
        y_max: 39680,
        touch_major_max: 4080,
        slot_max: 9,
    }
}

// ----------------- uinput setup -----------------
fn open_event_dev(path: &Path) -> io::Result<fs::File> {
    fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC)
        .open(path)
}

fn open_uinput() -> io::Result<fs::File> {
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NONBLOCK)
        .open("/dev/uinput")
}

fn xioctl(fd: RawFd, req: u32, arg: c_int) -> io::Result<()> {
    let r = unsafe { libc::ioctl(fd, req as c_int, arg) };
    if r < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn xioctl_void(fd: RawFd, req: u32) -> io::Result<()> {
    let r = unsafe { libc::ioctl(fd, req as c_int) };
    if r < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn write_struct<T>(f: &mut fs::File, s: &T) -> io::Result<()> {
    let p = (s as *const T) as *const u8;
    let n = std::mem::size_of::<T>();
    let bytes = unsafe { std::slice::from_raw_parts(p, n) };
    f.write_all(bytes)
}

fn make_uinput_touch(mut uif: fs::File, ranges: &Ranges) -> io::Result<fs::File> {
    let fd = uif.as_raw_fd();

    xioctl(fd, UI_SET_EVBIT, EV_KEY as c_int)?;
    xioctl(fd, UI_SET_EVBIT, EV_ABS as c_int)?;

    xioctl(fd, UI_SET_KEYBIT, BTN_TOUCH as c_int)?;
    xioctl(fd, UI_SET_KEYBIT, BTN_TOOL_FINGER as c_int)?;

    for &abs in &[
        ABS_X,
        ABS_Y,
        ABS_MT_SLOT,
        ABS_MT_TOUCH_MAJOR,
        ABS_MT_POSITION_X,
        ABS_MT_POSITION_Y,
        ABS_MT_TRACKING_ID,
    ] {
        xioctl(fd, UI_SET_ABSBIT, abs as c_int)?;
    }
    xioctl(fd, UI_SET_PROPBIT, INPUT_PROP_DIRECT as c_int)?;

    let mut uidev = UInputUserDev {
        name: [0u8; 80],
        id: InputId {
            bustype: BUS_VIRTUAL,
            vendor: 0x18d1,
            product: 0x4ee7,
            version: 1,
        },
        ff_effects_max: 0,
        absmax: [0; ABS_CNT],
        absmin: [0; ABS_CNT],
        absfuzz: [0; ABS_CNT],
        absflat: [0; ABS_CNT],
    };
    let name = b"trig_touch_uinput\0";
    uidev.name[..name.len()].copy_from_slice(name);

    // Single-touch ranges
    uidev.absmin[ABS_X as usize] = ranges.x_min;
    uidev.absmax[ABS_X as usize] = ranges.x_max;
    uidev.absmin[ABS_Y as usize] = ranges.y_min;
    uidev.absmax[ABS_Y as usize] = ranges.y_max;

    // MT ranges
    uidev.absmin[ABS_MT_SLOT as usize] = 0;
    uidev.absmax[ABS_MT_SLOT as usize] = ranges.slot_max.max(1);
    uidev.absmin[ABS_MT_TOUCH_MAJOR as usize] = 0;
    uidev.absmax[ABS_MT_TOUCH_MAJOR as usize] = ranges.touch_major_max.max(1);

    uidev.absmin[ABS_MT_POSITION_X as usize] = ranges.x_min;
    uidev.absmax[ABS_MT_POSITION_X as usize] = ranges.x_max;
    uidev.absmin[ABS_MT_POSITION_Y as usize] = ranges.y_min;
    uidev.absmax[ABS_MT_POSITION_Y as usize] = ranges.y_max;

    uidev.absmin[ABS_MT_TRACKING_ID as usize] = 0;
    uidev.absmax[ABS_MT_TRACKING_ID as usize] = 65535;

    write_struct(&mut uif, &uidev)?;
    xioctl_void(fd, UI_DEV_CREATE)?;
    thread::sleep(Duration::from_millis(200));
    Ok(uif)
}

fn emit(uif: &mut fs::File, ty: u16, code: u16, value: i32) -> io::Result<()> {
    let ev = InputEvent {
        time: TimeVal { tv_sec: 0, tv_usec: 0 },
        type_: ty,
        code,
        value,
    };
    write_struct(uif, &ev)
}
fn syn(uif: &mut fs::File) -> io::Result<()> {
    emit(uif, EV_SYN, SYN_REPORT, 0)
}

fn touch_down(
    uif: &mut fs::File,
    slot: i32,
    tid: i32,
    x: i32,
    y: i32,
    major: i32,
) -> io::Result<()> {
    emit(uif, EV_KEY, BTN_TOUCH, 1)?;
    emit(uif, EV_KEY, BTN_TOOL_FINGER, 1)?;
    emit(uif, EV_ABS, ABS_MT_SLOT, slot)?;
    emit(uif, EV_ABS, ABS_MT_TRACKING_ID, tid)?;
    emit(uif, EV_ABS, ABS_MT_POSITION_X, x)?;
    emit(uif, EV_ABS, ABS_MT_POSITION_Y, y)?;
    emit(uif, EV_ABS, ABS_MT_TOUCH_MAJOR, major)?;
    emit(uif, EV_ABS, ABS_X, x)?;
    emit(uif, EV_ABS, ABS_Y, y)?;
    syn(uif)?;
    Ok(())
}

fn touch_up(uif: &mut fs::File, slot: i32) -> io::Result<()> {
    emit(uif, EV_ABS, ABS_MT_SLOT, slot)?;
    emit(uif, EV_ABS, ABS_MT_TRACKING_ID, -1)?;
    syn(uif)?;
    Ok(())
}

fn cleanup_uinput(uif: &mut fs::File, slot_max: i32) {
    let max_slot = slot_max.clamp(1, 50);
    for slot in 0..=max_slot {
        let _ = emit(uif, EV_ABS, ABS_MT_SLOT, slot);
        let _ = emit(uif, EV_ABS, ABS_MT_TRACKING_ID, -1);
    }
    let _ = emit(uif, EV_KEY, BTN_TOUCH, 0);
    let _ = emit(uif, EV_KEY, BTN_TOOL_FINGER, 0);
    let _ = syn(uif);
}

fn force_release_all(uif: &mut fs::File, slot_max: i32) {
    cleanup_uinput(uif, slot_max);
}

fn read_input_event(f: &mut fs::File) -> io::Result<InputEvent> {
    let mut buf = [0u8; std::mem::size_of::<InputEvent>()];
    f.read_exact(&mut buf)?;
    Ok(unsafe { std::ptr::read_unaligned(buf.as_ptr() as *const InputEvent) })
}

fn map_point(px: i32, py: i32, screen_w: i32, screen_h: i32, ranges: &Ranges) -> (i32, i32) {
    let sw = screen_w.max(1);
    let sh = screen_h.max(1);
    let den_x = (sw - 1).max(1);
    let den_y = (sh - 1).max(1);
    let xrng = (ranges.x_max - ranges.x_min).max(1);
    let yrng = (ranges.y_max - ranges.y_min).max(1);
    let x = ranges.x_min + (px * xrng) / den_x;
    let y = ranges.y_min + (py * yrng) / den_y;
    (x, y)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SideConfig {
    pub enabled: bool,
    pub x_px: i32,
    pub y_px: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveConfig {
    pub enabled: bool,
    pub left: SideConfig,
    pub right: SideConfig,
}

struct Inner {
    uif: Arc<Mutex<fs::File>>,
    stop: AtomicBool,

    // global enable (game foreground + screen on + triggers.enabled)
    active: AtomicBool,
    gen: AtomicU64,

    // per-side enable and mapped ABS coords
    left_enabled: AtomicBool,
    right_enabled: AtomicBool,
    left_x: AtomicI32,
    left_y: AtomicI32,
    right_x: AtomicI32,
    right_y: AtomicI32,

    // bookkeeping
    virt_active: AtomicI32,
    tid_l: AtomicU32,
    tid_r: AtomicU32,
    touch_major: i32,
}

#[derive(Clone)]
pub struct TriggerManager {
    inner: Arc<Inner>,
    screen_w: i32,
    screen_h: i32,
    ranges: Ranges,
}

impl TriggerManager {
    /// Try to initialize triggers subsystem. If devices are not found, returns error.
    pub fn init() -> io::Result<Self> {
        let devs = scan_input_devices()?;
        let (left, right, touch) = choose_devices(&devs)?;

        println!("TRIG: left={} name='{}'", left.devnode.display(), left.name);
        println!("TRIG: right={} name='{}'", right.devnode.display(), right.name);
        if let Some(t) = &touch {
            println!("TRIG: touch={} name='{}' (ranges source)", t.devnode.display(), t.name);
        } else {
            println!("TRIG: touch not detected (ranges fallback)");
        }

        let (screen_w, screen_h) = discover_screen_size();
        println!("TRIG: screen {}x{}", screen_w, screen_h);

        let ranges = discover_ranges(&touch);
        println!(
            "TRIG: ranges X[{}..{}] Y[{}..{}] slot_max={} major_max={}",
            ranges.x_min, ranges.x_max, ranges.y_min, ranges.y_max, ranges.slot_max, ranges.touch_major_max
        );

        let uif = open_uinput()?;
        let mut uif = make_uinput_touch(uif, &ranges)?;
        // startup cleanup
        force_release_all(&mut uif, ranges.slot_max);

        let uif_arc = Arc::new(Mutex::new(uif));

        let inner = Arc::new(Inner {
            uif: uif_arc.clone(),
            stop: AtomicBool::new(false),
            active: AtomicBool::new(false),
            gen: AtomicU64::new(1),
            left_enabled: AtomicBool::new(false),
            right_enabled: AtomicBool::new(false),
            left_x: AtomicI32::new(ranges.x_min),
            left_y: AtomicI32::new(ranges.y_min),
            right_x: AtomicI32::new(ranges.x_min),
            right_y: AtomicI32::new(ranges.y_min),
            virt_active: AtomicI32::new(0),
            tid_l: AtomicU32::new(1000),
            tid_r: AtomicU32::new(2000),
            touch_major: (ranges.touch_major_max / 16).clamp(5, ranges.touch_major_max.max(5)),
        });

        // Spawn listener threads.
        spawn_trigger_thread(inner.clone(), left.devnode, KEY_F7, 0);
        spawn_trigger_thread(inner.clone(), right.devnode, KEY_F8, 1);

        Ok(Self {
            inner,
            screen_w,
            screen_h,
            ranges,
        })
    }

    /// Apply new trigger config. This will take effect quickly (< 250ms).
    pub fn set_config(&self, cfg: ActiveConfig) {
        let any_side = cfg.enabled && (cfg.left.enabled || cfg.right.enabled);

        if !any_side {
            self.disable();
            return;
        }

        let (lx, ly) = map_point(cfg.left.x_px, cfg.left.y_px, self.screen_w, self.screen_h, &self.ranges);
        let (rx, ry) = map_point(cfg.right.x_px, cfg.right.y_px, self.screen_w, self.screen_h, &self.ranges);

        self.inner.left_x.store(lx, Ordering::SeqCst);
        self.inner.left_y.store(ly, Ordering::SeqCst);
        self.inner.right_x.store(rx, Ordering::SeqCst);
        self.inner.right_y.store(ry, Ordering::SeqCst);

        self.inner.left_enabled.store(cfg.left.enabled, Ordering::SeqCst);
        self.inner.right_enabled.store(cfg.right.enabled, Ordering::SeqCst);
        self.inner.active.store(true, Ordering::SeqCst);

        println!(
            "TRIGCFG: set left={} px=({}, {}) raw=({}, {}) | right={} px=({}, {}) raw=({}, {})",
            cfg.left.enabled, cfg.left.x_px, cfg.left.y_px, lx, ly,
            cfg.right.enabled, cfg.right.x_px, cfg.right.y_px, rx, ry
        );

        self.bump_gen();
    }

    /// Disable triggers and force-release any active virtual touches.
    pub fn disable(&self) {
        self.inner.active.store(false, Ordering::SeqCst);
        self.inner.left_enabled.store(false, Ordering::SeqCst);
        self.inner.right_enabled.store(false, Ordering::SeqCst);

        // Force release now (don’t wait for hardware events).
        {
            let mut uif = self.inner.uif.lock().unwrap();
            force_release_all(&mut uif, self.ranges.slot_max);
        }
        self.inner.virt_active.store(0, Ordering::SeqCst);
        self.bump_gen();
    }

    fn bump_gen(&self) {
        self.inner.gen.fetch_add(1, Ordering::SeqCst);
    }
}

impl Drop for TriggerManager {
    fn drop(&mut self) {
        self.inner.stop.store(true, Ordering::SeqCst);
        // best-effort cleanup
        {
            let mut uif = self.inner.uif.lock().unwrap();
            force_release_all(&mut uif, self.ranges.slot_max);
            let _ = unsafe { libc::ioctl(uif.as_raw_fd(), UI_DEV_DESTROY as c_int) };
        }
    }
}

fn spawn_trigger_thread(inner: Arc<Inner>, dev_path: PathBuf, key_code: u16, slot: i32) {
    thread::spawn(move || {
        let mut f = match open_event_dev(&dev_path) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("TRIG: failed to open {}: {}", dev_path.display(), e);
                return;
            }
        };
        println!("TRIG: listen {} (slot={})", dev_path.display(), slot);

        let mut pressed = false;
        let mut abs0_seen = false;
        let mut keyup_seen = false;
        let mut abs_state: Option<bool> = None;
        let mut key_state: Option<bool> = None;
        let mut last_gen = inner.gen.load(Ordering::SeqCst);

        loop {
            if inner.stop.load(Ordering::Relaxed) {
                break;
            }

            let ev = match read_input_event(&mut f) {
                Ok(e) => e,
                Err(e) => {
                    if inner.stop.load(Ordering::Relaxed) {
                        break;
                    }
                    eprintln!("TRIG: read error on {}: {}", dev_path.display(), e);
                    return;
                }
            };

            let cur_gen = inner.gen.load(Ordering::SeqCst);
            if cur_gen != last_gen {
                if pressed {
                    let mut uif = inner.uif.lock().unwrap();
                    let _ = touch_up(&mut uif, slot);
                    dec_active(&inner);
                    println!("TRIG: {} UP(reconfig)", if slot == 0 { "L" } else { "R" });
                    pressed = false;
                }
                abs_state = None;
                key_state = None;
                abs0_seen = false;
                keyup_seen = false;
                last_gen = cur_gen;
            }

            let active = inner.active.load(Ordering::SeqCst);
            let side_enabled = if slot == 0 {
                inner.left_enabled.load(Ordering::SeqCst)
            } else {
                inner.right_enabled.load(Ordering::SeqCst)
            };

            let mut is_press_evt = false;
            let mut is_release_evt = false;

            if ev.type_ == EV_ABS && ev.code == ABS_DISTANCE {
                if ev.value == 0 {
                    abs_state = Some(false);
                    abs0_seen = true;
                    is_release_evt = true;
                } else {
                    abs_state = Some(true);
                    is_press_evt = true;
                }
            }

            if ev.type_ == EV_KEY && ev.code == key_code {
                if ev.value != 0 {
                    key_state = Some(true);
                    is_press_evt = true;
                } else {
                    key_state = Some(false);
                    keyup_seen = true;
                    is_release_evt = true;
                }
            }

            if !pressed && is_press_evt {
                if !(active && side_enabled) {
                    continue;
                }

                pressed = true;
                abs0_seen = false;
                keyup_seen = false;
                if abs_state.is_none() { abs_state = Some(true); }
                if key_state.is_none() { key_state = Some(true); }

                let (x, y) = if slot == 0 {
                    (inner.left_x.load(Ordering::SeqCst), inner.left_y.load(Ordering::SeqCst))
                } else {
                    (inner.right_x.load(Ordering::SeqCst), inner.right_y.load(Ordering::SeqCst))
                };

                let tid = if slot == 0 {
                    inner.tid_l.fetch_add(1, Ordering::SeqCst)
                } else {
                    inner.tid_r.fetch_add(1, Ordering::SeqCst)
                } as i32;
                let tid = (tid % 65000).max(1);

                inc_active(&inner);
                {
                    let mut uif = inner.uif.lock().unwrap();
                    match touch_down(&mut uif, slot, tid, x, y, inner.touch_major) {
                        Ok(_) => println!("TRIG: {} DOWN tid={} raw=({}, {})", if slot == 0 { "L" } else { "R" }, tid, x, y),
                        Err(e) => eprintln!("TRIG: touch_down failed: {}", e),
                    }
                }
                continue;
            }

            if pressed && is_release_evt {
                let do_release = abs0_seen && keyup_seen && abs_state == Some(false) && key_state == Some(false);
                if do_release {
                    {
                        let mut uif = inner.uif.lock().unwrap();
                        let _ = touch_up(&mut uif, slot);
                    }
                    dec_active(&inner);
                    println!("TRIG: {} UP", if slot == 0 { "L" } else { "R" });
                    pressed = false;
                    abs_state = None;
                    key_state = None;
                    abs0_seen = false;
                    keyup_seen = false;
                }
            }
        }

        if pressed {
            let mut uif = inner.uif.lock().unwrap();
            let _ = touch_up(&mut uif, slot);
            dec_active(&inner);
        }
    });
}

fn inc_active(inner: &Inner) {
    inner.virt_active.fetch_add(1, Ordering::SeqCst);
}

fn dec_active(inner: &Inner) {
    let mut cur = inner.virt_active.load(Ordering::SeqCst);
    loop {
        if cur <= 0 {
            break;
        }
        match inner.virt_active.compare_exchange(cur, cur - 1, Ordering::SeqCst, Ordering::SeqCst)
        {
            Ok(_) => break,
            Err(v) => cur = v,
        }
    }
    if inner.virt_active.load(Ordering::SeqCst) == 0 {
        let mut uif = inner.uif.lock().unwrap();
        let _ = emit(&mut uif, EV_KEY, BTN_TOUCH, 0);
        let _ = emit(&mut uif, EV_KEY, BTN_TOOL_FINGER, 0);
        let _ = syn(&mut uif);
    }
}
