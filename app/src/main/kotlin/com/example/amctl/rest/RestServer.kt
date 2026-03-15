package com.example.amctl.rest

import com.example.amctl.mcp.auth.BearerTokenAuth
import com.example.amctl.services.accessibility.AccessibilityServiceProvider
import com.example.amctl.services.accessibility.AccessibilityTreeParser
import com.example.amctl.services.accessibility.CompactTreeFormatter
import com.example.amctl.services.accessibility.ElementFinder
import com.example.amctl.services.accessibility.FindBy
import com.example.amctl.services.accessibility.MultiWindowResult
import com.example.amctl.services.accessibility.ScrollAmount
import com.example.amctl.services.accessibility.ScrollDirection
import com.example.amctl.services.accessibility.WindowData
import com.example.amctl.services.system.ToolRouter
import io.ktor.http.ContentType
import io.ktor.http.HttpStatusCode
import io.ktor.serialization.kotlinx.json.json
import io.ktor.server.application.install
import io.ktor.server.engine.EmbeddedServer
import io.ktor.server.engine.embeddedServer
import io.ktor.server.netty.Netty
import io.ktor.server.netty.NettyApplicationEngine
import io.ktor.server.plugins.contentnegotiation.ContentNegotiation
import io.ktor.server.request.receive
import io.ktor.server.response.respondText
import io.ktor.server.routing.get
import io.ktor.server.routing.post
import io.ktor.server.routing.route
import io.ktor.server.routing.routing
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json

class RestServer(
    private val port: Int,
    private val bindAddress: String,
    private val bearerToken: String,
    private val toolRouter: ToolRouter,
    private val accessibilityProvider: AccessibilityServiceProvider,
    private val treeParser: AccessibilityTreeParser,
    private val compactTreeFormatter: CompactTreeFormatter,
    private val elementFinder: ElementFinder,
) {
    private var server: EmbeddedServer<NettyApplicationEngine, NettyApplicationEngine.Configuration>? = null

    private val json = Json { ignoreUnknownKeys = true; encodeDefaults = true }

    fun start() {
        server = embeddedServer(Netty, port = port, host = bindAddress) {
            install(ContentNegotiation) { json(json) }
            install(BearerTokenAuth) { token = bearerToken }

            routing {
                get("/health") {
                    call.respondText("""{"status":"healthy","type":"rest"}""", ContentType.Application.Json)
                }

                route("/api") {
                    screenRoutes()
                    touchRoutes()
                    keyRoutes()
                    textRoutes()
                    nodeRoutes()
                    appRoutes()
                }
            }
        }.start(wait = false)
    }

    fun stop() {
        server?.stop(gracePeriodMillis = 1000, timeoutMillis = 5000)
        server = null
    }

    @Serializable
    data class ApiResponse(val ok: Boolean, val data: String? = null, val error: String? = null)

    private fun ok(data: String) = json.encodeToString(ApiResponse.serializer(), ApiResponse(ok = true, data = data))
    private fun err(msg: String) = json.encodeToString(ApiResponse.serializer(), ApiResponse(ok = false, error = msg))

    @Suppress("LongMethod")
    private fun io.ktor.server.routing.Route.screenRoutes() {
        get("/screen") {
            val screenInfo = toolRouter.getScreenInfo()
            val windows = mutableListOf<WindowData>()
            var degraded = false

            if (accessibilityProvider.isReady()) {
                val accessibilityWindows = accessibilityProvider.getAccessibilityWindows()
                if (accessibilityWindows.isNotEmpty()) {
                    for (window in accessibilityWindows) {
                        val root = window.root ?: continue
                        val tree = treeParser.parseTree(root, rootParentId = "root_w${window.id}")
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
                    val rootNode = accessibilityProvider.getRootNode()
                    if (rootNode != null) {
                        degraded = true
                        windows.add(WindowData(windowId = 0, windowType = "APPLICATION", packageName = rootNode.packageName?.toString(), focused = true, tree = treeParser.parseTree(rootNode)))
                        @Suppress("DEPRECATION") rootNode.recycle()
                    }
                }
            } else {
                degraded = true
            }

            val modeHeader = "[mode: ${toolRouter.currentMode}]"
            val result = MultiWindowResult(windows = windows, degraded = degraded)
            val tsv = "$modeHeader\n${compactTreeFormatter.formatMultiWindow(result, screenInfo)}"
            call.respondText(ok(tsv), ContentType.Application.Json)
        }

        get("/screenshot") {
            val maxDim = call.request.queryParameters["max_dim"]?.toIntOrNull() ?: 700
            val quality = call.request.queryParameters["quality"]?.toIntOrNull() ?: 80
            val result = toolRouter.captureScreen(quality = quality, maxWidth = maxDim, maxHeight = maxDim)
            if (result.isSuccess) {
                val data = result.getOrThrow()
                call.respondText(ok(data.data), ContentType.Application.Json)
            } else {
                call.respondText(err(result.exceptionOrNull()?.message ?: "Screenshot failed"), ContentType.Application.Json, HttpStatusCode.InternalServerError)
            }
        }
    }

    @Serializable data class TapRequest(val x: Float, val y: Float)
    @Serializable data class LongPressRequest(val x: Float, val y: Float, val duration: Long = 500)
    @Serializable data class SwipeRequest(val x1: Float, val y1: Float, val x2: Float, val y2: Float, val duration: Long = 300)
    @Serializable data class ScrollRequest(val direction: String, val amount: String = "medium")

    private fun io.ktor.server.routing.Route.touchRoutes() {
        post("/tap") {
            val req = call.receive<TapRequest>()
            toolRouter.tap(req.x, req.y).respond(call, "Tapped (${req.x}, ${req.y})")
        }
        post("/long-press") {
            val req = call.receive<LongPressRequest>()
            toolRouter.longPress(req.x, req.y, req.duration).respond(call, "Long pressed (${req.x}, ${req.y})")
        }
        post("/double-tap") {
            val req = call.receive<TapRequest>()
            toolRouter.doubleTap(req.x, req.y).respond(call, "Double tapped (${req.x}, ${req.y})")
        }
        post("/swipe") {
            val req = call.receive<SwipeRequest>()
            toolRouter.swipe(req.x1, req.y1, req.x2, req.y2, req.duration).respond(call, "Swiped")
        }
        post("/scroll") {
            val req = call.receive<ScrollRequest>()
            val dir = try { ScrollDirection.valueOf(req.direction.uppercase()) } catch (_: Exception) {
                call.respondText(err("Invalid direction: ${req.direction}"), ContentType.Application.Json, HttpStatusCode.BadRequest); return@post
            }
            val amt = try { ScrollAmount.valueOf(req.amount.uppercase()) } catch (_: Exception) { ScrollAmount.MEDIUM }
            toolRouter.scroll(dir, amt).respond(call, "Scrolled ${req.direction}")
        }
    }

    @Serializable data class KeyRequest(val key_code: Int)

    private fun io.ktor.server.routing.Route.keyRoutes() {
        post("/press/back") {
            toolRouter.pressBack().respond(call, "Pressed Back")
        }
        post("/press/home") {
            toolRouter.pressHome().respond(call, "Pressed Home")
        }
        post("/press/key") {
            val req = call.receive<KeyRequest>()
            toolRouter.pressKey(req.key_code).respond(call, "Pressed key ${req.key_code}")
        }
    }

    @Serializable data class TextRequest(val text: String)

    private fun io.ktor.server.routing.Route.textRoutes() {
        post("/text") {
            val req = call.receive<TextRequest>()
            val success = toolRouter.inputText(req.text)
            if (success) {
                call.respondText(ok("Typed text"), ContentType.Application.Json)
            } else {
                call.respondText(err("Text input failed"), ContentType.Application.Json, HttpStatusCode.InternalServerError)
            }
        }
    }

    @Serializable data class FindNodesRequest(val by: String, val value: String, val exact_match: Boolean = false)
    @Serializable data class NodeIdRequest(val node_id: String)

    private fun io.ktor.server.routing.Route.nodeRoutes() {
        post("/nodes/find") {
            val req = call.receive<FindNodesRequest>()
            if (!accessibilityProvider.isReady()) {
                call.respondText(err("Accessibility not available"), ContentType.Application.Json, HttpStatusCode.ServiceUnavailable); return@post
            }
            val rootNode = accessibilityProvider.getRootNode()
                ?: run { call.respondText(err("No root node"), ContentType.Application.Json, HttpStatusCode.ServiceUnavailable); return@post }
            val tree = treeParser.parseTree(rootNode)
            @Suppress("DEPRECATION") rootNode.recycle()
            val by = try { FindBy.valueOf(req.by.uppercase()) } catch (_: Exception) {
                call.respondText(err("Invalid by: ${req.by}"), ContentType.Application.Json, HttpStatusCode.BadRequest); return@post
            }
            val elements = elementFinder.findElements(tree, by, req.value, req.exact_match)
            if (elements.isEmpty()) {
                call.respondText(ok("No nodes found matching ${req.by}='${req.value}'"), ContentType.Application.Json)
            } else {
                val text = elements.joinToString("\n") { e ->
                    "${e.id}\t${e.className}\ttext=${e.text}\tdesc=${e.contentDescription}\tres=${e.resourceId}\tbounds=${e.bounds}"
                }
                call.respondText(ok("Found ${elements.size} node(s):\n$text"), ContentType.Application.Json)
            }
        }

        post("/nodes/click") {
            val req = call.receive<NodeIdRequest>()
            if (!accessibilityProvider.isReady()) {
                call.respondText(err("Accessibility not available"), ContentType.Application.Json, HttpStatusCode.ServiceUnavailable); return@post
            }
            val rootNode = accessibilityProvider.getRootNode()
                ?: run { call.respondText(err("No root node"), ContentType.Application.Json, HttpStatusCode.ServiceUnavailable); return@post }
            val tree = treeParser.parseTree(rootNode)
            @Suppress("DEPRECATION") rootNode.recycle()
            val node = elementFinder.findNodeById(tree, req.node_id)
            if (node == null) {
                call.respondText(err("Node not found: ${req.node_id}"), ContentType.Application.Json, HttpStatusCode.NotFound); return@post
            }
            val b = node.bounds
            val cx = (b.left + b.right) / 2f
            val cy = (b.top + b.bottom) / 2f
            toolRouter.tap(cx, cy).respond(call, "Clicked node ${req.node_id}")
        }
    }

    @Serializable data class LaunchRequest(val package_name: String)
    @Serializable data class StopRequest(val package_name: String)
    @Serializable data class ShellRequest(val command: String)
    @Serializable data class IntentRequest(
        val action: String? = null,
        val data: String? = null,
        val package_name: String? = null,
        val component: String? = null,
        val extras: Map<String, String>? = null,
    )

    private fun io.ktor.server.routing.Route.appRoutes() {
        post("/app/launch") {
            val req = call.receive<LaunchRequest>()
            val result = toolRouter.appController.launch(req.package_name)
            if (result.isSuccess) {
                call.respondText(ok(result.getOrThrow()), ContentType.Application.Json)
            } else {
                call.respondText(err(result.exceptionOrNull()?.message ?: "Launch failed"), ContentType.Application.Json, HttpStatusCode.BadRequest)
            }
        }

        post("/app/stop") {
            val req = call.receive<StopRequest>()
            toolRouter.appController.forceStop(req.package_name).respond(call, "Stopped ${req.package_name}")
        }

        get("/app/top") {
            val top = toolRouter.appController.getTopActivity()
            if (top != null) {
                call.respondText(ok(top), ContentType.Application.Json)
            } else {
                call.respondText(err("Could not determine top activity"), ContentType.Application.Json, HttpStatusCode.InternalServerError)
            }
        }

        get("/packages") {
            val filter = call.request.queryParameters["filter"]
            val includeSystem = call.request.queryParameters["include_system"]?.toBoolean() ?: false
            val result = toolRouter.appController.listPackages(filter = filter, thirdPartyOnly = !includeSystem)
            if (result.isSuccess) {
                call.respondText(ok(result.getOrThrow().joinToString("\n")), ContentType.Application.Json)
            } else {
                call.respondText(err(result.exceptionOrNull()?.message ?: "Failed"), ContentType.Application.Json, HttpStatusCode.InternalServerError)
            }
        }

        post("/shell") {
            val req = call.receive<ShellRequest>()
            val result = toolRouter.appController.execShell(req.command)
            if (result.isSuccess) {
                call.respondText(ok(result.getOrThrow().ifBlank { "(no output)" }), ContentType.Application.Json)
            } else {
                call.respondText(err(result.exceptionOrNull()?.message ?: "Shell failed"), ContentType.Application.Json, HttpStatusCode.InternalServerError)
            }
        }

        post("/intent") {
            val req = call.receive<IntentRequest>()
            if (req.action == null && req.data == null && req.component == null) {
                call.respondText(err("At least one of action, data, or component required"), ContentType.Application.Json, HttpStatusCode.BadRequest); return@post
            }
            val result = toolRouter.appController.startIntent(
                action = req.action, dataUri = req.data, packageName = req.package_name, component = req.component, extras = req.extras,
            )
            if (result.isSuccess) {
                call.respondText(ok(result.getOrThrow()), ContentType.Application.Json)
            } else {
                call.respondText(err(result.exceptionOrNull()?.message ?: "Intent failed"), ContentType.Application.Json, HttpStatusCode.BadRequest)
            }
        }
    }

    private suspend fun Result<Unit>.respond(call: io.ktor.server.application.ApplicationCall, successMsg: String) {
        if (isSuccess) {
            call.respondText(ok(successMsg), ContentType.Application.Json)
        } else {
            call.respondText(err(exceptionOrNull()?.message ?: "Failed"), ContentType.Application.Json, HttpStatusCode.InternalServerError)
        }
    }
}
