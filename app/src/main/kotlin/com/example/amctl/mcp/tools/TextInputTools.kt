package com.example.amctl.mcp.tools

import com.example.amctl.services.accessibility.AccessibilityServiceProvider
import com.example.amctl.services.accessibility.AccessibilityTreeParser
import com.example.amctl.services.accessibility.ActionExecutor
import io.modelcontextprotocol.kotlin.sdk.server.Server
import io.modelcontextprotocol.kotlin.sdk.types.CallToolResult
import io.modelcontextprotocol.kotlin.sdk.types.TextContent
import io.modelcontextprotocol.kotlin.sdk.types.ToolSchema
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put

object TextInputTools {
    fun register(
        server: Server,
        actionExecutor: ActionExecutor,
        accessibilityProvider: AccessibilityServiceProvider,
        treeParser: AccessibilityTreeParser,
    ) {
        registerTypeText(server, actionExecutor, accessibilityProvider, treeParser)
        registerClearText(server, actionExecutor, accessibilityProvider, treeParser)
        registerPressKey(server, actionExecutor)
    }

    private fun registerTypeText(server: Server, executor: ActionExecutor, provider: AccessibilityServiceProvider, parser: AccessibilityTreeParser) {
        server.addTool(
            name = "amctl_type_text",
            description = "Type text into a text field",
            inputSchema = ToolSchema(
                properties = buildJsonObject {
                    put("node_id", buildJsonObject { put("type", "string") })
                    put("text", buildJsonObject { put("type", "string") })
                },
                required = listOf("node_id", "text"),
            ),
        ) { request ->
            val nodeId = request.arguments?.get("node_id")?.jsonPrimitive?.content ?: return@addTool errorResult("Missing node_id")
            val text = request.arguments?.get("text")?.jsonPrimitive?.content ?: return@addTool errorResult("Missing text")
            val windows = getFreshWindows(provider, parser)
            executor.setTextOnNode(nodeId, text, windows).toCallToolResult("Typed text into '$nodeId'")
        }
    }

    private fun registerClearText(server: Server, executor: ActionExecutor, provider: AccessibilityServiceProvider, parser: AccessibilityTreeParser) {
        server.addTool(
            name = "amctl_clear_text",
            description = "Clear text in a text field",
            inputSchema = ToolSchema(
                properties = buildJsonObject { put("node_id", buildJsonObject { put("type", "string") }) },
                required = listOf("node_id"),
            ),
        ) { request ->
            val nodeId = request.arguments?.get("node_id")?.jsonPrimitive?.content ?: return@addTool errorResult("Missing node_id")
            val windows = getFreshWindows(provider, parser)
            executor.setTextOnNode(nodeId, "", windows).toCallToolResult("Cleared text in '$nodeId'")
        }
    }

    private fun registerPressKey(server: Server, executor: ActionExecutor) {
        server.addTool(
            name = "amctl_press_key",
            description = "Press a key",
            inputSchema = ToolSchema(
                properties = buildJsonObject { put("key", buildJsonObject { put("type", "string"); put("description", "ENTER, BACK, DEL, HOME, TAB, SPACE") }) },
                required = listOf("key"),
            ),
        ) { request ->
            val key = request.arguments?.get("key")?.jsonPrimitive?.content?.uppercase() ?: return@addTool errorResult("Missing key")
            when (key) {
                "BACK" -> executor.pressBack().toCallToolResult("Pressed BACK")
                "HOME" -> executor.pressHome().toCallToolResult("Pressed HOME")
                else -> CallToolResult(content = listOf(TextContent(text = "Key '$key' not supported yet")), isError = true)
            }
        }
    }
}
