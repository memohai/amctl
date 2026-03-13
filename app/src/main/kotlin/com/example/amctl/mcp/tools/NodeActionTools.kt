package com.example.amctl.mcp.tools

import com.example.amctl.services.accessibility.AccessibilityServiceProvider
import com.example.amctl.services.accessibility.AccessibilityTreeParser
import com.example.amctl.services.accessibility.ActionExecutor
import com.example.amctl.services.accessibility.ElementFinder
import com.example.amctl.services.accessibility.FindBy
import com.example.amctl.services.accessibility.ScrollDirection
import com.example.amctl.services.accessibility.WindowData
import io.modelcontextprotocol.kotlin.sdk.server.Server
import io.modelcontextprotocol.kotlin.sdk.types.CallToolResult
import io.modelcontextprotocol.kotlin.sdk.types.TextContent
import io.modelcontextprotocol.kotlin.sdk.types.ToolSchema
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put

object NodeActionTools {
    fun register(
        server: Server,
        actionExecutor: ActionExecutor,
        elementFinder: ElementFinder,
        accessibilityProvider: AccessibilityServiceProvider,
        treeParser: AccessibilityTreeParser,
    ) {
        registerFindNodes(server, elementFinder, accessibilityProvider, treeParser)
        registerClickNode(server, actionExecutor, accessibilityProvider, treeParser)
        registerLongClickNode(server, actionExecutor, accessibilityProvider, treeParser)
        registerScrollToNode(server, actionExecutor, accessibilityProvider, treeParser)
    }

    private fun registerFindNodes(server: Server, finder: ElementFinder, provider: AccessibilityServiceProvider, parser: AccessibilityTreeParser) {
        server.addTool(
            name = "amctl_find_nodes",
            description = "Find UI nodes by criteria",
            inputSchema = ToolSchema(
                properties = buildJsonObject {
                    put("by", buildJsonObject { put("type", "string"); put("description", "text, content_desc, resource_id, class_name") })
                    put("value", buildJsonObject { put("type", "string") })
                    put("exact_match", buildJsonObject { put("type", "boolean"); put("description", "Exact match (default: false)") })
                },
                required = listOf("by", "value"),
            ),
        ) { request ->
            val byStr = request.arguments?.get("by")?.jsonPrimitive?.content ?: return@addTool errorResult("Missing by")
            val value = request.arguments?.get("value")?.jsonPrimitive?.content ?: return@addTool errorResult("Missing value")
            val exactMatch = request.arguments?.get("exact_match")?.jsonPrimitive?.boolean ?: false
            val by = try { FindBy.valueOf(byStr.uppercase()) } catch (_: Exception) { return@addTool errorResult("Invalid by: $byStr") }
            val windows = getFreshWindows(provider, parser)
            val elements = finder.findElements(windows, by, value, exactMatch)
            if (elements.isEmpty()) {
                CallToolResult(content = listOf(TextContent(text = "No nodes found matching $byStr='$value'")))
            } else {
                val text = elements.joinToString("\n") { e ->
                    "${e.id}\t${e.className}\ttext=${e.text}\tdesc=${e.contentDescription}\tres=${e.resourceId}\tbounds=${e.bounds}"
                }
                CallToolResult(content = listOf(TextContent(text = "Found ${elements.size} node(s):\n$text")))
            }
        }
    }

    private fun registerClickNode(server: Server, executor: ActionExecutor, provider: AccessibilityServiceProvider, parser: AccessibilityTreeParser) {
        server.addTool(
            name = "amctl_click_node",
            description = "Click a UI node by ID",
            inputSchema = ToolSchema(
                properties = buildJsonObject { put("node_id", buildJsonObject { put("type", "string") }) },
                required = listOf("node_id"),
            ),
        ) { request ->
            val nodeId = request.arguments?.get("node_id")?.jsonPrimitive?.content ?: return@addTool errorResult("Missing node_id")
            val windows = getFreshWindows(provider, parser)
            executor.clickNode(nodeId, windows).toCallToolResult("Clicked node '$nodeId'")
        }
    }

    private fun registerLongClickNode(server: Server, executor: ActionExecutor, provider: AccessibilityServiceProvider, parser: AccessibilityTreeParser) {
        server.addTool(
            name = "amctl_long_click_node",
            description = "Long click a UI node by ID",
            inputSchema = ToolSchema(
                properties = buildJsonObject { put("node_id", buildJsonObject { put("type", "string") }) },
                required = listOf("node_id"),
            ),
        ) { request ->
            val nodeId = request.arguments?.get("node_id")?.jsonPrimitive?.content ?: return@addTool errorResult("Missing node_id")
            val windows = getFreshWindows(provider, parser)
            executor.longClickNode(nodeId, windows).toCallToolResult("Long clicked node '$nodeId'")
        }
    }

    private fun registerScrollToNode(server: Server, executor: ActionExecutor, provider: AccessibilityServiceProvider, parser: AccessibilityTreeParser) {
        server.addTool(
            name = "amctl_scroll_to_node",
            description = "Scroll to make a node visible",
            inputSchema = ToolSchema(
                properties = buildJsonObject { put("node_id", buildJsonObject { put("type", "string") }) },
                required = listOf("node_id"),
            ),
        ) { request ->
            val nodeId = request.arguments?.get("node_id")?.jsonPrimitive?.content ?: return@addTool errorResult("Missing node_id")
            val windows = getFreshWindows(provider, parser)
            executor.scrollNode(nodeId, ScrollDirection.DOWN, windows).toCallToolResult("Scrolled to node '$nodeId'")
        }
    }
}

internal fun getFreshWindows(provider: AccessibilityServiceProvider, parser: AccessibilityTreeParser): List<WindowData> {
    val windows = mutableListOf<WindowData>()
    val accessibilityWindows = provider.getAccessibilityWindows()
    if (accessibilityWindows.isNotEmpty()) {
        for (window in accessibilityWindows) {
            val root = window.root ?: continue
            val tree = parser.parseTree(root, rootParentId = "root_w${window.id}")
            windows.add(
                WindowData(
                    windowId = window.id,
                    windowType = AccessibilityTreeParser.mapWindowType(window.type),
                    packageName = root.packageName?.toString(),
                    title = window.title?.toString(),
                    layer = window.layer,
                    focused = window.isFocused,
                    tree = tree,
                ),
            )
            @Suppress("DEPRECATION") root.recycle()
        }
    } else {
        val rootNode = provider.getRootNode() ?: return emptyList()
        windows.add(
            WindowData(windowId = 0, windowType = "APPLICATION", packageName = rootNode.packageName?.toString(), focused = true, tree = parser.parseTree(rootNode)),
        )
        @Suppress("DEPRECATION") rootNode.recycle()
    }
    return windows
}
