package com.example.amctl.mcp

import android.util.Log
import io.ktor.http.ContentType
import io.ktor.http.HttpHeaders
import io.ktor.http.HttpStatusCode
import io.ktor.server.application.Application
import io.ktor.server.application.ApplicationCall
import io.ktor.server.response.header
import io.ktor.server.response.respondText
import io.ktor.server.routing.delete
import io.ktor.server.routing.get
import io.ktor.server.routing.post
import io.ktor.server.routing.route
import io.ktor.server.routing.routing
import io.modelcontextprotocol.kotlin.sdk.server.Server
import io.modelcontextprotocol.kotlin.sdk.server.StreamableHttpServerTransport
import java.util.concurrent.ConcurrentHashMap

private const val TAG = "amctl:StreamableHttp"
private const val MCP_SESSION_ID_HEADER = "mcp-session-id"

@Suppress("LongMethod")
fun Application.mcpStreamableHttp(block: () -> Server) {
    val transports = ConcurrentHashMap<String, StreamableHttpServerTransport>()

    routing {
        route("/mcp") {
            post {
                val sessionId = call.request.headers[MCP_SESSION_ID_HEADER]
                val transport: StreamableHttpServerTransport

                if (!sessionId.isNullOrEmpty()) {
                    transport = transports[sessionId] ?: run {
                        call.respondText("""{"error":"Session not found"}""", ContentType.Application.Json, HttpStatusCode.NotFound)
                        return@post
                    }
                } else {
                    transport = StreamableHttpServerTransport(enableJsonResponse = true)

                    transport.setOnSessionInitialized { id ->
                        transports[id] = transport
                        Log.d(TAG, "Session initialized: $id")
                    }

                    transport.setOnSessionClosed { id ->
                        transports.remove(id)
                        Log.d(TAG, "Session closed: $id")
                    }

                    val server = block()
                    server.onClose {
                        transport.sessionId?.let { transports.remove(it) }
                    }
                    server.createSession(transport)
                }

                transport.handlePostRequest(null, call)
            }

            get {
                call.response.header(HttpHeaders.Allow, "POST, DELETE")
                call.respondText("""{"error":"Method Not Allowed"}""", ContentType.Application.Json, HttpStatusCode.MethodNotAllowed)
            }

            delete {
                val sessionId = call.request.headers[MCP_SESSION_ID_HEADER]
                if (sessionId.isNullOrEmpty()) {
                    call.respondText("""{"error":"No session ID"}""", ContentType.Application.Json, HttpStatusCode.BadRequest)
                    return@delete
                }
                val transport = transports[sessionId] ?: run {
                    call.respondText("""{"error":"Session not found"}""", ContentType.Application.Json, HttpStatusCode.NotFound)
                    return@delete
                }
                transport.handleDeleteRequest(call)
            }
        }
    }
}
