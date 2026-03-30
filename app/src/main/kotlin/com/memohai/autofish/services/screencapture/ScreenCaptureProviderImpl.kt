package com.memohai.autofish.services.screencapture

import android.graphics.Bitmap
import com.memohai.autofish.core.ToolException
import com.memohai.autofish.data.model.ScreenshotData
import com.memohai.autofish.services.accessibility.AccessibilityServiceProvider
import com.memohai.autofish.services.accessibility.AutoFishAccessibilityService
import javax.inject.Inject

class ScreenCaptureProviderImpl
    @Inject
    constructor(
        private val screenshotEncoder: ScreenshotEncoder,
        private val accessibilityServiceProvider: AccessibilityServiceProvider,
    ) : ScreenCaptureProvider {
        @Suppress("ReturnCount")
        override suspend fun captureScreenshot(
            quality: Int,
            maxWidth: Int?,
            maxHeight: Int?,
        ): Result<ScreenshotData> {
            if (!accessibilityServiceProvider.isReady()) {
                return Result.failure(ToolException.PermissionDenied("Accessibility service not enabled"))
            }
            val service = accessibilityServiceProvider.getContext() as? AutoFishAccessibilityService
                ?: return Result.failure(ToolException.PermissionDenied("Accessibility service not available"))

            val bitmap = service.takeScreenshotBitmap()
                ?: return Result.failure(ToolException.ActionFailed("Screenshot capture failed"))

            var resizedBitmap: Bitmap? = null
            return try {
                resizedBitmap = screenshotEncoder.resizeBitmapProportional(bitmap, maxWidth, maxHeight)
                val data = screenshotEncoder.bitmapToScreenshotData(resizedBitmap, quality)
                Result.success(data)
            } finally {
                if (resizedBitmap != null && resizedBitmap !== bitmap) resizedBitmap.recycle()
                bitmap.recycle()
            }
        }

        override fun isScreenCaptureAvailable(): Boolean = accessibilityServiceProvider.isReady()
    }
