@file:Suppress("DEPRECATION")

package com.memohai.autofish.ui.screens

import android.content.Intent
import android.provider.Settings
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Home
import androidx.compose.material.icons.filled.Info
import androidx.compose.material.icons.filled.KeyboardArrowDown
import androidx.compose.material.icons.filled.List
import androidx.compose.material.icons.filled.MoreVert
import androidx.compose.material.icons.filled.Pause
import androidx.compose.material.icons.filled.PlayArrow
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Warning
import androidx.compose.material.icons.filled.Error
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.FilterList
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.PointerType
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.pointer.positionChange
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import com.memohai.autofish.BuildConfig
import com.memohai.autofish.R
import com.memohai.autofish.data.model.AppLanguage
import com.memohai.autofish.data.model.AppThemeMode
import com.memohai.autofish.data.model.ServerStatus
import com.memohai.autofish.services.logging.ServiceLogBus
import com.memohai.autofish.services.logging.ServiceLogEntry
import com.memohai.autofish.ui.components.ConnectionInfoCard
import com.memohai.autofish.ui.components.ServiceConfigurationSection
import com.memohai.autofish.ui.components.ServerStatusCard
import com.memohai.autofish.ui.viewmodels.MainViewModel
import kotlinx.coroutines.launch
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun HomeScreen(viewModel: MainViewModel = hiltViewModel()) {
    val serverConfig by viewModel.serverConfig.collectAsState()
    val serviceServerStatus by viewModel.serviceServerStatus.collectAsState()
    val deviceIp by viewModel.deviceIp.collectAsState()
    val shizukuStatus by viewModel.shizukuStatus.collectAsState()
    val controlMode by viewModel.controlMode.collectAsState()
    val accessibilityEnabled by viewModel.accessibilityEnabled.collectAsState()
    val notificationsEnabled by viewModel.notificationsEnabled.collectAsState()
    val refPanelState by viewModel.refPanelState.collectAsState()
    val context = LocalContext.current
    val isServiceRunning = serviceServerStatus is ServerStatus.Running
    var selectedTab by rememberSaveable { mutableStateOf(UiTab.Home) }
    var selectedSettingPage by rememberSaveable { mutableStateOf(SettingPage.Menu) }
    var showLanguageRestartDialog by rememberSaveable { mutableStateOf(false) }
    val logs = remember { mutableStateListOf<ServiceLogEntry>() }
    val pausedBuffer = remember { mutableListOf<ServiceLogEntry>() }
    var logPaused by rememberSaveable { mutableStateOf(false) }
    var logSearchVisible by rememberSaveable { mutableStateOf(false) }
    var logSearchQuery by rememberSaveable { mutableStateOf("") }
    var logMenuExpanded by remember { mutableStateOf(false) }
    var logLevelSubmenuExpanded by remember { mutableStateOf(false) }
    var logLevelFilter by rememberSaveable { mutableStateOf(LogLevelFilter.ALL) }
    val latestLogPaused by rememberUpdatedState(logPaused)
    val canGoBack = selectedTab == UiTab.Setting && selectedSettingPage != SettingPage.Menu
    val topTitle = when (selectedTab) {
        UiTab.Home -> stringResource(R.string.tab_home)
        UiTab.Log -> stringResource(R.string.tab_log)
        UiTab.Setting -> when (selectedSettingPage) {
            SettingPage.Menu -> stringResource(R.string.tab_setting)
            SettingPage.App -> stringResource(R.string.settings_app)
            SettingPage.Service -> stringResource(R.string.settings_service)
            SettingPage.About -> stringResource(R.string.settings_about)
        }
    }

    BackHandler(enabled = selectedSettingPage != SettingPage.Menu || selectedTab != UiTab.Home) {
        when {
            selectedSettingPage != SettingPage.Menu -> selectedSettingPage = SettingPage.Menu
            selectedTab != UiTab.Home -> selectedTab = UiTab.Home
        }
    }

    val installDate by remember {
        mutableStateOf(
            runCatching {
                val packageInfo = context.packageManager.getPackageInfo(context.packageName, 0)
                val formatter = SimpleDateFormat("yyyy-MM-dd HH:mm:ss", Locale.getDefault())
                formatter.format(Date(packageInfo.firstInstallTime))
            }.getOrDefault("Unknown"),
        )
    }

    LaunchedEffect(Unit) {
        viewModel.refreshDeviceIp()
        logs.clear()
        logs.addAll(ServiceLogBus.snapshot())
        ServiceLogBus.events.collect { entry ->
            if (latestLogPaused) {
                pausedBuffer.add(entry)
            } else {
                logs.add(entry)
            }
        }
    }
    LaunchedEffect(isServiceRunning) {
        if (isServiceRunning) {
            viewModel.refreshDeviceIp()
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(topTitle) },
                navigationIcon = {
                    if (canGoBack) {
                        IconButton(onClick = { selectedSettingPage = SettingPage.Menu }) {
                            Icon(
                                imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                                contentDescription = stringResource(R.string.back),
                            )
                        }
                    }
                },
                actions = {
                    if (selectedTab == UiTab.Log) {
                        IconButton(onClick = {
                            if (logPaused && pausedBuffer.isNotEmpty()) {
                                logs.addAll(pausedBuffer)
                                pausedBuffer.clear()
                            }
                            logPaused = !logPaused
                        }) {
                            Icon(
                                imageVector = if (logPaused) Icons.Default.PlayArrow else Icons.Default.Pause,
                                contentDescription = if (logPaused) {
                                    stringResource(R.string.resume_logs)
                                } else {
                                    stringResource(R.string.pause_logs)
                                },
                            )
                        }
                        IconButton(onClick = { logSearchVisible = !logSearchVisible }) {
                            Icon(
                                imageVector = Icons.Default.Search,
                                contentDescription = stringResource(R.string.search_logs),
                            )
                        }
                        Box {
                            IconButton(onClick = { logMenuExpanded = true }) {
                                Icon(
                                    imageVector = Icons.Default.MoreVert,
                                    contentDescription = stringResource(R.string.more_options),
                                )
                            }
                            DropdownMenu(
                                expanded = logMenuExpanded,
                                onDismissRequest = { logMenuExpanded = false },
                            ) {
                                DropdownMenuItem(
                                    text = { Text(stringResource(R.string.log_level)) },
                                    leadingIcon = {
                                        Icon(Icons.Default.FilterList, contentDescription = null)
                                    },
                                    onClick = {
                                        logMenuExpanded = false
                                        logLevelSubmenuExpanded = true
                                    },
                                )
                                DropdownMenuItem(
                                    text = { Text(stringResource(R.string.clear_logs)) },
                                    leadingIcon = {
                                        Icon(Icons.Default.Delete, contentDescription = null)
                                    },
                                    onClick = {
                                        ServiceLogBus.clear()
                                        pausedBuffer.clear()
                                        logs.clear()
                                        logMenuExpanded = false
                                        logLevelSubmenuExpanded = false
                                    },
                                )
                            }
                            DropdownMenu(
                                expanded = logLevelSubmenuExpanded,
                                onDismissRequest = { logLevelSubmenuExpanded = false },
                            ) {
                                LogLevelFilter.entries.forEach { filter ->
                                    DropdownMenuItem(
                                        text = { Text(filter.displayName()) },
                                        leadingIcon = {
                                            Icon(filter.icon(), contentDescription = null)
                                        },
                                        onClick = {
                                            logLevelFilter = filter
                                            logLevelSubmenuExpanded = false
                                        },
                                    )
                                }
                            }
                        }
                    }
                },
            )
        },
        bottomBar = {
            NavigationBar {
                NavigationBarItem(
                    selected = selectedTab == UiTab.Home,
                    onClick = { selectedTab = UiTab.Home },
                    icon = { Icon(Icons.Default.Home, contentDescription = stringResource(R.string.tab_home)) },
                    label = { Text(stringResource(R.string.tab_home)) },
                )
                NavigationBarItem(
                    selected = selectedTab == UiTab.Log,
                    onClick = { selectedTab = UiTab.Log },
                    icon = { Icon(Icons.Default.List, contentDescription = stringResource(R.string.tab_log)) },
                    label = { Text(stringResource(R.string.tab_log)) },
                )
                NavigationBarItem(
                    selected = selectedTab == UiTab.Setting,
                    onClick = { selectedTab = UiTab.Setting },
                    icon = { Icon(Icons.Default.Settings, contentDescription = stringResource(R.string.tab_setting)) },
                    label = { Text(stringResource(R.string.tab_setting)) },
                )
            }
        },
    ) { innerPadding ->
        if (showLanguageRestartDialog) {
            AlertDialog(
                onDismissRequest = { showLanguageRestartDialog = false },
                title = { Text(stringResource(R.string.language_restart_title)) },
                text = { Text(stringResource(R.string.language_restart_message)) },
                confirmButton = {
                    TextButton(onClick = {
                        showLanguageRestartDialog = false
                        viewModel.restartApp()
                    }) {
                        Text(stringResource(R.string.restart_now))
                    }
                },
                dismissButton = {
                    TextButton(onClick = { showLanguageRestartDialog = false }) {
                        Text(stringResource(R.string.later))
                    }
                },
            )
        }

        when (selectedTab) {
            UiTab.Home -> {
                val scrollState = rememberScrollState()
                Column(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(innerPadding)
                        .padding(horizontal = 16.dp)
                        .verticalScroll(scrollState)
                        .mouseDragScrollable(scrollState),
                    verticalArrangement = Arrangement.spacedBy(16.dp),
                ) {
                    ServerStatusCard(
                        title = stringResource(R.string.service_api_server),
                        serverStatus = serviceServerStatus,
                        onToggle = { enabled ->
                            if (enabled) viewModel.startServiceServer() else viewModel.stopServiceServer()
                        },
                    )
                    ConnectionInfoCard(
                        deviceIp = deviceIp,
                        servicePort = serverConfig.servicePort,
                        serviceBearerToken = serverConfig.serviceBearerToken,
                        isServiceRunning = isServiceRunning,
                    )
                    Card(modifier = Modifier.fillMaxWidth()) {
                        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            Row(verticalAlignment = Alignment.CenterVertically) {
                                Text(stringResource(R.string.control_mode), style = MaterialTheme.typography.titleMedium)
                                Text(
                                    text = "  $controlMode",
                                    style = MaterialTheme.typography.bodyMedium,
                                    color = if (controlMode != "ACCESSIBILITY") {
                                        MaterialTheme.colorScheme.primary
                                    } else {
                                        MaterialTheme.colorScheme.onSurfaceVariant
                                    },
                                )
                            }

                            Text(stringResource(R.string.shizuku), style = MaterialTheme.typography.titleSmall)
                            Text(
                                text = shizukuStatus,
                                style = MaterialTheme.typography.bodyMedium,
                                color = when {
                                    shizukuStatus.contains("Authorized") -> MaterialTheme.colorScheme.primary
                                    shizukuStatus.contains("Not") -> MaterialTheme.colorScheme.error
                                    else -> MaterialTheme.colorScheme.onSurfaceVariant
                                },
                            )
                            if (shizukuStatus.contains("Not Authorized")) {
                                Button(onClick = { viewModel.requestShizukuPermission() }) {
                                    Text(stringResource(R.string.request_shizuku_permission))
                                }
                            }
                        }
                    }
                    Card(modifier = Modifier.fillMaxWidth()) {
                        Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                            Text(stringResource(R.string.accessibility_service), style = MaterialTheme.typography.titleMedium)
                            Text(
                                text = if (accessibilityEnabled) stringResource(R.string.enabled) else stringResource(R.string.disabled),
                                style = MaterialTheme.typography.bodyMedium,
                                color = if (accessibilityEnabled) {
                                    MaterialTheme.colorScheme.primary
                                } else {
                                    MaterialTheme.colorScheme.error
                                },
                            )
                            if (!accessibilityEnabled) {
                                Button(onClick = {
                                    context.startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS))
                                }) {
                                    Text(stringResource(R.string.open_accessibility_settings))
                                }
                            }
                        }
                    }
                }
            }

            UiTab.Log -> {
                LogTabContent(
                    innerPadding = innerPadding,
                    logs = logs,
                    isSearchVisible = logSearchVisible,
                    searchQuery = logSearchQuery,
                    onSearchQueryChange = { logSearchQuery = it },
                    levelFilter = logLevelFilter,
                    hasRunningService = isServiceRunning,
                )
            }

            UiTab.Setting -> {
                val scrollState = rememberScrollState()
                Column(
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(innerPadding)
                        .padding(horizontal = 16.dp)
                        .verticalScroll(scrollState)
                        .mouseDragScrollable(scrollState),
                    verticalArrangement = Arrangement.spacedBy(16.dp),
                ) {
                    when (selectedSettingPage) {
                        SettingPage.Menu -> {
                            Text(
                                text = stringResource(R.string.tab_setting),
                                style = MaterialTheme.typography.labelLarge,
                                color = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                            )
                            Card(modifier = Modifier.fillMaxWidth()) {
                                Column(verticalArrangement = Arrangement.spacedBy(0.dp)) {
                                    ListItem(
                                        headlineContent = { Text(stringResource(R.string.settings_app)) },
                                        leadingContent = { Icon(Icons.Default.Settings, contentDescription = null) },
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .clickable { selectedSettingPage = SettingPage.App },
                                        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                                        trailingContent = { Text(">", color = MaterialTheme.colorScheme.onSurfaceVariant) },
                                    )

                                    ListItem(
                                        headlineContent = { Text(stringResource(R.string.settings_service)) },
                                        leadingContent = { Icon(Icons.Default.List, contentDescription = null) },
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .clickable { selectedSettingPage = SettingPage.Service },
                                        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                                        trailingContent = { Text(">", color = MaterialTheme.colorScheme.onSurfaceVariant) },
                                    )

                                    ListItem(
                                        headlineContent = { Text(stringResource(R.string.settings_about)) },
                                        leadingContent = { Icon(Icons.Default.Home, contentDescription = null) },
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .clickable { selectedSettingPage = SettingPage.About },
                                        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                                        trailingContent = { Text(">", color = MaterialTheme.colorScheme.onSurfaceVariant) },
                                    )
                                }
                            }
                        }

                        SettingPage.App -> {
                            Text(
                                text = stringResource(R.string.settings_app),
                                style = MaterialTheme.typography.labelLarge,
                                color = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                            )
                            Card(
                                modifier = Modifier.fillMaxWidth(),
                                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
                            ) {
                                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                                    Text(stringResource(R.string.app_details), style = MaterialTheme.typography.titleMedium)

                                    Text(stringResource(R.string.language), style = MaterialTheme.typography.labelLarge)
                                    Row(verticalAlignment = Alignment.CenterVertically) {
                                        RadioButton(
                                            selected = serverConfig.appLanguage == AppLanguage.SYSTEM,
                                            onClick = {
                                                if (serverConfig.appLanguage != AppLanguage.SYSTEM) {
                                                    viewModel.updateAppLanguage(AppLanguage.SYSTEM)
                                                    showLanguageRestartDialog = true
                                                }
                                            },
                                        )
                                        Text(stringResource(R.string.follow_system), modifier = Modifier.padding(end = 12.dp))
                                        RadioButton(
                                            selected = serverConfig.appLanguage == AppLanguage.CHINESE,
                                            onClick = {
                                                if (serverConfig.appLanguage != AppLanguage.CHINESE) {
                                                    viewModel.updateAppLanguage(AppLanguage.CHINESE)
                                                    showLanguageRestartDialog = true
                                                }
                                            },
                                        )
                                        Text(stringResource(R.string.language_zh), modifier = Modifier.padding(end = 12.dp))
                                        RadioButton(
                                            selected = serverConfig.appLanguage == AppLanguage.ENGLISH,
                                            onClick = {
                                                if (serverConfig.appLanguage != AppLanguage.ENGLISH) {
                                                    viewModel.updateAppLanguage(AppLanguage.ENGLISH)
                                                    showLanguageRestartDialog = true
                                                }
                                            },
                                        )
                                        Text(stringResource(R.string.language_en))
                                    }

                                    Text(stringResource(R.string.theme), style = MaterialTheme.typography.labelLarge)
                                    Row(verticalAlignment = Alignment.CenterVertically) {
                                        RadioButton(
                                            selected = serverConfig.appThemeMode == AppThemeMode.LIGHT,
                                            onClick = { viewModel.updateAppThemeMode(AppThemeMode.LIGHT) },
                                        )
                                        Text(stringResource(R.string.theme_light), modifier = Modifier.padding(end = 16.dp))
                                        RadioButton(
                                            selected = serverConfig.appThemeMode == AppThemeMode.DARK,
                                            onClick = { viewModel.updateAppThemeMode(AppThemeMode.DARK) },
                                        )
                                        Text(stringResource(R.string.theme_dark))
                                    }

                                    Text(stringResource(R.string.notifications), style = MaterialTheme.typography.labelLarge)
                                    Text(
                                        text = if (notificationsEnabled) stringResource(R.string.enabled) else stringResource(R.string.disabled),
                                        style = MaterialTheme.typography.bodyMedium,
                                        color = if (notificationsEnabled) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.error,
                                    )
                                }
                            }
                        }

                        SettingPage.Service -> {
                            Text(
                                text = stringResource(R.string.settings_service),
                                style = MaterialTheme.typography.labelLarge,
                                color = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                            )
                            ServiceConfigurationSection(
                                config = serverConfig,
                                isServerRunning = isServiceRunning,
                                onPortChange = viewModel::updateServicePort,
                                onRegenerateToken = viewModel::generateNewServiceBearerToken,
                            )
                            Card(
                                modifier = Modifier.fillMaxWidth(),
                                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
                            ) {
                                Row(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .padding(16.dp),
                                    horizontalArrangement = Arrangement.SpaceBetween,
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Column(
                                        modifier = Modifier.weight(1f),
                                        verticalArrangement = Arrangement.spacedBy(4.dp),
                                    ) {
                                        Text(
                                            text = stringResource(R.string.overlay_visible),
                                            style = MaterialTheme.typography.titleMedium,
                                        )
                                        Text(
                                            text = stringResource(R.string.overlay_visible_desc),
                                            style = MaterialTheme.typography.bodySmall,
                                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                                        )
                                    }
                                    Switch(
                                        checked = serverConfig.serviceOverlayVisible,
                                        onCheckedChange = viewModel::updateServiceOverlayVisible,
                                    )
                                }
                            }
                            Card(
                                modifier = Modifier.fillMaxWidth(),
                                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
                            ) {
                                Column(
                                    modifier = Modifier
                                        .fillMaxWidth()
                                        .padding(16.dp),
                                    verticalArrangement = Arrangement.spacedBy(8.dp),
                                ) {
                                    Row(
                                        modifier = Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Column(
                                            modifier = Modifier.weight(1f),
                                            verticalArrangement = Arrangement.spacedBy(4.dp),
                                        ) {
                                            Text(
                                                text = stringResource(R.string.ref_state),
                                                style = MaterialTheme.typography.titleMedium,
                                            )
                                        }
                                        Switch(
                                            checked = serverConfig.serviceRefVisible,
                                            onCheckedChange = viewModel::updateServiceRefVisible,
                                            enabled = isServiceRunning,
                                        )
                                    }
                                    Row(
                                        modifier = Modifier.fillMaxWidth(),
                                        horizontalArrangement = Arrangement.SpaceBetween,
                                        verticalAlignment = Alignment.CenterVertically,
                                    ) {
                                        Text(
                                            text = stringResource(R.string.ref_auto_refresh),
                                            style = MaterialTheme.typography.bodyMedium,
                                        )
                                        Switch(
                                            checked = refPanelState?.autoRefresh ?: true,
                                            onCheckedChange = viewModel::updateRefAutoRefresh,
                                            enabled = isServiceRunning,
                                        )
                                    }
                                    Text(
                                        text = stringResource(
                                            R.string.ref_state_summary,
                                            refPanelState?.version ?: 0L,
                                            refPanelState?.count ?: 0,
                                        ),
                                        style = MaterialTheme.typography.bodyMedium,
                                    )
                                }
                            }
                        }

                        SettingPage.About -> {
                            Text(
                                text = stringResource(R.string.settings_about),
                                style = MaterialTheme.typography.labelLarge,
                                color = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp),
                            )
                            Card(
                                modifier = Modifier.fillMaxWidth(),
                                colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
                            ) {
                                Column(modifier = Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                                    Text(stringResource(R.string.settings_about), style = MaterialTheme.typography.titleMedium)
                                    Text("${stringResource(R.string.app_name_label)}: Autofish", style = MaterialTheme.typography.bodyMedium)
                                    Text("${stringResource(R.string.version_label)}: ${BuildConfig.VERSION_NAME} (${BuildConfig.VERSION_CODE})", style = MaterialTheme.typography.bodyMedium)
                                    Text(
                                        "${stringResource(R.string.build_type_label)}: ${if (BuildConfig.DEBUG) "Debug" else "Release"}",
                                        style = MaterialTheme.typography.bodyMedium,
                                    )
                                    Text("${stringResource(R.string.install_date_label)}: $installDate", style = MaterialTheme.typography.bodyMedium)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun LogTabContent(
    innerPadding: PaddingValues,
    logs: List<ServiceLogEntry>,
    isSearchVisible: Boolean,
    searchQuery: String,
    onSearchQueryChange: (String) -> Unit,
    levelFilter: LogLevelFilter,
    hasRunningService: Boolean,
) {
    val listState = rememberLazyListState()
    val coroutineScope = rememberCoroutineScope()
    val hasAnyHistoryLogs by remember(logs) {
        derivedStateOf { logs.isNotEmpty() }
    }
    val displayedLogs by remember(logs, levelFilter, searchQuery, isSearchVisible) {
        derivedStateOf {
            logs.filter { entry ->
                val levelMatched = levelFilter.matches(entry.level)
                val query = searchQuery.trim()
                val queryMatched = if (query.isEmpty()) {
                    true
                } else {
                    entry.message.contains(query, ignoreCase = true) ||
                        entry.source.contains(query, ignoreCase = true) ||
                        entry.level.contains(query, ignoreCase = true)
                }
                levelMatched && queryMatched
            }
        }
    }

    val isAtBottom by remember {
        derivedStateOf {
            val layoutInfo = listState.layoutInfo
            if (layoutInfo.totalItemsCount == 0) {
                true
            } else {
                val lastVisibleItem = layoutInfo.visibleItemsInfo.lastOrNull()
                lastVisibleItem != null && lastVisibleItem.index >= layoutInfo.totalItemsCount - 1
            }
        }
    }
    val showScrollToBottomFab by remember {
        derivedStateOf {
            if (displayedLogs.isEmpty()) {
                false
            } else {
                val lastVisibleIndex = listState.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: -1
                lastVisibleIndex < displayedLogs.lastIndex
            }
        }
    }

    LaunchedEffect(displayedLogs.size, isAtBottom) {
        if (isAtBottom && displayedLogs.isNotEmpty() && !isSearchVisible) {
            listState.animateScrollToItem(displayedLogs.lastIndex)
        }
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .padding(innerPadding)
            .padding(horizontal = 16.dp),
    ) {
        Column(
            modifier = Modifier.fillMaxSize(),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            AnimatedVisibility(
                visible = isSearchVisible,
                enter = expandVertically(animationSpec = tween(180)) + fadeIn(animationSpec = tween(180)),
                exit = shrinkVertically(animationSpec = tween(140)) + fadeOut(animationSpec = tween(140)),
            ) {
                OutlinedTextField(
                    value = searchQuery,
                    onValueChange = onSearchQueryChange,
                    modifier = Modifier.fillMaxWidth(),
                    singleLine = true,
                    label = { Text(stringResource(R.string.search_logs)) },
                )
            }

            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .weight(1f),
            ) {
                if (!hasRunningService && !hasAnyHistoryLogs) {
                    Box(
                        modifier = Modifier.fillMaxSize(),
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(
                            text = stringResource(R.string.service_not_started),
                            style = MaterialTheme.typography.bodyLarge,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                } else if (displayedLogs.isEmpty()) {
                    Column(
                        modifier = Modifier
                            .fillMaxSize()
                            .padding(16.dp),
                        verticalArrangement = Arrangement.Center,
                        horizontalAlignment = Alignment.CenterHorizontally,
                    ) {
                        Text(
                            text = stringResource(R.string.no_logs_matched),
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                } else {
                    LazyColumn(
                        state = listState,
                        modifier = Modifier.fillMaxSize(),
                        contentPadding = PaddingValues(vertical = 4.dp),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        items(displayedLogs, key = { it.id }) { entry ->
                            LogListItem(entry)
                        }
                    }
                }
            }
        }

        if (showScrollToBottomFab) {
            FloatingActionButton(
                onClick = {
                    coroutineScope.launch {
                        listState.animateScrollToItem(displayedLogs.lastIndex)
                    }
                },
                modifier = Modifier
                    .align(Alignment.BottomEnd)
                    .padding(bottom = 12.dp, end = 4.dp),
            ) {
                Icon(
                    imageVector = Icons.Default.KeyboardArrowDown,
                    contentDescription = stringResource(R.string.scroll_to_bottom),
                )
            }
        }
    }
}

@Composable
private fun LogListItem(entry: ServiceLogEntry) {
    val time = SimpleDateFormat("HH:mm:ss.SSS", Locale.getDefault()).format(Date(entry.timestampMs))
    val levelColor = when (entry.level.uppercase(Locale.ROOT)) {
        "ERROR" -> MaterialTheme.colorScheme.error
        "WARN", "WARNING" -> Color(0xFFB26A00)
        "INFO" -> MaterialTheme.colorScheme.primary
        else -> MaterialTheme.colorScheme.onSurfaceVariant
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerLow),
    ) {
        Column(
            modifier = Modifier
                .fillMaxWidth()
                .padding(horizontal = 12.dp, vertical = 10.dp),
            verticalArrangement = Arrangement.spacedBy(4.dp),
        ) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    text = time,
                    style = MaterialTheme.typography.labelSmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    text = entry.level.uppercase(Locale.ROOT),
                    style = MaterialTheme.typography.labelSmall,
                    color = levelColor,
                )
            }
            Text(
                text = entry.source,
                style = MaterialTheme.typography.labelSmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            Text(
                text = entry.message,
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurface,
            )
        }
    }
}

private enum class LogLevelFilter {
    ALL,
    INFO,
    WARN,
    ERROR,
    ;

    fun matches(level: String): Boolean {
        val upper = level.uppercase(Locale.ROOT)
        return when (this) {
            ALL -> true
            INFO -> upper == "INFO"
            WARN -> upper == "WARN" || upper == "WARNING"
            ERROR -> upper == "ERROR"
        }
    }
}

@Composable
private fun LogLevelFilter.displayName(): String = when (this) {
    LogLevelFilter.ALL -> stringResource(R.string.log_level_all)
    LogLevelFilter.INFO -> stringResource(R.string.log_level_info)
    LogLevelFilter.WARN -> stringResource(R.string.log_level_warn)
    LogLevelFilter.ERROR -> stringResource(R.string.log_level_error)
}

private fun LogLevelFilter.icon(): ImageVector = when (this) {
    LogLevelFilter.ALL -> Icons.Default.List
    LogLevelFilter.INFO -> Icons.Default.Info
    LogLevelFilter.WARN -> Icons.Default.Warning
    LogLevelFilter.ERROR -> Icons.Default.Error
}

private enum class UiTab {
    Home,
    Log,
    Setting,
}

private enum class SettingPage {
    Menu,
    App,
    Service,
    About,
}

private fun Modifier.mouseDragScrollable(
    scrollState: ScrollState,
): Modifier = pointerInput(scrollState) {
    awaitEachGesture {
        val down = awaitFirstDown(requireUnconsumed = false)
        if (down.type != PointerType.Mouse) return@awaitEachGesture

        var pointerId = down.id
        while (true) {
            val event = awaitPointerEvent()
            val change = event.changes.firstOrNull { it.id == pointerId } ?: break
            if (!change.pressed) break

            val dy = change.positionChange().y
            if (dy != 0f) {
                scrollState.dispatchRawDelta(-dy)
                change.consume()
            }
        }
    }
}
