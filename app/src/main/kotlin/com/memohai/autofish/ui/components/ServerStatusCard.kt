package com.memohai.autofish.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.ripple
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.memohai.autofish.R
import com.memohai.autofish.data.model.ServerStatus

@Composable
@Suppress("FunctionNaming", "LongMethod")
fun ServerStatusCard(
    serverStatus: ServerStatus,
    onToggle: (Boolean) -> Unit,
    modifier: Modifier = Modifier,
    title: String = "Control Server",
) {
    val isRunning = serverStatus is ServerStatus.Running
    val isTransitioning = serverStatus is ServerStatus.Starting || serverStatus is ServerStatus.Stopping
    val interactionSource = remember { MutableInteractionSource() }
    val stateDesc = if (isRunning) {
        stringResource(R.string.enabled)
    } else {
        stringResource(R.string.disabled)
    }

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
                .toggleable(
                    value = isRunning,
                    enabled = !isTransitioning,
                    role = Role.Switch,
                    interactionSource = interactionSource,
                    indication = ripple(),
                    onValueChange = onToggle,
                )
                .semantics(mergeDescendants = true) {
                    stateDescription = stateDesc
                }
                .padding(16.dp),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Column {
                Text(
                    text = title,
                    style = MaterialTheme.typography.titleMedium,
                )
                Text(
                    text = when (serverStatus) {
                        is ServerStatus.Stopped -> stringResource(R.string.server_stopped)
                        is ServerStatus.Starting -> stringResource(R.string.server_starting)
                        is ServerStatus.Running -> stringResource(
                            R.string.server_running_format,
                            serverStatus.address,
                            serverStatus.port,
                        )
                        is ServerStatus.Stopping -> stringResource(R.string.server_stopping)
                        is ServerStatus.Error -> stringResource(R.string.server_error_format, serverStatus.message)
                    },
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            Switch(
                checked = isRunning,
                onCheckedChange = null,
                enabled = !isTransitioning,
            )
        }
    }
}
