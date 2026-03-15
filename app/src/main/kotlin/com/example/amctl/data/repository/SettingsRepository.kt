package com.example.amctl.data.repository

import com.example.amctl.data.model.BindingAddress
import com.example.amctl.data.model.ServerConfig
import kotlinx.coroutines.flow.Flow

interface SettingsRepository {
    val serverConfig: Flow<ServerConfig>

    suspend fun getServerConfig(): ServerConfig
    suspend fun updatePort(port: Int)
    suspend fun updateBindingAddress(bindingAddress: BindingAddress)
    suspend fun updateBearerToken(token: String)
    suspend fun generateNewBearerToken(): String
    suspend fun updateAutoStartOnBoot(enabled: Boolean)
    suspend fun updateRestPort(port: Int)
    suspend fun updateRestBearerToken(token: String)
    suspend fun generateNewRestBearerToken(): String

    fun validatePort(port: Int): Result<Int>
}
