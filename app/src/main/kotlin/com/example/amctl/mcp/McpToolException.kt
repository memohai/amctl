package com.example.amctl.mcp

sealed class McpToolException(message: String) : RuntimeException(message) {
    class PermissionDenied(message: String) : McpToolException(message)
    class InvalidParams(message: String) : McpToolException(message)
    class ActionFailed(message: String) : McpToolException(message)
    class NodeNotFound(message: String) : McpToolException(message)
}
