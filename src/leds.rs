use crate::user_config::{
    ExternalLedColor, ExternalLedMode, ExternalLedSetting, FanLedColor, FanLedMode, FanLedSetting,
};
use std::{
    fs,
    io,
    path::Path,
    thread,
    sync::Mutex,
    time::Duration,
};

const BASE: &str = "/sys/class/leds/aw22xxx_led";

fn p(rel: &str) -> String {
    format!("{}/{}", BASE, rel)
}

fn write_str(path: &str, val: &str) -> io::Result<()> {
    fs::write(path, format!("{}\n", val).as_bytes())
}

fn read_str(path: &str) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn write_u64(path: &str, val: u64) -> io::Result<()> {
    write_str(path, &val.to_string())
}

/// OUT init: only ensure HW enable (hwen=1) if present.
///
/// NOTE: We intentionally do **not** touch brightness here anymore.
/// On some systems/services the brightness node may be managed elsewhere, and
/// writing it during early boot can lead to inconsistent behavior.
fn ensure_external_init() {
    let hwen = p("hwen");
    if Path::new(&hwen).exists() {
        let _ = write_u64(&hwen, 1);
    }
}

fn write_effect_and_cfg(effect: &str) -> io::Result<()> {
    let eff = p("effect");
    let cfg = p("cfg");
    write_str(&eff, effect)?;
    write_u64(&cfg, 1)?;
    Ok(())
}

// ---------- OUT (external) encoding (from full_rgb.sh) ----------

/// External modes that actually work on this firmware: steady/breathe/flashing.
/// Any other mode is normalized to steady.
fn external_mode_base(m: ExternalLedMode) -> u64 {
    match m {
        ExternalLedMode::Static => 96,  // 0x60
        ExternalLedMode::Breath => 112, // 0x70
        ExternalLedMode::Blink => 128,  // 0x80
        _ => 96, // normalize unsupported -> steady
    }
}

fn external_color_idx(mode: ExternalLedMode, c: ExternalLedColor) -> u64 {
    // IMPORTANT: on this firmware the "external" palette (idx 0..7) depends on the *mode*.
    // Derived from your probe log (Jan 2, 2026):
    //
    // STEADY (base 0x60):
    //   0=red, 1=violet, 2=white, 3=green, 4=blue, 5=cyan, 6=violet(alt), 7=yellow
    // BREATHE (base 0x70):
    //   0=violet, 1=red, 2=white, 3=green, 4=blue, 5=cyan, 6=violet(alt), 7=yellow
    // FLASHING (base 0x80):
    //   0=orange, 1=pink, 2=white, 3=green, 4=blue, 5=cyan, 6=violet, 7=yellow
    //
    // We keep config compatibility (snake_case color names), but map to the
    // closest available slot for the selected mode. Unsupported colors fall back.
    let m = match mode {
        ExternalLedMode::Static | ExternalLedMode::Breath | ExternalLedMode::Blink => mode,
        _ => ExternalLedMode::Static, // normalize unsupported -> steady
    };

    match m {
        ExternalLedMode::Static => match c {
            ExternalLedColor::Multi | ExternalLedColor::Red => 0,
            ExternalLedColor::Purple => 1,
            ExternalLedColor::White => 2,
            ExternalLedColor::Green => 3,
            ExternalLedColor::Blue => 4,
            ExternalLedColor::Cyan => 5,
            ExternalLedColor::Pink => 6,
            ExternalLedColor::Yellow => 7,
            ExternalLedColor::Orange => 0, // unsupported in steady -> fallback
        },
        ExternalLedMode::Breath => match c {
            ExternalLedColor::Multi | ExternalLedColor::Purple => 0,
            ExternalLedColor::Red => 1,
            ExternalLedColor::White => 2,
            ExternalLedColor::Green => 3,
            ExternalLedColor::Blue => 4,
            ExternalLedColor::Cyan => 5,
            ExternalLedColor::Pink => 6,
            ExternalLedColor::Yellow => 7,
            ExternalLedColor::Orange => 1, // unsupported in breathe -> closest (red slot)
        },
        ExternalLedMode::Blink => match c {
            ExternalLedColor::Multi | ExternalLedColor::Orange => 0,
            ExternalLedColor::Pink => 1,
            ExternalLedColor::White => 2,
            ExternalLedColor::Green => 3,
            ExternalLedColor::Blue => 4,
            ExternalLedColor::Cyan => 5,
            ExternalLedColor::Purple => 6,
            ExternalLedColor::Yellow => 7,
            ExternalLedColor::Red => 1, // unsupported in flashing -> closest (pink slot)
        },
        _ => 0, // unreachable due to normalization above
    }
}

fn external_code(setting: &ExternalLedSetting) -> String {
    let v = external_mode_base(setting.mode) + external_color_idx(setting.mode, setting.color);
    // IMPORTANT: on this firmware the driver reliably accepts *plain* hex without 0x prefix
    // (matches your working command: printf "%02x" ... ; echo "$code" > effect)
    format!("{:02x}", v)
}

// ---------- FAN encoding (from full_rgb.sh) ----------

fn fan_color_nibble(c: FanLedColor) -> u8 {
    match c {
        // Corrected according to probe log:
        //   code a1 looked purple, code a6 looked pink/rose.
        // Swap rose<->purple compared to the APK's nominal naming.
        FanLedColor::Rose => 0x6,
        FanLedColor::Yellow => 0x2,
        FanLedColor::Green => 0x3,
        FanLedColor::Blue => 0x4,
        FanLedColor::Cyan => 0x5,
        FanLedColor::Purple => 0x1,
        FanLedColor::Orange => 0x7,
        FanLedColor::Mixed1 => 0x8,
        FanLedColor::Mixed2 => 0x9,
        FanLedColor::Mixed3 => 0xA,
        FanLedColor::Mixed4 => 0xB,
        FanLedColor::Mixed5 => 0xC,
        FanLedColor::Mixed6 => 0xD,
        FanLedColor::Mixed7 => 0xF, // mixed_7
    }
}

fn fan_base(m: FanLedMode, c: FanLedColor) -> u8 {
    match m {
        FanLedMode::Off => 0x00, // not used (off handled by code=2)
        FanLedMode::Blink => 0x20,
        FanLedMode::Breath => 0x30,
        FanLedMode::Flow => 0x40,
        FanLedMode::Static => {
            // steady: mixed_* uses 0x90, solid colors use 0xA0
            if matches!(
                c,
                FanLedColor::Mixed1
                    | FanLedColor::Mixed2
                    | FanLedColor::Mixed3
                    | FanLedColor::Mixed4
                    | FanLedColor::Mixed5
                    | FanLedColor::Mixed6
                    | FanLedColor::Mixed7
            ) {
                0x90
            } else {
                0xA0
            }
        }
    }
}

fn fan_code(setting: &FanLedSetting) -> String {
    // full_rgb.sh uses:
    // - OFF => echo 2 > effect
    // - ON  => printf "%x" (no 0x)
    if setting.mode == FanLedMode::Off {
        return "2".to_string();
    }
    let base = fan_base(setting.mode, setting.color);
    let n = fan_color_nibble(setting.color);
    let v = (base as u16) + (n as u16);
    format!("{:x}", v)
}

fn fan_enabled(setting: &Option<FanLedSetting>) -> bool {
    matches!(setting, Some(s) if s.mode != FanLedMode::Off)
}

fn external_enabled(setting: &Option<ExternalLedSetting>) -> bool {
    setting.is_some()
}

/// Apply LED state using *composition* (smart order), because FAN and OUT share the same sysfs node.
///
/// Rules (mirrors the working shell script):
/// - If FAN is enabled: apply OUT (on/off) first, then apply FAN last.
/// - If FAN is disabled: apply FAN OFF first, then apply OUT (on/off) last.
fn apply_composed(fan: &Option<FanLedSetting>, ext: &Option<ExternalLedSetting>) -> io::Result<()> {
    let fan_on = fan_enabled(fan);
    let ext_on = external_enabled(ext);

    // Build strings for debugging / predictable writes.
    let fan_code_s = if fan_on {
        fan_code(fan.as_ref().unwrap())
    } else {
        "2".to_string()
    };
    let ext_code_s = if ext_on { external_code(ext.as_ref().unwrap()) } else { "00".to_string() };

    if fan_on {
        // OUT first
        write_effect_and_cfg(&ext_code_s)?;
        // Some firmwares need a small delay between sequential writes, otherwise the
        // second write can cancel/brick the first one (as you observed). 0.8s is what
        // you requested.
        thread::sleep(Duration::from_millis(800));
        // FAN last
        write_effect_and_cfg(&fan_code_s)?;
    } else {
        // FAN off first
        write_effect_and_cfg(&fan_code_s)?;
        thread::sleep(Duration::from_millis(800));
        // OUT last
        write_effect_and_cfg(&ext_code_s)?;
    }

    // One-line log for troubleshooting.
    println!(
        "LED: fan_on={} ext_on={} fan_code={} ext_code={} (order: {})",
        fan_on,
        ext_on,
        fan_code_s,
        ext_code_s,
        if fan_on { "ext->fan" } else { "fan_off->ext" }
    );

    Ok(())
}

/// Thread-safe LED controller.
///
/// IMPORTANT: On this firmware FAN and OUT share the same `effect` node, and writes can
/// override each other depending on order. We therefore apply state using *composition*
/// (smart order) instead of assuming the driver keeps both states independently.
#[derive(Debug, Default)]
pub struct Leds {
    inner: Mutex<LedsInner>,
}

#[derive(Debug, Default)]
struct LedsInner {
    // base (profile-selected) state
    base_fan_desired: Option<FanLedSetting>,
    base_external_desired: Option<ExternalLedSetting>,

    // last applied snapshots (what we actually wrote)
    last_fan_applied: Option<FanLedSetting>,
    last_external_applied: Option<ExternalLedSetting>,

    // notification override for OUT (when Some, overrides base_external_desired)
    external_override: Option<ExternalLedSetting>,
}

impl Leds {
    pub fn new() -> Self {
        ensure_external_init();
        Self::default()
    }

    fn snapshot_to_apply(g: &LedsInner) -> (Option<FanLedSetting>, Option<ExternalLedSetting>) {
        let fan = g.base_fan_desired.clone();
        let ext = g
            .external_override
            .clone()
            .or_else(|| g.base_external_desired.clone());
        (fan, ext)
    }

    fn apply_and_update(&self) {
        let (fan, ext) = {
            let g = self.inner.lock().unwrap();
            Self::snapshot_to_apply(&g)
        };

        let res = apply_composed(&fan, &ext);
        if let Err(e) = &res {
            eprintln!("LED: apply error: {}", e);
        }

        let ok = res.is_ok();

        // Update snapshots (best-effort, even if apply failed we still record intent).
        let mut g = self.inner.lock().unwrap();
        if ok {
            g.last_fan_applied = if fan_enabled(&fan) { fan.clone() } else { None };
            g.last_external_applied = ext.clone();
        } else {
            // keep last_* as-is on error
        }
    }

    /// Set base FAN desired (independent from OUT). Applied immediately.
    pub fn set_fan_desired(&self, setting: Option<FanLedSetting>) {
        {
            let mut g = self.inner.lock().unwrap();
            if g.base_fan_desired == setting {
                return;
            }
            g.base_fan_desired = setting;
        }

        // Apply composed state so turning FAN off doesn't kill OUT, and vice-versa.
        self.apply_and_update();
    }

    /// Set base OUT desired. Applied immediately unless notification override is active.
    pub fn set_external_desired(&self, setting: Option<ExternalLedSetting>) {
        let override_active = {
            let mut g = self.inner.lock().unwrap();
            if g.base_external_desired == setting {
                return;
            }
            g.base_external_desired = setting;
            g.external_override.is_some()
        };

        if override_active {
            // Notification override owns OUT right now. Remember desired only.
            return;
        }

        self.apply_and_update();
    }

    pub fn get_fan_state(&self) -> (Option<FanLedSetting>, Option<FanLedSetting>) {
        let g = self.inner.lock().unwrap();
        (g.base_fan_desired.clone(), g.last_fan_applied.clone())
    }

    pub fn get_external_state(&self) -> (Option<ExternalLedSetting>, Option<ExternalLedSetting>) {
        let g = self.inner.lock().unwrap();
        (g.base_external_desired.clone(), g.last_external_applied.clone())
    }

    /// Start OUT notification override. Always re-applies FAN if enabled.
    pub fn external_start(&self, setting: ExternalLedSetting) -> io::Result<()> {
        {
            let mut g = self.inner.lock().unwrap();
            g.external_override = Some(setting);
        }

        // Apply composed state (override OUT + base FAN).
        let (fan, ext) = {
            let g = self.inner.lock().unwrap();
            Self::snapshot_to_apply(&g)
        };

        apply_composed(&fan, &ext)?;

        // Update snapshots.
        let mut g = self.inner.lock().unwrap();
        g.last_external_applied = ext;
        g.last_fan_applied = if fan_enabled(&fan) { fan } else { None };

        Ok(())
    }

    /// Stop OUT notification override and restore base OUT + FAN.
    pub fn external_stop(&self) {
        {
            let mut g = self.inner.lock().unwrap();
            g.external_override = None;
        }

        self.apply_and_update();
    }
}
