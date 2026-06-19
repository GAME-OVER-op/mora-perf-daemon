package com.example.aw22xxxconfig.ui.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.ArrowBack
import androidx.compose.material3.*
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.example.aw22xxxconfig.MoraViewModel
import com.example.aw22xxxconfig.ui.components.KeyValue
import com.example.aw22xxxconfig.ui.components.MoraCard
import com.example.aw22xxxconfig.ui.components.SectionHeader

@Composable
fun FlashImageScreen(
    viewModel: MoraViewModel,
    title: String,
    subtitle: String,
    warning: String,
    imagePath: String,
    firstBlock: String,
    secondBlock: String,
    confirmText: String,
    buttonText: String,
    onBack: () -> Unit,
    onFlash: () -> Unit,
) {
    val state by viewModel.flashImageState.collectAsState()
    var confirmed by remember { mutableStateOf(false) }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .verticalScroll(rememberScrollState())
            .padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
    ) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            IconButton(onClick = onBack, enabled = !state.loading) {
                Icon(Icons.Rounded.ArrowBack, contentDescription = null)
            }
            SectionHeader(title, subtitle)
        }

        MoraCard("Warning") {
            Text(warning, color = MaterialTheme.colorScheme.error)
        }

        MoraCard("Targets") {
            KeyValue("Image", imagePath)
            KeyValue("First slot", firstBlock)
            KeyValue("Second slot", secondBlock)
        }

        MoraCard("Confirmation") {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Checkbox(
                    checked = confirmed,
                    enabled = !state.loading,
                    onCheckedChange = { confirmed = it },
                )
                Text(confirmText, modifier = Modifier.weight(1f))
            }
            Button(
                onClick = onFlash,
                enabled = confirmed && !state.loading,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(buttonText)
            }
            if (state.loading) {
                LinearProgressIndicator(modifier = Modifier.fillMaxWidth())
            }
            state.message?.let {
                Text(it, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
    }
}
