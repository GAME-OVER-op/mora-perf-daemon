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

enum class MoraScreen(val route: String) {
    HOME("home"),
    PROFILES("profiles"),
    GAMES("games"),
    LED("led"),
    SETTINGS("settings"),
    GAME_DETAIL("games/detail/{packageName}");

    fun detailRoute(packageName: String) = "games/detail/$packageName"
}
