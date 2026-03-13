package com.example.amctl.mcp.tools

import com.example.amctl.services.accessibility.ActionExecutor
import io.modelcontextprotocol.kotlin.sdk.server.Server
import io.modelcontextprotocol.kotlin.sdk.types.ToolSchema
import kotlinx.serialization.json.buildJsonObject

object SystemActionTools {
    fun register(server: Server, actionExecutor: ActionExecutor) {
        val emptySchema = ToolSchema(properties = buildJsonObject {})

        server.addTool(name = "amctl_press_back", description = "Press the Back button", inputSchema = emptySchema) {
            actionExecutor.pressBack().toCallToolResult("Pressed Back")
        }

        server.addTool(name = "amctl_press_home", description = "Press the Home button", inputSchema = emptySchema) {
            actionExecutor.pressHome().toCallToolResult("Pressed Home")
        }
    }
}
