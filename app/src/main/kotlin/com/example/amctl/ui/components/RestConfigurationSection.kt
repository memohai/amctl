package com.example.amctl.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ContentCopy
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.Card
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.example.amctl.data.model.ServerConfig

@Composable
fun RestConfigurationSection(
    config: ServerConfig,
    isServerRunning: Boolean,
    onPortChange: (Int) -> Unit,
    onRegenerateToken: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val clipboardManager = LocalClipboardManager.current

    Card(modifier = modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text("REST API Configuration", style = MaterialTheme.typography.titleMedium)

            OutlinedTextField(
                value = config.restPort.toString(),
                onValueChange = { it.toIntOrNull()?.let(onPortChange) },
                label = { Text("Port") },
                enabled = !isServerRunning,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )

            Text("Bearer Token", style = MaterialTheme.typography.labelLarge)
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(
                    text = if (config.restBearerToken.isNotEmpty()) config.restBearerToken else "Not generated",
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.weight(1f),
                )
                IconButton(onClick = {
                    if (config.restBearerToken.isNotEmpty()) {
                        clipboardManager.setText(AnnotatedString(config.restBearerToken))
                    }
                }) {
                    Icon(Icons.Default.ContentCopy, contentDescription = "Copy token")
                }
                IconButton(
                    onClick = onRegenerateToken,
                    enabled = !isServerRunning,
                ) {
                    Icon(Icons.Default.Refresh, contentDescription = "Regenerate token")
                }
            }
        }
    }
}
