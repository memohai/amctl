package com.memohai.autofish.services.service

import android.content.Context
import io.mockk.every
import io.mockk.mockk
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.io.TempDir
import java.io.File

class ConnectionHintWriterTest {
    @TempDir
    lateinit var tempDir: File

    @Test
    fun `write should persist non-sensitive connection hint`() {
        val context =
            mockk<Context> {
                every { getExternalFilesDir(null) } returns tempDir
            }
        val writer = ConnectionHintWriter(context)

        writer.write(servicePort = 18081, serviceRunning = false)

        val hintFile = tempDir.resolve(ConnectionHintWriter.FILE_NAME)
        assertTrue(hintFile.exists())
        val json = Json.parseToJsonElement(hintFile.readText()).jsonObject
        assertEquals("18081", json.getValue("servicePort").jsonPrimitive.content)
        assertEquals("false", json.getValue("serviceRunning").jsonPrimitive.content)
        assertFalse(json.containsKey("token"))
        assertFalse(json.containsKey("screen"))
        assertFalse(json.containsKey("refs"))
    }
}
