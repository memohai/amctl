package com.example.amctl.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.example.amctl.data.model.ServerStatus

@Composable
fun ServerStatusCard(
    serverStatus: ServerStatus,
    onToggle: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
) {
    val isRunning = serverStatus is ServerStatus.Running
    val isTransitioning = serverStatus is ServerStatus.Starting || serverStatus is ServerStatus.Stopping

    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(
            containerColor = when (serverStatus) {
                is ServerStatus.Running -> MaterialTheme.colorScheme.primaryContainer
                is ServerStatus.Error -> MaterialTheme.colorScheme.errorContainer
                else -> MaterialTheme.colorScheme.surfaceVariant
            },
        ),
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column {
                Text(
                    text = "MCP Server",
                    style = MaterialTheme.typography.titleMedium,
                )
                Text(
                    text = when (serverStatus) {
                        is ServerStatus.Stopped -> "Stopped"
                        is ServerStatus.Starting -> "Starting..."
                        is ServerStatus.Running -> "Running on ${serverStatus.address}:${serverStatus.port}"
                        is ServerStatus.Stopping -> "Stopping..."
                        is ServerStatus.Error -> "Error: ${serverStatus.message}"
                    },
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Switch(
                checked = isRunning,
                onCheckedChange = onToggle,
                enabled = !isTransitioning,
            )
        }
    }
}
