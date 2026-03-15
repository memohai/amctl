package com.example.amctl.services.rest

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Intent
import android.os.IBinder
import android.util.Log
import com.example.amctl.data.model.ServerStatus
import com.example.amctl.data.repository.SettingsRepository
import com.example.amctl.rest.RestServer
import com.example.amctl.services.accessibility.AccessibilityServiceProvider
import com.example.amctl.services.accessibility.AccessibilityTreeParser
import com.example.amctl.services.accessibility.CompactTreeFormatter
import com.example.amctl.services.accessibility.ElementFinder
import com.example.amctl.services.system.ToolRouter
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
class RestServerService : Service() {

    @Inject lateinit var settingsRepository: SettingsRepository
    @Inject lateinit var accessibilityServiceProvider: AccessibilityServiceProvider
    @Inject lateinit var treeParser: AccessibilityTreeParser
    @Inject lateinit var compactTreeFormatter: CompactTreeFormatter
    @Inject lateinit var elementFinder: ElementFinder
    @Inject lateinit var toolRouter: ToolRouter

    private val serviceScope = CoroutineScope(Dispatchers.IO + SupervisorJob())
    private var restServer: RestServer? = null

    companion object {
        private const val TAG = "amctl:RestService"
        private const val CHANNEL_ID = "amctl_rest_server"
        private const val NOTIFICATION_ID = 2
        const val ACTION_START = "com.example.amctl.ACTION_START_REST_SERVER"
        const val ACTION_STOP = "com.example.amctl.ACTION_STOP_REST_SERVER"

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
                val server = RestServer(
                    port = config.restPort,
                    bindAddress = config.bindingAddress.address,
                    bearerToken = config.restBearerToken,
                    toolRouter = toolRouter,
                    accessibilityProvider = accessibilityServiceProvider,
                    treeParser = treeParser,
                    compactTreeFormatter = compactTreeFormatter,
                    elementFinder = elementFinder,
                )
                server.start()
                restServer = server
                _serverStatus.value = ServerStatus.Running(config.restPort, config.bindingAddress.address)
                Log.i(TAG, "REST server started on ${config.bindingAddress.address}:${config.restPort}")
            } catch (e: Exception) {
                Log.e(TAG, "Failed to start REST server", e)
                _serverStatus.value = ServerStatus.Error(e.message ?: "Unknown error")
                stopSelf()
            }
        }
    }

    private fun stopServer() {
        _serverStatus.value = ServerStatus.Stopping
        restServer?.stop()
        restServer = null
        _serverStatus.value = ServerStatus.Stopped
        stopForeground(STOP_FOREGROUND_REMOVE)
        stopSelf()
    }

    override fun onDestroy() {
        restServer?.stop()
        restServer = null
        _serverStatus.value = ServerStatus.Stopped
        serviceScope.cancel()
        super.onDestroy()
    }

    private fun createNotificationChannel() {
        val channel = NotificationChannel(CHANNEL_ID, "amctl REST API", NotificationManager.IMPORTANCE_LOW).apply {
            description = "REST API server foreground service"
        }
        getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }

    private fun buildNotification(): Notification =
        Notification.Builder(this, CHANNEL_ID)
            .setContentTitle("amctl")
            .setContentText("REST API server is running")
            .setSmallIcon(android.R.drawable.ic_menu_manage)
            .setOngoing(true)
            .build()
}
