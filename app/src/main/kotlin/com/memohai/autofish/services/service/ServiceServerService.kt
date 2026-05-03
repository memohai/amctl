package com.memohai.autofish.services.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.os.IBinder
import android.util.Log
import com.memohai.autofish.data.model.ServerConfig
import com.memohai.autofish.data.model.ServerStatus
import com.memohai.autofish.data.repository.SettingsRepository
import com.memohai.autofish.service.ServiceServer
import com.memohai.autofish.services.accessibility.AccessibilityServiceProvider
import com.memohai.autofish.services.accessibility.AccessibilityTreeParser
import com.memohai.autofish.services.accessibility.CompactTreeFormatter
import com.memohai.autofish.services.accessibility.ElementFinder
import com.memohai.autofish.services.logging.ServiceLogBus
import com.memohai.autofish.services.system.ToolRouter
import dagger.hilt.android.AndroidEntryPoint
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import javax.inject.Inject

@AndroidEntryPoint
class ServiceServerService : Service() {
    @Inject
    lateinit var settingsRepository: SettingsRepository

    @Inject
    lateinit var accessibilityServiceProvider: AccessibilityServiceProvider

    @Inject
    lateinit var treeParser: AccessibilityTreeParser

    @Inject
    lateinit var compactTreeFormatter: CompactTreeFormatter

    @Inject
    lateinit var elementFinder: ElementFinder

    @Inject
    lateinit var toolRouter: ToolRouter

    @Inject
    lateinit var connectionHintWriter: ConnectionHintWriter

    private val serviceScope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private var serviceServer: ServiceServer? = null
    private var lastServicePort: Int = ServerConfig.DEFAULT_PORT

    companion object {
        private const val TAG = "autofish:Service"
        private const val CHANNEL_ID = "autofish_service_server"
        private const val NOTIFICATION_ID = 2
        const val ACTION_START = "com.memohai.autofish.ACTION_START_SERVICE_SERVER"
        const val ACTION_STOP = "com.memohai.autofish.ACTION_STOP_SERVICE_SERVER"

        private val _serverStatus = MutableStateFlow<ServerStatus>(ServerStatus.Stopped)
        val serverStatus: StateFlow<ServerStatus> = _serverStatus
        @Volatile
        private var runningInstance: ServiceServerService? = null

        fun setOverlayVisible(visible: Boolean) {
            runningInstance?.setOverlayVisibleInternal(visible)
        }

        fun setRefAutoRefresh(enabled: Boolean) {
            runningInstance?.setRefAutoRefreshInternal(enabled)
        }

        fun setRefVisible(visible: Boolean) {
            runningInstance?.setRefVisibleInternal(visible)
        }

        fun getRefPanelState(limit: Int = 120): ServiceServer.RefPanelStatePayload? = runningInstance?.getRefPanelStateInternal(limit)
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(
        intent: Intent?,
        flags: Int,
        startId: Int,
    ): Int {
        runningInstance = this
        when (intent?.action) {
            ACTION_STOP -> stopServer()
            else -> startServer()
        }
        return START_STICKY
    }

    @Suppress("TooGenericExceptionCaught")
    private fun startServer() {
        _serverStatus.value = ServerStatus.Starting
        ServiceLogBus.info("SERVICE", "Start requested")
        createNotificationChannel()
        startForeground(NOTIFICATION_ID, buildNotification())

        serviceScope.launch {
            try {
                val config = settingsRepository.getServerConfig()
                lastServicePort = config.servicePort
                val server =
                    ServiceServer(
                        port = config.servicePort,
                        bindAddress = config.bindingAddress.address,
                        bearerToken = config.serviceBearerToken,
                        toolRouter = toolRouter,
                        accessibilityProvider = accessibilityServiceProvider,
                        treeParser = treeParser,
                        compactTreeFormatter = compactTreeFormatter,
                        elementFinder = elementFinder,
                    )
                server.start()
                serviceServer = server
                server.setOverlayVisible(config.serviceOverlayVisible)
                server.setRefVisible(config.serviceRefVisible)
                connectionHintWriter.write(
                    servicePort = config.servicePort,
                    serviceRunning = true,
                )
                _serverStatus.value = ServerStatus.Running(config.servicePort, config.bindingAddress.address)
                Log.i(TAG, "Service server started on ${config.bindingAddress.address}:${config.servicePort}")
                ServiceLogBus.info("SERVICE", "Started on ${config.bindingAddress.address}:${config.servicePort}")
            } catch (e: Exception) {
                Log.e(TAG, "Failed to start Service server", e)
                _serverStatus.value = ServerStatus.Error(e.message ?: "Unknown error")
                ServiceLogBus.error("SERVICE", "Start failed: ${e.message ?: "Unknown error"}")
                stopSelf()
            }
        }
    }

    private fun stopServer() {
        _serverStatus.value = ServerStatus.Stopping
        ServiceLogBus.info("SERVICE", "Stop requested")
        serviceScope.launch {
            runCatching { serviceServer?.stop() }
                .onFailure { e ->
                    Log.e(TAG, "Failed to stop Service server", e)
                    ServiceLogBus.error("SERVICE", "Stop failed: ${e.message ?: "Unknown error"}")
                }
            serviceServer = null
            val config = runCatching { settingsRepository.getServerConfig() }.getOrNull()
            if (config != null) {
                lastServicePort = config.servicePort
            }
            connectionHintWriter.write(
                servicePort = config?.servicePort ?: lastServicePort,
                serviceRunning = false,
            )
            _serverStatus.value = ServerStatus.Stopped
            ServiceLogBus.info("SERVICE", "Stopped")
            stopForeground(STOP_FOREGROUND_REMOVE)
            stopSelf()
        }
    }

    override fun onDestroy() {
        serviceServer?.stop()
        serviceServer = null
        connectionHintWriter.write(servicePort = lastServicePort, serviceRunning = false)
        runningInstance = null
        _serverStatus.value = ServerStatus.Stopped
        serviceScope.cancel()
        super.onDestroy()
    }

    private fun setOverlayVisibleInternal(visible: Boolean) {
        serviceScope.launch {
            runCatching { serviceServer?.setOverlayVisible(visible) }
                .onFailure { e ->
                    Log.w(TAG, "Failed to update overlay visible=$visible", e)
                }
        }
    }

    private fun setRefAutoRefreshInternal(enabled: Boolean) {
        serviceScope.launch {
            runCatching { serviceServer?.setRefAutoRefresh(enabled) }
                .onFailure { e ->
                    Log.w(TAG, "Failed to update ref auto refresh=$enabled", e)
                }
        }
    }

    private fun setRefVisibleInternal(visible: Boolean) {
        serviceScope.launch {
            runCatching { serviceServer?.setRefVisible(visible) }
                .onFailure { e ->
                    Log.w(TAG, "Failed to update ref visible=$visible", e)
                }
        }
    }

    private fun getRefPanelStateInternal(limit: Int): ServiceServer.RefPanelStatePayload? =
        runCatching { serviceServer?.getRefPanelState(limit) }
            .onFailure { e -> Log.w(TAG, "Failed to read ref panel state", e) }
            .getOrNull()

    private fun createNotificationChannel() {
        val channel =
            NotificationChannel(
                CHANNEL_ID,
                getString(com.memohai.autofish.R.string.service_channel_name),
                NotificationManager.IMPORTANCE_LOW,
            ).apply {
                description = getString(com.memohai.autofish.R.string.service_channel_desc)
            }
        getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }

    private fun buildNotification(): Notification =
        Notification.Builder(this, CHANNEL_ID)
            .setContentTitle("Autofish")
            .setContentText(getString(com.memohai.autofish.R.string.service_running_notification))
            .setSmallIcon(android.R.drawable.ic_menu_manage)
            .setOngoing(true)
            .build()
}
