use crate::user_config::{ExternalLedSetting, FanLedSetting, ProfileConfig, ProfileType, UserConfig};

#[derive(Clone, Debug)]
pub struct BaseLedSelection {
    /// Human readable source: "Charging" / "Gaming" / "Normal" / "Off"
    pub source: String,
    pub fan: Option<FanLedSetting>,
    pub external: Option<ExternalLedSetting>,
}

fn find_profile<'a>(
    cfg: &'a UserConfig,
    kind: ProfileType,
    name_fallback: &str,
) -> Option<&'a ProfileConfig> {
    cfg.profiles
        .iter()
        .find(|p| p.profile_type == kind)
        .or_else(|| cfg.profiles.iter().find(|p| p.name == name_fallback))
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

pub fn select_active_mode_profile(cfg: &UserConfig, game_mode: bool) -> ProfileConfig {
    if game_mode {
        let g = get_gaming_profile(cfg);
        if g.enabled {
            return g;
        }
    }
    get_normal_profile(cfg)
}

/// LED selection following full_rgb.sh behavior:
/// - FAN and OUT share the same `effect` node, but the driver keeps their state independently
///   (fan commands are hex without 0x, out commands are 0xNN).
/// - Therefore FAN and External can be active simultaneously.
/// - When applying OUT, the daemon must re-apply FAN (handled in leds.rs).
pub fn select_base_led(
    cfg: &UserConfig,
    _screen_on: bool,
    charging_effective: bool,
    game_mode: bool,
) -> BaseLedSelection {
    // 1) Charging overrides LED output (but must not disable game mode itself).
    if charging_effective {
        let fan = cfg.charging.fan_led.clone();
        let external = cfg.charging.external_led.clone();
        if fan.is_some() || external.is_some() {
            return BaseLedSelection {
                source: "Charging".to_string(),
                fan,
                external,
            };
        }
        // Charging enabled but no LED settings -> fall through.
    }

    // 2) Gaming
    let g = get_gaming_profile(cfg);
    if game_mode && g.enabled {
        let fan = g.fan_led.clone();
        let external = g.external_led.clone();
        if fan.is_some() || external.is_some() {
            return BaseLedSelection {
                source: "Gaming".to_string(),
                fan,
                external,
            };
        }
    }

    // 3) Normal
    // NOTE: On this device the LED driver can be slow and the daemon may overwrite
    // manual changes. Also, tying LEDs to `screen_on` caused confusing behavior
    // ("nothing works" when the screen state toggles or is detected incorrectly).
    // We therefore apply the Normal profile regardless of screen state.
    let n = get_normal_profile(cfg);
    if n.enabled {
        let fan = n.fan_led.clone();
        let external = n.external_led.clone();
        if fan.is_some() || external.is_some() {
            return BaseLedSelection {
                source: "Normal".to_string(),
                fan,
                external,
            };
        }
    }

    BaseLedSelection {
        source: "Off".to_string(),
        fan: None,
        external: None,
    }
}
