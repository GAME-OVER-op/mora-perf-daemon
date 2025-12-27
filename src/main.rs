use std::{
    collections::HashMap,
    fs,
    io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, Instant},
};

const ICON_DST: &str = "/data/local/tmp/mora.png";
const ICON_URI: &str = "file:///data/local/tmp/mora.png";
const ICON_BYTES: &[u8] = include_bytes!("assets/mora.png");

// ===== Thermal zones (fixed) =====
// CPU avg zones:
const CPU_ZONE_IDS: &[u32] = &[
    10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 25, 26, 27, 28, 29,
];
// GPU avg zones:
const GPU_ZONE_IDS: &[u32] = &[41, 42, 43, 44, 45, 46, 47, 48];
// Battery zone:
const BAT_ZONE_ID: u32 = 74;

fn zone_temp_path(id: u32) -> PathBuf {
    PathBuf::from(format!("/sys/class/thermal/thermal_zone{}/temp", id))
}

fn read_to_string(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn read_u64(path: &Path) -> Option<u64> {
    let s = read_to_string(path)?;
    s.trim().parse::<u64>().ok()
}

fn write_num(path: &Path, val: u64) -> io::Result<()> {
    fs::write(path, format!("{}\n", val).as_bytes())
}

fn write_u64_if_needed(
    path: &Path,
    target: u64,
    cache: &mut HashMap<PathBuf, u64>,
    force_check_current: bool,
) -> io::Result<bool> {
    if !path.exists() {
        return Ok(false);
    }

    if let Some(last) = cache.get(path).copied() {
        if !force_check_current && last == target {
            return Ok(false);
        }
    }

    if force_check_current {
        if let Some(cur) = read_u64(path) {
            if cur == target {
                cache.insert(path.to_path_buf(), target);
                return Ok(false);
            }
        }
    }

    write_num(path, target)?;
    cache.insert(path.to_path_buf(), target);
    Ok(true)
}

// ===== Notification (cmd notification via shell uid=2000) =====

fn sh_escape_single_quotes(s: &str) -> String {
    s.replace('\'', r#"'\''"#)
}

fn ensure_icon_on_disk() {
    let dst = Path::new(ICON_DST);

    let need_write = match fs::metadata(dst) {
        Ok(m) => m.len() != ICON_BYTES.len() as u64,
        Err(_) => true,
    };

    if need_write {
        if let Err(e) = fs::write(dst, ICON_BYTES) {
            println!("NOTIFY: icon write fail ({})", e);
            return;
        }
        let _ = fs::set_permissions(dst, fs::Permissions::from_mode(0o644));
        println!("NOTIFY: icon ready ({})", ICON_DST);
    }
}

fn post_notification(message: &str) {
    let msg = sh_escape_single_quotes(message);

    let cmd = format!(
        "cmd notification post \
         -i {icon} -I {icon} \
         -S messaging --conversation 'MORA' --message 'M9RA: {msg}' \
         -t 'MORA' 'Tag' 'MORA' >/dev/null 2>&1",
        icon = ICON_URI,
        msg = msg
    );

    let _ = Command::new("su")
        .args(["-lp", "2000", "-c", &cmd])
        .status();
}

// ===== Temperature reading helpers (AVG) =====

fn read_temp_mc(path: &Path) -> Option<i32> {
    let s = read_to_string(path)?;
    s.trim().parse::<i32>().ok()
}

fn read_avg_temp_mc(paths: &[PathBuf]) -> Option<i32> {
    let mut sum: i64 = 0;
    let mut n: i64 = 0;
    for p in paths {
        if let Some(v) = read_temp_mc(p) {
            sum += v as i64;
            n += 1;
        }
    }
    if n == 0 {
        None
    } else {
        Some((sum / n) as i32)
    }
}

// ===== Temp limiter thresholds (+10°C) =====

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TempZone {
    Cool,
    Z100,
    Z110,
    Z120,
    Z130,
}

impl TempZone {
    fn reduction_percent(self) -> u32 {
        match self {
            TempZone::Cool => 0,
            TempZone::Z100 => 10,
            TempZone::Z110 => 15,
            TempZone::Z120 => 25,
            TempZone::Z130 => 40,
        }
    }
}

fn zone_with_hysteresis(temp_mc: i32, prev: TempZone) -> TempZone {
    let t100 = 100_000;
    let t110 = 110_000;
    let t120 = 120_000;
    let t130 = 130_000;
    let h = 2_000;

    match prev {
        TempZone::Cool => {
            if temp_mc >= t130 {
                TempZone::Z130
            } else if temp_mc >= t120 {
                TempZone::Z120
            } else if temp_mc >= t110 {
                TempZone::Z110
            } else if temp_mc >= t100 {
                TempZone::Z100
            } else {
                TempZone::Cool
            }
        }
        TempZone::Z100 => {
            if temp_mc >= t130 {
                TempZone::Z130
            } else if temp_mc >= t120 {
                TempZone::Z120
            } else if temp_mc >= t110 {
                TempZone::Z110
            } else if temp_mc < t100 - h {
                TempZone::Cool
            } else {
                TempZone::Z100
            }
        }
        TempZone::Z110 => {
            if temp_mc >= t130 {
                TempZone::Z130
            } else if temp_mc >= t120 {
                TempZone::Z120
            } else if temp_mc < t110 - h {
                TempZone::Z100
            } else {
                TempZone::Z110
            }
        }
        TempZone::Z120 => {
            if temp_mc >= t130 {
                TempZone::Z130
            } else if temp_mc < t120 - h {
                TempZone::Z110
            } else {
                TempZone::Z120
            }
        }
        TempZone::Z130 => {
            if temp_mc < t130 - h {
                TempZone::Z120
            } else {
                TempZone::Z130
            }
        }
    }
}

// ===== Screen state =====

#[derive(Debug)]
enum ScreenProbe {
    FbBlank(PathBuf),
    BacklightBright(PathBuf),
    BacklightPower(PathBuf),
}

fn detect_screen_probe() -> Option<ScreenProbe> {
    let fb_blank = PathBuf::from("/sys/class/graphics/fb0/blank");
    if fb_blank.exists() {
        return Some(ScreenProbe::FbBlank(fb_blank));
    }

    let bl_dir = Path::new("/sys/class/backlight");
    if let Ok(entries) = fs::read_dir(bl_dir) {
        for e in entries.flatten() {
            let p = e.path();
            let bright = p.join("brightness");
            if bright.exists() {
                return Some(ScreenProbe::BacklightBright(bright));
            }
            let blp = p.join("bl_power");
            if blp.exists() {
                return Some(ScreenProbe::BacklightPower(blp));
            }
        }
    }
    None
}

fn raw_screen_on(probe: &ScreenProbe) -> bool {
    match probe {
        ScreenProbe::FbBlank(p) => read_to_string(p)
            .and_then(|s| s.trim().parse::<i32>().ok())
            .map(|v| v == 0)
            .unwrap_or(true),
        ScreenProbe::BacklightBright(p) => read_to_string(p)
            .and_then(|s| s.trim().parse::<i32>().ok())
            .map(|v| v > 0)
            .unwrap_or(true),
        ScreenProbe::BacklightPower(p) => read_to_string(p)
            .and_then(|s| s.trim().parse::<i32>().ok())
            .map(|v| v == 0)
            .unwrap_or(true),
    }
}

// ===== Charging probe =====

#[derive(Debug)]
struct ChargeProbe {
    online_paths: Vec<PathBuf>,
    battery_status_path: Option<PathBuf>,
}

impl ChargeProbe {
    fn detect() -> Option<Self> {
        let base = Path::new("/sys/class/power_supply");
        let mut online_paths = Vec::new();
        let mut battery_status_path = None;

        let entries = fs::read_dir(base).ok()?;
        for e in entries.flatten() {
            let p = e.path();
            if !p.is_dir() {
                continue;
            }

            let ty = p.join("type");
            let ty_s = fs::read_to_string(&ty).unwrap_or_default().trim().to_string();
            let online = p.join("online");
            let status = p.join("status");

            if ty_s.eq_ignore_ascii_case("Battery") {
                if status.exists() && battery_status_path.is_none() {
                    battery_status_path = Some(status);
                }
                continue;
            }

            if online.exists() {
                online_paths.push(online);
            }
        }

        if online_paths.is_empty() && battery_status_path.is_none() {
            None
        } else {
            Some(Self {
                online_paths,
                battery_status_path,
            })
        }
    }

    fn is_charging(&self) -> bool {
        for p in self.online_paths.iter() {
            if read_u64(p).unwrap_or(0) == 1 {
                return true;
            }
        }
        if let Some(st) = &self.battery_status_path {
            if let Some(s) = read_to_string(st) {
                let s = s.trim();
                if s.eq_ignore_ascii_case("Charging") || s.eq_ignore_ascii_case("Full") {
                    return true;
                }
            }
        }
        false
    }
}

// ===== Fan rules =====

struct Fan {
    enable_path: PathBuf,
    level_path: PathBuf,
    level: u8, // 0..5
}

impl Fan {
    fn new() -> Option<Self> {
        let enable_path = PathBuf::from("/sys/kernel/fan/fan_enable");
        let level_path = PathBuf::from("/sys/kernel/fan/fan_speed_level");
        if enable_path.exists() && level_path.exists() {
            Some(Self {
                enable_path,
                level_path,
                level: 0,
            })
        } else {
            None
        }
    }

    // SoC (CPU/GPU) fan curve (your old "-10°C for fan" logic):
    fn level_from_soc_temp(temp_mc: i32) -> u8 {
        if temp_mc < 50_000 {
            0
        } else if temp_mc < 60_000 {
            1
        } else if temp_mc < 70_000 {
            2
        } else if temp_mc < 80_000 {
            3
        } else if temp_mc < 90_000 {
            4
        } else {
            5
        }
    }

    // Battery fan curve (charging):
    // >42 => 5; >35 => 4; >30 => 3; >25 => 2; 15..25 => 1; <15 => 0
    fn level_from_battery_temp(temp_mc: i32) -> u8 {
        if temp_mc < 15_000 {
            0
        } else if temp_mc < 25_000 {
            1
        } else if temp_mc < 30_000 {
            2
        } else if temp_mc < 35_000 {
            3
        } else if temp_mc < 42_000 {
            4
        } else {
            5
        }
    }

    fn apply(
        &mut self,
        cache: &mut HashMap<PathBuf, u64>,
        soc_temp_mc: i32,
        batt_temp_mc: Option<i32>,
        screen_on: bool,
        charging: bool,
    ) {
        let soc_level = if soc_temp_mc >= 0 {
            Self::level_from_soc_temp(soc_temp_mc)
        } else {
            0
        };

        let target = if charging {
            let batt_level = batt_temp_mc.map(Self::level_from_battery_temp).unwrap_or(0);
            // when charging: use max(soc, battery)
            soc_level.max(batt_level)
        } else if screen_on {
            soc_level
        } else {
            0
        };

        // smooth +/-1
        let next = if target > self.level {
            self.level + 1
        } else if target < self.level {
            self.level - 1
        } else {
            self.level
        };

        if next == self.level {
            return;
        }
        self.level = next;

        if self.level == 0 {
            let _ = write_u64_if_needed(&self.enable_path, 0, cache, true);
            println!("FAN: off");
            return;
        }

        let lvl = self.level as u64;

        // enable -> level -> enable (как у тебя в терминале)
        let _ = write_u64_if_needed(&self.enable_path, 1, cache, true);
        let _ = write_u64_if_needed(&self.level_path, lvl, cache, true);
        let _ = write_u64_if_needed(&self.enable_path, 1, cache, true);

        println!("FAN: {}", self.level);
    }
}

// ===== GPU util (fixed) =====

fn parse_u8_token(s: &str) -> Option<u8> {
    let tok = s.split_whitespace().next()?.trim();
    let tok = tok.trim_end_matches('%');
    tok.parse::<u8>().ok()
}

fn read_gpu_util_from_busy_percentage(path: &Path) -> Option<u8> {
    let s = read_to_string(path)?;
    let v = parse_u8_token(&s)?;
    Some(v.min(100))
}

fn read_gpu_util_from_gpubusy_ratio(path: &Path) -> Option<u8> {
    let s = read_to_string(path)?;
    let mut it = s.split_whitespace();
    let busy: u64 = it.next()?.parse().ok()?;
    let total: u64 = it.next()?.parse().ok()?;
    if total == 0 {
        return Some(0);
    }
    Some(((busy.saturating_mul(100) / total).min(100)) as u8)
}

fn read_gpu_util_any(busy_percent_path: Option<&Path>, gpubusy_path: &Path) -> u8 {
    if let Some(p) = busy_percent_path {
        if let Some(v) = read_gpu_util_from_busy_percentage(p) {
            if v > 0 {
                return v;
            }
            if let Some(v2) = read_gpu_util_from_gpubusy_ratio(gpubusy_path) {
                return v2;
            }
            return v;
        }
    }
    read_gpu_util_from_gpubusy_ratio(gpubusy_path).unwrap_or(0)
}

// ===== CPU util (/proc/stat) =====

#[derive(Clone)]
struct CpuStatSample {
    per_cpu: Vec<(u64, u64)>, // (idle_all, total)
    t: Instant,
}

fn parse_proc_stat_percpu() -> Option<Vec<(u64, u64)>> {
    let s = fs::read_to_string("/proc/stat").ok()?;
    let mut out: Vec<(u64, u64)> = Vec::new();

    for line in s.lines() {
        if !line.starts_with("cpu") {
            continue;
        }
        if line.starts_with("cpu ") {
            continue;
        }

        let mut it = line.split_whitespace();
        let cpu_label = it.next()?;
        let idx: usize = cpu_label.get(3..)?.parse().ok()?;

        let mut vals: Vec<u64> = Vec::with_capacity(10);
        for v in it {
            if let Ok(x) = v.parse::<u64>() {
                vals.push(x);
            } else {
                break;
            }
        }
        if vals.len() < 4 {
            continue;
        }

        let user = vals[0];
        let nice = vals[1];
        let system = vals[2];
        let idle = vals[3];
        let iowait = *vals.get(4).unwrap_or(&0);
        let irq = *vals.get(5).unwrap_or(&0);
        let softirq = *vals.get(6).unwrap_or(&0);
        let steal = *vals.get(7).unwrap_or(&0);

        let idle_all = idle + iowait;
        let total = user + nice + system + idle + iowait + irq + softirq + steal;

        if out.len() <= idx {
            out.resize(idx + 1, (0, 0));
        }
        out[idx] = (idle_all, total);
    }

    if out.is_empty() { None } else { Some(out) }
}

fn cpu_utils_by_core(prev: &mut Option<CpuStatSample>) -> Option<Vec<u8>> {
    let now = Instant::now();
    let cur = parse_proc_stat_percpu()?;

    let util = if let Some(p) = prev {
        let mut out = Vec::with_capacity(cur.len());
        for (i, (idle, total)) in cur.iter().enumerate() {
            let (pidle, ptotal) = p.per_cpu.get(i).copied().unwrap_or((0, 0));
            let didle = idle.saturating_sub(pidle);
            let dtotal = total.saturating_sub(ptotal);
            let u = if dtotal == 0 {
                0
            } else {
                let busy = dtotal.saturating_sub(didle);
                (busy.saturating_mul(100) / dtotal).min(100) as u8
            };
            out.push(u);
        }
        out
    } else {
        vec![0u8; cur.len()]
    };

    *prev = Some(CpuStatSample { per_cpu: cur, t: now });
    Some(util)
}

fn avg_util(utils: &[u8], cpus: &[usize]) -> u8 {
    if cpus.is_empty() {
        return 0;
    }
    let mut sum = 0u32;
    let mut n = 0u32;
    for &c in cpus {
        if let Some(&u) = utils.get(c) {
            sum += u as u32;
            n += 1;
        }
    }
    if n == 0 { 0 } else { (sum / n) as u8 }
}

// ===== Background process watcher (screen OFF) =====

fn read_total_cpu_jiffies() -> Option<u64> {
    let s = fs::read_to_string("/proc/stat").ok()?;
    for line in s.lines() {
        if line.starts_with("cpu ") {
            let mut sum = 0u64;
            for v in line.split_whitespace().skip(1) {
                if let Ok(x) = v.parse::<u64>() {
                    sum = sum.saturating_add(x);
                }
            }
            return Some(sum);
        }
    }
    None
}

fn parse_pid_stat(line: &str) -> Option<(String, u64)> {
    let l = line.find('(')?;
    let r = line.rfind(')')?;
    if r <= l { return None; }

    let comm = line[l + 1..r].to_string();
    let after = &line[r + 1..];

    let mut it = after.split_whitespace();
    let _state = it.next()?; // field 3

    let rest: Vec<&str> = it.collect();
    if rest.len() < 12 {
        return None;
    }

    let utime: u64 = rest[10].parse().ok()?;
    let stime: u64 = rest[11].parse().ok()?;
    Some((comm, utime.saturating_add(stime)))
}

struct ProcWatch {
    last_total: u64,
    last_ticks: HashMap<u32, u64>,
}

impl ProcWatch {
    fn new() -> Self {
        Self { last_total: 0, last_ticks: HashMap::new() }
    }

    fn scan_top(&mut self) -> Option<(u32, String, u8)> {
        let total = read_total_cpu_jiffies()?;
        if self.last_total == 0 {
            self.last_total = total;
            return None;
        }
        let dtotal = total.saturating_sub(self.last_total);
        self.last_total = total;
        if dtotal == 0 {
            return None;
        }

        let mut new_map: HashMap<u32, u64> = HashMap::new();
        let mut best: Option<(u32, String, u64)> = None;

        if let Ok(entries) = fs::read_dir("/proc") {
            for e in entries.flatten() {
                let name = match e.file_name().into_string() {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let pid: u32 = match name.parse() {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let stat_path = format!("/proc/{}/stat", pid);
                let line = match fs::read_to_string(&stat_path) {
                    Ok(s) => s,
                    Err(_) => continue,
                };

                let (comm, ticks) = match parse_pid_stat(&line) {
                    Some(v) => v,
                    None => continue,
                };

                let prev = self.last_ticks.get(&pid).copied().unwrap_or(ticks);
                let dt = ticks.saturating_sub(prev);

                new_map.insert(pid, ticks);

                if dt == 0 {
                    continue;
                }

                let replace = match best.as_ref() {
                    None => true,
                    Some((_bp, _bc, bdt)) => dt > *bdt,
                };
                if replace {
                    best = Some((pid, comm, dt));
                }
            }
        }

        self.last_ticks = new_map;

        let (pid, comm, dt) = best?;
        let pct = ((dt.saturating_mul(100) / dtotal).min(100)) as u8;
        Some((pid, comm, pct))
    }
}

// ===== Frequencies (hardcoded) =====

const CPU0_FREQS: &[u64] = &[
    364800, 460800, 556800, 672000, 787200, 902400, 1017600, 1132800, 1248000,
    1344000, 1459200, 1574400, 1689600, 1804800, 1920000, 2035200, 2150400, 2265600,
];
const CPU2_FREQS: &[u64] = &[
    499200, 614400, 729600, 844800, 960000, 1075200, 1190400, 1286400, 1401600,
    1497600, 1612800, 1708800, 1824000, 1920000, 2035200, 2131200, 2188800, 2246400,
    2323200, 2380800, 2438400, 2515200, 2572800, 2630400, 2707200, 2764800, 2841600,
    2899200, 2956800, 3014400, 3072000, 3148800,
];
const CPU5_FREQS: &[u64] = &[
    499200, 614400, 729600, 844800, 960000, 1075200, 1190400, 1286400, 1401600,
    1497600, 1612800, 1708800, 1824000, 1920000, 2035200, 2131200, 2188800, 2246400,
    2323200, 2380800, 2438400, 2515200, 2572800, 2630400, 2707200, 2764800, 2841600,
    2899200, 2956800,
];
const CPU7_FREQS: &[u64] = &[
    480000, 576000, 672000, 787200, 902400, 1017600, 1132800, 1248000, 1363200,
    1478400, 1593600, 1708800, 1824000, 1939200, 2035200, 2112000, 2169600, 2246400,
    2304000, 2380800, 2438400, 2496000, 2553600, 2630400, 2688000, 2745600, 2803200,
    2880000, 2937600, 2995200, 3052800, 3110400, 3187200, 3244800, 3302400,
];
const GPU_FREQS: &[u64] = &[
    231000000, 310000000, 366000000, 422000000, 500000000, 578000000, 629000000,
    680000000, 720000000, 770000000, 834000000, 903000000, 916000000,
];

fn clamp_to_table(freqs: &[u64], cap: u64) -> usize {
    let mut lo = 0usize;
    let mut hi = freqs.len();
    while lo + 1 < hi {
        let mid = (lo + hi) / 2;
        if freqs[mid] <= cap { lo = mid; } else { hi = mid; }
    }
    lo
}

fn base_index_from_ratio(freqs: &[u64], ratio: f32) -> usize {
    if freqs.is_empty() { return 0; }
    let n = freqs.len();
    let x = (ratio * (n.saturating_sub(1) as f32)).round() as i32;
    x.clamp(0, (n - 1) as i32) as usize
}

fn fmt_khz(khz: u64) -> String {
    if khz >= 1_000_000 {
        format!("{:.2}GHz", khz as f64 / 1_000_000.0)
    } else {
        format!("{:.0}MHz", khz as f64 / 1_000.0)
    }
}

fn fmt_hz(hz: u64) -> String {
    if hz >= 1_000_000_000 {
        format!("{:.2}GHz", hz as f64 / 1_000_000_000.0)
    } else {
        format!("{:.0}MHz", hz as f64 / 1_000_000.0)
    }
}

// ===== Domain control =====

struct Domain {
    label: &'static str,
    freqs: &'static [u64],
    min_freq: u64,
    max_freq: u64,
    min_path: PathBuf,
    max_path: PathBuf,

    base_index: usize,
    idx: usize,

    last_util: u8,
    max_step_up_next_apply: usize,
    hold_until: Instant,
    low_accum: Duration,

    up_util: u8,
    spike_delta2: u8,
    spike_delta4: u8,
    high_jump2: u8,
    high_jump4: u8,

    down_util1: u8,
    down_util2: u8,
    down_after1: Duration,
    down_after2: Duration,

    last_applied_idx: usize,
    last_applied_freq: u64,
    is_gpu: bool,
}

impl Domain {
    fn desired_step_update(&mut self, util: u8, now: Instant, dt: Duration) -> bool {
        let old_idx = self.idx;

        let delta = if util > self.last_util { util - self.last_util } else { 0 };
        self.last_util = util;

        let mut jump_up: usize = 0;
        if util >= self.high_jump4 || delta >= self.spike_delta4 {
            jump_up = 4;
        } else if util >= self.high_jump2 || delta >= self.spike_delta2 {
            jump_up = 2;
        } else if util >= self.up_util {
            jump_up = 1;
        }

        if jump_up > 0 && self.idx + 1 < self.freqs.len() {
            let new_idx = (self.idx + jump_up).min(self.freqs.len() - 1);
            if new_idx != self.idx {
                self.idx = new_idx;
                self.max_step_up_next_apply = jump_up;
                self.hold_until = now + Duration::from_millis(800);
            }
            self.low_accum = Duration::ZERO;
        } else {
            self.max_step_up_next_apply = 1;
        }

        if now >= self.hold_until {
            if util <= self.down_util2 {
                self.low_accum += dt;
                if self.low_accum >= self.down_after2 {
                    self.low_accum = Duration::ZERO;
                    if self.idx > self.base_index {
                        self.idx -= 1;
                    }
                }
            } else if util <= self.down_util1 {
                self.low_accum += dt;
                if self.low_accum >= self.down_after1 {
                    self.low_accum = Duration::ZERO;
                    if self.idx > self.base_index {
                        self.idx -= 1;
                    }
                }
            } else {
                self.low_accum = Duration::ZERO;
            }
        }

        self.idx != old_idx
    }

    fn max_step_down_for_zone(zone: TempZone) -> usize {
        match zone {
            TempZone::Z130 => 3,
            TempZone::Z120 => 2,
            _ => 1,
        }
    }

    fn apply(&mut self, zone: TempZone, cache: &mut HashMap<PathBuf, u64>, force_check: bool) -> io::Result<bool> {
        let reduction = zone.reduction_percent();
        let thermal_cap = if reduction == 0 {
            self.max_freq
        } else {
            let keep = 100u64 - reduction as u64;
            (self.max_freq.saturating_mul(keep)) / 100u64
        };

        let desired_freq = self.freqs[self.idx];
        let cap = desired_freq.min(thermal_cap);
        let computed_idx = clamp_to_table(self.freqs, cap);
        let mut target_idx = computed_idx;

        let up_limit = self.max_step_up_next_apply.max(1);
        if target_idx > self.last_applied_idx + up_limit {
            target_idx = self.last_applied_idx + up_limit;
        }
        self.max_step_up_next_apply = 1;

        if target_idx + 1 <= self.last_applied_idx {
            let max_down = Self::max_step_down_for_zone(zone);
            let min_allowed = self.last_applied_idx.saturating_sub(max_down);
            if target_idx < min_allowed {
                target_idx = min_allowed;
            }
        }

        let target_freq = self.freqs[target_idx];

        let _ = write_u64_if_needed(&self.min_path, self.min_freq, cache, force_check)?;
        let wrote_max = write_u64_if_needed(&self.max_path, target_freq, cache, force_check)?;

        if target_idx != self.last_applied_idx || target_freq != self.last_applied_freq || (force_check && wrote_max) {
            self.last_applied_idx = target_idx;
            self.last_applied_freq = target_freq;

            if self.is_gpu {
                println!("{}: cap {}", self.label, fmt_hz(target_freq));
            } else {
                println!("{}: cap {}", self.label, fmt_khz(target_freq));
            }
        }

        Ok(wrote_max)
    }
}

// ===== Your function (unchanged) =====

fn disable_thermal_services() {
    println!("Отключаем сервисы управления температурой...");

    let stop_services = vec![
        "android.thermal-hal",
        "vendor.thermal-engine",
        "vendor.thermal_manager",
        "vendor.thermal-manager",
        "vendor.thermal-hal-2-0",
        "vendor.thermal-symlinks",
        "thermal_mnt_hal_service",
        "thermal",
        "mi_thermald",
        "thermald",
        "thermalloadalgod",
        "thermalservice",
        "sec-thermal-1-0",
        "debug_pid.sec-thermal-1-0",
        "thermal-engine",
        "vendor.thermal-hal-1-0",
        "vendor-thermal-1-0",
        "thermal-hal",
        "vendor.qti.hardware.perf2-hal-service",
        "qti-msdaemon_vendor-0",
        "qti-msdaemon_vendor-1",
        "qti-ssdaemon_vendor",
    ];

    for service in stop_services {
        let output = Command::new("stop").arg(service).output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    println!("Остановлен сервис: {}", service);
                } else {
                    println!("Не удалось остановить {}: код {}", service, output.status);
                }
            }
            Err(e) => {
                println!("Ошибка при остановке {}: {}", service, e);
            }
        }

        thread::sleep(Duration::from_millis(50));
    }

    let setprop_commands = vec![
        ("init.svc.thermal", "stopped"),
        ("init.svc.thermal-managers", "stopped"),
        ("init.svc.thermal_manager", "stopped"),
        ("init.svc.thermal_mnt_hal_service", "stopped"),
        ("init.svc.thermal-engine", "stopped"),
        ("init.svc.mi-thermald", "stopped"),
        ("init.svc.thermalloadalgod", "stopped"),
        ("init.svc.thermalservice", "stopped"),
        ("init.svc.thermal-hal", "stopped"),
        ("init.svc.vendor.thermal-symlinks", ""),
        ("init.svc.android.thermal-hal", "stopped"),
        ("init.svc.vendor.thermal-hal", "stopped"),
        ("init.svc.thermal-manager", "stopped"),
        ("init.svc.vendor-thermal-hal-1-0", "stopped"),
        ("init.svc.vendor.thermal-hal-1-0", "stopped"),
        ("init.svc.vendor.thermal-hal-2-0.mtk", "stopped"),
        ("init.svc.vendor.thermal-hal-2-0", "stopped"),
    ];

    for (prop, value) in setprop_commands {
        let output = Command::new("setprop").arg(prop).arg(value).output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    println!("Установлено свойство: {}={}", prop, value);
                } else {
                    println!("Не удалось установить {}={}: код {}", prop, value, output.status);
                }
            }
            Err(e) => {
                println!("Ошибка при установке {}={}: {}", prop, value, e);
            }
        }

        thread::sleep(Duration::from_millis(50));
    }

    println!("Завершена отключение сервисов управления температурой");
}

fn main() {
    println!("perf_daemon starting");

    disable_thermal_services();
    ensure_icon_on_disk();

    // Build thermal paths (only existing ones)
    let mut cpu_paths: Vec<PathBuf> = Vec::new();
    for &id in CPU_ZONE_IDS {
        let p = zone_temp_path(id);
        if p.exists() {
            cpu_paths.push(p);
        }
    }

    let mut gpu_paths: Vec<PathBuf> = Vec::new();
    for &id in GPU_ZONE_IDS {
        let p = zone_temp_path(id);
        if p.exists() {
            gpu_paths.push(p);
        }
    }

    let bat_path = {
        let p = zone_temp_path(BAT_ZONE_ID);
        if p.exists() { Some(p) } else { None }
    };

    println!(
        "THERM: CPU avg zones {} | GPU avg zones {} | BAT {}",
        cpu_paths.len(),
        gpu_paths.len(),
        if bat_path.is_some() { "ok" } else { "missing" }
    );

    let screen_probe = detect_screen_probe();
    if let Some(p) = &screen_probe {
        println!("SCREEN: {:?}", p);
    } else {
        println!("SCREEN: probe not found (assume ON)");
    }

    let charge_probe = ChargeProbe::detect();
    if charge_probe.is_some() {
        println!("CHG: ok");
    } else {
        println!("CHG: probe not found (assume OFF)");
    }

    let mut fan = Fan::new();
    if fan.is_some() {
        let en = read_u64(Path::new("/sys/kernel/fan/fan_enable")).unwrap_or(0);
        let lv = read_u64(Path::new("/sys/kernel/fan/fan_speed_level")).unwrap_or(0);
        println!("FAN: sysfs ok (en={} lvl={})", en, lv);
    } else {
        println!("FAN: sysfs not found (skip)");
    }

    let gpu_busy_percent_path = {
        let p = PathBuf::from("/sys/class/kgsl/kgsl-3d0/gpu_busy_percentage");
        if p.exists() { println!("GPUUTIL: gpu_busy_percentage"); Some(p) }
        else { println!("GPUUTIL: gpubusy"); None }
    };

    let base0 = base_index_from_ratio(CPU0_FREQS, 0.62);
    let base2 = base_index_from_ratio(CPU2_FREQS, 0.48);
    let base5 = base_index_from_ratio(CPU5_FREQS, 0.48);
    let base7 = base_index_from_ratio(CPU7_FREQS, 0.35);
    let baseg = base_index_from_ratio(GPU_FREQS, 0.50);

    let now = Instant::now();

    let mut cpu0 = Domain {
        label: "CPU0",
        freqs: CPU0_FREQS,
        min_freq: CPU0_FREQS[0],
        max_freq: *CPU0_FREQS.last().unwrap(),
        min_path: "/sys/devices/system/cpu/cpufreq/policy0/scaling_min_freq".into(),
        max_path: "/sys/devices/system/cpu/cpufreq/policy0/scaling_max_freq".into(),
        base_index: base0,
        idx: base0,
        last_util: 0,
        max_step_up_next_apply: 1,
        hold_until: now,
        low_accum: Duration::ZERO,
        up_util: 70,
        spike_delta2: 20,
        spike_delta4: 35,
        high_jump2: 85,
        high_jump4: 95,
        down_util1: 60,
        down_util2: 50,
        down_after1: Duration::from_secs(6),
        down_after2: Duration::from_secs(3),
        last_applied_idx: base0,
        last_applied_freq: CPU0_FREQS[base0],
        is_gpu: false,
    };

    let mut cpu2 = Domain {
        label: "CPU2",
        freqs: CPU2_FREQS,
        min_freq: CPU2_FREQS[0],
        max_freq: *CPU2_FREQS.last().unwrap(),
        min_path: "/sys/devices/system/cpu/cpufreq/policy2/scaling_min_freq".into(),
        max_path: "/sys/devices/system/cpu/cpufreq/policy2/scaling_max_freq".into(),
        base_index: base2,
        idx: base2,
        last_util: 0,
        max_step_up_next_apply: 1,
        hold_until: now,
        low_accum: Duration::ZERO,
        up_util: 70,
        spike_delta2: 20,
        spike_delta4: 35,
        high_jump2: 85,
        high_jump4: 95,
        down_util1: 60,
        down_util2: 50,
        down_after1: Duration::from_secs(6),
        down_after2: Duration::from_secs(3),
        last_applied_idx: base2,
        last_applied_freq: CPU2_FREQS[base2],
        is_gpu: false,
    };

    let mut cpu5 = Domain {
        label: "CPU5",
        freqs: CPU5_FREQS,
        min_freq: CPU5_FREQS[0],
        max_freq: *CPU5_FREQS.last().unwrap(),
        min_path: "/sys/devices/system/cpu/cpufreq/policy5/scaling_min_freq".into(),
        max_path: "/sys/devices/system/cpu/cpufreq/policy5/scaling_max_freq".into(),
        base_index: base5,
        idx: base5,
        last_util: 0,
        max_step_up_next_apply: 1,
        hold_until: now,
        low_accum: Duration::ZERO,
        up_util: 70,
        spike_delta2: 20,
        spike_delta4: 35,
        high_jump2: 85,
        high_jump4: 95,
        down_util1: 60,
        down_util2: 50,
        down_after1: Duration::from_secs(6),
        down_after2: Duration::from_secs(3),
        last_applied_idx: base5,
        last_applied_freq: CPU5_FREQS[base5],
        is_gpu: false,
    };

    let mut cpu7 = Domain {
        label: "CPU7",
        freqs: CPU7_FREQS,
        min_freq: CPU7_FREQS[0],
        max_freq: *CPU7_FREQS.last().unwrap(),
        min_path: "/sys/devices/system/cpu/cpufreq/policy7/scaling_min_freq".into(),
        max_path: "/sys/devices/system/cpu/cpufreq/policy7/scaling_max_freq".into(),
        base_index: base7,
        idx: base7,
        last_util: 0,
        max_step_up_next_apply: 1,
        hold_until: now,
        low_accum: Duration::ZERO,
        up_util: 70,
        spike_delta2: 20,
        spike_delta4: 35,
        high_jump2: 85,
        high_jump4: 95,
        down_util1: 60,
        down_util2: 50,
        down_after1: Duration::from_secs(7),
        down_after2: Duration::from_secs(4),
        last_applied_idx: base7,
        last_applied_freq: CPU7_FREQS[base7],
        is_gpu: false,
    };

    let mut gpu = Domain {
        label: "GPU",
        freqs: GPU_FREQS,
        min_freq: GPU_FREQS[0],
        max_freq: *GPU_FREQS.last().unwrap(),
        min_path: "/sys/class/kgsl/kgsl-3d0/devfreq/min_freq".into(),
        max_path: "/sys/class/kgsl/kgsl-3d0/devfreq/max_freq".into(),
        base_index: baseg,
        idx: baseg,
        last_util: 0,
        max_step_up_next_apply: 1,
        hold_until: now,
        low_accum: Duration::ZERO,
        up_util: 70,
        spike_delta2: 20,
        spike_delta4: 35,
        high_jump2: 85,
        high_jump4: 95,
        down_util1: 60,
        down_util2: 50,
        down_after1: Duration::from_secs(5),
        down_after2: Duration::from_secs(3),
        last_applied_idx: baseg,
        last_applied_freq: GPU_FREQS[baseg],
        is_gpu: true,
    };

    println!(
        "BASE: CPU0 {} | CPU2 {} | CPU5 {} | CPU7 {} | GPU {}",
        fmt_khz(cpu0.freqs[cpu0.base_index]),
        fmt_khz(cpu2.freqs[cpu2.base_index]),
        fmt_khz(cpu5.freqs[cpu5.base_index]),
        fmt_khz(cpu7.freqs[cpu7.base_index]),
        fmt_hz(gpu.freqs[gpu.base_index]),
    );

    let cluster0 = [0usize, 1usize];
    let cluster2 = [2usize, 3usize, 4usize];
    let cluster5 = [5usize, 6usize];
    let cluster7 = [7usize];

    let gpubusy_path = Path::new("/sys/class/kgsl/kgsl-3d0/gpubusy");

    let mut cache: HashMap<PathBuf, u64> = HashMap::new();
    let mut prev_cpu: Option<CpuStatSample> = None;

    let mut last_zone = TempZone::Cool;

    let mut last_enforce = Instant::now();
    let enforce_every_active = Duration::from_secs(6);
    let enforce_every_idle = Duration::from_secs(18);

    let mut off_streak: u8 = 0;
    let mut screen_on_state = true;
    let mut screen_off_since: Option<Instant> = None;

    let mut charging = false;
    let mut last_chg_check = Instant::now();
    let chg_check_every = Duration::from_secs(2);

    let mut idle_mode = false;
    let mut idle_accum = Duration::ZERO;

    let mut proc_watch = ProcWatch::new();
    let mut last_proc_check = Instant::now();
    let proc_check_active = Duration::from_secs(3);
    let proc_check_idle = Duration::from_secs(6);
    let bg_threshold_pct: u8 = 15;

    let mut suspicious: HashMap<String, u8> = HashMap::new();
    let long_off_threshold = Duration::from_secs(30);

    let mut last_loop = Instant::now();
    let mut stable_for = Duration::ZERO;

    loop {
        let now = Instant::now();
        let dt = now.duration_since(last_loop);
        last_loop = now;

        // charging
        if now.duration_since(last_chg_check) >= chg_check_every {
            let new_chg = charge_probe.as_ref().map(|p| p.is_charging()).unwrap_or(false);
            if new_chg != charging {
                charging = new_chg;
                println!("CHG: {}", if charging { "ON" } else { "OFF" });

                if charging {
                    post_notification("Charger connected: charging fan policy enabled");
                } else {
                    post_notification("Charger disconnected: normal fan policy enabled");
                }
            }
            last_chg_check = now;
        }

        // screen
        let screen_on = if let Some(p) = &screen_probe {
            let on = raw_screen_on(p);
            if on {
                off_streak = 0;
                true
            } else {
                off_streak = off_streak.saturating_add(1);
                off_streak < 2
            }
        } else {
            true
        };

        // screen transition -> suspicious notify
        if screen_on != screen_on_state {
            screen_on_state = screen_on;
            if !screen_on {
                screen_off_since = Some(now);
                suspicious.clear();
            } else {
                if let Some(since) = screen_off_since.take() {
                    let off_dur = now.duration_since(since);
                    if off_dur >= long_off_threshold && !suspicious.is_empty() {
                        let mut v: Vec<(String, u8)> =
                            suspicious.iter().map(|(k, &p)| (k.clone(), p)).collect();
                        v.sort_by(|a, b| b.1.cmp(&a.1));
                        v.truncate(3);

                        let mut msg = String::from("Suspicious background processes: ");
                        for (i, (name, pct)) in v.iter().enumerate() {
                            if i > 0 { msg.push_str(", "); }
                            msg.push_str(&format!("{} {}%", name, pct));
                        }
                        post_notification(&msg);
                    }
                }
            }
        }

        // temps (AVG) and choose higher for control
        let cpu_avg_mc = read_avg_temp_mc(&cpu_paths).unwrap_or(-1);
        let gpu_avg_mc = read_avg_temp_mc(&gpu_paths).unwrap_or(-1);

        let soc_temp_mc = match (cpu_avg_mc >= 0, gpu_avg_mc >= 0) {
            (true, true) => cpu_avg_mc.max(gpu_avg_mc),
            (true, false) => cpu_avg_mc,
            (false, true) => gpu_avg_mc,
            (false, false) => -1,
        };

        let batt_temp_mc = bat_path.as_ref().and_then(|p| read_temp_mc(p));

        let zone = if soc_temp_mc >= 0 {
            zone_with_hysteresis(soc_temp_mc, last_zone)
        } else {
            last_zone
        };

        if zone != last_zone {
            println!(
                "TEMP: cpu {:.1}C | gpu {:.1}C | use {:.1}C -> {:?} (reduce {}%)",
                cpu_avg_mc as f32 / 1000.0,
                gpu_avg_mc as f32 / 1000.0,
                soc_temp_mc as f32 / 1000.0,
                zone,
                zone.reduction_percent()
            );
            last_zone = zone;
        }

        // GPU util
        let ug = read_gpu_util_any(gpu_busy_percent_path.as_deref(), gpubusy_path);

        // CPU util
        let cpu_utils = cpu_utils_by_core(&mut prev_cpu).unwrap_or_default();
        let u0 = avg_util(&cpu_utils, &cluster0);
        let u2 = avg_util(&cpu_utils, &cluster2);
        let u5 = avg_util(&cpu_utils, &cluster5);
        let u7 = avg_util(&cpu_utils, &cluster7);
        let max_cpu_cluster = u0.max(u2).max(u5).max(u7);

        // bg scan (screen OFF)
        let mut bg_over = false;
        if !screen_on {
            let every = if idle_mode { proc_check_idle } else { proc_check_active };
            if now.duration_since(last_proc_check) >= every {
                if let Some((_pid, comm, pct)) = proc_watch.scan_top() {
                    if pct >= bg_threshold_pct {
                        bg_over = true;
                        let entry = suspicious.entry(comm).or_insert(0);
                        *entry = (*entry).max(pct);
                    }
                }
                last_proc_check = now;
            }
        }

        // fan (updated)
        if let Some(f) = fan.as_mut() {
            f.apply(&mut cache, soc_temp_mc, batt_temp_mc, screen_on, charging);
        }

        // idle mode
        let idle_cond = !screen_on && !bg_over && max_cpu_cluster < 15 && ug < 10;

        if idle_cond {
            idle_accum += dt;
        } else {
            idle_accum = Duration::ZERO;
        }

        if !idle_mode && idle_accum >= Duration::from_secs(10) {
            idle_mode = true;
            println!("IDLE: enter");

            if !charging {
                cpu0.idx = cpu0.base_index;
                cpu2.idx = cpu2.base_index;
                cpu5.idx = cpu5.base_index;
                cpu7.idx = cpu7.base_index;
                gpu.idx = gpu.base_index;
            }
        }

        if idle_mode && !idle_cond {
            idle_mode = false;
            println!("IDLE: exit");
        }

        // enforce
        let enforce_every = if idle_mode { enforce_every_idle } else { enforce_every_active };
        let force_check = now.duration_since(last_enforce) >= enforce_every;
        if force_check {
            last_enforce = now;
        }

        // desired idx update
        let mut any_step = false;
        any_step |= cpu0.desired_step_update(u0, now, dt);
        any_step |= cpu2.desired_step_update(u2, now, dt);
        any_step |= cpu5.desired_step_update(u5, now, dt);
        any_step |= cpu7.desired_step_update(u7, now, dt);
        any_step |= gpu.desired_step_update(ug, now, dt);

        // apply caps (zone based on higher of cpu/gpu avg)
        let mut any_write = false;
        any_write |= cpu0.apply(zone, &mut cache, force_check).unwrap_or(false);
        any_write |= cpu2.apply(zone, &mut cache, force_check).unwrap_or(false);
        any_write |= cpu5.apply(zone, &mut cache, force_check).unwrap_or(false);
        any_write |= cpu7.apply(zone, &mut cache, force_check).unwrap_or(false);
        any_write |= gpu.apply(zone, &mut cache, force_check).unwrap_or(false);

        // STAT
        if force_check {
            let batt_str = batt_temp_mc
                .map(|v| format!("{:.1}C", v as f32 / 1000.0))
                .unwrap_or_else(|| "?".to_string());

            if soc_temp_mc >= 0 {
                println!(
                    "STAT: cpu {:.1}C | gpu {:.1}C | use {:.1}C | bat {} | CPU[{} {} {} {}]% | GPU {}% | scr {} | chg {}{}",
                    cpu_avg_mc as f32 / 1000.0,
                    gpu_avg_mc as f32 / 1000.0,
                    soc_temp_mc as f32 / 1000.0,
                    batt_str,
                    u0, u2, u5, u7,
                    ug,
                    if screen_on { "ON" } else { "OFF" },
                    if charging { "ON" } else { "OFF" },
                    if idle_mode { " | idle" } else { "" },
                );
            } else {
                println!(
                    "STAT: temp ? | bat {} | CPU[{} {} {} {}]% | GPU {}% | scr {} | chg {}{}",
                    batt_str,
                    u0, u2, u5, u7,
                    ug,
                    if screen_on { "ON" } else { "OFF" },
                    if charging { "ON" } else { "OFF" },
                    if idle_mode { " | idle" } else { "" },
                );
            }
        }

        // sleep
        if any_write || any_step {
            stable_for = Duration::ZERO;
        } else {
            stable_for += dt;
        }

        let sleep_ms = match zone {
            TempZone::Z120 | TempZone::Z130 => 450,
            _ => {
                if idle_mode {
                    4500
                } else if any_write || any_step {
                    700
                } else if stable_for >= Duration::from_secs(20) {
                    2000
                } else {
                    1200
                }
            }
        };

        thread::sleep(Duration::from_millis(sleep_ms));
    }
}
