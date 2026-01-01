use crate::user_config::{
    ExternalLedColor, ExternalLedMode, ExternalLedSetting, FanLedColor, FanLedMode, FanLedSetting,
};
use std::{
    fs,
    io,
    path::Path,
    sync::{Mutex},
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

fn ensure_external_init() {
    // Same as in full_rgb.sh: hwen=1, brightness=max_brightness
    let hwen = p("hwen");
    if Path::new(&hwen).exists() {
        let _ = write_u64(&hwen, 1);
    }

    let br = p("brightness");
    let maxbr = p("max_brightness");
    if Path::new(&br).exists() && Path::new(&maxbr).exists() {
        if let Some(s) = read_str(&maxbr) {
            if let Ok(n) = s.trim().parse::<u64>() {
                let _ = write_u64(&br, n);
            }
        }
    }
}

fn write_effect_and_cfg(effect: &str) -> io::Result<()> {
    let eff = p("effect");
    let cfg = p("cfg");
    write_str(&eff, effect)?;
    write_u64(&cfg, 1)?;
    Ok(())
}

fn external_mode_base(m: ExternalLedMode) -> u64 {
    match m {
        ExternalLedMode::Sound => 80,
        ExternalLedMode::Static => 96,
        ExternalLedMode::Breath => 112,
        ExternalLedMode::Blink => 128,
        ExternalLedMode::Sparkle => 144,
        ExternalLedMode::Flow => 160,
    }
}

fn external_color_idx(c: ExternalLedColor) -> u64 {
    match c {
        ExternalLedColor::Multi => 0,
        ExternalLedColor::Red => 1,
        ExternalLedColor::Yellow => 2,
        ExternalLedColor::Blue => 3,
        ExternalLedColor::Green => 4,
        ExternalLedColor::Cyan => 5,
        ExternalLedColor::White => 6,
        ExternalLedColor::Purple => 7,
    }
}

fn external_code(setting: ExternalLedSetting) -> String {
    let v = external_mode_base(setting.mode) + external_color_idx(setting.color);
    format!("0x{:02X}", v)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DesiredEffect {
    Fan(FanLedSetting),
    External(ExternalLedSetting),
}

fn fan_color_nibble(c: FanLedColor) -> u8 {
    match c {
        FanLedColor::Rose => 0x1,
        FanLedColor::Yellow => 0x2,
        FanLedColor::Green => 0x3,
        FanLedColor::Blue => 0x4,
        FanLedColor::Cyan => 0x5,
        FanLedColor::Purple => 0x6,
        FanLedColor::Orange => 0x7,
        FanLedColor::Mixed1 => 0x8,
        FanLedColor::Mixed2 => 0x9,
        FanLedColor::Mixed3 => 0xA,
        FanLedColor::Mixed4 => 0xB,
        FanLedColor::Mixed5 => 0xC,
        FanLedColor::Mixed6 => 0xD,
        FanLedColor::White => 0xF,
    }
}

fn fan_base(m: FanLedMode, c: FanLedColor) -> u8 {
    match m {
        FanLedMode::Off => 0x00,
        FanLedMode::Blink => 0x20,
        FanLedMode::Breath => 0x30,
        FanLedMode::Flow => 0x40,
        FanLedMode::Static => {
            if matches!(c, FanLedColor::Mixed1
                | FanLedColor::Mixed2
                | FanLedColor::Mixed3
                | FanLedColor::Mixed4
                | FanLedColor::Mixed5
                | FanLedColor::Mixed6
                | FanLedColor::White)
            {
                0x90
            } else {
                0xA0
            }
        }
    }
}

fn fan_code(setting: &FanLedSetting) -> String {
    if matches!(setting.mode, FanLedMode::Off) {
        // full_rgb.sh FAN off = effect=2, cfg=1
        return "2".to_string();
    }
    let base = fan_base(setting.mode, setting.color);
    let n = fan_color_nibble(setting.color);
    let v = (base as u16) + (n as u16);
    // full_rgb.sh uses printf "%x" (no 0x)
    format!("{:x}", v)
}

#[derive(Debug, Default)]
struct LedsInner {
    external_override: bool,
    desired: Option<DesiredEffect>,
    last_applied: Option<DesiredEffect>,
    last_domain: Option<EffectDomain>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectDomain {
    Fan,
    External,
}

fn domain_of_effect(e: &DesiredEffect) -> EffectDomain {
    match e {
        DesiredEffect::Fan(_) => EffectDomain::Fan,
        DesiredEffect::External(_) => EffectDomain::External,
    }
}

fn apply_off(domain: EffectDomain) {
    match domain {
        EffectDomain::Fan => {
            let _ = write_effect_and_cfg("2");
        }
        EffectDomain::External => {
            let _ = write_effect_and_cfg("0x00");
        }
    }
}

fn apply_effect(e: &DesiredEffect) {
    match e {
        DesiredEffect::Fan(s) => {
            let _ = write_effect_and_cfg(&fan_code(s));
        }
        DesiredEffect::External(s) => {
            ensure_external_init();
            let _ = write_effect_and_cfg(&external_code(s.clone()));
        }
    }
}

/// Thread-safe LED controller.
///
/// IMPORTANT: external notification scenario can temporarily override the LED device.
/// During override, fan settings are only remembered and applied after external stops.
#[derive(Debug, Default)]
pub struct Leds {
    inner: Mutex<LedsInner>,
}

impl Leds {
    pub fn new() -> Self {
        ensure_external_init();
        Self::default()
    }

    /// Set the desired *base* LED effect (fan or external). This is separate from
    /// the notification override.
    pub fn set_base_desired(&self, desired: Option<DesiredEffect>) {
        let mut g = self.inner.lock().unwrap();

        if g.desired == desired {
            return;
        }

        g.desired = desired;

        // If no external override -> apply immediately.
        if !g.external_override {
            // Avoid borrowing `g` immutably and mutably at the same time.
            let desired_now = g.desired.clone();
            let last_dom = g.last_domain;

            match desired_now {
                Some(e) => {
                    apply_effect(&e);
                    g.last_applied = Some(e.clone());
                    g.last_domain = Some(domain_of_effect(&e));
                }
                None => {
                    if let Some(dom) = last_dom {
                        apply_off(dom);
                    }
                    g.last_applied = None;
                    g.last_domain = None;
                }
            }
        }
    }

    pub fn get_base_state(&self) -> (Option<DesiredEffect>, Option<DesiredEffect>) {
        let g = self.inner.lock().unwrap();
        (g.desired.clone(), g.last_applied.clone())
    }

    // ------------------------------------------------------------
    // Backwards-compatible FAN-only helpers (used by older code/UI)
    // ------------------------------------------------------------
    pub fn set_fan_desired(&self, setting: Option<FanLedSetting>) {
        self.set_base_desired(setting.map(DesiredEffect::Fan));
    }

    pub fn get_fan_state(&self) -> (Option<FanLedSetting>, Option<FanLedSetting>) {
        let (d, l) = self.get_base_state();
        let d = match d {
            Some(DesiredEffect::Fan(s)) => Some(s),
            _ => None,
        };
        let l = match l {
            Some(DesiredEffect::Fan(s)) => Some(s),
            _ => None,
        };
        (d, l)
    }

    pub fn external_start(&self, setting: ExternalLedSetting) -> io::Result<()> {
        ensure_external_init();
        let code = external_code(setting);

        // mark override before applying so fan updates won't clobber the effect
        {
            let mut g = self.inner.lock().unwrap();
            g.external_override = true;
        }

        // Some firmwares/drivers only (re)apply the effect reliably when we reset first.
        // This matches the manual workflow in full_rgb.sh (toggle off/on) and makes
        // repeated notifications restart the blinking deterministically.
        let _ = write_effect_and_cfg("0x00");
        std::thread::sleep(std::time::Duration::from_millis(10));

        write_effect_and_cfg(&code)
    }

    pub fn external_stop(&self) {
        // OFF external code (OUT off)
        let _ = write_effect_and_cfg("0x00");

        // drop override + restore base desired effect
        let mut g = self.inner.lock().unwrap();
        g.external_override = false;

        // Avoid borrowing `g` immutably and mutably at the same time.
        let desired_now = g.desired.clone();
        let last_dom = g.last_domain;

        match desired_now {
            Some(e) => {
                apply_effect(&e);
                g.last_applied = Some(e.clone());
                g.last_domain = Some(domain_of_effect(&e));
            }
            None => {
                // If previous base was FAN, use FAN off code=2. Otherwise OUT off is enough.
                if let Some(dom) = last_dom {
                    apply_off(dom);
                }
                g.last_applied = None;
                g.last_domain = None;
            }
        }
    }
}
