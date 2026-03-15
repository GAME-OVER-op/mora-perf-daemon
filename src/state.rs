use crate::games::GamesRuntime;
use crate::user_config::{ExternalLedSetting, FanLedSetting, NotificationsStopKind, UserConfig};
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct LedRuntimeState {
    pub base_external_desired: Option<ExternalLedSetting>,
    pub base_external_last_applied: Option<ExternalLedSetting>,

    pub fan_desired: Option<FanLedSetting>,
    pub fan_last_applied: Option<FanLedSetting>,

    pub external_active: bool,
    pub external_setting: Option<ExternalLedSetting>,
    pub external_stop_kind: NotificationsStopKind,
    pub external_started_at: Option<Instant>,
    pub external_ends_at: Option<Instant>,
}

impl Default for LedRuntimeState {
    fn default() -> Self {
        Self {
            base_external_desired: None,
            base_external_last_applied: None,
            fan_desired: None,
            fan_last_applied: None,
            external_active: false,
            external_setting: None,
            external_stop_kind: NotificationsStopKind::UntilScreenOn,
            external_started_at: None,
            external_ends_at: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct InfoState {
    pub cpu_avg_mc: Option<i32>,
    pub gpu_avg_mc: Option<i32>,
    pub soc_mc: Option<i32>,
    pub batt_mc: Option<i32>,

    pub temp_zone: String,
    pub reduce_percent: u8,

    pub screen_on: bool,
    // Hardware charging state (as detected by mora)
    pub charging: bool,
    // Config switch (user can disable charging behavior)
    pub charging_enabled: bool,
    // Effective state used in logic = charging && charging_enabled
    pub charging_effective: bool,
    pub game_mode: bool,
    pub idle_mode: bool,

    pub active_profile: String,
    pub led_profile: String,

    // Battery percentage (0..100), if available
    pub battery_percent: Option<u8>,
    // Smart battery saver runtime status
    pub battery_saver_active: bool,
    pub battery_saver_override: bool,
    pub battery_saver_disabled_cores: Vec<u8>,
    pub battery_saver_reapply_in_sec: Option<u64>,

    // Screen-off core saver runtime status
    pub screen_off_core_saver_active: bool,
    pub screen_off_core_saver_disabled_cores: Vec<u8>,

    // Split charge runtime status
    pub split_charge_active: bool,
    pub split_charge_package: Option<String>,
    pub split_charge_node: Option<String>,
    pub split_charge_stop_battery_percent: Option<u8>,
    pub split_charge_last_error: Option<String>,

    // Triggers (shoulder buttons -> virtual touch)
    pub triggers_active: bool,
    pub triggers_left: bool,
    pub triggers_right: bool,
    pub triggers_pkg: Option<String>,
}

impl Default for InfoState {
    fn default() -> Self {
        Self {
            cpu_avg_mc: None,
            gpu_avg_mc: None,
            soc_mc: None,
            batt_mc: None,
            temp_zone: String::new(),
            reduce_percent: 0,
            screen_on: true,
            charging: false,
            charging_enabled: true,
            charging_effective: false,
            game_mode: false,
            idle_mode: false,
            active_profile: String::new(),
            led_profile: String::new(),

            battery_percent: None,
            battery_saver_active: false,
            battery_saver_override: false,
            battery_saver_disabled_cores: Vec::new(),
            battery_saver_reapply_in_sec: None,
            screen_off_core_saver_active: false,
            screen_off_core_saver_disabled_cores: Vec::new(),
            split_charge_active: false,
            split_charge_package: None,
            split_charge_node: None,
            split_charge_stop_battery_percent: None,
            split_charge_last_error: None,

            triggers_active: false,
            triggers_left: false,
            triggers_right: false,
            triggers_pkg: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SharedState {
    pub config: UserConfig,
    pub config_rev: u64,
    pub last_config_error: Option<String>,

    pub games: GamesRuntime,
    pub games_rev: u64,
    pub last_games_error: Option<String>,

    pub info: InfoState,
    pub leds: LedRuntimeState,
}

impl SharedState {
    pub fn new(config: UserConfig, games: GamesRuntime) -> Self {
        Self {
            config,
            config_rev: 0,
            last_config_error: None,

            games,
            games_rev: 0,
            last_games_error: None,

            info: InfoState::default(),
            leds: LedRuntimeState::default(),
        }
    }
}
