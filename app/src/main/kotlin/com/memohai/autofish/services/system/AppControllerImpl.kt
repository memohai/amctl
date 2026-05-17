package com.memohai.autofish.services.system

import android.util.Log
import javax.inject.Inject

private class IntentStartException(message: String) : IllegalStateException(message)

class AppControllerImpl
    @Inject
    constructor(
        private val shizukuProvider: ShizukuProvider,
    ) : AppController {

        override fun launch(packageName: String): Result<String> = try {
            val resolveOutput = shizukuProvider.exec(
                "cmd package resolve-activity --brief $packageName | tail -1",
            ).trim()

            if (resolveOutput.isBlank() || resolveOutput.contains("No activity")) {
                Result.failure(IllegalArgumentException("No launchable activity for $packageName"))
            } else {
                shizukuProvider.exec("am start -n $resolveOutput")
                Result.success(resolveOutput)
            }
        } catch (e: ShizukuExecutionException) {
            Log.w(TAG, "launch failed: $packageName", e)
            Result.failure(e)
        }

        override fun forceStop(packageName: String): Result<Unit> = try {
            shizukuProvider.exec("am force-stop $packageName")
            Result.success(Unit)
        } catch (e: ShizukuExecutionException) {
            Log.w(TAG, "forceStop failed: $packageName", e)
            Result.failure(e)
        }

        override fun getTopActivity(): String? = try {
            val output = shizukuProvider.exec(
                "dumpsys activity activities | " +
                    "grep -E 'topResumedActivity=|ResumedActivity:|mResumedActivity=' | head -1",
            ).trim()
            if (output.isNotBlank()) {
                parseTopActivityOutput(output)
            } else {
                null
            }
        } catch (e: ShizukuExecutionException) {
            Log.w(TAG, "getTopActivity failed", e)
            null
        }

        override fun listPackages(filter: String?, thirdPartyOnly: Boolean): Result<List<String>> = try {
            val flag = if (thirdPartyOnly) "-3" else ""
            val output = shizukuProvider.exec("pm list packages $flag").trim()
            val packages = output.lines()
                .filter { it.startsWith("package:") }
                .map { it.removePrefix("package:").trim() }
                .let { list ->
                    if (filter.isNullOrBlank()) list
                    else list.filter { it.contains(filter, ignoreCase = true) }
                }
                .sorted()
            Result.success(packages)
        } catch (e: ShizukuExecutionException) {
            Log.w(TAG, "listPackages failed", e)
            Result.failure(e)
        }

        override fun execShell(command: String): Result<String> = try {
            val output = shizukuProvider.exec(command)
            Result.success(output)
        } catch (e: ShizukuExecutionException) {
            Log.w(TAG, "execShell failed: $command", e)
            Result.failure(e)
        }

        override fun startIntent(
            action: String?,
            dataUri: String?,
            packageName: String?,
            component: String?,
            extras: Map<String, String>?,
        ): Result<String> = try {
            val cmd = buildString {
                append("am start")
                action?.let { append(" -a ").append(it) }
                dataUri?.let { append(" -d '").append(it).append("'") }
                packageName?.let { append(" -p ").append(it) }
                component?.let { append(" -n ").append(it) }
                extras?.forEach { (k, v) -> append(" --es '").append(k).append("' '").append(v).append("'") }
            }
            val output = shizukuProvider.exec(cmd).trim()
            if (output.contains("Error") || output.contains("Exception")) {
                Result.failure(IntentStartException(output))
            } else {
                Result.success(output.ifBlank { "Intent started" })
            }
        } catch (e: ShizukuExecutionException) {
            Log.w(TAG, "startIntent failed", e)
            Result.failure(e)
        }

        companion object {
            private const val TAG = "autofish:AppCtrl"
            private val ACTIVITY_PATTERN = Regex("""([A-Za-z0-9_.]+/[A-Za-z0-9_.$]+)""")

            internal fun parseTopActivityOutput(output: String): String? {
                val trimmed = output.trim()
                if (trimmed.isBlank()) {
                    return null
                }
                return ACTIVITY_PATTERN.find(trimmed)?.groupValues?.get(1)
            }
        }
    }
