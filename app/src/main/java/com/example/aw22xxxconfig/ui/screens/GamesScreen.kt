package com.example.aw22xxxconfig.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.ChevronRight
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import com.example.aw22xxxconfig.MoraViewModel
import com.example.aw22xxxconfig.data.model.GameEntry
import com.example.aw22xxxconfig.data.model.InstalledApp
import com.example.aw22xxxconfig.ui.components.MoraCard
import com.example.aw22xxxconfig.ui.components.SectionHeader

@Composable
fun GamesScreen(viewModel: MoraViewModel, openDetails: (String) -> Unit) {
    val games by viewModel.games.collectAsState()
    val installed by viewModel.installedApps.collectAsState()
    var showAdd by remember { mutableStateOf(false) }
    var search by remember { mutableStateOf("") }

    Column(modifier = Modifier.fillMaxSize().padding(16.dp), verticalArrangement = Arrangement.spacedBy(16.dp)) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.SpaceBetween, modifier = Modifier.fillMaxWidth()) {
            SectionHeader("Games", "Tracked apps and per-game settings")
            FilledIconButton(onClick = { showAdd = true }) {
                Icon(Icons.Rounded.Add, contentDescription = null)
            }
        }
        OutlinedTextField(value = search, onValueChange = { search = it }, modifier = Modifier.fillMaxWidth(), label = { Text("Search games") })
        LazyColumn(verticalArrangement = Arrangement.spacedBy(12.dp)) {
            items(games.filter { game ->
                val app = installed.firstOrNull { it.packageName == game.packageName }
                val name = app?.label ?: game.packageName
                name.contains(search, ignoreCase = true) || game.packageName.contains(search, ignoreCase = true)
            }) { game ->
                GameRow(game = game, app = installed.firstOrNull { it.packageName == game.packageName }, onOpen = { openDetails(game.packageName) }, onDelete = { viewModel.removeGame(game.packageName) })
            }
        }
    }

    if (showAdd) {
        AddGameDialog(
            apps = installed.filter { it.label.contains(search, true) || it.packageName.contains(search, true) },
            onDismiss = { showAdd = false },
            onAdd = {
                viewModel.addGame(it)
                showAdd = false
            }
        )
    }
}

@Composable
private fun GameRow(game: GameEntry, app: InstalledApp?, onOpen: () -> Unit, onDelete: () -> Unit) {
    MoraCard(title = app?.label ?: game.packageName) {
        Text(game.packageName, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
        Spacer(Modifier.height(8.dp))
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
            AssistChip(onClick = onDelete, label = { Text("Delete") })
            FilledTonalButton(onClick = onOpen) {
                Text("Open")
                Icon(Icons.Rounded.ChevronRight, contentDescription = null)
            }
        }
    }
}

@Composable
private fun AddGameDialog(apps: List<InstalledApp>, onDismiss: () -> Unit, onAdd: (InstalledApp) -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        confirmButton = {},
        title = { Text("Add game") },
        text = {
            LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.heightIn(max = 420.dp)) {
                items(apps) { app ->
                    ElevatedCard(onClick = { onAdd(app) }) {
                        Row(Modifier.fillMaxWidth().padding(12.dp), verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                            AsyncImage(model = app.icon, contentDescription = null, modifier = Modifier.size(40.dp))
                            Column(Modifier.weight(1f)) {
                                Text(app.label)
                                Text(app.packageName, color = MaterialTheme.colorScheme.onSurfaceVariant, maxLines = 1, overflow = TextOverflow.Ellipsis)
                            }
                        }
                    }
                }
            }
        }
    )
}
