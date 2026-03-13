package com.example.amctl.services.screencapture

import com.example.amctl.data.model.ScreenshotData

interface ScreenCaptureProvider {
    suspend fun captureScreenshot(
        quality: Int = DEFAULT_QUALITY,
        maxWidth: Int? = null,
        maxHeight: Int? = null,
    ): Result<ScreenshotData>

    fun isScreenCaptureAvailable(): Boolean

    companion object {
        const val DEFAULT_QUALITY = 80
    }
}
