package com.example.aw22xxxconfig.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.example.aw22xxxconfig.MoraViewModel
import com.example.aw22xxxconfig.ui.components.KeyValue
import com.example.aw22xxxconfig.ui.components.MoraCard
import com.example.aw22xxxconfig.ui.components.SectionHeader

@Composable
fun HomeScreen(viewModel: MoraViewModel) {
    val state by viewModel.state.collectAsState()

    LazyColumn(
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        item {
            SectionHeader("mora", "Local Red Magic performance status")
        }
        item {
            MoraCard("Quick controls") {
                Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Column(Modifier.weight(1f)) {
                        Text("Daemon notifications")
                        Text("Allow daemon notifications", color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    Switch(checked = state.daemonNotifications, onCheckedChange = viewModel::setDaemonNotifications)
                }
                Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
                    Column(Modifier.weight(1f)) {
                        Text("Battery saver")
                        Text("Smart saver switch", color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    Switch(checked = state.battery.saver.enabled, onCheckedChange = viewModel::setBatterySaver)
                }
            }
        }
        item {
            LazyVerticalGrid(
                columns = GridCells.Fixed(2),
                modifier = Modifier.height(440.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                val cards = listOf(
                    "CPU" to formatTemp(state.temps.cpu),
                    "GPU" to formatTemp(state.temps.gpu),
                    "SOC" to formatTemp(state.temps.soc),
                    "Battery" to formatTemp(state.temps.batt),
                    "Battery %" to "${state.battery.percent ?: 0}%",
                    "Profile" to (state.activeProfile ?: "—"),
                    "LED profile" to (state.ledProfile ?: "—"),
                    "Games tracked" to state.games.count.toString(),
                )
                items(cards) { (name, value) ->
                    MoraCard(title = name) { Text(value, style = MaterialTheme.typography.headlineMedium) }
                }
            }
        }
        item {
            MoraCard("Live state") {
                KeyValue("Zone", state.zone.name ?: "—")
                KeyValue("Thermal reduction", "${state.zone.reducePercent ?: 0}%")
                KeyValue("Charging HW", yesNo(state.charging.hw))
                KeyValue("Charging effective", yesNo(state.charging.effective))
                KeyValue("Screen on", yesNo(state.screenOn))
                KeyValue("Game mode", yesNo(state.gameMode))
                KeyValue("Triggers active", yesNo(state.triggers.active))
                KeyValue("Idle mode", yesNo(state.idleMode))
                KeyValue("VmRSS", state.mem.vmRssKb?.let { "$it KB" } ?: "—")
                KeyValue("Last config error", state.lastConfigError ?: "none")
            }
        }
    }
}

private fun yesNo(value: Boolean) = if (value) "Yes" else "No"
private fun formatTemp(value: Double?) = value?.let { "%.1f°C".format(it) } ?: "—"
