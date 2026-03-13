@file:Suppress("DEPRECATION")

package com.example.amctl.ui.screens

import android.content.Intent
import android.provider.Settings
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import com.example.amctl.data.model.ServerStatus
import com.example.amctl.ui.components.ConfigurationSection
import com.example.amctl.ui.components.ServerStatusCard
import com.example.amctl.ui.viewmodels.MainViewModel

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(viewModel: MainViewModel = hiltViewModel()) {
    val serverConfig by viewModel.serverConfig.collectAsState()
    val serverStatus by viewModel.serverStatus.collectAsState()
    val deviceIp by viewModel.deviceIp.collectAsState()
    val context = LocalContext.current
    val isRunning = serverStatus is ServerStatus.Running

    Scaffold(
        topBar = {
            TopAppBar(title = { Text("amctl") })
        },
    ) { innerPadding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(innerPadding)
                .padding(horizontal = 16.dp)
                .verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            ServerStatusCard(
                serverStatus = serverStatus,
                onToggle = { enabled ->
                    if (enabled) viewModel.startServer() else viewModel.stopServer()
                },
            )

            ConfigurationSection(
                config = serverConfig,
                isServerRunning = isRunning,
                onPortChange = viewModel::updatePort,
                onBindingAddressChange = viewModel::updateBindingAddress,
                onRegenerateToken = viewModel::generateNewBearerToken,
            )

            Card(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text("Accessibility Service", style = MaterialTheme.typography.titleMedium)
                    Text(
                        text = if (viewModel.isAccessibilityEnabled()) "Enabled" else "Disabled",
                        style = MaterialTheme.typography.bodyMedium,
                        color = if (viewModel.isAccessibilityEnabled()) {
                            MaterialTheme.colorScheme.primary
                        } else {
                            MaterialTheme.colorScheme.error
                        },
                    )
                    if (!viewModel.isAccessibilityEnabled()) {
                        Button(onClick = {
                            context.startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS))
                        }) {
                            Text("Open Accessibility Settings")
                        }
                    }
                }
            }

            Card(modifier = Modifier.fillMaxWidth()) {
                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
                    Text("Connection Info", style = MaterialTheme.typography.titleMedium)
                    Text("Device IP: ${deviceIp ?: "Unknown"}", style = MaterialTheme.typography.bodyMedium)
                    Text("Port: ${serverConfig.port}", style = MaterialTheme.typography.bodyMedium)
                    Text("Token: ${serverConfig.bearerToken.take(8)}...", style = MaterialTheme.typography.bodyMedium)
                }
            }
        }
    }
}
