package com.memohai.autofish.ui.viewmodels

import android.app.Application
import android.content.Intent
import androidx.core.app.NotificationManagerCompat
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.memohai.autofish.data.model.BindingAddress
import com.memohai.autofish.data.model.AppLanguage
import com.memohai.autofish.data.model.AppThemeMode
import com.memohai.autofish.data.model.ServerConfig
import com.memohai.autofish.data.model.ServerStatus
import com.memohai.autofish.data.repository.SettingsRepository
import com.memohai.autofish.services.accessibility.AutoFishAccessibilityService
import com.memohai.autofish.service.ServiceServer
import com.memohai.autofish.services.service.ServiceServerService
import com.memohai.autofish.services.system.ShizukuProvider
import com.memohai.autofish.services.system.ToolRouter
import com.memohai.autofish.utils.NetworkUtils
import com.memohai.autofish.utils.PermissionUtils
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
@Suppress("TooManyFunctions")
class MainViewModel
    @Inject
    constructor(
        private val application: Application,
        private val settingsRepository: SettingsRepository,
        private val shizukuProvider: ShizukuProvider,
        private val toolRouter: ToolRouter,
    ) : AndroidViewModel(application) {

        val serverConfig: StateFlow<ServerConfig> = settingsRepository.serverConfig
            .stateIn(viewModelScope, SharingStarted.WhileSubscribed(CONFIG_STOP_TIMEOUT_MS), ServerConfig())

        val serviceServerStatus: StateFlow<ServerStatus> = ServiceServerService.serverStatus

        private val _deviceIp = MutableStateFlow(NetworkUtils.getDeviceIpAddress())
        val deviceIp: StateFlow<String?> = _deviceIp.asStateFlow()

        private val _shizukuStatus = MutableStateFlow(getShizukuStatusText())
        val shizukuStatus: StateFlow<String> = _shizukuStatus.asStateFlow()

        private val _controlMode = MutableStateFlow(toolRouter.currentMode.name)
        val controlMode: StateFlow<String> = _controlMode.asStateFlow()
        private val _accessibilityEnabled = MutableStateFlow(isAccessibilityEnabledNow())
        val accessibilityEnabled: StateFlow<Boolean> = _accessibilityEnabled.asStateFlow()
        private val _notificationsEnabled =
            MutableStateFlow(NotificationManagerCompat.from(application).areNotificationsEnabled())
        val notificationsEnabled: StateFlow<Boolean> = _notificationsEnabled.asStateFlow()
        private val _refPanelState = MutableStateFlow<ServiceServer.RefPanelStatePayload?>(null)
        val refPanelState: StateFlow<ServiceServer.RefPanelStatePayload?> = _refPanelState.asStateFlow()

        companion object {
            private const val SHIZUKU_PERMISSION_REQUEST_CODE = 1001
            private const val STATUS_POLL_INTERVAL_MS = 3000L
            private const val CONFIG_STOP_TIMEOUT_MS = 5000L
            private const val SHIZUKU_PERMISSION_REFRESH_DELAY_MS = 1000L
        }

        init {
            viewModelScope.launch {
                while (true) {
                    delay(STATUS_POLL_INTERVAL_MS)
                    refreshStatuses()
                }
            }
        }

        fun requestShizukuPermission() {
            shizukuProvider.requestPermission(SHIZUKU_PERMISSION_REQUEST_CODE)
            viewModelScope.launch {
                delay(SHIZUKU_PERMISSION_REFRESH_DELAY_MS)
                refreshStatuses()
            }
        }

        fun startServiceServer() {
            val intent = Intent(application, ServiceServerService::class.java).apply {
                action = ServiceServerService.ACTION_START
            }
            application.startForegroundService(intent)
        }

        fun stopServiceServer() {
            val intent = Intent(application, ServiceServerService::class.java).apply {
                action = ServiceServerService.ACTION_STOP
            }
            application.startService(intent)
        }

        fun updateBindingAddress(address: BindingAddress) {
            viewModelScope.launch { settingsRepository.updateBindingAddress(address) }
        }

        fun updateServicePort(port: Int) {
            settingsRepository.validatePort(port).onSuccess {
                viewModelScope.launch { settingsRepository.updateServicePort(it) }
            }
        }

        fun generateNewServiceBearerToken() {
            viewModelScope.launch { settingsRepository.generateNewServiceBearerToken() }
        }

        fun updateServiceOverlayVisible(visible: Boolean) {
            viewModelScope.launch {
                settingsRepository.updateServiceOverlayVisible(visible)
                ServiceServerService.setOverlayVisible(visible)
            }
        }

        fun updateServiceRefVisible(visible: Boolean) {
            viewModelScope.launch {
                settingsRepository.updateServiceRefVisible(visible)
                if (visible) {
                    settingsRepository.updateServiceOverlayVisible(true)
                    ServiceServerService.setOverlayVisible(true)
                }
                ServiceServerService.setRefVisible(visible)
            }
        }

        fun updateRefAutoRefresh(enabled: Boolean) {
            ServiceServerService.setRefAutoRefresh(enabled)
        }

        fun updateAppLanguage(language: AppLanguage) {
            viewModelScope.launch { settingsRepository.updateAppLanguage(language) }
        }

        fun updateAppThemeMode(themeMode: AppThemeMode) {
            viewModelScope.launch { settingsRepository.updateAppThemeMode(themeMode) }
        }

        fun restartApp() {
            val intent = application.packageManager.getLaunchIntentForPackage(application.packageName)
            if (intent != null) {
                intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
                application.startActivity(intent)
                Runtime.getRuntime().exit(0)
            }
        }

        fun refreshDeviceIp() {
            _deviceIp.value = NetworkUtils.getDeviceIpAddress()
        }

        private fun refreshStatuses() {
            _shizukuStatus.value = getShizukuStatusText()
            _controlMode.value = toolRouter.currentMode.name
            _accessibilityEnabled.value = isAccessibilityEnabledNow()
            _notificationsEnabled.value = NotificationManagerCompat.from(application).areNotificationsEnabled()
            _refPanelState.value = ServiceServerService.getRefPanelState()
        }

        private fun isAccessibilityEnabledNow(): Boolean =
            PermissionUtils.isAccessibilityServiceEnabled(application, AutoFishAccessibilityService::class.java)

        private fun getShizukuStatusText(): String = when {
            shizukuProvider.isAvailable() -> "Authorized"
            shizukuProvider.isInstalled() && !shizukuProvider.hasPermission() -> "Not Authorized"
            shizukuProvider.isInstalled() -> "Running (checking permission...)"
            else -> "Not Running"
        }
    }
