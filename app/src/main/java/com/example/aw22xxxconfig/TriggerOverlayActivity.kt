package com.example.aw22xxxconfig

import android.app.Activity
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectDragGestures
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.dp
import com.example.aw22xxxconfig.ui.theme.MoraTheme
import kotlin.math.abs
import kotlin.math.roundToInt

class TriggerOverlayActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val leftX = intent.getIntExtra(EXTRA_LEFT_X, 180)
        val leftY = intent.getIntExtra(EXTRA_LEFT_Y, 520)
        val rightX = intent.getIntExtra(EXTRA_RIGHT_X, 936)
        val rightY = intent.getIntExtra(EXTRA_RIGHT_Y, 520)

        setContent {
            MoraTheme {
                TriggerOverlayEditor(
                    initialLeft = Offset(leftX.toFloat(), leftY.toFloat()),
                    initialRight = Offset(rightX.toFloat(), rightY.toFloat()),
                    onCancel = {
                        setResult(Activity.RESULT_CANCELED)
                        finish()
                    },
                    onSave = { left, right ->
                        setResult(Activity.RESULT_OK, android.content.Intent().apply {
                            putExtra(EXTRA_LEFT_X, left.x.roundToInt().coerceAtLeast(0))
                            putExtra(EXTRA_LEFT_Y, left.y.roundToInt().coerceAtLeast(0))
                            putExtra(EXTRA_RIGHT_X, right.x.roundToInt().coerceAtLeast(0))
                            putExtra(EXTRA_RIGHT_Y, right.y.roundToInt().coerceAtLeast(0))
                        })
                        finish()
                    }
                )
            }
        }
    }

    companion object {
        const val EXTRA_LEFT_X = "left_x"
        const val EXTRA_LEFT_Y = "left_y"
        const val EXTRA_RIGHT_X = "right_x"
        const val EXTRA_RIGHT_Y = "right_y"
    }
}

@Composable
private fun TriggerOverlayEditor(
    initialLeft: Offset,
    initialRight: Offset,
    onCancel: () -> Unit,
    onSave: (Offset, Offset) -> Unit,
) {
    val configuration = LocalConfiguration.current
    val density = LocalDensity.current
    val screenW = with(density) { configuration.screenWidthDp.dp.toPx() }
    val screenH = with(density) { configuration.screenHeightDp.dp.toPx() }
    var left by remember { mutableStateOf(initialLeft) }
    var right by remember { mutableStateOf(initialRight) }
    var draggingLeft by remember { mutableStateOf(false) }
    var draggingRight by remember { mutableStateOf(false) }

    fun clampPoint(p: Offset): Offset = Offset(
        p.x.coerceIn(0f, screenW.coerceAtLeast(1f)),
        p.y.coerceIn(0f, screenH.coerceAtLeast(1f)),
    )

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Color(0xF0050506))
            .pointerInput(left, right) {
                detectDragGestures(
                    onDragStart = { pos ->
                        draggingLeft = distance(pos, left) <= 96f || pos.x < size.width / 2f
                        draggingRight = !draggingLeft
                    },
                    onDragEnd = { draggingLeft = false; draggingRight = false },
                    onDragCancel = { draggingLeft = false; draggingRight = false },
                    onDrag = { change, dragAmount ->
                        change.consume()
                        if (draggingLeft) left = clampPoint(left + dragAmount)
                        if (draggingRight) right = clampPoint(right + dragAmount)
                    }
                )
            }
    ) {
        Canvas(modifier = Modifier.fillMaxSize()) {
            val h = size.height
            val w = size.width
            val blue = Color(0xFF3A86FF)
            val red = Color(0xFFFF3B5C)
            drawRoundRect(blue.copy(alpha = 0.35f), topLeft = Offset(18f, h * 0.18f), size = Size(28f, h * 0.64f), cornerRadius = androidx.compose.ui.geometry.CornerRadius(18f, 18f))
            drawRoundRect(red.copy(alpha = 0.35f), topLeft = Offset(w - 46f, h * 0.18f), size = Size(28f, h * 0.64f), cornerRadius = androidx.compose.ui.geometry.CornerRadius(18f, 18f))
            drawLine(blue.copy(alpha = 0.55f), Offset(44f, left.y), Offset(left.x, left.y), strokeWidth = 8f, cap = StrokeCap.Round)
            drawLine(red.copy(alpha = 0.55f), Offset(w - 44f, right.y), Offset(right.x, right.y), strokeWidth = 8f, cap = StrokeCap.Round)
            drawTriggerPoint(left, blue)
            drawTriggerPoint(right, red)
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
            Text("Blue = left, red = right. Drag circles; cross center is the press point.", color = MaterialTheme.colorScheme.onSurfaceVariant)
        }

        Row(
            modifier = Modifier.align(Alignment.BottomCenter).padding(18.dp),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Button(onClick = onCancel, colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF2B2B33))) { Text("Cancel") }
            Button(onClick = { onSave(left, right) }) { Text("Save") }
        }
    }
}

private fun distance(a: Offset, b: Offset): Float = maxOf(abs(a.x - b.x), abs(a.y - b.y))

private fun androidx.compose.ui.graphics.drawscope.DrawScope.drawTriggerPoint(center: Offset, color: Color) {
    drawCircle(color.copy(alpha = 0.95f), radius = 42f, center = center)
    drawCircle(Color.Black.copy(alpha = 0.35f), radius = 42f, center = center, style = Stroke(width = 4f))
    drawLine(Color.White, Offset(center.x - 20f, center.y), Offset(center.x + 20f, center.y), strokeWidth = 5f, cap = StrokeCap.Round)
    drawLine(Color.White, Offset(center.x, center.y - 20f), Offset(center.x, center.y + 20f), strokeWidth = 5f, cap = StrokeCap.Round)
}
