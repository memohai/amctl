package com.memohai.autofish.data.repository

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.intPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import com.memohai.autofish.data.model.AppLanguage
import com.memohai.autofish.data.model.AppThemeMode
import com.memohai.autofish.data.model.BindingAddress
import com.memohai.autofish.data.model.ServerConfig
import com.memohai.autofish.services.service.ConnectionHintWriter
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import java.util.UUID
import javax.inject.Inject

class SettingsRepositoryImpl
    @Inject
    constructor(
        private val dataStore: DataStore<Preferences>,
        private val connectionHintWriter: ConnectionHintWriter,
    ) : SettingsRepository {
        override val serverConfig: Flow<ServerConfig> =
            dataStore.data.map { prefs -> mapPreferencesToServerConfig(prefs) }

        override suspend fun getServerConfig(): ServerConfig {
            var config = dataStore.data.first().let { mapPreferencesToServerConfig(it) }
            if (config.serviceBearerToken.isEmpty()) {
                val token = UUID.randomUUID().toString()
                updateServiceBearerToken(token)
                config = config.copy(serviceBearerToken = token)
            }
            return config
        }

        override suspend fun updateBindingAddress(bindingAddress: BindingAddress) {
            dataStore.edit { it[BINDING_ADDRESS_KEY] = bindingAddress.name }
        }

        override suspend fun updateAutoStartOnBoot(enabled: Boolean) {
            dataStore.edit { it[AUTO_START_KEY] = enabled }
        }

        override suspend fun updateServicePort(port: Int) {
            dataStore.edit { it[SERVICE_PORT_KEY] = port }
            connectionHintWriter.write(servicePort = port, serviceRunning = false)
        }

        override suspend fun updateServiceBearerToken(token: String) {
            dataStore.edit { it[SERVICE_BEARER_TOKEN_KEY] = token }
        }

        override suspend fun generateNewServiceBearerToken(): String {
            val token = UUID.randomUUID().toString()
            updateServiceBearerToken(token)
            return token
        }

        override suspend fun updateServiceOverlayVisible(visible: Boolean) {
            dataStore.edit { it[SERVICE_OVERLAY_VISIBLE_KEY] = visible }
        }

        override suspend fun updateServiceRefVisible(visible: Boolean) {
            dataStore.edit { it[SERVICE_REF_VISIBLE_KEY] = visible }
        }

        override suspend fun updateAppLanguage(language: AppLanguage) {
            dataStore.edit { it[APP_LANGUAGE_KEY] = language.name }
        }

        override suspend fun updateAppThemeMode(themeMode: AppThemeMode) {
            dataStore.edit { it[APP_THEME_MODE_KEY] = themeMode.name }
        }

        override fun validatePort(port: Int): Result<Int> =
            if (port in ServerConfig.MIN_PORT..ServerConfig.MAX_PORT) {
                Result.success(port)
            } else {
                Result.failure(
                    IllegalArgumentException(
                        "Port must be between ${ServerConfig.MIN_PORT} and ${ServerConfig.MAX_PORT}",
                    ),
                )
            }

        private fun mapPreferencesToServerConfig(prefs: Preferences): ServerConfig {
            val bindingAddressName = prefs[BINDING_ADDRESS_KEY] ?: BindingAddress.ALL_INTERFACES.name
            val appLanguageName = prefs[APP_LANGUAGE_KEY] ?: AppLanguage.SYSTEM.name
            val appThemeModeName = prefs[APP_THEME_MODE_KEY] ?: AppThemeMode.LIGHT.name
            return ServerConfig(
                bindingAddress =
                    BindingAddress.entries
                        .firstOrNull { it.name == bindingAddressName }
                        ?.takeUnless { it == BindingAddress.LOCALHOST }
                        ?: BindingAddress.ALL_INTERFACES,
                autoStartOnBoot = prefs[AUTO_START_KEY] ?: false,
                servicePort = prefs[SERVICE_PORT_KEY] ?: ServerConfig.DEFAULT_PORT,
                serviceBearerToken = prefs[SERVICE_BEARER_TOKEN_KEY] ?: "",
                serviceOverlayVisible = prefs[SERVICE_OVERLAY_VISIBLE_KEY] ?: false,
                serviceRefVisible = prefs[SERVICE_REF_VISIBLE_KEY] ?: false,
                appLanguage = AppLanguage.entries.firstOrNull { it.name == appLanguageName } ?: AppLanguage.SYSTEM,
                appThemeMode = AppThemeMode.entries.firstOrNull { it.name == appThemeModeName } ?: AppThemeMode.LIGHT,
            )
        }

        companion object {
            private val BINDING_ADDRESS_KEY = stringPreferencesKey("binding_address")
            private val AUTO_START_KEY = booleanPreferencesKey("auto_start_on_boot")
            private val SERVICE_PORT_KEY = intPreferencesKey("service_port")
            private val SERVICE_BEARER_TOKEN_KEY = stringPreferencesKey("service_bearer_token")
            private val SERVICE_OVERLAY_VISIBLE_KEY = booleanPreferencesKey("service_overlay_visible")
            private val SERVICE_REF_VISIBLE_KEY = booleanPreferencesKey("service_ref_visible")
            private val APP_LANGUAGE_KEY = stringPreferencesKey("app_language")
            private val APP_THEME_MODE_KEY = stringPreferencesKey("app_theme_mode")
        }
    }
