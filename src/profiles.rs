use crate::user_config::{ExternalLedSetting, FanLedSetting, ProfileConfig, ProfileType, UserConfig};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BaseLedDesired {
    Fan(FanLedSetting),
    External(ExternalLedSetting),
}

#[derive(Clone, Debug)]
pub struct BaseLedSelection {
    /// Human readable source: "Charging" / "Gaming" / "Normal" / "Off"
    pub source: String,
    pub desired: Option<BaseLedDesired>,
}

fn find_profile<'a>(cfg: &'a UserConfig, kind: ProfileType, name_fallback: &str) -> Option<&'a ProfileConfig> {
    cfg.profiles
        .iter()
        .find(|p| p.profile_type == kind || p.name.eq_ignore_ascii_case(name_fallback))
}

pub fn get_normal_profile(cfg: &UserConfig) -> ProfileConfig {
    find_profile(cfg, ProfileType::Normal, "Normal")
        .cloned()
        .unwrap_or_else(ProfileConfig::normal_default)
}

pub fn get_gaming_profile(cfg: &UserConfig) -> ProfileConfig {
    find_profile(cfg, ProfileType::Gaming, "Gaming")
        .cloned()
        .unwrap_or_else(ProfileConfig::gaming_default)
}

/// Active mode profile (for mora logic/status): Gaming only when game_mode is true and gaming profile is enabled.
pub fn select_active_mode_profile(cfg: &UserConfig, game_mode: bool) -> ProfileConfig {
    let g = get_gaming_profile(cfg);
    if game_mode && g.enabled {
        return g;
    }
    get_normal_profile(cfg)
}

/// Select base LED effect using fixed priority (no numeric priorities):
/// Charging (only when charging_effective) > Gaming (only when game_mode) > Normal.
///
/// Important: Normal profile never turns on LEDs when screen is OFF.
///
/// Hardware note (full_rgb.sh): a single "effect" register controls both FAN and OUT.
/// So FAN and External effects are mutually exclusive. If a profile has both configured,
/// External has priority.
pub fn select_base_led(cfg: &UserConfig, screen_on: bool, charging_effective: bool, game_mode: bool) -> BaseLedSelection {
    // 1) Charging overrides LED output (but must not disable game mode itself).
    if charging_effective {
        // Prefer external if configured, then fan.
        if let Some(ext) = cfg.charging.external_led.clone() {
            return BaseLedSelection { source: "Charging".to_string(), desired: Some(BaseLedDesired::External(ext)) };
        }
        if let Some(fan) = cfg.charging.fan_led.clone() {
            return BaseLedSelection { source: "Charging".to_string(), desired: Some(BaseLedDesired::Fan(fan)) };
        }
        // Charging enabled but no LED settings -> fall through.
    }

    // 2) Gaming
    let g = get_gaming_profile(cfg);
    if game_mode && g.enabled {
        if let Some(ext) = g.external_led.clone() {
            return BaseLedSelection { source: "Gaming".to_string(), desired: Some(BaseLedDesired::External(ext)) };
        }
        if let Some(fan) = g.fan_led.clone() {
            return BaseLedSelection { source: "Gaming".to_string(), desired: Some(BaseLedDesired::Fan(fan)) };
        }
    }

    // 3) Normal (screen ON only)
    let n = get_normal_profile(cfg);
    if n.enabled && screen_on {
        if let Some(ext) = n.external_led.clone() {
            return BaseLedSelection { source: "Normal".to_string(), desired: Some(BaseLedDesired::External(ext)) };
        }
        if let Some(fan) = n.fan_led.clone() {
            return BaseLedSelection { source: "Normal".to_string(), desired: Some(BaseLedDesired::Fan(fan)) };
        }
    }

    BaseLedSelection { source: "Off".to_string(), desired: None }
}
