package com.example.aw22xxxconfig.ui.screens

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import com.example.aw22xxxconfig.MoraViewModel
import com.example.aw22xxxconfig.data.model.TriggerPoint
import com.example.aw22xxxconfig.data.model.TriggersConfig
import com.example.aw22xxxconfig.ui.components.MoraCard
import kotlinx.coroutines.delay
import kotlin.math.roundToInt

@Composable
fun GameDetailScreen(viewModel: MoraViewModel, packageName: String, onBack: () -> Unit) {
    val games by viewModel.games.collectAsState()
    val state by viewModel.state.collectAsState()
    val app = viewModel.appForPackage(packageName)
    val game = games.firstOrNull { it.packageName == packageName }

    if (game == null) {
        ErrorScreen(message = "Game not found", onRetry = onBack)
        return
    }

    var splitChargePercent by remember(game.packageName, game.splitCharge.stopBatteryPercent) {
        mutableFloatStateOf(game.splitCharge.stopBatteryPercent.toFloat())
    }
    var showTriggerEditor by remember { mutableStateOf(false) }
    var editorLeft by remember(game.packageName) { mutableStateOf(Offset(180f, 520f)) }
    var editorRight by remember(game.packageName) { mutableStateOf(Offset(936f, 520f)) }

    LaunchedEffect(showTriggerEditor, packageName, editorLeft, editorRight) {
        if (!showTriggerEditor) return@LaunchedEffect
        val preview = TriggersConfig(
            enabled = true,
            left = TriggerPoint(enabled = true, x = editorLeft.x.roundToInt().coerceAtLeast(0), y = editorLeft.y.roundToInt().coerceAtLeast(0)),
            right = TriggerPoint(enabled = true, x = editorRight.x.roundToInt().coerceAtLeast(0), y = editorRight.y.roundToInt().coerceAtLeast(0)),
        )
        viewModel.setTriggerPreview(packageName, preview)
    }

    LaunchedEffect(showTriggerEditor) {
        if (!showTriggerEditor) return@LaunchedEffect
        while (showTriggerEditor) {
            viewModel.refreshStateOnly()
            delay(140)
        }
    }

    Column(
        modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
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
                            viewModel.setGameSplitCharge(packageName, splitCharge.copy(stopBatteryPercent = percent))
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
            FilledTonalButton(onClick = {
                editorLeft = Offset(triggers.left.x.ifZero(180).toFloat(), triggers.left.y.ifZero(520).toFloat())
                editorRight = Offset(triggers.right.x.ifZero(936).toFloat(), triggers.right.y.ifZero(520).toFloat())
                showTriggerEditor = true
            }) { Text("Configure trigger points") }
            TriggerSummary("Left trigger", triggers.left, onPick = {
                editorLeft = Offset(triggers.left.x.ifZero(180).toFloat(), triggers.left.y.ifZero(520).toFloat())
                editorRight = Offset(triggers.right.x.ifZero(936).toFloat(), triggers.right.y.ifZero(520).toFloat())
                showTriggerEditor = true
            }) {
                viewModel.setGameTriggers(packageName, triggers.copy(left = TriggerPoint(enabled = false, x = triggers.left.x, y = triggers.left.y)))
            }
            TriggerSummary("Right trigger", triggers.right, onPick = {
                editorLeft = Offset(triggers.left.x.ifZero(180).toFloat(), triggers.left.y.ifZero(520).toFloat())
                editorRight = Offset(triggers.right.x.ifZero(936).toFloat(), triggers.right.y.ifZero(520).toFloat())
                showTriggerEditor = true
            }) {
                viewModel.setGameTriggers(packageName, triggers.copy(right = TriggerPoint(enabled = false, x = triggers.right.x, y = triggers.right.y)))
            }
        }
    }

    if (showTriggerEditor) {
        TriggerPickerFullscreen(
            initialLeft = editorLeft,
            initialRight = editorRight,
            leftPressed = state.triggers.leftPressed,
            rightPressed = state.triggers.rightPressed,
            onLeftChanged = { editorLeft = it },
            onRightChanged = { editorRight = it },
            onDismiss = {
                showTriggerEditor = false
                viewModel.clearTriggerPreview()
            },
            onSave = { left, right ->
                val triggers = game.triggers ?: TriggersConfig(enabled = true)
                val updated = triggers.copy(
                    enabled = true,
                    left = TriggerPoint(true, left.x.roundToInt().coerceAtLeast(0), left.y.roundToInt().coerceAtLeast(0)),
                    right = TriggerPoint(true, right.x.roundToInt().coerceAtLeast(0), right.y.roundToInt().coerceAtLeast(0)),
                )
                viewModel.clearTriggerPreview()
                viewModel.setGameTriggers(packageName, updated)
                showTriggerEditor = false
            }
        )
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

@Composable
private fun TriggerPickerFullscreen(
    initialLeft: Offset,
    initialRight: Offset,
    leftPressed: Boolean,
    rightPressed: Boolean,
    onLeftChanged: (Offset) -> Unit,
    onRightChanged: (Offset) -> Unit,
    onDismiss: () -> Unit,
    onSave: (Offset, Offset) -> Unit,
) {
    var left by remember(initialLeft) { mutableStateOf(initialLeft) }
    var right by remember(initialRight) { mutableStateOf(initialRight) }
    var dragTarget by remember { mutableStateOf<TriggerHandle?>(null) }

    LaunchedEffect(left) { onLeftChanged(left) }
    LaunchedEffect(right) { onRightChanged(right) }

    Dialog(
        onDismissRequest = onDismiss,
        properties = DialogProperties(usePlatformDefaultWidth = false, dismissOnClickOutside = false)
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(Color(0xF0050506))
                .pointerInput(Unit) {
                    fun clampPoint(p: Offset): Offset = Offset(
                        p.x.coerceIn(0f, size.width.toFloat().coerceAtLeast(1f)),
                        p.y.coerceIn(0f, size.height.toFloat().coerceAtLeast(1f)),
                    )
                    detectDragGestures(
                        onDragStart = { pos -> dragTarget = pickTriggerHandle(pos, left, right) },
                        onDragEnd = { dragTarget = null },
                        onDragCancel = { dragTarget = null },
                        onDrag = { change, dragAmount ->
                            change.consume()
                            when (dragTarget) {
                                TriggerHandle.LEFT -> left = clampPoint(left + dragAmount)
                                TriggerHandle.RIGHT -> right = clampPoint(right + dragAmount)
                                null -> Unit
                            }
                        }
                    )
                }
        ) {
            Canvas(modifier = Modifier.fillMaxSize()) {
                val h = size.height
                val w = size.width
                val blue = if (leftPressed) Color(0xFF86B7FF) else Color(0xFF3A86FF)
                val red = if (rightPressed) Color(0xFFFF8BA0) else Color(0xFFFF3B5C)
                drawRoundRect(blue.copy(alpha = if (leftPressed) 0.85f else 0.35f), topLeft = Offset(18f, h * 0.18f), size = Size(28f, h * 0.64f), cornerRadius = androidx.compose.ui.geometry.CornerRadius(18f, 18f))
                drawRoundRect(red.copy(alpha = if (rightPressed) 0.85f else 0.35f), topLeft = Offset(w - 46f, h * 0.18f), size = Size(28f, h * 0.64f), cornerRadius = androidx.compose.ui.geometry.CornerRadius(18f, 18f))
                drawLine(blue.copy(alpha = if (leftPressed) 1f else 0.55f), Offset(44f, left.y), Offset(left.x, left.y), strokeWidth = if (leftPressed) 14f else 8f, cap = StrokeCap.Round)
                drawLine(red.copy(alpha = if (rightPressed) 1f else 0.55f), Offset(w - 44f, right.y), Offset(right.x, right.y), strokeWidth = if (rightPressed) 14f else 8f, cap = StrokeCap.Round)
                drawTriggerPoint(left, blue, leftPressed)
                drawTriggerPoint(right, red, rightPressed)
            }

            Column(
                modifier = Modifier
                    .align(Alignment.TopCenter)
                    .padding(16.dp)
                    .background(Color(0xCC111318), RoundedCornerShape(18.dp))
                    .padding(horizontal = 18.dp, vertical = 12.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Text("Trigger coordinates", style = MaterialTheme.typography.titleMedium)
                Text("Press the shoulder trigger: matching bar and point will light up.", color = MaterialTheme.colorScheme.onSurfaceVariant)
                Text(
                    when {
                        leftPressed && rightPressed -> "Both triggers pressed"
                        leftPressed -> "Left trigger pressed"
                        rightPressed -> "Right trigger pressed"
                        else -> "Waiting for trigger press"
                    },
                    color = if (leftPressed || rightPressed) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            Row(
                modifier = Modifier.align(Alignment.BottomCenter).padding(18.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Button(onClick = onDismiss, colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF2B2B33))) { Text("Cancel") }
                Button(onClick = { onSave(left, right) }) { Text("Save") }
            }
        }
    }
}

private enum class TriggerHandle { LEFT, RIGHT }

private fun pickTriggerHandle(touch: Offset, left: Offset, right: Offset): TriggerHandle {
    val leftDistance = distance(touch, left)
    val rightDistance = distance(touch, right)
    val grabRadius = 150f
    return when {
        leftDistance <= grabRadius && leftDistance <= rightDistance -> TriggerHandle.LEFT
        rightDistance <= grabRadius -> TriggerHandle.RIGHT
        leftDistance <= rightDistance -> TriggerHandle.LEFT
        else -> TriggerHandle.RIGHT
    }
}

private fun distance(a: Offset, b: Offset): Float {
    val dx = a.x - b.x
    val dy = a.y - b.y
    return kotlin.math.sqrt(dx * dx + dy * dy)
}

private fun androidx.compose.ui.graphics.drawscope.DrawScope.drawTriggerPoint(center: Offset, color: Color, active: Boolean) {
    drawCircle(color.copy(alpha = if (active) 1f else 0.95f), radius = if (active) 52f else 42f, center = center)
    drawCircle(Color.Black.copy(alpha = 0.35f), radius = if (active) 52f else 42f, center = center, style = Stroke(width = if (active) 6f else 4f))
    drawLine(Color.White, Offset(center.x - 20f, center.y), Offset(center.x + 20f, center.y), strokeWidth = if (active) 7f else 5f, cap = StrokeCap.Round)
    drawLine(Color.White, Offset(center.x, center.y - 20f), Offset(center.x, center.y + 20f), strokeWidth = if (active) 7f else 5f, cap = StrokeCap.Round)
}
