package com.example.aw22xxxconfig.ui.screens

import android.app.ActivityManager
import android.content.Context
import android.os.Build
import android.os.Environment
import android.os.StatFs
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import com.example.aw22xxxconfig.BuildConstants
import com.example.aw22xxxconfig.MoraViewModel
import com.example.aw22xxxconfig.data.model.ConnectionState
import com.example.aw22xxxconfig.ui.components.KeyValue
import com.example.aw22xxxconfig.ui.components.MoraCard
import com.example.aw22xxxconfig.ui.components.SectionHeader
import java.util.Locale

@Composable
fun SettingsScreen(viewModel: MoraViewModel) {
    val connection by viewModel.connection.collectAsState()
    val state by viewModel.state.collectAsState()
    val config by viewModel.config.collectAsState()
    val context = LocalContext.current
    val androidFeatures by viewModel.androidFeatures.collectAsState()

    Column(
        modifier = Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp)
    ) {
        SectionHeader("Settings", "Device information and service diagnostics")
        MoraCard("Device") {
            KeyValue("Target line", BuildConstants.SUPPORTED_DEVICE_FAMILY)
            KeyValue("Model", listOfNotNull(Build.MANUFACTURER, Build.MODEL).joinToString(" "))
            KeyValue("Device", Build.DEVICE ?: "—")
            KeyValue("Android", Build.VERSION.RELEASE ?: "14+")
            KeyValue("Build number", Build.DISPLAY ?: Build.ID ?: "—")
            KeyValue("RAM", formatBytes(readTotalRam(context)))
            KeyValue("Storage", formatStorage(context))
        }
        MoraCard("Service") {
            KeyValue("Status", when (connection) {
                is ConnectionState.Loading -> "Loading"
                is ConnectionState.Ready -> "Ready"
                is ConnectionState.Error -> "Error"
            })
            KeyValue("Last config error", state.lastConfigError ?: "none")
            KeyValue("Tracked games", state.games.count.toString())
            KeyValue("Game drivers", state.games.driverCount.toString())
            Spacer(modifier = Modifier.height(12.dp))
            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text("Use phone cooler?")
                    if (!config.usePhoneCooler) {
                        Text(
                            "If disabled, Mora will not control the phone cooler. Lighting and other features will continue to work.",
                            color = androidx.compose.material3.MaterialTheme.colorScheme.onSurfaceVariant
                        )
                    }
                }
                Switch(
                    checked = config.usePhoneCooler,
                    onCheckedChange = viewModel::setUsePhoneCooler
                )
            }
            Spacer(modifier = Modifier.height(12.dp))
            Button(onClick = { viewModel.refresh(); viewModel.refreshAndroidFeatures() }) { Text("Refresh now") }
        }
        MoraCard("Android features") {
            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text("Double tap to wake")
                }
                if (androidFeatures.loading) {
                    CircularProgressIndicator(modifier = Modifier.size(24.dp), strokeWidth = 2.dp)
                } else {
                    Switch(
                        checked = androidFeatures.doubleTapToWakeEnabled,
                        onCheckedChange = viewModel::setDoubleTapToWakeEnabled
                    )
                }
            }
        }
    }
}

private fun readTotalRam(context: Context): Long {
    val activityManager = context.getSystemService(Context.ACTIVITY_SERVICE) as ActivityManager
    val info = ActivityManager.MemoryInfo()
    activityManager.getMemoryInfo(info)
    return info.totalMem
}

private fun formatStorage(context: Context): String {
    val path = Environment.getDataDirectory().path
    val stat = StatFs(path)
    val total = stat.totalBytes
    val free = stat.availableBytes
    return "${formatBytes(total - free)} / ${formatBytes(total)}"
}

private fun formatBytes(bytes: Long): String {
    if (bytes <= 0L) return "0 GB"
    val gb = bytes.toDouble() / (1024.0 * 1024.0 * 1024.0)
    return String.format(Locale.US, "%.1f GB", gb)
}
