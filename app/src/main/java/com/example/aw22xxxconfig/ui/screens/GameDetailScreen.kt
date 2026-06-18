package com.example.aw22xxxconfig.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.platform.LocalContext
import android.app.Activity
import android.content.Intent
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import com.example.aw22xxxconfig.TriggerOverlayActivity
import com.example.aw22xxxconfig.MoraViewModel
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

    val context = LocalContext.current
    var splitChargePercent by remember(game.packageName, game.splitCharge.stopBatteryPercent) {
        mutableFloatStateOf(game.splitCharge.stopBatteryPercent.toFloat())
    }

    val triggerLauncher = rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
        if (result.resultCode == Activity.RESULT_OK) {
            val data = result.data ?: return@rememberLauncherForActivityResult
            val current = game.triggers ?: TriggersConfig(enabled = true)
            val updated = current.copy(
                enabled = true,
                left = TriggerPoint(
                    enabled = true,
                    x = data.getIntExtra(TriggerOverlayActivity.EXTRA_LEFT_X, current.left.x),
                    y = data.getIntExtra(TriggerOverlayActivity.EXTRA_LEFT_Y, current.left.y),
                ),
                right = TriggerPoint(
                    enabled = true,
                    x = data.getIntExtra(TriggerOverlayActivity.EXTRA_RIGHT_X, current.right.x),
                    y = data.getIntExtra(TriggerOverlayActivity.EXTRA_RIGHT_Y, current.right.y),
                ),
            )
            viewModel.setGameTriggers(packageName, updated)
        }
    }

    fun openTriggerEditor() {
        val triggers = game.triggers ?: TriggersConfig(enabled = true)
        triggerLauncher.launch(Intent(context, TriggerOverlayActivity::class.java).apply {
            putExtra(TriggerOverlayActivity.EXTRA_LEFT_X, triggers.left.x.ifZero(180))
            putExtra(TriggerOverlayActivity.EXTRA_LEFT_Y, triggers.left.y.ifZero(520))
            putExtra(TriggerOverlayActivity.EXTRA_RIGHT_X, triggers.right.x.ifZero(936))
            putExtra(TriggerOverlayActivity.EXTRA_RIGHT_Y, triggers.right.y.ifZero(520))
        })
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
            FilledTonalButton(onClick = { openTriggerEditor() }) { Text("Configure trigger points") }
            TriggerSummary("Left trigger", triggers.left, onPick = { openTriggerEditor() }) {
                viewModel.setGameTriggers(packageName, triggers.copy(left = TriggerPoint(enabled = false, x = triggers.left.x, y = triggers.left.y)))
            }
            TriggerSummary("Right trigger", triggers.right, onPick = { openTriggerEditor() }) {
                viewModel.setGameTriggers(packageName, triggers.copy(right = TriggerPoint(enabled = false, x = triggers.right.x, y = triggers.right.y)))
            }
        }
    }
}

private fun Int.ifZero(default: Int): Int = if (this == 0) default else this

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
