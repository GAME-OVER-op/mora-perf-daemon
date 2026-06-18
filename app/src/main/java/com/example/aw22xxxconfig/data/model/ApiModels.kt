package com.example.aw22xxxconfig.data.model

import kotlinx.serialization.ExperimentalSerializationApi
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.JsonNames

@Serializable
data class StateResponse(
    val temps: Temps = Temps(),
    val zone: Zone = Zone(),
    @SerialName("screen_on") val screenOn: Boolean = false,
    val charging: ChargingState = ChargingState(),
    val battery: BatteryState = BatteryState(),
    @SerialName("game_mode") val gameMode: Boolean = false,
    val triggers: TriggerRuntime = TriggerRuntime(),
    @SerialName("idle_mode") val idleMode: Boolean = false,
    @SerialName("daemon_notifications") val daemonNotifications: Boolean = false,
    @SerialName("active_profile") val activeProfile: String? = null,
    @SerialName("led_profile") val ledProfile: String? = null,
    val mem: MemoryState = MemoryState(),
    @SerialName("last_config_error") val lastConfigError: String? = null,
    val games: GamesRuntime = GamesRuntime(),
)

@Serializable data class Temps(val cpu: Double? = null, val gpu: Double? = null, val soc: Double? = null, val batt: Double? = null)
@Serializable data class Zone(val name: String? = null, @SerialName("reduce_percent") val reducePercent: Int? = null)
@Serializable data class ChargingState(val hw: Boolean = false, val enabled: Boolean = false, val effective: Boolean = false)
@Serializable data class BatteryState(val percent: Int? = null, val saver: BatterySaverRuntime = BatterySaverRuntime())
@Serializable data class BatterySaverRuntime(
    val enabled: Boolean = false,
    val active: Boolean = false,
    @kotlinx.serialization.SerialName("override") val overrideValue: Boolean = false,
    @SerialName("disabled_cores") val disabledCores: List<Int> = emptyList(),
    @SerialName("reapply_in_sec") val reapplyInSec: Long? = null,
)
@Serializable data class TriggerRuntime(
    val active: Boolean = false,
    val preview: Boolean = false,
    @SerialName("package") val packageName: String? = null,
    val left: Boolean = false,
    val right: Boolean = false,
    @SerialName("left_pressed") val leftPressed: Boolean = false,
    @SerialName("right_pressed") val rightPressed: Boolean = false,
)
@Serializable data class MemoryState(@SerialName("VmRSS_kb") val vmRssKb: Long? = null)
@Serializable data class GamesRuntime(val count: Int = 0, @SerialName("driver_count") val driverCount: Int = 0, @SerialName("last_error") val lastError: String? = null)

@Serializable
data class UserConfig(
    @SerialName("api_token") val apiToken: String = "",
    @SerialName("daemon_notifications") val daemonNotifications: Boolean = true,
    @SerialName("use_phone_cooler") val usePhoneCooler: Boolean = true,
    @SerialName("battery_saver") val batterySaver: BatterySaverConfig = BatterySaverConfig(),
    val charging: ChargingConfig = ChargingConfig(),
    val notifications: NotificationsConfig = NotificationsConfig(),
    @SerialName("fan_led") val fanLedDefaults: FanLedDefaults = FanLedDefaults(),
    val profiles: List<ProfileConfig> = emptyList(),
)

@Serializable data class BatterySaverConfig(val enabled: Boolean = false)
@Serializable data class FanLedDefaults(val enabled: Boolean = true, val mode: FanLedMode = FanLedMode.STEADY, val color: FanLedColor = FanLedColor.MIXED7)
@Serializable data class ChargingConfig(val enabled: Boolean = true, @SerialName("fan_led") val fanLed: FanLedSetting? = null, @SerialName("external_led") val externalLed: ExternalLedSetting? = null)
@Serializable data class NotificationsConfig(
    val enabled: Boolean = true,
    @SerialName("stop_condition") val stopCondition: StopConditionWrapper = StopConditionWrapper(),
    @SerialName("for_seconds") val forSeconds: Long = 10,
    @SerialName("external_led") val externalLed: ExternalLedSetting = ExternalLedSetting(),
)
@Serializable data class StopConditionWrapper(@SerialName("type") val type: NotificationsStopKind = NotificationsStopKind.UNTIL_SCREEN_ON)
@Serializable enum class NotificationsStopKind { @SerialName("until_screen_on") UNTIL_SCREEN_ON, @SerialName("for_seconds") FOR_SECONDS }

@Serializable data class ProfileConfig(
    val name: String,
    @SerialName("type") val profileType: ProfileType,
    val priority: Int,
    val enabled: Boolean,
    @SerialName("fan_led") val fanLed: FanLedSetting? = null,
    @SerialName("external_led") val externalLed: ExternalLedSetting? = null,
)
@Serializable enum class ProfileType { @SerialName("normal") NORMAL, @SerialName("gaming") GAMING, @SerialName("custom") CUSTOM }

@Serializable data class FanLedSetting(val mode: FanLedMode = FanLedMode.STEADY, val color: FanLedColor = FanLedColor.MIXED7)
@Serializable data class ExternalLedSetting(val mode: ExternalLedMode = ExternalLedMode.STEADY, val color: ExternalLedColor = ExternalLedColor.RED)

@Serializable enum class FanLedMode { @SerialName("off") OFF, @SerialName("flow") FLOW, @SerialName("breathe") BREATHE, @SerialName("flashing") FLASHING, @SerialName("burst_flash") BURST_FLASH, @SerialName("steady") STEADY }
@OptIn(ExperimentalSerializationApi::class)
@Serializable enum class FanLedColor {
    @SerialName("red") RED,
    @SerialName("mix_color1") @JsonNames("mixed_1") MIXED1,
    @SerialName("rose") ROSE,
    @SerialName("mix_color2") @JsonNames("mixed_2") MIXED2,
    @SerialName("orange") ORANGE,
    @SerialName("mix_color3") @JsonNames("mixed_3") MIXED3,
    @SerialName("yellow") YELLOW,
    @SerialName("mix_color4") @JsonNames("mixed_4") MIXED4,
    @SerialName("green") GREEN,
    @SerialName("mix_color5") @JsonNames("mixed_5") MIXED5,
    @SerialName("cyan") CYAN,
    @SerialName("mix_color6") @JsonNames("mixed_6") MIXED6,
    @SerialName("blue") BLUE,
    @SerialName("mix_color7") @JsonNames("mixed_7") MIXED7,
    @SerialName("purple") PURPLE
}
@Serializable enum class ExternalLedMode { @SerialName("sound") SOUND, @SerialName("steady") STEADY, @SerialName("breathe") BREATHE, @SerialName("flashing") FLASHING, @SerialName("double_flash") DOUBLE_FLASH, @SerialName("flow") FLOW, @SerialName("ripple") RIPPLE, @SerialName("echo") ECHO, @SerialName("jump") JUMP, @SerialName("burst_flash") BURST_FLASH, @SerialName("cycle_flash") CYCLE_FLASH }
@Serializable enum class ExternalLedColor {
    @SerialName("multi") MULTI, @SerialName("red") RED, @SerialName("yellow") YELLOW, @SerialName("blue") BLUE,
    @SerialName("green") GREEN, @SerialName("cyan") CYAN, @SerialName("white") WHITE, @SerialName("purple") PURPLE,
    @SerialName("pink") PINK, @SerialName("orange") ORANGE
}

@Serializable data class GamesFile(val games: List<GameEntry> = emptyList())
@Serializable data class GameEntry(
    @SerialName("package") val packageName: String,
    @SerialName("game_driver") val gameDriver: Boolean = false,
    @SerialName("fan_min_level") val fanMinLevel: Int? = null,
    @SerialName("gpu_turbo") val gpuTurbo: Boolean = false,
    val triggers: TriggersConfig? = null,
    @SerialName("split_charge") val splitCharge: SplitChargeConfig = SplitChargeConfig(),
    @SerialName("disable_thermal_limit") val disableThermalLimit: Boolean = false,
)
@Serializable data class TriggersConfig(val enabled: Boolean = false, val left: TriggerPoint = TriggerPoint(), val right: TriggerPoint = TriggerPoint())
@Serializable data class TriggerPoint(val enabled: Boolean = false, val x: Int = 0, val y: Int = 0)
@Serializable data class SplitChargeConfig(
    val enabled: Boolean = false,
    @SerialName("stop_battery_percent") val stopBatteryPercent: Int = 20,
)

@Serializable data class TogglePayload(val enabled: Boolean)
@Serializable data class GameAddPayload(
    @SerialName("package") val packageName: String,
    @SerialName("game_driver") val gameDriver: Boolean = false,
    @SerialName("fan_min_level") val fanMinLevel: Int? = null,
    @SerialName("gpu_turbo") val gpuTurbo: Boolean = false,
    val triggers: TriggersConfig? = null,
    @SerialName("split_charge") val splitCharge: SplitChargeConfig = SplitChargeConfig(),
    @SerialName("disable_thermal_limit") val disableThermalLimit: Boolean = false,
)
@Serializable data class GameRemovePayload(@SerialName("package") val packageName: String)
@Serializable data class GameSetDriverPayload(@SerialName("package") val packageName: String, @SerialName("game_driver") val gameDriver: Boolean)
@Serializable data class GameSetGpuTurboPayload(@SerialName("package") val packageName: String, @SerialName("gpu_turbo") val gpuTurbo: Boolean)
@Serializable data class GameSetFanMinPayload(@SerialName("package") val packageName: String, @SerialName("fan_min_level") val fanMinLevel: Int)
@Serializable data class GameSetTriggersPayload(@SerialName("package") val packageName: String, val triggers: TriggersConfig)
@Serializable data class TriggerPreviewPayload(@SerialName("package") val packageName: String? = null, val triggers: TriggersConfig)
@Serializable data class GameSetSplitChargePayload(@SerialName("package") val packageName: String, @SerialName("split_charge") val splitCharge: SplitChargeConfig)
@Serializable data class GameSetDisableThermalLimitPayload(@SerialName("package") val packageName: String, @SerialName("disable_thermal_limit") val disableThermalLimit: Boolean)

@Serializable data class UiSavePayload(val charging: UiCharging, val notifications: NotificationsConfig, val profiles: UiProfiles)
@Serializable data class UiCharging(val enabled: Boolean, @SerialName("fan_enabled") val fanEnabled: Boolean, @SerialName("fan_led") val fanLed: FanLedSetting)
@Serializable data class UiProfiles(val normal: UiProfile, val gaming: UiProfile)
@Serializable data class UiProfile(
    val enabled: Boolean,
    @SerialName("fan_enabled") val fanEnabled: Boolean,
    @SerialName("fan_led") val fanLed: FanLedSetting,
    @SerialName("ext_enabled") val extEnabled: Boolean,
    @SerialName("external_led") val externalLed: ExternalLedSetting,
)
