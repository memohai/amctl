package com.memohai.autofish.services.system

import android.util.Log
import javax.inject.Inject

class ShellInputInjector
    @Inject
    constructor(
        private val shizukuProvider: ShizukuProvider,
    ) {
        fun tap(x: Float, y: Float): Boolean = execQuiet("input tap $x $y")

        fun longPress(x: Float, y: Float, durationMs: Long): Boolean =
            execQuiet("input swipe $x $y $x $y $durationMs")

        fun doubleTap(x: Float, y: Float): Boolean =
            execQuiet("input tap $x $y && sleep 0.1 && input tap $x $y")

        fun swipe(x1: Float, y1: Float, x2: Float, y2: Float, durationMs: Long): Boolean =
            execQuiet("input swipe $x1 $y1 $x2 $y2 $durationMs")

        fun keyEvent(keyCode: Int): Boolean =
            execQuiet("input keyevent $keyCode")

        fun text(text: String): Boolean {
            val escaped = text.replace("'", "'\\''")
            return execQuiet("input text '$escaped'")
        }

        private fun execQuiet(command: String): Boolean = try {
            shizukuProvider.exec(command)
            true
        } catch (e: ShizukuExecutionException) {
            Log.w(TAG, "Shell input failed: $command", e)
            false
        }

        companion object {
            private const val TAG = "autofish:ShellInput"
        }
    }
