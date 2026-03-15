package com.example.aw22xxxconfig.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.example.aw22xxxconfig.MoraViewModel
import com.example.aw22xxxconfig.data.model.*
import com.example.aw22xxxconfig.ui.components.MoraCard
import com.example.aw22xxxconfig.ui.components.SectionHeader

@Composable
fun ProfilesScreen(viewModel: MoraViewModel) {
    val config by viewModel.config.collectAsState()
    val normal = config.profiles.firstOrNull { it.profileType == ProfileType.NORMAL }
    val gaming = config.profiles.firstOrNull { it.profileType == ProfileType.GAMING }

    Column(
        modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        SectionHeader("Profiles", "Auto-saved profile and notification settings")
        if (normal != null && gaming != null) {
            ProfileEditorCard("Normal profile", normal, onChanged = { edited ->
                viewModel.saveProfiles(buildSavePayload(config, edited, gaming))
            })
            ProfileEditorCard("Gaming profile", gaming, onChanged = { edited ->
                viewModel.saveProfiles(buildSavePayload(config, normal, edited))
            })
        }
        MoraCard("Charging") {
            var enabled by remember(config) { mutableStateOf(config.charging.enabled) }
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Column(Modifier.weight(1f)) {
                    Text("Charging logic")
                    Text("Enable charging-aware profile behavior", color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Switch(checked = enabled, onCheckedChange = {
                    enabled = it
                    viewModel.saveProfiles(
                        UiSavePayload(
                            charging = UiCharging(it, config.charging.fanLed != null, config.charging.fanLed ?: FanLedSetting(FanLedMode.STEADY, FanLedColor.ROSE)),
                            notifications = config.notifications,
                            profiles = UiProfiles(
                                normal = normal?.toUiProfile() ?: UiProfile(true, false, FanLedSetting(), false, ExternalLedSetting()),
                                gaming = gaming?.toUiProfile() ?: UiProfile(true, false, FanLedSetting(), false, ExternalLedSetting()),
                            )
                        )
                    )
                })
            }
        }
        MoraCard("Notifications") {
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Column(Modifier.weight(1f)) {
                    Text("Daemon notification LED")
                    Text("Controlled through /api/save", color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Switch(checked = config.notifications.enabled, onCheckedChange = { enabled ->
                    viewModel.saveProfiles(
                        UiSavePayload(
                            charging = UiCharging(config.charging.enabled, config.charging.fanLed != null, config.charging.fanLed ?: FanLedSetting()),
                            notifications = config.notifications.copy(enabled = enabled),
                            profiles = UiProfiles(
                                normal = normal?.toUiProfile() ?: UiProfile(true, false, FanLedSetting(), false, ExternalLedSetting()),
                                gaming = gaming?.toUiProfile() ?: UiProfile(true, false, FanLedSetting(), false, ExternalLedSetting()),
                            )
                        )
                    )
                })
            }
        }
    }
}

@Composable
private fun ProfileEditorCard(title: String, profile: ProfileConfig, onChanged: (ProfileConfig) -> Unit) {
    var enabled by remember(profile) { mutableStateOf(profile.enabled) }
    var fanEnabled by remember(profile) { mutableStateOf(profile.fanLed != null) }
    var extEnabled by remember(profile) { mutableStateOf(profile.externalLed != null) }

    MoraCard(title) {
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text("Enabled")
            Switch(checked = enabled, onCheckedChange = {
                enabled = it
                onChanged(profile.copy(enabled = it))
            })
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text("Fan LED")
            Switch(checked = fanEnabled, onCheckedChange = {
                fanEnabled = it
                onChanged(profile.copy(fanLed = if (it) profile.fanLed ?: FanLedSetting(FanLedMode.BREATHE, FanLedColor.ROSE) else null))
            })
        }
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            Text("External LED")
            Switch(checked = extEnabled, onCheckedChange = {
                extEnabled = it
                onChanged(profile.copy(externalLed = if (it) profile.externalLed ?: ExternalLedSetting(ExternalLedMode.STEADY, ExternalLedColor.RED) else null))
            })
        }
    }
}

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
