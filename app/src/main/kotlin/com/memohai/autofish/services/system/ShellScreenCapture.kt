package com.memohai.autofish.services.system

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.util.Log
import javax.inject.Inject

class ShellScreenCapture
    @Inject
    constructor(
        private val shizukuProvider: ShizukuProvider,
    ) {
        fun capture(): Bitmap? = try {
            val bytes = shizukuProvider.execBytes("screencap -p")
            if (bytes.isNotEmpty()) {
                BitmapFactory.decodeByteArray(bytes, 0, bytes.size)
            } else {
                null
            }
        } catch (e: ShizukuExecutionException) {
            Log.w(TAG, "Shell screencap failed", e)
            null
        }

        companion object {
            private const val TAG = "autofish:ShellCapture"
        }
    }
