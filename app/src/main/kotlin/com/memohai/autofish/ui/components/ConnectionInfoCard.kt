package com.memohai.autofish.ui.components

import android.widget.Toast
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.unit.dp
import com.memohai.autofish.R

@Composable
@Suppress("FunctionName")
fun ConnectionInfoCard(
    deviceIp: String?,
    servicePort: Int,
    serviceBearerToken: String,
    isServiceRunning: Boolean,
    modifier: Modifier = Modifier,
) {
    val clipboardManager = LocalClipboardManager.current
    val context = LocalContext.current
    val copiedText = stringResource(R.string.copied_to_clipboard)
    val lanUrlComment = stringResource(R.string.copy_lan_url_comment)
    val ip = deviceIp?.takeIf { it.isNotBlank() }
    val token = serviceBearerToken.takeIf { it.isNotBlank() }
    val url = ip?.let { "http://$it:$servicePort" }
    val canCopy = ip != null && token != null

    fun copy(text: String) {
        clipboardManager.setText(AnnotatedString(text))
        Toast.makeText(context, copiedText, Toast.LENGTH_SHORT).show()
    }

    Card(modifier = modifier.fillMaxWidth()) {
        Column(
            modifier = Modifier.padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
                Text(
                    text = stringResource(R.string.connection_info),
                    style = MaterialTheme.typography.titleMedium,
                )
                Text(
                    text =
                        if (isServiceRunning) {
                            stringResource(R.string.connection_info_desc)
                        } else {
                            stringResource(R.string.connection_service_stopped_hint)
                        },
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            ConnectionInfoRow(
                label = stringResource(R.string.ip_label),
                value = ip ?: stringResource(R.string.unknown),
            )
            ConnectionInfoRow(
                label = stringResource(R.string.port_label),
                value = servicePort.toString(),
            )
            ConnectionInfoRow(
                label = stringResource(R.string.url_label),
                value = url ?: stringResource(R.string.unknown),
            )
            ConnectionInfoRow(
                label = stringResource(R.string.token_label),
                value = token?.let(::maskToken) ?: stringResource(R.string.not_generated),
            )

            if (ip == null) {
                Text(
                    text = stringResource(R.string.connection_ip_unknown_hint),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                )
            }
            if (token == null) {
                Text(
                    text = stringResource(R.string.connection_token_missing_hint),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.error,
                )
            }

            Button(
                onClick = {
                    copy(
                        """
                        # $lanUrlComment
                        af config set remote.url "$url"
                        af config set remote.token "$token"
                        """.trimIndent(),
                    )
                },
                enabled = canCopy,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(stringResource(R.string.copy_agent_config))
            }

            TextButton(
                onClick = {
                    copy(
                        """
                        # $lanUrlComment
                        IP=$ip
                        PORT=$servicePort
                        URL=$url
                        TOKEN=$token
                        """.trimIndent(),
                    )
                },
                enabled = canCopy,
                modifier = Modifier.fillMaxWidth(),
            ) {
                Text(stringResource(R.string.copy_raw_connection_info))
            }
        }
    }
}

@Composable
@Suppress("FunctionName")
private fun ConnectionInfoRow(
    label: String,
    value: String,
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Text(
            text = label,
            style = MaterialTheme.typography.labelMedium,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(end = 16.dp),
        )
        Text(
            text = value,
            style = MaterialTheme.typography.bodyMedium,
        )
    }
}

private fun maskToken(token: String): String =
    if (token.length <= 12) {
        "****"
    } else {
        "${token.take(6)}...${token.takeLast(4)}"
    }
