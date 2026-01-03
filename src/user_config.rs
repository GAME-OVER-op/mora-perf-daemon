use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    io,
    io::Read,
    path::{Path, PathBuf},
};

pub const CONFIG_PATH: &str = "/data/adb/modules/mora_perf_deamon/config/config.json";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserConfig {
    /// Static API token used by local clients (Android app) to access /api/* endpoints.
    ///
    /// Web UI routes are disabled; unauthenticated requests will get an empty 404 response.
    /// If this field is missing or empty, the daemon will generate one and persist it.
    #[serde(default)]
    pub api_token: String,

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
            api_token: String::new(),
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
    /// If both fan_led and external_led are set, both will be applied (full_rgb.sh logic).
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

        // Normalize LED modes to the subset that actually works on this firmware.
        // (UI only exposes supported values, but config files may contain older ones.)
        self.normalize_leds();

        Ok(())
    }

    fn normalize_leds(&mut self) {
        // charging
        if let Some(ref mut s) = self.charging.fan_led {
            s.mode = normalize_fan_mode(s.mode);
        }
        if let Some(ref mut s) = self.charging.external_led {
            s.mode = normalize_external_mode(s.mode);
        }

        // notifications
        self.notifications.external_led.mode = normalize_external_mode(self.notifications.external_led.mode);

        // profiles
        for p in &mut self.profiles {
            if let Some(ref mut s) = p.fan_led {
                s.mode = normalize_fan_mode(s.mode);
            }
            if let Some(ref mut s) = p.external_led {
                s.mode = normalize_external_mode(s.mode);
            }
        }
    }
}

fn normalize_external_mode(m: ExternalLedMode) -> ExternalLedMode {
    // Supported: steady/breathe/flashing
    match m {
        ExternalLedMode::Static | ExternalLedMode::Breath | ExternalLedMode::Blink => m,
        _ => ExternalLedMode::Static,
    }
}

fn normalize_fan_mode(m: FanLedMode) -> FanLedMode {
    // Supported: flow/steady/flashing/breathe (+ off)
    match m {
        FanLedMode::Off | FanLedMode::Flow | FanLedMode::Breath | FanLedMode::Blink | FanLedMode::Static => m,
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
            default_color: FanLedColor::Mixed7,
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
    Pink,
    Orange,
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
            color: FanLedColor::Mixed7,
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
    Mixed7,
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
    /// If both fan_led and external_led are set, both will be applied (full_rgb.sh logic).
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
                color: FanLedColor::Mixed7,
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
                    let mut def = UserConfig::default();
                    let _ = ensure_api_token(&mut def);
                    let _ = write_config_atomic(path, &def);
                    def
                } else {
                    // Ensure token exists; persist if we generated it.
                    if ensure_api_token(&mut cfg).unwrap_or(false) {
                        let _ = write_config_atomic(path, &cfg);
                    }
                    cfg
                }
            }
            Err(e) => {
                eprintln!("CFG: failed to parse config: {} (reset to default)", e);
                let mut def = UserConfig::default();
                let _ = ensure_api_token(&mut def);
                let _ = write_config_atomic(path, &def);
                def
            }
        },
        Err(_) => {
            let mut def = UserConfig::default();
            let _ = ensure_api_token(&mut def);
            let _ = write_config_atomic(path, &def);
            def
        }
    }
}

fn ensure_api_token(cfg: &mut UserConfig) -> io::Result<bool> {
    if !cfg.api_token.trim().is_empty() {
        return Ok(false);
    }

    // Generate a stable token (hex) from /dev/urandom. Rooted environment guarantees access.
    let mut f = fs::File::open("/dev/urandom")?;
    let mut buf = [0u8; 32];
    f.read_exact(&mut buf)?;

    let mut out = String::with_capacity(buf.len() * 2);
    for b in buf {
        use std::fmt::Write;
        let _ = write!(&mut out, "{:02x}", b);
    }
    cfg.api_token = out;
    Ok(true)
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
