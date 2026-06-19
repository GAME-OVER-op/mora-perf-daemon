package com.example.aw22xxxconfig.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.example.aw22xxxconfig.MoraViewModel
import com.example.aw22xxxconfig.data.model.DebloatPackageState
import com.example.aw22xxxconfig.ui.components.MoraCard
import com.example.aw22xxxconfig.ui.components.SectionHeader

@Composable
fun DebloatScreen(viewModel: MoraViewModel, onBack: () -> Unit) {
    val packages by viewModel.debloatPackages.collectAsState()
    val loading by viewModel.debloatLoading.collectAsState()
    val hasEnabled = packages.any { it.installed && it.enabled }
    val actionable = packages.any { it.installed }

    LaunchedEffect(Unit) {
        viewModel.refreshDebloatPackages()
    }

    Column(
        modifier = Modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            IconButton(onClick = onBack) { Icon(Icons.Rounded.ArrowBack, contentDescription = null) }
            SectionHeader("System cleanup", "Disable unnecessary system apps")
        }

        MoraCard("Controls") {
            Text(
                "Turning a switch off clears app data, force-stops the app, and runs pm disable-user --user 0. Turning it on runs pm enable.",
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                style = MaterialTheme.typography.bodySmall,
            )
            Row(horizontalArrangement = Arrangement.spacedBy(12.dp), modifier = Modifier.fillMaxWidth()) {
                Button(
                    onClick = { viewModel.setAllDebloatPackagesEnabled(enabled = !hasEnabled) },
                    enabled = actionable && !loading,
                    modifier = Modifier.weight(1f),
                ) {
                    Text(if (hasEnabled) "Disable all" else "Enable all")
                }
                FilledTonalButton(
                    onClick = viewModel::refreshDebloatPackages,
                    enabled = !loading,
                    modifier = Modifier.weight(1f),
                ) {
                    Text("Refresh")
                }
            }
            if (loading) {
                LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
            }
        }

        LazyColumn(verticalArrangement = Arrangement.spacedBy(10.dp), modifier = Modifier.fillMaxSize()) {
            items(packages, key = { it.item.packageName }) { state ->
                DebloatPackageRow(
                    state = state,
                    enabled = !loading && state.installed,
                    onCheckedChange = { checked ->
                        viewModel.setDebloatPackageEnabled(state.item.packageName, checked)
                    }
                )
            }
        }
    }
}

@Composable
private fun DebloatPackageRow(
    state: DebloatPackageState,
    enabled: Boolean,
    onCheckedChange: (Boolean) -> Unit,
) {
    ElevatedCard(colors = CardDefaults.elevatedCardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow)) {
        Row(
            modifier = Modifier.fillMaxWidth().padding(14.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(state.item.title, style = MaterialTheme.typography.titleSmall)
                Text(
                    state.item.packageName,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    style = MaterialTheme.typography.bodySmall,
                )
                Text(
                    when {
                        !state.installed -> "Not found on this system"
                        state.enabled -> "Enabled"
                        else -> "Disabled"
                    },
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    style = MaterialTheme.typography.bodySmall,
                )
                state.error?.let {
                    Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall)
                }
            }
            Switch(
                checked = state.installed && state.enabled,
                enabled = enabled,
                onCheckedChange = onCheckedChange,
            )
        }
    }
}
