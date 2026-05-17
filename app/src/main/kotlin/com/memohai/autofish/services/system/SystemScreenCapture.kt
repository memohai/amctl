@file:Suppress("PrivateApi", "DiscouragedPrivateApi")

package com.memohai.autofish.services.system

import android.graphics.Bitmap
import android.graphics.Rect
import android.os.IBinder
import android.util.Log
import javax.inject.Inject

class SystemScreenCapture
    @Inject
    constructor() {

        fun capture(maxWidth: Int = 0, maxHeight: Int = 0): Bitmap? = try {
            captureApi34(maxWidth, maxHeight) ?: captureApi31(maxWidth, maxHeight)
        } catch (e: ReflectiveOperationException) {
            Log.w(TAG, "SurfaceControl.screenshot() reflection failed", e)
            null
        } catch (e: SecurityException) {
            Log.w(TAG, "SurfaceControl.screenshot() reflection failed", e)
            null
        } catch (e: ClassCastException) {
            Log.w(TAG, "SurfaceControl.screenshot() reflection failed", e)
            null
        }

        private fun captureApi34(maxWidth: Int, maxHeight: Int): Bitmap? = try {
            val surfaceControlClass = Class.forName("android.view.SurfaceControl")

            val getDisplayToken = surfaceControlClass.getMethod("getInternalDisplayToken")
            val displayToken = getDisplayToken.invoke(null) as? IBinder ?: return null

            val screenshotMethod = surfaceControlClass.getMethod(
                "screenshot",
                IBinder::class.java,
                Rect::class.java,
                Int::class.java,
                Int::class.java,
                Int::class.java,
            )
            screenshotMethod.invoke(null, displayToken, Rect(), maxWidth, maxHeight, 0) as? Bitmap
        } catch (e: NoSuchMethodException) {
            Log.d(TAG, "API 34 SurfaceControl screenshot method unavailable", e)
            null
        }

        private fun captureApi31(maxWidth: Int, maxHeight: Int): Bitmap? = try {
            val surfaceControlClass = Class.forName("android.view.SurfaceControl")

            val getDisplayIds = surfaceControlClass.getMethod("getPhysicalDisplayIds")
            val displayIds = getDisplayIds.invoke(null) as? LongArray
            val displayId = displayIds?.firstOrNull() ?: return null

            val getToken = surfaceControlClass.getMethod("getPhysicalDisplayToken", Long::class.java)
            val displayToken = getToken.invoke(null, displayId) as? IBinder ?: return null

            val screenshotMethod = surfaceControlClass.getMethod(
                "screenshot",
                IBinder::class.java,
                Rect::class.java,
                Int::class.java,
                Int::class.java,
                Int::class.java,
            )
            screenshotMethod.invoke(null, displayToken, Rect(), maxWidth, maxHeight, 0) as? Bitmap
        } catch (e: NoSuchMethodException) {
            Log.d(TAG, "API 31 SurfaceControl screenshot method unavailable", e)
            null
        }

        companion object {
            private const val TAG = "autofish:SysCapture"
        }
    }
