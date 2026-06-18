package com.example.aw22xxxconfig.ui.screens

import android.app.ActivityManager
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.Environment
import android.os.Settings
import android.os.StatFs
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
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
    val maintenance by viewModel.maintenance.collectAsState()
    var includeKeyboard by remember { mutableStateOf(false) }
    var showVendorBootConfirm by remember { mutableStateOf(false) }
    var confirmNx769j by remember { mutableStateOf(false) }
    val vendorBootSupported = viewModel.isVendorBootSupportedDevice()

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
                            color = MaterialTheme.colorScheme.onSurfaceVariant
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
            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text("Display over other apps")
                    Text("Needed for future trigger overlays above games.", color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Button(onClick = {
                    context.startActivity(Intent(Settings.ACTION_MANAGE_OVERLAY_PERMISSION, Uri.parse("package:${context.packageName}")))
                }) { Text("Open") }
            }
        }

        MoraCard("System cleanup") {
            Text(
                "Uses the fixed package list from android_debloat_root_v21.sh. Packages are disabled for user 0, not physically deleted.",
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text("Disable built-in system keyboard")
                    Text("Optional: com.android.inputmethod.latin", color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Switch(checked = includeKeyboard, onCheckedChange = { includeKeyboard = it })
            }
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                Button(enabled = !maintenance.running, onClick = { viewModel.checkSystemCleanup(includeKeyboard) }) { Text("Check") }
                Button(enabled = !maintenance.running, onClick = { viewModel.runSystemCleanup(includeKeyboard) }) { Text("Clean") }
                Button(enabled = !maintenance.running, onClick = { viewModel.restoreSystemCleanup(includeKeyboard) }) { Text("Restore") }
            }
        }

        MoraCard("Overclocked vendor_boot") {
            Text(
                "Image path: ${BuildConstants.VENDOR_BOOT_IMAGE_PATH}",
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            if (vendorBootSupported) {
                Text("Supported device detected: Red Magic 9 Pro / NX769J")
            } else {
                Text(
                    "Flashing is available only for Red Magic 9 Pro / NX769J. 9S Pro, 9 Pro Plus and other models are blocked.",
                    color = MaterialTheme.colorScheme.error,
                )
            }
            Button(
                enabled = vendorBootSupported && !maintenance.running,
                onClick = { showVendorBootConfirm = true },
                colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error),
            ) { Text("Flash vendor_boot") }
        }

        if (maintenance.running) {
            MoraCard("Maintenance") {
                Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                    CircularProgressIndicator(modifier = Modifier.size(24.dp), strokeWidth = 2.dp)
                    Text("Running root command…")
                }
            }
        }
        if (maintenance.log.isNotBlank()) {
            MoraCard("Last maintenance log") {
                Text(
                    maintenance.log.takeLast(6000),
                    modifier = Modifier
                        .fillMaxWidth()
                        .background(Color(0xFF090B10), RoundedCornerShape(12.dp))
                        .padding(12.dp),
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }
    }

    if (showVendorBootConfirm) {
        AlertDialog(
            onDismissRequest = { showVendorBootConfirm = false; confirmNx769j = false },
            confirmButton = {
                Button(
                    enabled = confirmNx769j,
                    onClick = {
                        showVendorBootConfirm = false
                        confirmNx769j = false
                        viewModel.flashVendorBoot()
                    },
                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.error),
                ) { Text("Yes, flash vendor_boot") }
            },
            dismissButton = {
                TextButton(onClick = { showVendorBootConfirm = false; confirmNx769j = false }) { Text("Cancel") }
            },
            title = { Text("Danger: overclocked vendor_boot", color = MaterialTheme.colorScheme.error) },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text(
                        "This vendor_boot image is rebuilt and contains overclocking. Flashing is risky and only allowed for Red Magic 9 Pro / NX769J.",
                        color = MaterialTheme.colorScheme.error,
                    )
                    Text("The image will be flashed to vendor_boot_a and vendor_boot_b, then the phone will reboot.")
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                        Text("I confirm this is NX769J")
                        Checkbox(checked = confirmNx769j, onCheckedChange = { confirmNx769j = it })
                    }
                }
            },
        )
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
