package com.example.aw22xxxconfig.ui.screens

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.example.aw22xxxconfig.MoraViewModel
import com.example.aw22xxxconfig.data.model.*
import com.example.aw22xxxconfig.ui.components.ColorDot
import com.example.aw22xxxconfig.ui.components.MoraCard
import com.example.aw22xxxconfig.ui.components.SectionHeader

@Composable
fun LedScreen(viewModel: MoraViewModel) {
    val config by viewModel.config.collectAsState()
    val normal = config.profiles.firstOrNull { it.profileType == ProfileType.NORMAL }
    val gaming = config.profiles.firstOrNull { it.profileType == ProfileType.GAMING }

    Column(
        modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        SectionHeader("LED", "Change supported lighting settings and save them instantly")

        if (normal != null && gaming != null) {
            FanLedEditorCard(
                title = "Charging fan LED",
                enabled = config.charging.fanLed != null,
                current = config.charging.fanLed ?: FanLedSetting(FanLedMode.STEADY, FanLedColor.ROSE),
                onSave = { enabled, fanLed ->
                    viewModel.saveProfiles(
                        UiSavePayload(
                            charging = UiCharging(config.charging.enabled, enabled, fanLed),
                            notifications = config.notifications,
                            profiles = UiProfiles(normal = normal.toUiProfile(), gaming = gaming.toUiProfile()),
                        )
                    )
                }
            )

            FanLedEditorCard(
                title = "Normal profile fan LED",
                enabled = normal.fanLed != null,
                current = normal.fanLed ?: FanLedSetting(FanLedMode.BREATHE, FanLedColor.ROSE),
                onSave = { enabled, fanLed ->
                    viewModel.saveProfiles(buildSavePayload(config, normal.copy(fanLed = if (enabled) fanLed else null), gaming))
                }
            )

            FanLedEditorCard(
                title = "Gaming profile fan LED",
                enabled = gaming.fanLed != null,
                current = gaming.fanLed ?: FanLedSetting(FanLedMode.FLOW, FanLedColor.MIXED7),
                onSave = { enabled, fanLed ->
                    viewModel.saveProfiles(buildSavePayload(config, normal, gaming.copy(fanLed = if (enabled) fanLed else null)))
                }
            )

            ExternalLedEditorCard(
                title = "Notification external LED",
                enabled = config.notifications.enabled,
                current = config.notifications.externalLed,
                onSave = { enabled, externalLed ->
                    viewModel.saveProfiles(
                        UiSavePayload(
                            charging = UiCharging(config.charging.enabled, config.charging.fanLed != null, config.charging.fanLed ?: FanLedSetting()),
                            notifications = config.notifications.copy(enabled = enabled, externalLed = externalLed),
                            profiles = UiProfiles(normal = normal.toUiProfile(), gaming = gaming.toUiProfile()),
                        )
                    )
                }
            )

            ExternalLedEditorCard(
                title = "Normal profile external LED",
                enabled = normal.externalLed != null,
                current = normal.externalLed ?: ExternalLedSetting(ExternalLedMode.STEADY, ExternalLedColor.RED),
                onSave = { enabled, externalLed ->
                    viewModel.saveProfiles(buildSavePayload(config, normal.copy(externalLed = if (enabled) externalLed else null), gaming))
                }
            )

            ExternalLedEditorCard(
                title = "Gaming profile external LED",
                enabled = gaming.externalLed != null,
                current = gaming.externalLed ?: ExternalLedSetting(ExternalLedMode.STEADY, ExternalLedColor.RED),
                onSave = { enabled, externalLed ->
                    viewModel.saveProfiles(buildSavePayload(config, normal, gaming.copy(externalLed = if (enabled) externalLed else null)))
                }
            )
        }
    }
}

@Composable
private fun FanLedEditorCard(
    title: String,
    enabled: Boolean,
    current: FanLedSetting,
    onSave: (Boolean, FanLedSetting) -> Unit,
) {
    MoraCard(title) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Column(Modifier.weight(1f)) {
                Text("Enabled")
                Text("Off mode is hidden as requested", color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Switch(checked = enabled, onCheckedChange = { onSave(it, current) })
        }

        Text("Mode", color = MaterialTheme.colorScheme.onSurfaceVariant)
        EnumModeRow(options = FAN_LED_MODES, selected = current.mode) { selected ->
            onSave(enabled, current.copy(mode = selected))
        }

        Text("Color", color = MaterialTheme.colorScheme.onSurfaceVariant)
        FanColorRow(selected = current.color) { color ->
            onSave(enabled, current.copy(color = color))
        }
    }
}

@Composable
private fun ExternalLedEditorCard(
    title: String,
    enabled: Boolean,
    current: ExternalLedSetting,
    onSave: (Boolean, ExternalLedSetting) -> Unit,
) {
    MoraCard(title) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Column(Modifier.weight(1f)) {
                Text("Enabled")
                Text("Only supported external LED modes are shown", color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
            Switch(checked = enabled, onCheckedChange = { onSave(it, current) })
        }

        Text("Mode", color = MaterialTheme.colorScheme.onSurfaceVariant)
        EnumModeRow(options = ExternalLedMode.entries, selected = current.mode) { selected ->
            onSave(enabled, current.copy(mode = selected))
        }

        Text("Color", color = MaterialTheme.colorScheme.onSurfaceVariant)
        ExternalColorRow(selected = current.color) { color ->
            onSave(enabled, current.copy(color = color))
        }
    }
}

@Composable
private fun <T : Enum<T>> EnumModeRow(
    options: List<T>,
    selected: T,
    onSelected: (T) -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        options.forEach { option ->
            FilterChip(
                selected = option == selected,
                onClick = { onSelected(option) },
                label = { Text(option.name.lowercase().replaceFirstChar { it.uppercase() }) }
            )
        }
    }
}

@Composable
private fun FanColorRow(selected: FanLedColor, onSelected: (FanLedColor) -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(10.dp)
    ) {
        FAN_LED_COLORS.forEach { (color, swatch) ->
            ColorDot(swatch, selected == color) { onSelected(color) }
        }
    }
}

@Composable
private fun ExternalColorRow(selected: ExternalLedColor, onSelected: (ExternalLedColor) -> Unit) {
    Row(
        modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
        horizontalArrangement = Arrangement.spacedBy(10.dp)
    ) {
        EXTERNAL_LED_COLORS.forEach { (color, swatch) ->
            ColorDot(swatch, selected == color) { onSelected(color) }
        }
    }
}

private val FAN_LED_MODES = listOf(FanLedMode.FLOW, FanLedMode.BREATHE, FanLedMode.FLASHING, FanLedMode.STEADY)

private val FAN_LED_COLORS = listOf(
    FanLedColor.ROSE to Color(0xFFFF3B5C),
    FanLedColor.YELLOW to Color(0xFFFFD86B),
    FanLedColor.GREEN to Color(0xFF4CD964),
    FanLedColor.BLUE to Color(0xFF3A86FF),
    FanLedColor.CYAN to Color(0xFF00C2FF),
    FanLedColor.PURPLE to Color(0xFF9B5DE5),
    FanLedColor.ORANGE to Color(0xFFFF914D),
    FanLedColor.MIXED1 to Color(0xFFE056FD),
    FanLedColor.MIXED2 to Color(0xFFFF6B6B),
    FanLedColor.MIXED3 to Color(0xFFFFA94D),
    FanLedColor.MIXED4 to Color(0xFF6BCB77),
    FanLedColor.MIXED5 to Color(0xFF4D96FF),
    FanLedColor.MIXED6 to Color(0xFFB892FF),
    FanLedColor.MIXED7 to Color(0xFFFF3B5C),
)

private val EXTERNAL_LED_COLORS = listOf(
    ExternalLedColor.MULTI to Color(0xFFFF3B5C),
    ExternalLedColor.RED to Color(0xFFFF3B5C),
    ExternalLedColor.YELLOW to Color(0xFFFFD86B),
    ExternalLedColor.BLUE to Color(0xFF3A86FF),
    ExternalLedColor.GREEN to Color(0xFF4CD964),
    ExternalLedColor.CYAN to Color(0xFF00C2FF),
    ExternalLedColor.WHITE to Color(0xFFF5F5F5),
    ExternalLedColor.PURPLE to Color(0xFF9B5DE5),
    ExternalLedColor.PINK to Color(0xFFFF66C4),
    ExternalLedColor.ORANGE to Color(0xFFFF914D),
)

private fun buildSavePayload(config: UserConfig, normal: ProfileConfig, gaming: ProfileConfig): UiSavePayload = UiSavePayload(
    charging = UiCharging(
        enabled = config.charging.enabled,
        fanEnabled = config.charging.fanLed != null,
        fanLed = config.charging.fanLed ?: FanLedSetting(FanLedMode.STEADY, FanLedColor.ROSE),
    ),
    notifications = config.notifications,
    profiles = UiProfiles(normal = normal.toUiProfile(), gaming = gaming.toUiProfile()),
)

private fun ProfileConfig.toUiProfile(): UiProfile = UiProfile(
    enabled = enabled,
    fanEnabled = fanLed != null,
    fanLed = fanLed ?: FanLedSetting(FanLedMode.BREATHE, FanLedColor.ROSE),
    extEnabled = externalLed != null,
    externalLed = externalLed ?: ExternalLedSetting(ExternalLedMode.STEADY, ExternalLedColor.RED),
)
