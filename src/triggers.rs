//! Hardware shoulder triggers -> virtual touch, merged with real finger touches.
//!
//! ## Why this exists / what changed
//! The previous implementation created a SEPARATE uinput touch device and let the
//! real touchscreen keep feeding Android directly. Android then saw TWO multitouch
//! devices and routed a window's gesture to whichever device most recently started
//! a contact, dropping the other one. Result: pressing a trigger killed finger
//! touch and vice-versa ("either touch OR triggers").
//!
//! ## New architecture: GRAB + MERGE through ONE virtual device
//! 1. We `EVIOCGRAB` the real touchscreen (exclusive). Fingers no longer reach
//!    Android directly; only this daemon reads them.
//! 2. We create ONE uinput multitouch device that mirrors the real touchscreen
//!    ranges, with the slot space extended by 2 reserved trigger slots.
//! 3. A forwarder thread re-emits every real finger frame verbatim into the
//!    virtual device (finger slots `0..=slot_max`).
//! 4. Trigger threads inject their touches into the SAME virtual device on the
//!    reserved slots (`slot_max+1` = left, `slot_max+2` = right).
//!
//! Android now sees a SINGLE device, so fingers and triggers are just different
//! slots of one contact report — like real extra fingers. Neither cancels the
//! other. `BTN_TOUCH` and the `BTN_TOOL_*` tap buttons are managed centrally from
//! the combined (finger + trigger) contact count so the two streams never fight
//! over the global "is anything touching" flag.
//!
//! ## Safety
//! If the touchscreen can't be grabbed, merge is skipped and triggers stay off
//! (we never fall back to the broken two-device mode). On forwarder exit / Drop we
//! `EVIOCGRAB(0)` to restore native touch. With `panic = "abort"`, any panic also
//! closes the fd, and the kernel auto-ungrabs.
//!
//! Higher-level config gating is unchanged:
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

const BTN_TOUCH: u16 = 0x14a; // 330
const BTN_TOOL_FINGER: u16 = 0x145; // 325
const BTN_TOOL_QUINTTAP: u16 = 0x148; // 328
const BTN_TOOL_DOUBLETAP: u16 = 0x14d; // 333
const BTN_TOOL_TRIPLETAP: u16 = 0x14e; // 334
const BTN_TOOL_QUADTAP: u16 = 0x14f; // 335

const INPUT_PROP_DIRECT: u16 = 0x01;

// ----------------- uinput / evdev ioctls -----------------
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

// _IOW('E', 0x90, int) -- exclusive grab of an evdev device.
const EVIOCGRAB: u32 = iow(b'E', 0x90, 4);

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

fn choose_devices(
    devs: &[EventDevInfo],
) -> io::Result<(EventDevInfo, EventDevInfo, Option<EventDevInfo>)> {
    let left = devs
        .iter()
        .filter(|d| d.has_abs_distance && d.has_key_f7)
        .max_by_key(|d| (d.name.contains("sar0") as i32, d.name.contains("nubia_tgk_aw") as i32))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "left trigger device (ABS_DISTANCE+KEY_F7) not found",
            )
        })?
        .clone();

    let right = devs
        .iter()
        .filter(|d| d.has_abs_distance && d.has_key_f8)
        .max_by_key(|d| (d.name.contains("sar1") as i32, d.name.contains("nubia_tgk_aw") as i32))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                "right trigger device (ABS_DISTANCE+KEY_F8) not found",
            )
        })?
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
        let (_, touch_major_max) =
            read_abs_range(&t.sysdir, "abs_mt_touch_major").unwrap_or((0, 4080));
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

// ----------------- open / ioctl helpers -----------------
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

/// Create the merged virtual touchscreen. Slot space is extended by 2 so the two
/// trigger contacts live on dedicated slots (`slot_max+1`, `slot_max+2`) that real
/// fingers never use.
fn make_uinput_touch(mut uif: fs::File, ranges: &Ranges) -> io::Result<fs::File> {
    let fd = uif.as_raw_fd();

    xioctl(fd, UI_SET_EVBIT, EV_KEY as c_int)?;
    xioctl(fd, UI_SET_EVBIT, EV_ABS as c_int)?;
    xioctl(fd, UI_SET_EVBIT, EV_SYN as c_int)?;

    for &btn in &[
        BTN_TOUCH,
        BTN_TOOL_FINGER,
        BTN_TOOL_DOUBLETAP,
        BTN_TOOL_TRIPLETAP,
        BTN_TOOL_QUADTAP,
        BTN_TOOL_QUINTTAP,
    ] {
        xioctl(fd, UI_SET_KEYBIT, btn as c_int)?;
    }

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

    // MT ranges -- extend the slot count by 2 reserved trigger slots.
    uidev.absmin[ABS_MT_SLOT as usize] = 0;
    uidev.absmax[ABS_MT_SLOT as usize] = ranges.slot_max.max(1) + 2;
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

/// Central management of the device-global tap buttons from the COMBINED contact
/// count (forwarded fingers + injected triggers). This is the key to merging the
/// two streams: neither side ever drives `BTN_TOUCH` to 0 while the other still
/// has a live contact.
fn set_tool_buttons(uif: &mut fs::File, count: i32) -> io::Result<()> {
    let c = count.max(0);
    emit(uif, EV_KEY, BTN_TOUCH, (c > 0) as i32)?;
    emit(uif, EV_KEY, BTN_TOOL_FINGER, (c == 1) as i32)?;
    emit(uif, EV_KEY, BTN_TOOL_DOUBLETAP, (c == 2) as i32)?;
    emit(uif, EV_KEY, BTN_TOOL_TRIPLETAP, (c == 3) as i32)?;
    emit(uif, EV_KEY, BTN_TOOL_QUADTAP, (c == 4) as i32)?;
    emit(uif, EV_KEY, BTN_TOOL_QUINTTAP, (c >= 5) as i32)?;
    Ok(())
}

/// Emit only the MT contact-down axes for a slot (no buttons, no SYN).
fn mt_contact_down(
    uif: &mut fs::File,
    slot: i32,
    tid: i32,
    x: i32,
    y: i32,
    major: i32,
) -> io::Result<()> {
    emit(uif, EV_ABS, ABS_MT_SLOT, slot)?;
    emit(uif, EV_ABS, ABS_MT_TRACKING_ID, tid)?;
    emit(uif, EV_ABS, ABS_MT_POSITION_X, x)?;
    emit(uif, EV_ABS, ABS_MT_POSITION_Y, y)?;
    emit(uif, EV_ABS, ABS_MT_TOUCH_MAJOR, major)?;
    Ok(())
}

/// Emit only the MT contact-up for a slot (no buttons, no SYN).
fn mt_contact_up(uif: &mut fs::File, slot: i32) -> io::Result<()> {
    emit(uif, EV_ABS, ABS_MT_SLOT, slot)?;
    emit(uif, EV_ABS, ABS_MT_TRACKING_ID, -1)?;
    Ok(())
}

/// Release every slot (fingers + triggers) and zero the tap buttons. Used only at
/// startup and teardown, never while merging is live.
fn force_release_all(uif: &mut fs::File, max_slot: i32) {
    let max_slot = max_slot.clamp(1, 50);
    for slot in 0..=max_slot {
        let _ = emit(uif, EV_ABS, ABS_MT_SLOT, slot);
        let _ = emit(uif, EV_ABS, ABS_MT_TRACKING_ID, -1);
    }
    let _ = set_tool_buttons(uif, 0);
    let _ = syn(uif);
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

    // contact bookkeeping
    virt_active: AtomicI32,  // live trigger contacts (0..=2)
    finger_count: AtomicI32, // live forwarded finger contacts
    tid_l: AtomicU32,
    tid_r: AtomicU32,
    touch_major: i32,

    // reserved trigger slots (above the finger slot range)
    slot_l: i32,
    slot_r: i32,

    // raw fd of the grabbed real touchscreen, or -1 if not grabbed.
    touch_grab_fd: AtomicI32,
}

impl Inner {
    fn contact_count(&self) -> i32 {
        self.finger_count.load(Ordering::SeqCst).max(0)
            + self.virt_active.load(Ordering::SeqCst).max(0)
    }
}

#[derive(Clone)]
pub struct TriggerManager {
    inner: Arc<Inner>,
    screen_w: i32,
    screen_h: i32,
    ranges: Ranges,
}

impl TriggerManager {
    /// Try to initialize the triggers subsystem. If the trigger devices are not
    /// found, returns an error. If the touchscreen can't be found/grabbed, merge
    /// is skipped and triggers stay disabled (we never use the broken two-device
    /// fallback).
    pub fn init() -> io::Result<Self> {
        let devs = scan_input_devices()?;
        let (left, right, touch) = choose_devices(&devs)?;

        println!("TRIG: left={} name='{}'", left.devnode.display(), left.name);
        println!("TRIG: right={} name='{}'", right.devnode.display(), right.name);
        if let Some(t) = &touch {
            println!("TRIG: touch={} name='{}' (grab+merge source)", t.devnode.display(), t.name);
        } else {
            println!("TRIG: touch device NOT found -- merge impossible, triggers disabled");
        }

        let (screen_w, screen_h) = discover_screen_size();
        println!("TRIG: screen {}x{}", screen_w, screen_h);

        let ranges = discover_ranges(&touch);
        println!(
            "TRIG: ranges X[{}..{}] Y[{}..{}] slot_max={} major_max={}",
            ranges.x_min, ranges.x_max, ranges.y_min, ranges.y_max, ranges.slot_max, ranges.touch_major_max
        );

        let slot_l = ranges.slot_max + 1;
        let slot_r = ranges.slot_max + 2;
        let cleanup_max = ranges.slot_max + 2;
        println!("TRIG: finger slots 0..={} | trigger slots L={} R={}", ranges.slot_max, slot_l, slot_r);

        let uif = open_uinput()?;
        let mut uif = make_uinput_touch(uif, &ranges)?;
        // startup cleanup (nothing is live yet, so releasing all slots is safe)
        force_release_all(&mut uif, cleanup_max);

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
            finger_count: AtomicI32::new(0),
            tid_l: AtomicU32::new(30000),
            tid_r: AtomicU32::new(40000),
            touch_major: (ranges.touch_major_max / 16).clamp(5, ranges.touch_major_max.max(5)),
            slot_l,
            slot_r,
            touch_grab_fd: AtomicI32::new(-1),
        });

        // Forwarder: grab the real touchscreen and merge its finger frames into the
        // virtual device. Without it, merging is impossible -> keep triggers off.
        match &touch {
            Some(t) => {
                spawn_forwarder(inner.clone(), t.devnode.clone(), ranges.slot_max);
                // Give the forwarder a moment to grab before listeners go live.
                thread::sleep(Duration::from_millis(150));
                spawn_trigger_thread(inner.clone(), left.devnode, KEY_F7, true, slot_l);
                spawn_trigger_thread(inner.clone(), right.devnode, KEY_F8, false, slot_r);
            }
            None => {
                eprintln!("TRIG: no touchscreen to grab; trigger listeners NOT started");
            }
        }

        Ok(Self {
            inner,
            screen_w,
            screen_h,
            ranges,
        })
    }

    /// Apply new trigger config. Takes effect quickly (< 250ms).
    pub fn set_config(&self, cfg: ActiveConfig) {
        let any_side = cfg.enabled && (cfg.left.enabled || cfg.right.enabled);

        if !any_side {
            self.disable();
            return;
        }

        // If we never grabbed the touchscreen, refuse to inject (would recreate the
        // broken two-device situation).
        if self.inner.touch_grab_fd.load(Ordering::SeqCst) < 0 {
            eprintln!("TRIG: set_config ignored -- touchscreen not grabbed (merge unavailable)");
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

    /// Disable triggers and release ONLY the trigger slots (never the finger slots,
    /// which the forwarder owns).
    pub fn disable(&self) {
        self.inner.active.store(false, Ordering::SeqCst);
        self.inner.left_enabled.store(false, Ordering::SeqCst);
        self.inner.right_enabled.store(false, Ordering::SeqCst);

        {
            let mut uif = self.inner.uif.lock().unwrap();
            let _ = mt_contact_up(&mut uif, self.inner.slot_l);
            let _ = mt_contact_up(&mut uif, self.inner.slot_r);
            self.inner.virt_active.store(0, Ordering::SeqCst);
            let count = self.inner.contact_count();
            let _ = set_tool_buttons(&mut uif, count);
            let _ = syn(&mut uif);
        }
        self.bump_gen();
    }

    fn bump_gen(&self) {
        self.inner.gen.fetch_add(1, Ordering::SeqCst);
    }
}

impl Drop for TriggerManager {
    fn drop(&mut self) {
        self.inner.stop.store(true, Ordering::SeqCst);

        // Restore native touch FIRST: ungrab the real touchscreen so the user keeps
        // working touch even if the forwarder thread is still blocked in read().
        let gfd = self.inner.touch_grab_fd.load(Ordering::SeqCst);
        if gfd >= 0 {
            let _ = xioctl(gfd, EVIOCGRAB, 0);
            self.inner.touch_grab_fd.store(-1, Ordering::SeqCst);
        }

        // Best-effort uinput teardown.
        {
            let mut uif = self.inner.uif.lock().unwrap();
            force_release_all(&mut uif, self.ranges.slot_max + 2);
            let _ = unsafe { libc::ioctl(uif.as_raw_fd(), UI_DEV_DESTROY as c_int) };
        }
    }
}

/// Grab the real touchscreen and re-emit every finger frame into the merged
/// virtual device, tracking the live finger count so tap buttons stay correct.
fn spawn_forwarder(inner: Arc<Inner>, touch_path: PathBuf, finger_slot_max: i32) {
    thread::spawn(move || {
        let mut f = match open_event_dev(&touch_path) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("TRIG: forwarder open {} failed: {}", touch_path.display(), e);
                return;
            }
        };
        let fd = f.as_raw_fd();
        if let Err(e) = xioctl(fd, EVIOCGRAB, 1) {
            eprintln!(
                "TRIG: EVIOCGRAB failed on {}: {} -- merge disabled, leaving native touch intact",
                touch_path.display(),
                e
            );
            return;
        }
        inner.touch_grab_fd.store(fd, Ordering::SeqCst);
        println!("TRIG: forwarder grabbed {} (merge active)", touch_path.display());

        let slots = (finger_slot_max.max(1) as usize) + 4;
        let mut finger_tid: Vec<i32> = vec![-1; slots];
        let mut cur_slot: usize = 0;
        // Real-device slot in effect when the current frame opened. MT-B drivers
        // omit ABS_MT_SLOT when it is unchanged, and trigger threads move the
        // shared uinput slot pointer between our frames, so we must re-assert this
        // at flush time.
        let mut frame_open_slot: i32 = 0;
        // Buffered (type, code, value) of the current frame, minus BTN_* and SYN.
        let mut frame: Vec<(u16, u16, i32)> = Vec::with_capacity(64);

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
                    eprintln!("TRIG: forwarder read error: {}", e);
                    break;
                }
            };

            // Track current slot (sticky across frames, per MT-B protocol).
            if ev.type_ == EV_ABS && ev.code == ABS_MT_SLOT {
                let s = ev.value.max(0) as usize;
                cur_slot = s.min(finger_tid.len() - 1);
            }
            // Track finger contacts via tracking-id transitions.
            if ev.type_ == EV_ABS && ev.code == ABS_MT_TRACKING_ID {
                let prev = finger_tid[cur_slot];
                if ev.value >= 0 && prev < 0 {
                    inner.finger_count.fetch_add(1, Ordering::SeqCst);
                    finger_tid[cur_slot] = ev.value;
                } else if ev.value < 0 && prev >= 0 {
                    let before = inner.finger_count.fetch_sub(1, Ordering::SeqCst);
                    if before <= 1 {
                        inner.finger_count.store(0, Ordering::SeqCst);
                    }
                    finger_tid[cur_slot] = -1;
                } else if ev.value >= 0 {
                    finger_tid[cur_slot] = ev.value;
                }
            }

            if ev.type_ == EV_SYN && ev.code == SYN_REPORT {
                // Flush the frame atomically so trigger writes never interleave
                // inside a finger report.
                let mut uif = inner.uif.lock().unwrap();
                // Re-assert the slot the real device was on when this frame opened.
                // Without it, buffered finger axes could land on a trigger slot
                // (the slot pointer is sticky and shared), leaking a stuck contact
                // that Android renders as a hovering pointer (hollow ring).
                let _ = emit(&mut uif, EV_ABS, ABS_MT_SLOT, frame_open_slot);
                for &(ty, code, value) in &frame {
                    let _ = emit(&mut uif, ty, code, value);
                }
                // Authoritative finger count from real slot occupancy (immune to
                // counter drift), plus live trigger contacts. This guarantees
                // BTN_TOUCH is never 0 while any slot still holds a contact.
                let nf = finger_tid.iter().filter(|&&t| t >= 0).count() as i32;
                inner.finger_count.store(nf, Ordering::SeqCst);
                let count = nf + inner.virt_active.load(Ordering::SeqCst).max(0);
                let _ = set_tool_buttons(&mut uif, count);
                let _ = syn(&mut uif);
                drop(uif);
                frame.clear();
                // Next frame opens on whatever slot this frame ended on (sticky).
                frame_open_slot = cur_slot as i32;
                continue;
            }

            // Strip device-global tap buttons (managed centrally). Forward all other
            // events (axes, non-REPORT SYN such as SYN_MT_REPORT) verbatim.
            let is_btn = ev.type_ == EV_KEY
                && matches!(
                    ev.code,
                    BTN_TOUCH
                        | BTN_TOOL_FINGER
                        | BTN_TOOL_DOUBLETAP
                        | BTN_TOOL_TRIPLETAP
                        | BTN_TOOL_QUADTAP
                        | BTN_TOOL_QUINTTAP
                );
            if !is_btn {
                frame.push((ev.type_, ev.code, ev.value));
            }
        }

        // Restore native touch on exit.
        let _ = xioctl(f.as_raw_fd(), EVIOCGRAB, 0);
        inner.touch_grab_fd.store(-1, Ordering::SeqCst);
        println!("TRIG: forwarder exit, ungrabbed {}", touch_path.display());
    });
}

fn trigger_press(inner: &Inner, slot: i32, tid: i32, x: i32, y: i32) {
    let mut uif = inner.uif.lock().unwrap();
    if mt_contact_down(&mut uif, slot, tid, x, y, inner.touch_major).is_ok() {
        inner.virt_active.fetch_add(1, Ordering::SeqCst);
        let count = inner.contact_count();
        let _ = set_tool_buttons(&mut uif, count);
        let _ = syn(&mut uif);
    }
}

fn trigger_release(inner: &Inner, slot: i32) {
    let mut uif = inner.uif.lock().unwrap();
    let _ = mt_contact_up(&mut uif, slot);
    let before = inner.virt_active.fetch_sub(1, Ordering::SeqCst);
    if before <= 1 {
        inner.virt_active.store(0, Ordering::SeqCst);
    }
    let count = inner.contact_count();
    let _ = set_tool_buttons(&mut uif, count);
    let _ = syn(&mut uif);
}

fn spawn_trigger_thread(inner: Arc<Inner>, dev_path: PathBuf, key_code: u16, is_left: bool, slot: i32) {
    thread::spawn(move || {
        let mut f = match open_event_dev(&dev_path) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("TRIG: failed to open {}: {}", dev_path.display(), e);
                return;
            }
        };
        let side = if is_left { "L" } else { "R" };
        println!("TRIG: listen {} (side={} slot={})", dev_path.display(), side, slot);

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
                    trigger_release(&inner, slot);
                    println!("TRIG: {} UP(reconfig)", side);
                    pressed = false;
                }
                abs_state = None;
                key_state = None;
                abs0_seen = false;
                keyup_seen = false;
                last_gen = cur_gen;
            }

            let active = inner.active.load(Ordering::SeqCst);
            let side_enabled = if is_left {
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
                if abs_state.is_none() {
                    abs_state = Some(true);
                }
                if key_state.is_none() {
                    key_state = Some(true);
                }

                let (x, y) = if is_left {
                    (inner.left_x.load(Ordering::SeqCst), inner.left_y.load(Ordering::SeqCst))
                } else {
                    (inner.right_x.load(Ordering::SeqCst), inner.right_y.load(Ordering::SeqCst))
                };

                let tid = if is_left {
                    inner.tid_l.fetch_add(1, Ordering::SeqCst)
                } else {
                    inner.tid_r.fetch_add(1, Ordering::SeqCst)
                } as i32;
                let tid = (tid % 65000).max(1);

                trigger_press(&inner, slot, tid, x, y);
                println!("TRIG: {} DOWN tid={} raw=({}, {})", side, tid, x, y);
                continue;
            }

            if pressed && is_release_evt {
                let do_release =
                    abs0_seen && keyup_seen && abs_state == Some(false) && key_state == Some(false);
                if do_release {
                    trigger_release(&inner, slot);
                    println!("TRIG: {} UP", side);
                    pressed = false;
                    abs_state = None;
                    key_state = None;
                    abs0_seen = false;
                    keyup_seen = false;
                }
            }
        }

        if pressed {
            trigger_release(&inner, slot);
        }
    });
}
