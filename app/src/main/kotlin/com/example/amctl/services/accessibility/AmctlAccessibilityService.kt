package com.example.amctl.services.accessibility

import android.accessibilityservice.AccessibilityService
import android.content.res.Configuration
import android.graphics.Bitmap
import android.util.DisplayMetrics
import android.view.accessibility.AccessibilityEvent
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull
import kotlin.coroutines.resume

class AmctlAccessibilityService : AccessibilityService() {

    companion object {
        @Volatile
        var instance: AmctlAccessibilityService? = null
            private set

        private const val SCREENSHOT_TIMEOUT_MS = 5_000L
    }

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {}

    override fun onInterrupt() {}

    override fun onDestroy() {
        instance = null
        super.onDestroy()
    }

    fun getScreenInfo(): ScreenInfo {
        val dm = resources.displayMetrics
        val orientation = when (resources.configuration.orientation) {
            Configuration.ORIENTATION_LANDSCAPE -> "landscape"
            else -> "portrait"
        }
        return ScreenInfo(
            width = dm.widthPixels,
            height = dm.heightPixels,
            densityDpi = dm.densityDpi,
            orientation = orientation,
        )
    }

    fun canTakeScreenshot(): Boolean = true

    @Suppress("NewApi")
    suspend fun takeScreenshotBitmap(): Bitmap? =
        withTimeoutOrNull(SCREENSHOT_TIMEOUT_MS) {
            suspendCancellableCoroutine { continuation ->
                takeScreenshot(
                    android.view.Display.DEFAULT_DISPLAY,
                    mainExecutor,
                    object : TakeScreenshotCallback {
                        override fun onSuccess(screenshot: ScreenshotResult) {
                            val bitmap = Bitmap.wrapHardwareBuffer(
                                screenshot.hardwareBuffer,
                                screenshot.colorSpace,
                            )
                            screenshot.hardwareBuffer.close()
                            if (continuation.isActive) continuation.resume(bitmap)
                        }
                        override fun onFailure(errorCode: Int) {
                            if (continuation.isActive) continuation.resume(null)
                        }
                    },
                )
            }
        }
}
