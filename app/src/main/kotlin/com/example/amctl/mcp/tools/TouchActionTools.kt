package com.example.amctl.mcp.tools

import com.example.amctl.services.accessibility.ActionExecutor
import com.example.amctl.services.accessibility.ScrollAmount
import com.example.amctl.services.accessibility.ScrollDirection
import io.modelcontextprotocol.kotlin.sdk.server.Server
import io.modelcontextprotocol.kotlin.sdk.types.CallToolResult
import io.modelcontextprotocol.kotlin.sdk.types.TextContent
import io.modelcontextprotocol.kotlin.sdk.types.ToolSchema
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.float
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.long
import kotlinx.serialization.json.put

object TouchActionTools {
    fun register(server: Server, actionExecutor: ActionExecutor) {
        registerTap(server, actionExecutor)
        registerLongPress(server, actionExecutor)
        registerDoubleTap(server, actionExecutor)
        registerSwipe(server, actionExecutor)
        registerScroll(server, actionExecutor)
    }

    private fun registerTap(server: Server, executor: ActionExecutor) {
        server.addTool(
            name = "amctl_tap",
            description = "Single tap at coordinates",
            inputSchema = ToolSchema(
                properties = buildJsonObject {
                    put("x", buildJsonObject { put("type", "number") })
                    put("y", buildJsonObject { put("type", "number") })
                },
                required = listOf("x", "y"),
            ),
        ) { request ->
            val x = request.arguments?.get("x")?.jsonPrimitive?.float ?: return@addTool errorResult("Missing x")
            val y = request.arguments?.get("y")?.jsonPrimitive?.float ?: return@addTool errorResult("Missing y")
            executor.tap(x, y).toCallToolResult("Tapped at ($x, $y)")
        }
    }

    private fun registerLongPress(server: Server, executor: ActionExecutor) {
        server.addTool(
            name = "amctl_long_press",
            description = "Long press at coordinates",
            inputSchema = ToolSchema(
                properties = buildJsonObject {
                    put("x", buildJsonObject { put("type", "number") })
                    put("y", buildJsonObject { put("type", "number") })
                    put("duration", buildJsonObject { put("type", "integer"); put("description", "Duration in ms") })
                },
                required = listOf("x", "y"),
            ),
        ) { request ->
            val x = request.arguments?.get("x")?.jsonPrimitive?.float ?: return@addTool errorResult("Missing x")
            val y = request.arguments?.get("y")?.jsonPrimitive?.float ?: return@addTool errorResult("Missing y")
            val duration = request.arguments?.get("duration")?.jsonPrimitive?.long ?: ActionExecutor.DEFAULT_LONG_PRESS_DURATION_MS
            executor.longPress(x, y, duration).toCallToolResult("Long pressed at ($x, $y) for ${duration}ms")
        }
    }

    private fun registerDoubleTap(server: Server, executor: ActionExecutor) {
        server.addTool(
            name = "amctl_double_tap",
            description = "Double tap at coordinates",
            inputSchema = ToolSchema(
                properties = buildJsonObject {
                    put("x", buildJsonObject { put("type", "number") })
                    put("y", buildJsonObject { put("type", "number") })
                },
                required = listOf("x", "y"),
            ),
        ) { request ->
            val x = request.arguments?.get("x")?.jsonPrimitive?.float ?: return@addTool errorResult("Missing x")
            val y = request.arguments?.get("y")?.jsonPrimitive?.float ?: return@addTool errorResult("Missing y")
            executor.doubleTap(x, y).toCallToolResult("Double tapped at ($x, $y)")
        }
    }

    private fun registerSwipe(server: Server, executor: ActionExecutor) {
        server.addTool(
            name = "amctl_swipe",
            description = "Swipe from (x1,y1) to (x2,y2)",
            inputSchema = ToolSchema(
                properties = buildJsonObject {
                    put("x1", buildJsonObject { put("type", "number") })
                    put("y1", buildJsonObject { put("type", "number") })
                    put("x2", buildJsonObject { put("type", "number") })
                    put("y2", buildJsonObject { put("type", "number") })
                    put("duration", buildJsonObject { put("type", "integer"); put("description", "Duration in ms") })
                },
                required = listOf("x1", "y1", "x2", "y2"),
            ),
        ) { request ->
            val x1 = request.arguments?.get("x1")?.jsonPrimitive?.float ?: return@addTool errorResult("Missing x1")
            val y1 = request.arguments?.get("y1")?.jsonPrimitive?.float ?: return@addTool errorResult("Missing y1")
            val x2 = request.arguments?.get("x2")?.jsonPrimitive?.float ?: return@addTool errorResult("Missing x2")
            val y2 = request.arguments?.get("y2")?.jsonPrimitive?.float ?: return@addTool errorResult("Missing y2")
            val duration = request.arguments?.get("duration")?.jsonPrimitive?.long ?: ActionExecutor.DEFAULT_SWIPE_DURATION_MS
            executor.swipe(x1, y1, x2, y2, duration).toCallToolResult("Swiped ($x1,$y1) -> ($x2,$y2)")
        }
    }

    private fun registerScroll(server: Server, executor: ActionExecutor) {
        server.addTool(
            name = "amctl_scroll",
            description = "Scroll in a direction",
            inputSchema = ToolSchema(
                properties = buildJsonObject {
                    put("direction", buildJsonObject { put("type", "string"); put("description", "up, down, left, right") })
                    put("amount", buildJsonObject { put("type", "string"); put("description", "small, medium, large (default: medium)") })
                },
                required = listOf("direction"),
            ),
        ) { request ->
            val dirStr = request.arguments?.get("direction")?.jsonPrimitive?.content ?: return@addTool errorResult("Missing direction")
            val direction = try { ScrollDirection.valueOf(dirStr.uppercase()) } catch (_: Exception) { return@addTool errorResult("Invalid direction: $dirStr") }
            val amountStr = request.arguments?.get("amount")?.jsonPrimitive?.content ?: "medium"
            val amount = try { ScrollAmount.valueOf(amountStr.uppercase()) } catch (_: Exception) { ScrollAmount.MEDIUM }
            executor.scroll(direction, amount).toCallToolResult("Scrolled $direction ($amountStr)")
        }
    }
}

internal fun Result<Unit>.toCallToolResult(successMessage: String): CallToolResult =
    if (isSuccess) CallToolResult(content = listOf(TextContent(text = successMessage)))
    else CallToolResult(content = listOf(TextContent(text = exceptionOrNull()?.message ?: "Unknown error")), isError = true)

internal fun errorResult(message: String): CallToolResult =
    CallToolResult(content = listOf(TextContent(text = message)), isError = true)
