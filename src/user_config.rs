use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    io,
    path::{Path, PathBuf},
};

pub const CONFIG_PATH: &str = "/data/adb/modules/mora_perf_deamon/config/config.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserConfig {
    #[serde(default)]
    pub charging: ChargingConfig,
    pub notifications: NotificationsConfig,
    pub fan_led: FanLedDefaults,
    pub profiles: Vec<ProfileConfig>,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            charging: ChargingConfig::default(),
            notifications: NotificationsConfig::default(),
            fan_led: FanLedDefaults::default(),
            profiles: vec![
                ProfileConfig::normal_default(),
                ProfileConfig::gaming_default(),
            ],
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChargingConfig {
    pub enabled: bool,

    /// Optional FAN LED setting used when charging is effective.
    /// If None - charging does not affect LED output.
    #[serde(default)]
    pub fan_led: Option<FanLedSetting>,

    /// Optional External LED setting for charging (usually disabled).
    /// If both fan_led and external_led are set, external has priority.
    #[serde(default)]
    pub external_led: Option<ExternalLedSetting>,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for ChargingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            fan_led: None,
            external_led: None,
            extra: BTreeMap::new(),
        }
    }
}

impl UserConfig {
    pub fn validate_and_normalize(&mut self) -> Result<(), String> {
        // Keep validation minimal for stability: do not enforce unique priorities/names.
        // UI manages only Normal/Gaming, and custom profiles are ignored.

        // Ensure at least one Normal profile exists.
        let has_normal = self
            .profiles
            .iter()
            .any(|p| matches!(p.profile_type, ProfileType::Normal));
        if !has_normal {
            self.profiles.push(ProfileConfig::normal_default());
        }

        // Ensure at least one Gaming profile exists.
        let has_gaming = self
            .profiles
            .iter()
            .any(|p| matches!(p.profile_type, ProfileType::Gaming));
        if !has_gaming {
            self.profiles.push(ProfileConfig::gaming_default());
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NotificationsConfig {
    pub enabled: bool,
    pub stop_condition: StopConditionWrapper,
    pub for_seconds: u64,
    pub external_led: ExternalLedSetting,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            stop_condition: StopConditionWrapper {
                kind: NotificationsStopKind::UntilScreenOn,
            },
            for_seconds: 10,
            external_led: ExternalLedSetting {
                mode: ExternalLedMode::Blink,
                color: ExternalLedColor::Blue,
            },
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StopConditionWrapper {
    #[serde(rename = "type")]
    pub kind: NotificationsStopKind,
}

#[derive(Clone, Debug, Serialize, Deserialize, Copy)]
#[serde(rename_all = "snake_case")]
pub enum NotificationsStopKind {
    UntilScreenOn,
    ForSeconds,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FanLedDefaults {
    pub default_mode: FanLedMode,
    pub default_color: FanLedColor,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for FanLedDefaults {
    fn default() -> Self {
        Self {
            default_mode: FanLedMode::Static,
            default_color: FanLedColor::White,
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExternalLedSetting {
    pub mode: ExternalLedMode,
    pub color: ExternalLedColor,
}

impl Default for ExternalLedSetting {
    fn default() -> Self {
        Self {
            mode: ExternalLedMode::Static,
            color: ExternalLedColor::White,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Copy, PartialEq, Eq)]
pub enum ExternalLedMode {
    #[serde(rename = "sound")]
    Sound,
    #[serde(rename = "steady", alias = "static")]
    Static,
    #[serde(rename = "breathe", alias = "breath")]
    Breath,
    #[serde(rename = "flashing", alias = "blink")]
    Blink,
    #[serde(rename = "scintillation", alias = "sparkle")]
    Sparkle,
    #[serde(rename = "flow")]
    Flow,
}

#[derive(Clone, Debug, Serialize, Deserialize, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExternalLedColor {
    Multi,
    Red,
    Yellow,
    Blue,
    Green,
    Cyan,
    White,
    Purple,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FanLedSetting {
    pub mode: FanLedMode,
    pub color: FanLedColor,
}

impl Default for FanLedSetting {
    fn default() -> Self {
        Self {
            mode: FanLedMode::Static,
            color: FanLedColor::White,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Copy, PartialEq, Eq)]
pub enum FanLedMode {
    #[serde(rename = "off")]
    Off,
    #[serde(rename = "flow")]
    Flow,
    #[serde(rename = "breathe", alias = "breath")]
    Breath,
    #[serde(rename = "flashing", alias = "blink")]
    Blink,
    #[serde(rename = "steady", alias = "static")]
    Static,
}

#[derive(Clone, Debug, Serialize, Deserialize, Copy, PartialEq, Eq)]
pub enum FanLedColor {
    #[serde(rename = "rose", alias = "red")]
    Rose,
    #[serde(rename = "yellow")]
    Yellow,
    #[serde(rename = "green")]
    Green,
    #[serde(rename = "blue")]
    Blue,
    #[serde(rename = "cyan")]
    Cyan,
    #[serde(rename = "purple")]
    Purple,
    #[serde(rename = "orange")]
    Orange,
    #[serde(rename = "mixed_1")]
    Mixed1,
    #[serde(rename = "mixed_2")]
    Mixed2,
    #[serde(rename = "mixed_3")]
    Mixed3,
    #[serde(rename = "mixed_4")]
    Mixed4,
    #[serde(rename = "mixed_5")]
    Mixed5,
    #[serde(rename = "mixed_6")]
    Mixed6,
    #[serde(rename = "mixed_7", alias = "white")]
    White,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub name: String,

    #[serde(rename = "type")]
    pub profile_type: ProfileType,

    pub priority: i32,
    pub enabled: bool,

    #[serde(default)]
    pub fan_led: Option<FanLedSetting>,

    /// Optional external LED setting for the profile.
    /// If both fan_led and external_led are set, external has priority.
    #[serde(default)]
    pub external_led: Option<ExternalLedSetting>,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl ProfileConfig {
    pub fn normal_default() -> Self {
        Self {
            name: "Normal".to_string(),
            profile_type: ProfileType::Normal,
            priority: 1,
            enabled: true,
            fan_led: Some(FanLedSetting {
                mode: FanLedMode::Off,
                color: FanLedColor::White,
            }),
            external_led: None,
            extra: BTreeMap::new(),
        }
    }

    pub fn gaming_default() -> Self {
        Self {
            name: "Gaming".to_string(),
            profile_type: ProfileType::Gaming,
            priority: 10,
            enabled: true,
            fan_led: Some(FanLedSetting {
                mode: FanLedMode::Breath,
                color: FanLedColor::Rose,
            }),
            external_led: None,
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProfileType {
    Normal,
    Gaming,
    Custom,
}

pub fn ensure_parent_dir(path: &Path) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

pub fn load_or_init(path: &Path) -> UserConfig {
    match fs::read_to_string(path) {
        Ok(s) => match serde_json::from_str::<UserConfig>(&s) {
            Ok(mut cfg) => {
                if let Err(e) = cfg.validate_and_normalize() {
                    eprintln!("CFG: invalid config: {} (reset to default)", e);
                    let def = UserConfig::default();
                    let _ = write_config_atomic(path, &def);
                    def
                } else {
                    cfg
                }
            }
            Err(e) => {
                eprintln!("CFG: failed to parse config: {} (reset to default)", e);
                let def = UserConfig::default();
                let _ = write_config_atomic(path, &def);
                def
            }
        },
        Err(_) => {
            let def = UserConfig::default();
            let _ = write_config_atomic(path, &def);
            def
        }
    }
}

pub fn write_config_atomic(path: &Path, cfg: &UserConfig) -> io::Result<()> {
    ensure_parent_dir(path)?;
    let tmp = PathBuf::from(format!("{}.tmp", path.display()));
    let data = serde_json::to_string_pretty(cfg)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    fs::write(&tmp, data.as_bytes())?;
    fs::rename(&tmp, path)?;
    Ok(())
}
