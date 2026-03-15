package com.example.aw22xxxconfig.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.ArrowBack
import androidx.compose.material3.*
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.ui.unit.dp
import com.example.aw22xxxconfig.MoraViewModel
import com.example.aw22xxxconfig.data.model.GameEntry
import com.example.aw22xxxconfig.data.model.TriggerPoint
import com.example.aw22xxxconfig.data.model.TriggersConfig
import com.example.aw22xxxconfig.ui.components.MoraCard
import kotlin.math.roundToInt

@Composable
fun GameDetailScreen(viewModel: MoraViewModel, packageName: String, onBack: () -> Unit) {
    val games by viewModel.games.collectAsState()
    val app = viewModel.appForPackage(packageName)
    val game = games.firstOrNull { it.packageName == packageName }

    if (game == null) {
        ErrorScreen(message = "Game not found", onRetry = onBack)
        return
    }

    var triggerDialog by remember { mutableStateOf<String?>(null) }
    var splitChargePercent by remember(game.packageName, game.splitCharge.stopBatteryPercent) {
        mutableFloatStateOf(game.splitCharge.stopBatteryPercent.toFloat())
    }

    Column(modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            IconButton(onClick = onBack) { Icon(Icons.Rounded.ArrowBack, contentDescription = null) }
            Column {
                Text(app?.label ?: packageName, style = MaterialTheme.typography.headlineSmall)
                Text(packageName, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        MoraCard("Game flags") {
            SettingRow("Game driver", game.gameDriver) { viewModel.setGameDriver(packageName, it) }
            SettingRow("GPU turbo", game.gpuTurbo) { viewModel.setGameGpuTurbo(packageName, it) }
            SettingRow("Disable thermal limit", game.disableThermalLimit) {
                viewModel.setGameDisableThermalLimit(packageName, it)
            }
            Text(
                if (game.disableThermalLimit) {
                    "Performance will not be reduced by Mora temperature caps while this game is active."
                } else {
                    "Mora thermal reduction stays active for this game."
                },
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                style = MaterialTheme.typography.bodySmall,
            )
            Text("Fan minimum level: ${game.fanMinLevel ?: 2}")
            Slider(
                value = (game.fanMinLevel ?: 2).toFloat(),
                onValueChange = { viewModel.setGameFanMin(packageName, it.roundToInt().coerceIn(2, 5)) },
                valueRange = 2f..5f,
                steps = 2,
            )
        }
        MoraCard("Split charge") {
            val splitCharge = game.splitCharge
            SettingRow("Enable split charge", splitCharge.enabled) {
                viewModel.setGameSplitCharge(packageName, splitCharge.copy(enabled = it))
            }
            Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text("Stop battery charging above ${splitChargePercent.roundToInt()}%")
                Slider(
                    value = splitChargePercent,
                    onValueChange = { splitChargePercent = it.coerceIn(0f, 100f) },
                    onValueChangeFinished = {
                        val percent = splitChargePercent.roundToInt().coerceIn(0, 100)
                        if (percent != splitCharge.stopBatteryPercent) {
                            viewModel.setGameSplitCharge(
                                packageName,
                                splitCharge.copy(stopBatteryPercent = percent)
                            )
                        }
                    },
                    valueRange = 0f..100f,
                    steps = 99,
                )
                Text(
                    "When the game is running and battery percent is higher than this value, the daemon will stop charging and keep rechecking the mode every 90 seconds.",
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }
        MoraCard("Triggers") {
            val triggers = game.triggers ?: TriggersConfig()
            SettingRow("Enable trigger mode", triggers.enabled) {
                viewModel.setGameTriggers(packageName, triggers.copy(enabled = it))
            }
            TriggerSummary("Left trigger", triggers.left, onPick = { triggerDialog = "left" }) {
                viewModel.setGameTriggers(packageName, triggers.copy(left = TriggerPoint(enabled = false, x = triggers.left.x, y = triggers.left.y)))
            }
            TriggerSummary("Right trigger", triggers.right, onPick = { triggerDialog = "right" }) {
                viewModel.setGameTriggers(packageName, triggers.copy(right = TriggerPoint(enabled = false, x = triggers.right.x, y = triggers.right.y)))
            }
        }
    }

    if (triggerDialog != null) {
        TriggerPickerFullscreen(
            title = if (triggerDialog == "left") "Left trigger" else "Right trigger",
            onDismiss = { triggerDialog = null },
            onPicked = { x, y ->
                val current = game.triggers ?: TriggersConfig(enabled = true)
                val updated = if (triggerDialog == "left") {
                    current.copy(enabled = true, left = TriggerPoint(true, x, y))
                } else {
                    current.copy(enabled = true, right = TriggerPoint(true, x, y))
                }
                viewModel.setGameTriggers(packageName, updated)
                triggerDialog = null
            }
        )
    }
}

@Composable
private fun SettingRow(label: String, checked: Boolean, onChanged: (Boolean) -> Unit) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        Text(label)
        Switch(checked = checked, onCheckedChange = onChanged)
    }
}

@Composable
private fun TriggerSummary(title: String, point: TriggerPoint, onPick: () -> Unit, onDisable: () -> Unit) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text(title, style = MaterialTheme.typography.titleMedium)
        Text("x=${point.x}, y=${point.y}, enabled=${point.enabled}", color = MaterialTheme.colorScheme.onSurfaceVariant)
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            FilledTonalButton(onClick = onPick) { Text("Pick") }
            AssistChip(onClick = onDisable, label = { Text("Disable") })
        }
    }
}

@Composable
private fun TriggerPickerFullscreen(title: String, onDismiss: () -> Unit, onPicked: (Int, Int) -> Unit) {
    Dialog(
        onDismissRequest = onDismiss,
        properties = DialogProperties(usePlatformDefaultWidth = false, dismissOnClickOutside = false)
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(Color(0xFF090B10))
                .pointerInput(Unit) {
                    detectTapGestures { offset ->
                        val x = (offset.x / size.width.toFloat() * 1116f).roundToInt().coerceAtLeast(0)
                        val y = (offset.y / size.height.toFloat() * 2480f).roundToInt().coerceAtLeast(0)
                        onPicked(x, y)
                    }
                }
        ) {
            Column(
                modifier = Modifier
                    .align(Alignment.Center)
                    .padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Text(title, style = MaterialTheme.typography.headlineSmall)
                Text(
                    "Tap the point where the trigger should press.",
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
                Text(
                    "The whole screen is active.",
                    color = MaterialTheme.colorScheme.onSurfaceVariant
                )
            }

            TextButton(
                onClick = onDismiss,
                modifier = Modifier
                    .align(Alignment.TopStart)
                    .padding(12.dp)
            ) {
                Text("Cancel")
            }
        }
    }
}
