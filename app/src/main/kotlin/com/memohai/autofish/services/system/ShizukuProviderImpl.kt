@file:Suppress("PrivateApi")

package com.memohai.autofish.services.system

import android.content.pm.PackageManager
import android.util.Log
import java.nio.charset.StandardCharsets
import kotlin.concurrent.thread
import rikka.shizuku.Shizuku
import javax.inject.Inject

open class ShizukuExecutionException(message: String, cause: Throwable? = null) : IllegalStateException(message, cause)

class ShizukuUnavailableException(message: String) : ShizukuExecutionException(message)

class ShizukuCommandException(message: String) : ShizukuExecutionException(message)

class ShizukuInvocationException(
    message: String,
    cause: Throwable? = null,
) : ShizukuExecutionException(message, cause)

@Suppress("TooManyFunctions")
class ShizukuProviderImpl
    @Inject
    constructor() : ShizukuProvider {

        private val newProcessMethod by lazy {
            try {
                Shizuku::class.java.getDeclaredMethod(
                    "newProcess",
                    Array<String>::class.java,
                    Array<String>::class.java,
                    String::class.java,
                ).also { it.isAccessible = true }
            } catch (e: ReflectiveOperationException) {
                Log.w(TAG, "Shizuku.newProcess method not found", e)
                null
            } catch (e: SecurityException) {
                Log.w(TAG, "Shizuku.newProcess method not found", e)
                null
            }
        }

        override fun isAvailable(): Boolean = try {
            Shizuku.pingBinder() && Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED
        } catch (e: IllegalStateException) {
            Log.d(TAG, "Shizuku availability check failed", e)
            false
        } catch (e: SecurityException) {
            Log.d(TAG, "Shizuku availability check denied", e)
            false
        }

        override fun isInstalled(): Boolean = try {
            Shizuku.pingBinder()
        } catch (e: IllegalStateException) {
            Log.d(TAG, "Shizuku install check failed", e)
            false
        } catch (e: SecurityException) {
            Log.d(TAG, "Shizuku install check denied", e)
            false
        }

        override fun hasPermission(): Boolean = try {
            Shizuku.checkSelfPermission() == PackageManager.PERMISSION_GRANTED
        } catch (e: IllegalStateException) {
            Log.d(TAG, "Shizuku permission check failed", e)
            false
        } catch (e: SecurityException) {
            Log.d(TAG, "Shizuku permission check denied", e)
            false
        }

        override fun exec(command: String): String = runShizukuCommand(command) {
            val process = startProcess(arrayOf("sh", "-c", command))
            val result = collectProcessResult(process)
            if (result.exitCode != 0) {
                throw buildExecException(command, result.exitCode, result.stderr)
            }
            result.stdout.toString(StandardCharsets.UTF_8)
        }

        override fun execBytes(command: String): ByteArray = runShizukuCommand(command) {
            val process = startProcess(arrayOf("sh", "-c", command))
            val result = collectProcessResult(process)
            if (result.exitCode != 0) {
                throw buildExecException(command, result.exitCode, result.stderr)
            }
            result.stdout
        }

        override fun requestPermission(requestCode: Int) {
            Shizuku.requestPermission(requestCode)
        }

        override fun addPermissionResultListener(listener: (Int, Int) -> Unit) {
            Shizuku.addRequestPermissionResultListener { requestCode, grantResult ->
                listener(requestCode, grantResult)
            }
        }

        override fun removePermissionResultListener(listener: (Int, Int) -> Unit) {
            // Shizuku listener removal requires the same instance;
            // callers should manage their own listener lifecycle
        }

        private fun startProcess(cmd: Array<String>): Process {
            val method = newProcessMethod
                ?: throw ShizukuUnavailableException("Shizuku.newProcess not available")
            return try {
                method.invoke(null, cmd, null as Array<String>?, null as String?) as? Process
                    ?: throw ShizukuInvocationException("Shizuku.newProcess returned non-process value")
            } catch (e: ReflectiveOperationException) {
                throwInvocationFailure(e)
            } catch (e: SecurityException) {
                throwInvocationFailure(e)
            }
        }

        private fun <T> runShizukuCommand(command: String, block: () -> T): T = try {
            block()
        } catch (e: ShizukuExecutionException) {
            throw e
        } catch (e: InterruptedException) {
            Thread.currentThread().interrupt()
            throw ShizukuInvocationException("Shizuku command interrupted: $command", e)
        } catch (e: java.io.IOException) {
            throw ShizukuInvocationException("Shizuku command I/O failed: $command", e)
        }

        private fun throwInvocationFailure(error: Throwable): Nothing {
            Log.w(TAG, "Shizuku.newProcess invocation failed", error)
            throw ShizukuInvocationException("Shizuku exec failed: ${error.message}", error)
        }

        private fun collectProcessResult(process: Process): ProcessResult {
            var stdout = ByteArray(0)
            var stderr = ByteArray(0)
            var stdoutError: java.io.IOException? = null
            var stderrError: java.io.IOException? = null

            val stdoutThread = thread(name = "shizuku-stdout-reader", start = true) {
                try {
                    stdout = process.inputStream.readBytes()
                } catch (e: java.io.IOException) {
                    stdoutError = e
                }
            }
            val stderrThread = thread(name = "shizuku-stderr-reader", start = true) {
                try {
                    stderr = process.errorStream.readBytes()
                } catch (e: java.io.IOException) {
                    stderrError = e
                }
            }

            val exitCode = process.waitFor()
            stdoutThread.join()
            stderrThread.join()
            stdoutError?.let { throw it }
            stderrError?.let { throw it }

            return ProcessResult(exitCode = exitCode, stdout = stdout, stderr = stderr)
        }

        private fun buildExecException(command: String, exitCode: Int, stderr: ByteArray): ShizukuCommandException {
            val stderrText = stderr.toString(StandardCharsets.UTF_8).trim()
            val message = if (stderrText.isNotEmpty()) {
                "Shizuku command failed (exit=$exitCode): $command; stderr=$stderrText"
            } else {
                "Shizuku command failed (exit=$exitCode): $command"
            }
            return ShizukuCommandException(message)
        }

        private data class ProcessResult(
            val exitCode: Int,
            val stdout: ByteArray,
            val stderr: ByteArray,
        )

        companion object {
            private const val TAG = "autofish:Shizuku"
        }
    }
