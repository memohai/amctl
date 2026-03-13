package com.example.amctl.services.mcp

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.os.IBinder
import android.util.Log
import com.example.amctl.data.model.ServerStatus
import com.example.amctl.data.repository.SettingsRepository
import com.example.amctl.mcp.McpServer
import com.example.amctl.services.accessibility.AccessibilityServiceProvider
import com.example.amctl.services.accessibility.AccessibilityTreeParser
import com.example.amctl.services.accessibility.ActionExecutor
import com.example.amctl.services.accessibility.CompactTreeFormatter
import com.example.amctl.services.accessibility.ElementFinder
import com.example.amctl.services.screencapture.ScreenCaptureProvider
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
class McpServerService : Service() {

    @Inject lateinit var settingsRepository: SettingsRepository
    @Inject lateinit var accessibilityServiceProvider: AccessibilityServiceProvider
    @Inject lateinit var treeParser: AccessibilityTreeParser
    @Inject lateinit var compactTreeFormatter: CompactTreeFormatter
    @Inject lateinit var elementFinder: ElementFinder
    @Inject lateinit var actionExecutor: ActionExecutor
    @Inject lateinit var screenCaptureProvider: ScreenCaptureProvider

    private val serviceScope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private var mcpServer: McpServer? = null

    companion object {
        private const val TAG = "amctl:McpService"
        private const val CHANNEL_ID = "amctl_server"
        private const val NOTIFICATION_ID = 1
        const val ACTION_START = "com.example.amctl.ACTION_START_SERVER"
        const val ACTION_STOP = "com.example.amctl.ACTION_STOP_SERVER"

        private val _serverStatus = MutableStateFlow<ServerStatus>(ServerStatus.Stopped)
        val serverStatus: StateFlow<ServerStatus> = _serverStatus
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_STOP -> stopServer()
            else -> startServer()
        }
        return START_STICKY
    }

    @Suppress("TooGenericExceptionCaught")
    private fun startServer() {
        _serverStatus.value = ServerStatus.Starting
        createNotificationChannel()
        startForeground(NOTIFICATION_ID, buildNotification())

        serviceScope.launch {
            try {
                val config = settingsRepository.getServerConfig()
                val server = McpServer(
                    port = config.port,
                    bindAddress = config.bindingAddress.address,
                    bearerToken = config.bearerToken,
                    accessibilityServiceProvider = accessibilityServiceProvider,
                    treeParser = treeParser,
                    compactTreeFormatter = compactTreeFormatter,
                    elementFinder = elementFinder,
                    actionExecutor = actionExecutor,
                    screenCaptureProvider = screenCaptureProvider,
                )
                server.start()
                mcpServer = server
                _serverStatus.value = ServerStatus.Running(config.port, config.bindingAddress.address)
                Log.i(TAG, "MCP server started on ${config.bindingAddress.address}:${config.port}")
            } catch (e: Exception) {
                Log.e(TAG, "Failed to start MCP server", e)
                _serverStatus.value = ServerStatus.Error(e.message ?: "Unknown error")
                stopSelf()
            }
        }
    }

    private fun stopServer() {
        _serverStatus.value = ServerStatus.Stopping
        mcpServer?.stop()
        mcpServer = null
        _serverStatus.value = ServerStatus.Stopped
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    override fun onDestroy() {
        mcpServer?.stop()
        mcpServer = null
        _serverStatus.value = ServerStatus.Stopped
        serviceScope.cancel()
        super.onDestroy()
    }

    private fun createNotificationChannel() {
        val channel = NotificationChannel(CHANNEL_ID, "amctl Server", NotificationManager.IMPORTANCE_LOW).apply {
            description = "MCP server foreground service"
        }
        getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }

    private fun buildNotification(): Notification =
        Notification.Builder(this, CHANNEL_ID)
            .setContentTitle("amctl")
            .setContentText("MCP server is running")
            .setSmallIcon(android.R.drawable.ic_menu_manage)
            .setOngoing(true)
            .build()
}
