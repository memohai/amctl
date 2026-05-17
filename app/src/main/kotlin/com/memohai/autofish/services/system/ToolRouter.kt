package com.memohai.autofish.services.system

import android.content.Context
import android.graphics.Bitmap
import android.util.Log
import android.view.KeyEvent
import com.memohai.autofish.data.model.ScreenshotData
import com.memohai.autofish.services.accessibility.AccessibilityServiceProvider
import com.memohai.autofish.services.accessibility.ActionExecutor
import com.memohai.autofish.services.accessibility.ScreenInfo
import com.memohai.autofish.services.screencapture.ScreenCaptureProvider
import com.memohai.autofish.services.screencapture.ScreenshotEncoder
import dagger.hilt.android.qualifiers.ApplicationContext
import kotlinx.coroutines.CancellationException
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
@Suppress("LongParameterList", "TooManyFunctions")
class ToolRouter
    @Inject
    constructor(
        @param:ApplicationContext private val context: Context,
        private val shizukuProvider: ShizukuProvider,
        private val systemScreenCapture: SystemScreenCapture,
        private val shellScreenCapture: ShellScreenCapture,
        private val systemInputInjector: SystemInputInjector,
        private val shellInputInjector: ShellInputInjector,
        private val appControllerImpl: AppControllerImpl,
        private val actionExecutor: ActionExecutor,
        private val screenCaptureProvider: ScreenCaptureProvider,
        private val accessibilityProvider: AccessibilityServiceProvider,
        private val screenshotEncoder: ScreenshotEncoder,
    ) {
        enum class Mode { SYSTEM_API, SHELL_CMD, ACCESSIBILITY }

        val currentMode: Mode
            get() = when {
                shizukuProvider.isAvailable() && systemInputInjector.isAvailable -> Mode.SYSTEM_API
                shizukuProvider.isAvailable() -> Mode.SHELL_CMD
                else -> Mode.ACCESSIBILITY
            }

        val isV2Available: Boolean
            get() = shizukuProvider.isAvailable()

        val appController: AppController
            get() = appControllerImpl

        suspend fun captureScreen(
            quality: Int = ScreenCaptureProvider.DEFAULT_QUALITY,
            maxWidth: Int? = null,
            maxHeight: Int? = null,
        ): Result<ScreenshotData> {
            val preferredCapture = if (shizukuProvider.isAvailable()) {
                val bitmap = systemScreenCapture.capture(
                    maxWidth = maxWidth ?: 0,
                    maxHeight = maxHeight ?: 0,
                )
                bitmap ?: shellScreenCapture.capture()
            } else {
                null
            }

            return if (preferredCapture != null) {
                encodeBitmap(preferredCapture, quality, maxWidth, maxHeight)
            } else {
                if (shizukuProvider.isAvailable()) {
                    Log.w(TAG, "v2 screen capture failed, falling back to Accessibility")
                }
                screenCaptureProvider.captureScreenshot(quality, maxWidth, maxHeight)
            }
        }

        suspend fun tap(x: Float, y: Float): Result<Unit> =
            routeInput(
                systemAction = { systemInputInjector.tap(x, y) },
                shellAction = { shellInputInjector.tap(x, y) },
                fallback = { actionExecutor.tap(x, y) },
            )

        suspend fun longPress(x: Float, y: Float, durationMs: Long): Result<Unit> =
            routeInput(
                systemAction = { systemInputInjector.longPress(x, y, durationMs) },
                shellAction = { shellInputInjector.longPress(x, y, durationMs) },
                fallback = { actionExecutor.longPress(x, y, durationMs) },
            )

        suspend fun doubleTap(x: Float, y: Float): Result<Unit> =
            routeInput(
                systemAction = { systemInputInjector.doubleTap(x, y) },
                shellAction = { shellInputInjector.doubleTap(x, y) },
                fallback = { actionExecutor.doubleTap(x, y) },
            )

        suspend fun swipe(
            x1: Float,
            y1: Float,
            x2: Float,
            y2: Float,
            durationMs: Long,
        ): Result<Unit> =
            routeInput(
                systemAction = { systemInputInjector.swipe(x1, y1, x2, y2, durationMs) },
                shellAction = { shellInputInjector.swipe(x1, y1, x2, y2, durationMs) },
                fallback = { actionExecutor.swipe(x1, y1, x2, y2, durationMs) },
            )

        suspend fun pressBack(): Result<Unit> =
            routeInput(
                systemAction = { systemInputInjector.keyEvent(KeyEvent.KEYCODE_BACK) },
                shellAction = { shellInputInjector.keyEvent(KeyEvent.KEYCODE_BACK) },
                fallback = { actionExecutor.pressBack() },
            )

        suspend fun pressHome(): Result<Unit> =
            routeInput(
                systemAction = { systemInputInjector.keyEvent(KeyEvent.KEYCODE_HOME) },
                shellAction = { shellInputInjector.keyEvent(KeyEvent.KEYCODE_HOME) },
                fallback = { actionExecutor.pressHome() },
            )

        suspend fun pressKey(keyCode: Int): Result<Unit> =
            routeInput(
                systemAction = { systemInputInjector.keyEvent(keyCode) },
                shellAction = { shellInputInjector.keyEvent(keyCode) },
                fallback = {
                    Result.failure(IllegalStateException("Key injection requires Shizuku or Accessibility"))
                },
            )

        suspend fun scroll(
            direction: com.memohai.autofish.services.accessibility.ScrollDirection,
            amount: com.memohai.autofish.services.accessibility.ScrollAmount,
        ): Result<Unit> {
            if (shizukuProvider.isAvailable()) {
                val result = try {
                    performPreferredScroll(direction, amount)
                } catch (e: CancellationException) {
                    throw e
                } catch (e: ShizukuExecutionException) {
                    Log.w(TAG, "preferred scroll failed, falling back to Accessibility", e)
                    null
                } catch (e: IllegalStateException) {
                    Log.w(TAG, "preferred scroll failed, falling back to Accessibility", e)
                    null
                } catch (e: SecurityException) {
                    Log.w(TAG, "preferred scroll failed, falling back to Accessibility", e)
                    null
                } catch (e: ClassCastException) {
                    Log.w(TAG, "preferred scroll failed, falling back to Accessibility", e)
                    null
                }
                if (result != null && result.isSuccess) {
                    return result
                }
            }
            return actionExecutor.scroll(direction, amount)
        }

        private suspend fun performPreferredScroll(
            direction: com.memohai.autofish.services.accessibility.ScrollDirection,
            amount: com.memohai.autofish.services.accessibility.ScrollAmount,
        ): Result<Unit> {
            val screenInfo = getScreenInfo()
            val w = screenInfo.width.toFloat()
            val h = screenInfo.height.toFloat()
            val cx = w / 2f
            val cy = h / 2f
            val dist = when (direction) {
                com.memohai.autofish.services.accessibility.ScrollDirection.UP,
                com.memohai.autofish.services.accessibility.ScrollDirection.DOWN,
                -> h * amount.screenPercentage
                com.memohai.autofish.services.accessibility.ScrollDirection.LEFT,
                com.memohai.autofish.services.accessibility.ScrollDirection.RIGHT,
                -> w * amount.screenPercentage
            }
            val half = dist / 2f
            return when (direction) {
                com.memohai.autofish.services.accessibility.ScrollDirection.UP ->
                    swipe(cx, cy - half, cx, cy + half, SCROLL_DURATION_MS)
                com.memohai.autofish.services.accessibility.ScrollDirection.DOWN ->
                    swipe(cx, cy + half, cx, cy - half, SCROLL_DURATION_MS)
                com.memohai.autofish.services.accessibility.ScrollDirection.LEFT ->
                    swipe(cx - half, cy, cx + half, cy, SCROLL_DURATION_MS)
                com.memohai.autofish.services.accessibility.ScrollDirection.RIGHT ->
                    swipe(cx + half, cy, cx - half, cy, SCROLL_DURATION_MS)
            }
        }

        fun getScreenInfo(): ScreenInfo = when {
            accessibilityProvider.isReady() -> accessibilityProvider.getScreenInfo()
            shizukuProvider.isAvailable() -> getScreenInfoViaShell()
            else -> getScreenInfoFromContext()
        }

        private suspend fun routeInput(
            systemAction: () -> Boolean,
            shellAction: () -> Boolean,
            fallback: suspend () -> Result<Unit>,
        ): Result<Unit> {
            val handledByPreferredPath = shizukuProvider.isAvailable() &&
                ((systemInputInjector.isAvailable && systemAction()) || shellAction())
            return if (handledByPreferredPath) Result.success(Unit) else fallback()
        }

        private fun getScreenInfoViaShell(): ScreenInfo = try {
            val sizeOutput = shizukuProvider.exec("wm size").trim()
            val densityOutput = shizukuProvider.exec("wm density").trim()

            val sizeMatch = Regex("""(\d+)x(\d+)""").find(sizeOutput)
            val width = sizeMatch?.groupValues?.get(1)?.toIntOrNull() ?: DEFAULT_SCREEN_WIDTH
            val height = sizeMatch?.groupValues?.get(2)?.toIntOrNull() ?: DEFAULT_SCREEN_HEIGHT

            val densityMatch = Regex("""(\d+)""").find(densityOutput)
            val density = densityMatch?.groupValues?.get(1)?.toIntOrNull() ?: DEFAULT_DENSITY_DPI

            ScreenInfo(
                width = width,
                height = height,
                densityDpi = density,
                orientation = if (width < height) "portrait" else "landscape",
            )
        } catch (e: ShizukuExecutionException) {
            Log.w(TAG, "getScreenInfoViaShell failed", e)
            getScreenInfoFromContext()
        }

        @Suppress("DEPRECATION")
        private fun getScreenInfoFromContext(): ScreenInfo {
            val wm = context.getSystemService(Context.WINDOW_SERVICE) as android.view.WindowManager
            val display = wm.defaultDisplay
            val metrics = android.util.DisplayMetrics()
            display.getRealMetrics(metrics)
            return ScreenInfo(
                width = metrics.widthPixels,
                height = metrics.heightPixels,
                densityDpi = metrics.densityDpi,
                orientation = if (metrics.widthPixels < metrics.heightPixels) "portrait" else "landscape",
            )
        }

        fun inputText(text: String): Boolean {
            if (shizukuProvider.isAvailable()) {
                return shellInputInjector.text(text)
            }
            return false
        }

        private fun encodeBitmap(
            bitmap: Bitmap,
            quality: Int,
            maxWidth: Int?,
            maxHeight: Int?,
        ): Result<ScreenshotData> {
            var resized: Bitmap? = null
            return try {
                resized = screenshotEncoder.resizeBitmapProportional(bitmap, maxWidth, maxHeight)
                val data = screenshotEncoder.bitmapToScreenshotData(resized, quality)
                Result.success(data)
            } finally {
                if (resized != null && resized !== bitmap) resized.recycle()
                bitmap.recycle()
            }
        }

        companion object {
            private const val TAG = "autofish:ToolRouter"
            private const val SCROLL_DURATION_MS = 300L
            private const val DEFAULT_SCREEN_WIDTH = 1080
            private const val DEFAULT_SCREEN_HEIGHT = 1920
            private const val DEFAULT_DENSITY_DPI = 420
        }
    }
