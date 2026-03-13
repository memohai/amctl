package com.example.amctl.ui.viewmodels

import android.app.Application
import android.content.Intent
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.example.amctl.data.model.BindingAddress
import com.example.amctl.data.model.ServerConfig
import com.example.amctl.data.model.ServerStatus
import com.example.amctl.data.repository.SettingsRepository
import com.example.amctl.services.accessibility.AmctlAccessibilityService
import com.example.amctl.services.mcp.McpServerService
import com.example.amctl.utils.NetworkUtils
import com.example.amctl.utils.PermissionUtils
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.stateIn
import kotlinx.coroutines.launch
import javax.inject.Inject

@HiltViewModel
class MainViewModel
    @Inject
    constructor(
        private val application: Application,
        private val settingsRepository: SettingsRepository,
    ) : AndroidViewModel(application) {

        val serverConfig: StateFlow<ServerConfig> = settingsRepository.serverConfig
            .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), ServerConfig())

        val serverStatus: StateFlow<ServerStatus> = McpServerService.serverStatus

        private val _deviceIp = MutableStateFlow(NetworkUtils.getDeviceIpAddress())
        val deviceIp: StateFlow<String?> = _deviceIp.asStateFlow()

        fun isAccessibilityEnabled(): Boolean =
            PermissionUtils.isAccessibilityServiceEnabled(application, AmctlAccessibilityService::class.java)

        fun startServer() {
            val intent = Intent(application, McpServerService::class.java).apply {
                action = McpServerService.ACTION_START
            }
            application.startForegroundService(intent)
        }

        fun stopServer() {
            val intent = Intent(application, McpServerService::class.java).apply {
                action = McpServerService.ACTION_STOP
            }
            application.startService(intent)
        }

        fun updatePort(port: Int) {
            settingsRepository.validatePort(port).onSuccess {
                viewModelScope.launch { settingsRepository.updatePort(it) }
            }
        }

        fun updateBindingAddress(address: BindingAddress) {
            viewModelScope.launch { settingsRepository.updateBindingAddress(address) }
        }

        fun generateNewBearerToken() {
            viewModelScope.launch { settingsRepository.generateNewBearerToken() }
        }

        fun refreshDeviceIp() {
            _deviceIp.value = NetworkUtils.getDeviceIpAddress()
        }
    }
