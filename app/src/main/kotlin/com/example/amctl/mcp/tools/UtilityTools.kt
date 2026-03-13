package com.example.amctl.mcp.tools

import com.example.amctl.services.accessibility.AccessibilityServiceProvider
import com.example.amctl.services.accessibility.AccessibilityTreeParser
import com.example.amctl.services.accessibility.ElementFinder
import com.example.amctl.services.accessibility.FindBy
import io.modelcontextprotocol.kotlin.sdk.server.Server
import io.modelcontextprotocol.kotlin.sdk.types.CallToolResult
import io.modelcontextprotocol.kotlin.sdk.types.TextContent
import io.modelcontextprotocol.kotlin.sdk.types.ToolSchema
import kotlinx.coroutines.delay
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.long
import kotlinx.serialization.json.put

object UtilityTools {
    private const val POLL_INTERVAL_MS = 500L

    fun register(
        server: Server,
        elementFinder: ElementFinder,
        accessibilityProvider: AccessibilityServiceProvider,
        treeParser: AccessibilityTreeParser,
    ) {
        registerWaitForNode(server, elementFinder, accessibilityProvider, treeParser)
        registerWaitForIdle(server, accessibilityProvider, treeParser)
    }

    private fun registerWaitForNode(server: Server, finder: ElementFinder, provider: AccessibilityServiceProvider, parser: AccessibilityTreeParser) {
        server.addTool(
            name = "amctl_wait_for_node",
            description = "Wait for a UI node to appear",
            inputSchema = ToolSchema(
                properties = buildJsonObject {
                    put("by", buildJsonObject { put("type", "string"); put("description", "text, content_desc, resource_id, class_name") })
                    put("value", buildJsonObject { put("type", "string") })
                    put("timeout", buildJsonObject { put("type", "integer"); put("description", "Timeout in ms") })
                },
                required = listOf("by", "value", "timeout"),
            ),
        ) { request ->
            val byStr = request.arguments?.get("by")?.jsonPrimitive?.content ?: return@addTool errorResult("Missing by")
            val value = request.arguments?.get("value")?.jsonPrimitive?.content ?: return@addTool errorResult("Missing value")
            val timeout = request.arguments?.get("timeout")?.jsonPrimitive?.long ?: return@addTool errorResult("Missing timeout")
            val by = try { FindBy.valueOf(byStr.uppercase()) } catch (_: Exception) { return@addTool errorResult("Invalid by: $byStr") }

            val startTime = System.currentTimeMillis()
            while (System.currentTimeMillis() - startTime < timeout) {
                val windows = getFreshWindows(provider, parser)
                val elements = finder.findElements(windows, by, value)
                if (elements.isNotEmpty()) {
                    return@addTool CallToolResult(content = listOf(TextContent(text = "Node found: ${elements.first().id}")))
                }
                delay(POLL_INTERVAL_MS)
            }
            CallToolResult(content = listOf(TextContent(text = "Timeout: node with $byStr='$value' not found within ${timeout}ms")), isError = true)
        }
    }

    private fun registerWaitForIdle(server: Server, provider: AccessibilityServiceProvider, parser: AccessibilityTreeParser) {
        server.addTool(
            name = "amctl_wait_for_idle",
            description = "Wait for the UI to become idle (stable)",
            inputSchema = ToolSchema(
                properties = buildJsonObject {
                    put("timeout", buildJsonObject { put("type", "integer"); put("description", "Timeout in ms") })
                },
                required = listOf("timeout"),
            ),
        ) { request ->
            val timeout = request.arguments?.get("timeout")?.jsonPrimitive?.long ?: return@addTool errorResult("Missing timeout")

            var previousHash: Int? = null
            val startTime = System.currentTimeMillis()
            while (System.currentTimeMillis() - startTime < timeout) {
                val windows = getFreshWindows(provider, parser)
                val currentHash = windows.hashCode()
                if (previousHash != null && currentHash == previousHash) {
                    return@addTool CallToolResult(content = listOf(TextContent(text = "UI is idle")))
                }
                previousHash = currentHash
                delay(POLL_INTERVAL_MS)
            }
            CallToolResult(content = listOf(TextContent(text = "Timeout: UI did not become idle within ${timeout}ms")), isError = true)
        }
    }
}
