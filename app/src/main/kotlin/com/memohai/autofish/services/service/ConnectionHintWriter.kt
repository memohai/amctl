package com.memohai.autofish.services.service

import android.content.Context
import com.memohai.autofish.BuildConfig
import dagger.hilt.android.qualifiers.ApplicationContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import java.io.File
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class ConnectionHintWriter
    @Inject
    constructor(
        @param:ApplicationContext private val context: Context,
    ) {
        fun write(
            servicePort: Int,
            serviceRunning: Boolean,
        ) {
            val file = hintFile() ?: return
            runCatching {
                file.parentFile?.mkdirs()
                file.writeText(
                    Json.encodeToString(
                        ConnectionHint(
                            packageName = BuildConfig.APPLICATION_ID,
                            versionName = BuildConfig.VERSION_NAME,
                            versionCode = BuildConfig.VERSION_CODE,
                            servicePort = servicePort,
                            serviceRunning = serviceRunning,
                            updatedAt = System.currentTimeMillis(),
                        ),
                    ),
                )
            }
        }

        private fun hintFile(): File? = context.getExternalFilesDir(null)?.resolve(FILE_NAME)

        companion object {
            const val FILE_NAME = "connection-hint.json"
        }
    }

@Serializable
private data class ConnectionHint(
    val packageName: String,
    val versionName: String,
    val versionCode: Int,
    val servicePort: Int,
    val serviceRunning: Boolean,
    val updatedAt: Long,
)
