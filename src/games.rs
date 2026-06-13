use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    io,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::config;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggerSideConfig {
    #[serde(default)]
    pub enabled: bool,
    /// Pixel coordinates (fb0/virtual_size coordinate system).
    #[serde(default)]
    pub x: i32,
    #[serde(default)]
    pub y: i32,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for TriggerSideConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            x: 0,
            y: 0,
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TriggersConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub left: TriggerSideConfig,
    #[serde(default)]
    pub right: TriggerSideConfig,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for TriggersConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            left: TriggerSideConfig::default(),
            right: TriggerSideConfig::default(),
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SplitChargeConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_stop_battery_percent")]
    pub stop_battery_percent: u8,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

fn default_stop_battery_percent() -> u8 { 20 }

impl Default for SplitChargeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            stop_battery_percent: default_stop_battery_percent(),
            extra: BTreeMap::new(),
        }
    }
}

fn deserialize_fan_min_level_opt<'de, D>(deserializer: D) -> Result<Option<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let v: Option<u8> = Option::deserialize(deserializer)?;
    if let Some(x) = v {
        if !(2..=5).contains(&x) {
            return Err(serde::de::Error::custom(
                "fan_min_level must be in range 2..=5",
            ));
        }
    }
    Ok(v)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GameEntry {
    pub package: String,

    /// Enable Android updatable game driver for this package.
    #[serde(default)]
    pub game_driver: bool,

    /// Minimum fan level while this game is active (2..=5).
    /// If omitted, defaults to config::GAME_FAN_BASE.
    #[serde(
        default,
        deserialize_with = "deserialize_fan_min_level_opt",
        skip_serializing_if = "Option::is_none"
    )]
    pub fan_min_level: Option<u8>,

    /// GPU turbo mode for this game. If true, GPU min/max are pinned to maximum while active.
    #[serde(default)]
    pub gpu_turbo: bool,

    /// Optional triggers config for this game.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub triggers: Option<TriggersConfig>,

    /// Per-game split charge config.
    #[serde(default)]
    pub split_charge: SplitChargeConfig,

    /// If true, Mora will not reduce performance by thermal caps for this game.
    #[serde(default)]
    pub disable_thermal_limit: bool,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for GameEntry {
    fn default() -> Self {
        Self {
            package: String::new(),
            game_driver: false,
            fan_min_level: None,
            gpu_turbo: false,
            triggers: None,
            split_charge: SplitChargeConfig::default(),
            disable_thermal_limit: false,
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GamesFile {
    #[serde(default)]
    pub games: Vec<GameEntry>,

    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl Default for GamesFile {
    fn default() -> Self {
        Self {
            games: Vec::new(),
            extra: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GamesRuntime {
    pub file: GamesFile,
    pub pkg_set: HashSet<String>,
    pub driver_pkgs: Vec<String>,
    pub driver_string: String,
    pub fan_min: HashMap<String, u8>,
    pub gpu_turbo: HashMap<String, bool>,
    pub triggers: HashMap<String, TriggersConfig>,
    pub disable_thermal_limit: HashMap<String, bool>,
    pub split_charge: HashMap<String, SplitChargeConfig>,
}

impl GamesRuntime {
    pub fn from_file(mut file: GamesFile) -> Self {
        file.normalize();
        let mut pkg_set = HashSet::new();
        let mut fan_min: HashMap<String, u8> = HashMap::new();
        let mut gpu_turbo: HashMap<String, bool> = HashMap::new();
        let mut triggers: HashMap<String, TriggersConfig> = HashMap::new();
        let mut disable_thermal_limit: HashMap<String, bool> = HashMap::new();
        let mut split_charge: HashMap<String, SplitChargeConfig> = HashMap::new();
        for g in &file.games {
            pkg_set.insert(g.package.clone());
            fan_min.insert(
                g.package.clone(),
                g.fan_min_level.unwrap_or(config::GAME_FAN_BASE),
            );

            gpu_turbo.insert(g.package.clone(), g.gpu_turbo);
            disable_thermal_limit.insert(g.package.clone(), g.disable_thermal_limit);
            split_charge.insert(g.package.clone(), g.split_charge.clone());

            if let Some(t) = &g.triggers {
                // Keep triggers config even if disabled; runtime logic will decide.
                triggers.insert(g.package.clone(), t.clone());
            }
        }

        let mut driver_pkgs: Vec<String> = file
            .games
            .iter()
            .filter(|g| g.game_driver)
            .map(|g| g.package.clone())
            .collect();
        driver_pkgs.sort();
        driver_pkgs.dedup();
        let driver_string = driver_pkgs.join(",");

        Self {
            file,
            pkg_set,
            driver_pkgs,
            driver_string,
            fan_min,
            gpu_turbo,
            triggers,
            disable_thermal_limit,
            split_charge,
        }
    }

    pub fn is_game(&self, pkg: &str) -> bool {
        self.pkg_set.contains(pkg)
    }

    pub fn game_fan_min_level(&self, pkg: &str) -> u8 {
        self.fan_min
            .get(pkg)
            .copied()
            .unwrap_or(config::GAME_FAN_BASE)
    }

    pub fn game_gpu_turbo(&self, pkg: &str) -> bool {
        self.gpu_turbo.get(pkg).copied().unwrap_or(false)
    }


    pub fn triggers_for(&self, pkg: &str) -> Option<TriggersConfig> {
        self.triggers.get(pkg).cloned()
    }

    pub fn game_disable_thermal_limit(&self, pkg: &str) -> bool {
        self.disable_thermal_limit.get(pkg).copied().unwrap_or(false)
    }

    pub fn game_split_charge(&self, pkg: &str) -> SplitChargeConfig {
        self.split_charge.get(pkg).cloned().unwrap_or_default()
    }
}

fn sanitize_pkg(s: &str) -> String {
    let s = s.trim();
    s.trim_matches(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-'))
        .to_string()
}

impl GamesFile {
    /// Normalize entries:
    /// - sanitize package names
    /// - drop empty package entries
    /// - de-duplicate by package, keeping the LAST occurrence
    pub fn normalize(&mut self) {
        let mut out: Vec<GameEntry> = Vec::new();
        let mut pos: HashMap<String, usize> = HashMap::new();

        for mut g in std::mem::take(&mut self.games) {
            let pkg = sanitize_pkg(&g.package);
            if pkg.is_empty() {
                continue;
            }
            g.package = pkg.clone();

            if let Some(i) = pos.get(&pkg).copied() {
                // remove earlier entry so the new one becomes the last
                out.remove(i);
                // fix indices after removal
                for j in i..out.len() {
                    pos.insert(out[j].package.clone(), j);
                }
            }

            out.push(g);
            pos.insert(pkg, out.len() - 1);
        }

        self.games = out;
    }
}

pub fn read_games(path: &Path) -> io::Result<GamesFile> {
    let mut f = fs::File::open(path)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    let mut gf: GamesFile =
        serde_json::from_str(&s).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    gf.normalize();
    Ok(gf)
}

pub fn write_games_atomic(path: &Path, games: &GamesFile) -> io::Result<()> {
    let parent = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    fs::create_dir_all(&parent)?;

    let tmp = parent.join(".games.json.tmp");
    let data = serde_json::to_vec_pretty(games)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(&tmp, data)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

/// Load games.json. If missing or invalid, reset to an empty list and persist.
/// Returns runtime and optional error description.
pub fn load_or_init(path: &Path) -> (GamesRuntime, Option<String>) {
    match read_games(path) {
        Ok(gf) => (GamesRuntime::from_file(gf), None),
        Err(e) => {
            let empty = GamesFile::default();
            let _ = write_games_atomic(path, &empty);
            (GamesRuntime::from_file(empty), Some(format!("games reset: {}", e)))
        }
    }
}

fn escape_for_double_quotes(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Apply Android updatable game driver opt-in list.
/// Uses `settings put global updatable_driver_production_opt_in_apps "<csv>"`.
pub fn apply_updatable_driver_apps(csv: &str) {
    // Direct call first (daemon often runs as root).
    let st = Command::new("settings")
        .args([
            "put",
            "global",
            "updatable_driver_production_opt_in_apps",
            csv,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if st.is_err() || !st.as_ref().ok().map(|x| x.success()).unwrap_or(false) {
        // Fallback via shell.
        let q = escape_for_double_quotes(csv);
        let cmd = format!(
            "settings put global updatable_driver_production_opt_in_apps \"{}\"",
            q
        );
        let _ = Command::new("/system/bin/sh")
            .args(["-c", &cmd])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    println!(
        "GAMES: updatable_driver_production_opt_in_apps = {}",
        if csv.is_empty() { "\"\"" } else { csv }
    );
}
