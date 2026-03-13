package com.example.amctl.data.repository

import androidx.datastore.core.DataStore
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.intPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import com.example.amctl.data.model.BindingAddress
import com.example.amctl.data.model.ServerConfig
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.map
import java.util.UUID
import javax.inject.Inject

class SettingsRepositoryImpl
    @Inject
    constructor(
        private val dataStore: DataStore<Preferences>,
    ) : SettingsRepository {
        override val serverConfig: Flow<ServerConfig> =
            dataStore.data.map { prefs -> mapPreferencesToServerConfig(prefs) }

        override suspend fun getServerConfig(): ServerConfig {
            val config = dataStore.data.first().let { mapPreferencesToServerConfig(it) }
            if (config.bearerToken.isEmpty()) {
                val token = UUID.randomUUID().toString()
                updateBearerToken(token)
                return config.copy(bearerToken = token)
            }
            return config
        }

        override suspend fun updatePort(port: Int) {
            dataStore.edit { it[PORT_KEY] = port }
        }

        override suspend fun updateBindingAddress(bindingAddress: BindingAddress) {
            dataStore.edit { it[BINDING_ADDRESS_KEY] = bindingAddress.name }
        }

        override suspend fun updateBearerToken(token: String) {
            dataStore.edit { it[BEARER_TOKEN_KEY] = token }
        }

        override suspend fun generateNewBearerToken(): String {
            val token = UUID.randomUUID().toString()
            updateBearerToken(token)
            return token
        }

        override suspend fun updateAutoStartOnBoot(enabled: Boolean) {
            dataStore.edit { it[AUTO_START_KEY] = enabled }
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
            val bindingAddressName = prefs[BINDING_ADDRESS_KEY] ?: BindingAddress.LOCALHOST.name
            return ServerConfig(
                port = prefs[PORT_KEY] ?: ServerConfig.DEFAULT_PORT,
                bindingAddress =
                    BindingAddress.entries.firstOrNull { it.name == bindingAddressName }
                        ?: BindingAddress.LOCALHOST,
                bearerToken = prefs[BEARER_TOKEN_KEY] ?: "",
                autoStartOnBoot = prefs[AUTO_START_KEY] ?: false,
            )
        }

        companion object {
            private val PORT_KEY = intPreferencesKey("port")
            private val BINDING_ADDRESS_KEY = stringPreferencesKey("binding_address")
            private val BEARER_TOKEN_KEY = stringPreferencesKey("bearer_token")
            private val AUTO_START_KEY = booleanPreferencesKey("auto_start_on_boot")
        }
    }
