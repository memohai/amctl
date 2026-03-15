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
import com.example.amctl.services.rest.RestServerService
import com.example.amctl.services.system.ShizukuProvider
import com.example.amctl.services.system.ToolRouter
import com.example.amctl.utils.NetworkUtils
import com.example.amctl.utils.PermissionUtils
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlinx.coroutines.delay
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
        private val shizukuProvider: ShizukuProvider,
        private val toolRouter: ToolRouter,
    ) : AndroidViewModel(application) {

        val serverConfig: StateFlow<ServerConfig> = settingsRepository.serverConfig
            .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5000), ServerConfig())

        val serverStatus: StateFlow<ServerStatus> = McpServerService.serverStatus

        val restServerStatus: StateFlow<ServerStatus> = RestServerService.serverStatus

        private val _deviceIp = MutableStateFlow(NetworkUtils.getDeviceIpAddress())
        val deviceIp: StateFlow<String?> = _deviceIp.asStateFlow()

        private val _shizukuStatus = MutableStateFlow(getShizukuStatusText())
        val shizukuStatus: StateFlow<String> = _shizukuStatus.asStateFlow()

        private val _controlMode = MutableStateFlow(toolRouter.currentMode.name)
        val controlMode: StateFlow<String> = _controlMode.asStateFlow()

        companion object {
            private const val SHIZUKU_PERMISSION_REQUEST_CODE = 1001
            private const val STATUS_POLL_INTERVAL_MS = 3000L
        }

        init {
            viewModelScope.launch {
                while (true) {
                    delay(STATUS_POLL_INTERVAL_MS)
                    refreshShizukuStatus()
                }
            }
        }

        fun isAccessibilityEnabled(): Boolean =
            PermissionUtils.isAccessibilityServiceEnabled(application, AmctlAccessibilityService::class.java)

        fun requestShizukuPermission() {
            shizukuProvider.requestPermission(SHIZUKU_PERMISSION_REQUEST_CODE)
            viewModelScope.launch {
                delay(1000)
                refreshShizukuStatus()
            }
        }

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

        fun startRestServer() {
            val intent = Intent(application, RestServerService::class.java).apply {
                action = RestServerService.ACTION_START
            }
            application.startForegroundService(intent)
        }

        fun stopRestServer() {
            val intent = Intent(application, RestServerService::class.java).apply {
                action = RestServerService.ACTION_STOP
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

        fun updateRestPort(port: Int) {
            settingsRepository.validatePort(port).onSuccess {
                viewModelScope.launch { settingsRepository.updateRestPort(it) }
            }
        }

        fun generateNewRestBearerToken() {
            viewModelScope.launch { settingsRepository.generateNewRestBearerToken() }
        }

        fun refreshDeviceIp() {
            _deviceIp.value = NetworkUtils.getDeviceIpAddress()
        }

        private fun refreshShizukuStatus() {
            _shizukuStatus.value = getShizukuStatusText()
            _controlMode.value = toolRouter.currentMode.name
        }

        private fun getShizukuStatusText(): String = when {
            shizukuProvider.isAvailable() -> "Authorized"
            shizukuProvider.isInstalled() && !shizukuProvider.hasPermission() -> "Not Authorized"
            shizukuProvider.isInstalled() -> "Running (checking permission...)"
            else -> "Not Running"
        }
    }
