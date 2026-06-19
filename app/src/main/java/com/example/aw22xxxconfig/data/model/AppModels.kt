package com.example.aw22xxxconfig.data.model

import android.graphics.drawable.Drawable

sealed interface ConnectionState {
    data object Loading : ConnectionState
    data class Ready(val tokenSource: String) : ConnectionState
    data class Error(val message: String) : ConnectionState
}

data class InstalledApp(
    val label: String,
    val packageName: String,
    val icon: Drawable? = null,
)

data class DebloatPackage(
    val packageName: String,
    val title: String,
)

data class DebloatPackageState(
    val item: DebloatPackage,
    val installed: Boolean = false,
    val enabled: Boolean = false,
    val error: String? = null,
)

data class FlashImageState(
    val loading: Boolean = false,
    val message: String? = null,
)

enum class MoraScreen(val route: String) {
    HOME("home"),
    PROFILES("profiles"),
    GAMES("games"),
    LED("led"),
    SETTINGS("settings"),
    DEBLOAT("settings/debloat"),
    FLASH_OC("settings/flash-oc"),
    FLASH_ORANGEFOX("settings/flash-orangefox"),
    GAME_DETAIL("games/detail/{packageName}");

    fun detailRoute(packageName: String) = "games/detail/$packageName"
}
