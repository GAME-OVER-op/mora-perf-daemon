use crate::{leds::DesiredEffect, user_config::{ExternalLedSetting, FanLedSetting, NotificationsStopKind, UserConfig}};
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct LedRuntimeState {
    pub base_desired: Option<DesiredEffect>,
    pub base_last_applied: Option<DesiredEffect>,

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
            base_desired: None,
            base_last_applied: None,
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
        }
    }
}

#[derive(Clone, Debug)]
pub struct SharedState {
    pub config: UserConfig,
    pub config_rev: u64,
    pub last_config_error: Option<String>,

    pub info: InfoState,
    pub leds: LedRuntimeState,
}

impl SharedState {
    pub fn new(config: UserConfig) -> Self {
        Self {
            config,
            config_rev: 0,
            last_config_error: None,
            info: InfoState::default(),
            leds: LedRuntimeState::default(),
        }
    }
}
