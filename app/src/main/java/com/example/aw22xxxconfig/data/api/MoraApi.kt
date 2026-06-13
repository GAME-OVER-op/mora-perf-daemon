package com.example.aw22xxxconfig.data.api

import com.example.aw22xxxconfig.data.model.*
import retrofit2.http.Body
import retrofit2.http.GET
import retrofit2.http.POST

interface MoraApi {
    @GET("api/state") suspend fun getState(): StateResponse
    @GET("api/config") suspend fun getConfig(): UserConfig
    @GET("api/games") suspend fun getGames(): GamesFile
    @GET("api/daemon_notifications") suspend fun getDaemonNotifications(): TogglePayload
    @GET("api/battery_saver") suspend fun getBatterySaver(): TogglePayload
    @GET("api/use_phone_cooler") suspend fun getUsePhoneCooler(): TogglePayload

    @POST("api/daemon_notifications") suspend fun setDaemonNotifications(@Body payload: TogglePayload)
    @POST("api/battery_saver") suspend fun setBatterySaver(@Body payload: TogglePayload)
    @POST("api/use_phone_cooler") suspend fun setUsePhoneCooler(@Body payload: TogglePayload)
    @POST("api/games/add") suspend fun addGame(@Body payload: GameAddPayload)
    @POST("api/games/remove") suspend fun removeGame(@Body payload: GameRemovePayload)
    @POST("api/games/set_driver") suspend fun setGameDriver(@Body payload: GameSetDriverPayload)
    @POST("api/games/set_gpu_turbo") suspend fun setGameGpuTurbo(@Body payload: GameSetGpuTurboPayload)
    @POST("api/games/set_fan_min") suspend fun setGameFanMin(@Body payload: GameSetFanMinPayload)
    @POST("api/games/set_triggers") suspend fun setGameTriggers(@Body payload: GameSetTriggersPayload)
    @POST("api/games/set_split_charge") suspend fun setGameSplitCharge(@Body payload: GameSetSplitChargePayload)
    @POST("api/games/set_disable_thermal_limit") suspend fun setGameDisableThermalLimit(@Body payload: GameSetDisableThermalLimitPayload)
    @POST("api/save") suspend fun saveConfig(@Body payload: UiSavePayload)
}
