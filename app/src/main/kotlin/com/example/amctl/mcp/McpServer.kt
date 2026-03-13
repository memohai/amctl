package com.example.amctl.mcp

import com.example.amctl.mcp.auth.BearerTokenAuth
import com.example.amctl.mcp.tools.NodeActionTools
import com.example.amctl.mcp.tools.ScreenIntrospectionTools
import com.example.amctl.mcp.tools.SystemActionTools
import com.example.amctl.mcp.tools.TextInputTools
import com.example.amctl.mcp.tools.TouchActionTools
import com.example.amctl.mcp.tools.UtilityTools
import com.example.amctl.services.accessibility.AccessibilityServiceProvider
import com.example.amctl.services.accessibility.AccessibilityTreeParser
import com.example.amctl.services.accessibility.ActionExecutor
import com.example.amctl.services.accessibility.CompactTreeFormatter
import com.example.amctl.services.accessibility.ElementFinder
import com.example.amctl.services.screencapture.ScreenCaptureProvider
import io.ktor.http.ContentType
import io.ktor.http.HttpStatusCode
import io.ktor.serialization.kotlinx.json.json
import io.ktor.server.application.install
import io.ktor.server.engine.EmbeddedServer
import io.ktor.server.engine.embeddedServer
import io.ktor.server.netty.Netty
import io.ktor.server.netty.NettyApplicationEngine
import io.ktor.server.plugins.contentnegotiation.ContentNegotiation
import io.ktor.server.response.respondText
import io.ktor.server.routing.get
import io.ktor.server.routing.routing
import io.modelcontextprotocol.kotlin.sdk.server.Server
import io.modelcontextprotocol.kotlin.sdk.server.ServerOptions
import io.modelcontextprotocol.kotlin.sdk.types.Implementation
import io.modelcontextprotocol.kotlin.sdk.types.McpJson
import io.modelcontextprotocol.kotlin.sdk.types.ServerCapabilities

class McpServer(
    private val port: Int,
    private val bindAddress: String,
    private val bearerToken: String,
    private val accessibilityServiceProvider: AccessibilityServiceProvider,
    private val treeParser: AccessibilityTreeParser,
    private val compactTreeFormatter: CompactTreeFormatter,
    private val elementFinder: ElementFinder,
    private val actionExecutor: ActionExecutor,
    private val screenCaptureProvider: ScreenCaptureProvider,
) {
    private var server: EmbeddedServer<NettyApplicationEngine, NettyApplicationEngine.Configuration>? = null

    fun start() {
        val mcpSdkServer = Server(
            Implementation(name = "amctl", version = "0.1.0"),
            ServerOptions(capabilities = ServerCapabilities(tools = ServerCapabilities.Tools())),
        )

        ScreenIntrospectionTools.register(mcpSdkServer, accessibilityServiceProvider, treeParser, compactTreeFormatter, screenCaptureProvider)
        TouchActionTools.register(mcpSdkServer, actionExecutor)
        NodeActionTools.register(mcpSdkServer, actionExecutor, elementFinder, accessibilityServiceProvider, treeParser)
        TextInputTools.register(mcpSdkServer, actionExecutor, accessibilityServiceProvider, treeParser)
        SystemActionTools.register(mcpSdkServer, actionExecutor)
        UtilityTools.register(mcpSdkServer, elementFinder, accessibilityServiceProvider, treeParser)

        server = embeddedServer(
            factory = Netty,
            port = port,
            host = bindAddress,
        ) {
            install(ContentNegotiation) { json(McpJson) }
            install(BearerTokenAuth) { token = bearerToken }

            routing {
                get("/health") {
                    call.respondText("""{"status":"healthy"}""", ContentType.Application.Json, HttpStatusCode.OK)
                }
            }

            mcpStreamableHttp { mcpSdkServer }
        }.start(wait = false)
    }

    fun stop() {
        server?.stop(gracePeriodMillis = 1000, timeoutMillis = 5000)
        server = null
    }
}
