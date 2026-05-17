package com.memohai.autofish.ui.components

import android.content.ClipData
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
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.ClipEntry
import androidx.compose.ui.platform.LocalClipboard
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import com.memohai.autofish.R
import com.memohai.autofish.data.model.ServerConfig
import kotlinx.coroutines.launch

@Composable
@Suppress("FunctionNaming")
fun ServiceConfigurationSection(
    config: ServerConfig,
    isServerRunning: Boolean,
    onPortChange: (Int) -> Unit,
    onRegenerateToken: () -> Unit,
    modifier: Modifier = Modifier,
) {
    val clipboard = LocalClipboard.current
    val coroutineScope = rememberCoroutineScope()

    Card(modifier = modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(stringResource(R.string.service_settings), style = MaterialTheme.typography.titleMedium)

            OutlinedTextField(
                value = config.servicePort.toString(),
                onValueChange = { it.toIntOrNull()?.let(onPortChange) },
                label = { Text(stringResource(R.string.port_label)) },
                enabled = !isServerRunning,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
            )

            Text(stringResource(R.string.token_label), style = MaterialTheme.typography.labelLarge)
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(
                    text = if (config.serviceBearerToken.isNotEmpty()) {
                        config.serviceBearerToken
                    } else {
                        stringResource(R.string.not_generated)
                    },
                    style = MaterialTheme.typography.bodySmall,
                    modifier = Modifier.weight(1f),
                )
                IconButton(onClick = {
                    if (config.serviceBearerToken.isNotEmpty()) {
                        coroutineScope.launch {
                            clipboard.setClipEntry(
                                ClipEntry(ClipData.newPlainText("Autofish", config.serviceBearerToken)),
                            )
                        }
                    }
                }) {
                    Icon(Icons.Default.ContentCopy, contentDescription = stringResource(R.string.copy_token))
                }
                IconButton(
                    onClick = onRegenerateToken,
                    enabled = !isServerRunning,
                ) {
                    Icon(Icons.Default.Refresh, contentDescription = stringResource(R.string.regenerate_token))
                }
            }
        }
    }
}
