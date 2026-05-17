package com.memohai.autofish.data.repository

import com.memohai.autofish.data.model.BindingAddress
import com.memohai.autofish.data.model.AppLanguage
import com.memohai.autofish.data.model.AppThemeMode
import com.memohai.autofish.data.model.ServerConfig
import kotlinx.coroutines.flow.Flow

@Suppress("TooManyFunctions")
interface SettingsRepository {
    val serverConfig: Flow<ServerConfig>

    suspend fun getServerConfig(): ServerConfig
    suspend fun updateBindingAddress(bindingAddress: BindingAddress)
    suspend fun updateAutoStartOnBoot(enabled: Boolean)
    suspend fun updateServicePort(port: Int)
    suspend fun updateServiceBearerToken(token: String)
    suspend fun generateNewServiceBearerToken(): String
    suspend fun updateServiceOverlayVisible(visible: Boolean)
    suspend fun updateServiceRefVisible(visible: Boolean)
    suspend fun updateAppLanguage(language: AppLanguage)
    suspend fun updateAppThemeMode(themeMode: AppThemeMode)

    fun validatePort(port: Int): Result<Int>
}
