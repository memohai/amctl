package com.example.amctl.services.screencapture

import android.graphics.Bitmap
import android.util.Base64
import com.example.amctl.data.model.ScreenshotData
import java.io.ByteArrayOutputStream
import javax.inject.Inject

class ScreenshotEncoder
    @Inject
    constructor() {
        fun bitmapToScreenshotData(bitmap: Bitmap, quality: Int): ScreenshotData {
            val jpegBytes = encodeBitmapToJpeg(bitmap, quality)
            val base64Data = Base64.encodeToString(jpegBytes, Base64.NO_WRAP)
            return ScreenshotData(data = base64Data, width = bitmap.width, height = bitmap.height)
        }

        fun encodeBitmapToJpeg(bitmap: Bitmap, quality: Int): ByteArray {
            val clamped = quality.coerceIn(1, 100)
            val stream = ByteArrayOutputStream(bitmap.width * bitmap.height / 4)
            bitmap.compress(Bitmap.CompressFormat.JPEG, clamped, stream)
            return stream.toByteArray()
        }

        @Suppress("ReturnCount")
        fun resizeBitmapProportional(bitmap: Bitmap, maxWidth: Int?, maxHeight: Int?): Bitmap {
            if (maxWidth == null && maxHeight == null) return bitmap
            val origW = bitmap.width
            val origH = bitmap.height
            val (targetW, targetH) = when {
                maxWidth != null && maxHeight != null -> {
                    val scale = minOf(maxWidth.toFloat() / origW, maxHeight.toFloat() / origH, 1f)
                    Pair((origW * scale).toInt(), (origH * scale).toInt())
                }
                maxWidth != null -> {
                    val scale = (maxWidth.toFloat() / origW).coerceAtMost(1f)
                    Pair((origW * scale).toInt(), (origH * scale).toInt())
                }
                else -> {
                    val scale = (maxHeight!!.toFloat() / origH).coerceAtMost(1f)
                    Pair((origW * scale).toInt(), (origH * scale).toInt())
                }
            }
            val safeW = targetW.coerceAtLeast(1)
            val safeH = targetH.coerceAtLeast(1)
            if (safeW == origW && safeH == origH) return bitmap
            return Bitmap.createScaledBitmap(bitmap, safeW, safeH, true)
        }
    }
