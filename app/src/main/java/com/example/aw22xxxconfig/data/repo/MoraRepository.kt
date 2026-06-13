package com.example.aw22xxxconfig.data.repo

import com.example.aw22xxxconfig.data.api.ApiFactory
import com.example.aw22xxxconfig.data.api.MoraApi
import com.example.aw22xxxconfig.data.model.*
import com.example.aw22xxxconfig.data.root.RootHttpClient
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.json.Json

class MoraRepository(
    private val json: Json = Json {
        ignoreUnknownKeys = true
        explicitNulls = false
        encodeDefaults = true
    },
    private val rootHttpClient: RootHttpClient = RootHttpClient(),
) {
    private var api: MoraApi? = null
    private var token: String? = null

    fun connect(token: String) {
        this.token = token
        api = ApiFactory.create(token)
    }

    private fun client(): MoraApi = requireNotNull(api) { "API is not initialized" }
    private fun requireToken(): String = requireNotNull(token) { "API token is not initialized" }

    suspend fun state(): StateResponse = tryLocalThenRoot(
        local = { client().getState() },
        root = { json.decodeFromString(rootHttpClient.get("api/state", requireToken())) }
    )

    suspend fun config(): UserConfig = tryLocalThenRoot(
        local = { client().getConfig() },
        root = { json.decodeFromString(rootHttpClient.get("api/config", requireToken())) }
    )

    suspend fun games(): GamesFile = tryLocalThenRoot(
        local = { client().getGames() },
        root = { json.decodeFromString(rootHttpClient.get("api/games", requireToken())) }
    )

    suspend fun setDaemonNotifications(enabled: Boolean) = tryLocalThenRoot(
        local = { client().setDaemonNotifications(TogglePayload(enabled)) },
        root = { rootHttpClient.post("api/daemon_notifications", requireToken(), TogglePayload(enabled)); Unit }
    )

    suspend fun setBatterySaver(enabled: Boolean) = tryLocalThenRoot(
        local = { client().setBatterySaver(TogglePayload(enabled)) },
        root = { rootHttpClient.post("api/battery_saver", requireToken(), TogglePayload(enabled)); Unit }
    )

    suspend fun setUsePhoneCooler(enabled: Boolean) = tryLocalThenRoot(
        local = { client().setUsePhoneCooler(TogglePayload(enabled)) },
        root = { rootHttpClient.post("api/use_phone_cooler", requireToken(), TogglePayload(enabled)); Unit }
    )

    suspend fun addGame(payload: GameAddPayload) = tryLocalThenRoot(
        local = { client().addGame(payload) },
        root = { rootHttpClient.post("api/games/add", requireToken(), payload); Unit }
    )

    suspend fun removeGame(packageName: String) = tryLocalThenRoot(
        local = { client().removeGame(GameRemovePayload(packageName)) },
        root = { rootHttpClient.post("api/games/remove", requireToken(), GameRemovePayload(packageName)); Unit }
    )

    suspend fun setDriver(packageName: String, enabled: Boolean) = tryLocalThenRoot(
        local = { client().setGameDriver(GameSetDriverPayload(packageName, enabled)) },
        root = { rootHttpClient.post("api/games/set_driver", requireToken(), GameSetDriverPayload(packageName, enabled)); Unit }
    )

    suspend fun setGpuTurbo(packageName: String, enabled: Boolean) = tryLocalThenRoot(
        local = { client().setGameGpuTurbo(GameSetGpuTurboPayload(packageName, enabled)) },
        root = { rootHttpClient.post("api/games/set_gpu_turbo", requireToken(), GameSetGpuTurboPayload(packageName, enabled)); Unit }
    )

    suspend fun setFanMin(packageName: String, level: Int) = tryLocalThenRoot(
        local = { client().setGameFanMin(GameSetFanMinPayload(packageName, level)) },
        root = { rootHttpClient.post("api/games/set_fan_min", requireToken(), GameSetFanMinPayload(packageName, level)); Unit }
    )

    suspend fun setTriggers(packageName: String, triggers: TriggersConfig) = tryLocalThenRoot(
        local = { client().setGameTriggers(GameSetTriggersPayload(packageName, triggers)) },
        root = { rootHttpClient.post("api/games/set_triggers", requireToken(), GameSetTriggersPayload(packageName, triggers)); Unit }
    )

    suspend fun setSplitCharge(packageName: String, splitCharge: SplitChargeConfig) = tryLocalThenRoot(
        local = { client().setGameSplitCharge(GameSetSplitChargePayload(packageName, splitCharge)) },
        root = { rootHttpClient.post("api/games/set_split_charge", requireToken(), GameSetSplitChargePayload(packageName, splitCharge)); Unit }
    )

    suspend fun setDisableThermalLimit(packageName: String, enabled: Boolean) = tryLocalThenRoot(
        local = { client().setGameDisableThermalLimit(GameSetDisableThermalLimitPayload(packageName, enabled)) },
        root = { rootHttpClient.post("api/games/set_disable_thermal_limit", requireToken(), GameSetDisableThermalLimitPayload(packageName, enabled)); Unit }
    )

    suspend fun saveConfig(payload: UiSavePayload) = tryLocalThenRoot(
        local = { client().saveConfig(payload) },
        root = { rootHttpClient.post("api/save", requireToken(), payload); Unit }
    )

    private suspend fun <T> tryLocalThenRoot(local: suspend () -> T, root: suspend () -> T): T {
        return try {
            local()
        } catch (_: Throwable) {
            root()
        }
    }
}
