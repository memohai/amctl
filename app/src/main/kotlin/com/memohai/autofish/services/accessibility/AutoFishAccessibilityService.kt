package com.memohai.autofish.services.accessibility

import android.accessibilityservice.AccessibilityService
import android.content.res.Configuration
import android.graphics.Bitmap
import android.view.accessibility.AccessibilityEvent
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull
import java.util.concurrent.atomic.AtomicLong
import kotlin.coroutines.resume

class AutoFishAccessibilityService : AccessibilityService() {

    companion object {
        @Volatile
        var instance: AutoFishAccessibilityService? = null
            private set

        val uiChangeSeq: Long
            get() = uiChangeSeqAtomic.get()

        private val uiChangeSeqAtomic = AtomicLong(0)
        private const val SCREENSHOT_TIMEOUT_MS = 5_000L
    }

    override fun onServiceConnected() {
        super.onServiceConnected()
        instance = this
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent?) {
        val eventType = event?.eventType ?: return
        if (shouldMarkUiDirty(eventType)) {
            uiChangeSeqAtomic.incrementAndGet()
        }
    }

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

    private fun shouldMarkUiDirty(eventType: Int): Boolean = when (eventType) {
        AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED,
        AccessibilityEvent.TYPE_WINDOWS_CHANGED,
        AccessibilityEvent.TYPE_WINDOW_CONTENT_CHANGED,
        AccessibilityEvent.TYPE_VIEW_SCROLLED,
        AccessibilityEvent.TYPE_VIEW_TEXT_CHANGED,
        AccessibilityEvent.TYPE_VIEW_FOCUSED,
        AccessibilityEvent.TYPE_VIEW_CLICKED,
        AccessibilityEvent.TYPE_VIEW_LONG_CLICKED
        -> true
        else -> false
    }

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
