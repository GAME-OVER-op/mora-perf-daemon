package com.example.aw22xxxconfig.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.runtime.Composable

private val MoraColorScheme = darkColorScheme(
    primary = MoraRed,
    secondary = MoraBlue,
    tertiary = MoraYellow,
    background = MoraBackground,
    surface = MoraSurface,
    surfaceContainer = MoraSurfaceAlt,
)

@Composable
fun MoraTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = MoraColorScheme,
        typography = Typography,
        content = content,
    )
}
