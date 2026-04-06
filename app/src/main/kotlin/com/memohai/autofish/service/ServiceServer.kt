package com.memohai.autofish.service

import com.memohai.autofish.service.auth.BearerTokenAuth
import com.memohai.autofish.services.accessibility.AccessibilityServiceProvider
import com.memohai.autofish.services.accessibility.AccessibilityTreeParser
import com.memohai.autofish.services.accessibility.CompactTreeFormatter
import com.memohai.autofish.services.accessibility.ElementFinder
import com.memohai.autofish.services.accessibility.FindBy
import com.memohai.autofish.services.accessibility.MultiWindowResult
import com.memohai.autofish.services.accessibility.ScrollAmount
import com.memohai.autofish.services.accessibility.ScrollDirection
import com.memohai.autofish.services.accessibility.BoundsData
import com.memohai.autofish.services.accessibility.AccessibilityNodeData
import com.memohai.autofish.services.accessibility.WindowData
import com.memohai.autofish.services.system.ToolRouter
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
import kotlinx.serialization.encodeToString
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledExecutorService
import java.util.concurrent.TimeUnit
import kotlin.math.abs

internal fun shouldRebuildRefs(
    hasCachedRefs: Boolean,
    uiSeqChanged: Boolean,
    nowMs: Long,
    lastComputedAtMs: Long,
    forceIntervalMs: Long,
): Boolean {
    if (!hasCachedRefs) return true
    if (uiSeqChanged) return true
    if (lastComputedAtMs <= 0L) return true
    return nowMs - lastComputedAtMs >= forceIntervalMs
}

internal fun stableObservedTopActivity(before: String?, after: String?): String? {
    if (before.isNullOrBlank() || after.isNullOrBlank()) {
        return null
    }
    return before.takeIf { it == after }
}

internal data class RefAliasToken(
    val exactToken: String,
    val identityToken: String,
)

internal fun buildObservedRefTokenMap(
    refs: List<ServiceServer.RefNode>,
    exactTokenBuilder: (ServiceServer.RefNode) -> String,
    identityTokenBuilder: (ServiceServer.RefNode) -> String,
): Map<String, RefAliasToken> = refs.associate {
    it.ref to RefAliasToken(
        exactToken = exactTokenBuilder(it),
        identityToken = identityTokenBuilder(it),
    )
}

internal fun resolveRecordedTokenForRef(
    refAlias: String,
    observedMap: Map<String, RefAliasToken>,
): RefAliasToken? = observedMap[refAlias]

internal fun findRefNodeByToken(
    refs: List<ServiceServer.RefNode>,
    token: RefAliasToken,
    exactTokenBuilder: (ServiceServer.RefNode) -> String,
    identityTokenBuilder: (ServiceServer.RefNode) -> String,
): ServiceServer.RefNode? {
    val exactMatch = refs.firstOrNull { exactTokenBuilder(it) == token.exactToken }
    if (exactMatch != null) return exactMatch
    val identityMatches = refs.filter { identityTokenBuilder(it) == token.identityToken }
    return identityMatches.singleOrNull()
}

class ServiceServer(
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
    private val overlayManager = OverlayManager()
    private var overlayScheduler: ScheduledExecutorService? = null
    private var refScheduler: ScheduledExecutorService? = null
    @Volatile
    private var overlayConfig = OverlayConfig()
    @Volatile
    private var refConfig = RefConfig()
    @Volatile
    private var refState = RefState.empty()
    @Volatile
    private var lastObservedRefTokenMap: Map<String, RefAliasToken> = emptyMap()
    @Volatile
    private var lastRefUiChangeSeq: Long = -1L
    @Volatile
    private var lastRefComputedAtMs: Long = 0L
    private val forceRefRebuildIntervalMs = 1_500L

    fun start() {
        server = embeddedServer(Netty, port = port, host = bindAddress) {
            install(ContentNegotiation) { json(json) }
            install(BearerTokenAuth) { token = bearerToken }

            routing {
                get("/health") {
                    call.respondText("""{"status":"healthy","type":"service"}""", ContentType.Application.Json)
                }

                route("/api") {
                    screenRoutes()
                    touchRoutes()
                    keyRoutes()
                    textRoutes()
                    nodeRoutes()
                    overlayRoutes()
                    appRoutes()
                }
            }
        }.start(wait = false)
        refreshRefState(collectScreenSnapshot())
        lastRefUiChangeSeq = accessibilityProvider.getUiChangeSeq()
        lastRefComputedAtMs = System.currentTimeMillis()
        if (refConfig.autoRefresh) {
            startRefAutoRefresh()
        }
    }

    fun stop() {
        stopOverlayAutoRefresh()
        stopRefAutoRefresh()
        runCatching { overlayManager.setEnabled(false) }
        server?.stop(gracePeriodMillis = 1000, timeoutMillis = 5000)
        server = null
    }

    @Synchronized
    fun setOverlayVisible(visible: Boolean): Result<OverlayStatePayload> {
        if (!visible) {
            stopOverlayAutoRefresh()
            val result = overlayManager.setEnabled(false)
            if (result.isFailure) {
                return Result.failure(result.exceptionOrNull() ?: IllegalStateException("Overlay disable failed"))
            }
            overlayConfig = overlayConfig.copy(enabled = false)
            return Result.success(overlayManager.state().toPayload())
        }

        val snapshot = collectScreenSnapshot()
        val marks = buildMarks(
            windows = snapshot.windows,
            interactiveOnly = overlayConfig.interactiveOnly,
            maxMarks = overlayConfig.maxMarks,
        )
        val result = overlayManager.setEnabled(
            true,
            marks,
            offsetX = overlayConfig.offsetX,
            offsetY = overlayConfig.offsetY,
        )
        if (result.isFailure) {
            return Result.failure(result.exceptionOrNull() ?: IllegalStateException("Overlay enable failed"))
        }
        overlayConfig = overlayConfig.copy(enabled = true)
        if (overlayConfig.autoRefresh) {
            startOverlayAutoRefresh()
        } else {
            stopOverlayAutoRefresh()
        }
        return Result.success(overlayManager.state().toPayload())
    }

    @Synchronized
    fun setRefVisible(visible: Boolean): Result<Unit> {
        refConfig = refConfig.copy(visible = visible)
        if (visible) {
            val overlayRes = setOverlayVisible(true)
            if (overlayRes.isFailure) {
                return Result.failure(overlayRes.exceptionOrNull() ?: IllegalStateException("Failed to enable overlay for refs"))
            }
        } else if (overlayConfig.enabled) {
            val snapshot = collectScreenSnapshot()
            val marks = buildMarks(
                windows = snapshot.windows,
                interactiveOnly = overlayConfig.interactiveOnly,
                maxMarks = overlayConfig.maxMarks,
            )
            overlayManager.updateMarks(marks)
        }
        return Result.success(Unit)
    }

    @Serializable
    data class ApiResponse(val ok: Boolean, val data: String? = null, val error: String? = null)

    private fun ok(data: String) = json.encodeToString(ApiResponse.serializer(), ApiResponse(ok = true, data = data))
    private fun err(msg: String) = json.encodeToString(ApiResponse.serializer(), ApiResponse(ok = false, error = msg))

    @Suppress("LongMethod")
    private fun io.ktor.server.routing.Route.screenRoutes() {
        get("/observe") {
            val include = (call.request.queryParameters["include"] ?: "top,screen")
                .split(",").map { it.trim().lowercase() }.toSet()
            val maxRows = call.request.queryParameters["max_rows"]?.toIntOrNull() ?: 120

            val topActivityBefore = readTopActivityOrNull()
            val snapshot = collectScreenSnapshot()
            val topActivityAfter = readTopActivityOrNull()
            val topActivity = stableObservedTopActivity(topActivityBefore, topActivityAfter)
            val allNodes = snapshot.windows.flatMap { flattenNodes(it.tree) }
            val hasWebView = allNodes.any { (it.className ?: "").contains("WebView", ignoreCase = true) }
            val nodeReliability = if (hasWebView || allNodes.isEmpty()) "low" else "high"

            var screenSlice: ObserveScreenSlice? = null
            if ("screen" in include) {
                val rows = allNodes.take(maxRows).map { node ->
                    kotlinx.serialization.json.buildJsonObject {
                        put("id", kotlinx.serialization.json.JsonPrimitive(node.id))
                        put("class", kotlinx.serialization.json.JsonPrimitive(node.className))
                        node.text?.let { put("text", kotlinx.serialization.json.JsonPrimitive(it)) }
                        node.contentDescription?.let { put("desc", kotlinx.serialization.json.JsonPrimitive(it)) }
                        node.resourceId?.let { put("res_id", kotlinx.serialization.json.JsonPrimitive(it)) }
                        put("bounds", kotlinx.serialization.json.JsonPrimitive(
                            "${node.bounds.left},${node.bounds.top},${node.bounds.right},${node.bounds.bottom}"))
                        val flags = buildNodeFlags(node)
                        if (flags.isNotEmpty()) put("flags", kotlinx.serialization.json.JsonPrimitive(flags))
                    }
                }
                screenSlice = ObserveScreenSlice(rowCount = allNodes.size, rows = rows)
            }

            var refsSlice: ObserveRefsSlice? = null
            if ("refs" in include) {
                val state = refreshRefState(snapshot)
                lastObservedRefTokenMap = buildObservedRefTokenMap(state.refs, ::buildRefToken, ::buildRefIdentityToken)
                val refRows = state.refs.take(maxRows.coerceAtLeast(0)).map {
                    RefRowPayload(
                        ref = it.ref,
                        node_id = it.nodeId,
                        class_name = it.className,
                        text = it.text,
                        desc = it.desc,
                        res_id = it.resId,
                        bounds = "${it.bounds.left},${it.bounds.top},${it.bounds.right},${it.bounds.bottom}",
                        flags = buildRefFlags(it),
                    )
                }
                refsSlice = ObserveRefsSlice(
                    refVersion = state.version,
                    refCount = state.refs.size,
                    updatedAtMs = state.updatedAtMs,
                    rows = refRows,
                )
            }

            val payload = ObservePayload(
                topActivity = topActivity,
                mode = snapshot.mode,
                hasWebView = hasWebView,
                nodeReliability = nodeReliability,
                screen = screenSlice,
                refs = refsSlice,
            )
            call.respondText(ok(json.encodeToString(payload)), ContentType.Application.Json)
        }

        get("/screen") {
            val snapshot = collectScreenSnapshot()
            val result = MultiWindowResult(windows = snapshot.windows, degraded = snapshot.degraded)
            val topActivity = readTopActivityOrNull()
            val modeHeader = "[mode: ${snapshot.mode}]"
            val topHeader = "[topActivity: ${topActivity ?: ""}]"
            val tsv = "$modeHeader\n$topHeader\n${compactTreeFormatter.formatMultiWindow(result, snapshot.screenInfo)}"
            call.respondText(ok(tsv), ContentType.Application.Json)
        }

        get("/screen/refs") {
            val refsResult = resolveRefsState()
            val state = refsResult.state
            lastObservedRefTokenMap = buildObservedRefTokenMap(state.refs, ::buildRefToken, ::buildRefIdentityToken)
            val rows = state.refs.take(refConfig.maxRefs.coerceAtLeast(1)).map {
                RefRowPayload(
                    ref = it.ref,
                    node_id = it.nodeId,
                    class_name = it.className,
                    text = it.text,
                    desc = it.desc,
                    res_id = it.resId,
                    bounds = "${it.bounds.left},${it.bounds.top},${it.bounds.right},${it.bounds.bottom}",
                    flags = buildRefFlags(it),
                )
            }
            val hasWebView = rows.any { (it.class_name ?: "").contains("WebView", ignoreCase = true) }
            val nodeReliability = if (hasWebView || rows.isEmpty()) "low" else "high"
            val topActivity = readTopActivityOrNull()
            val payload = RefScreenPayload(
                refVersion = state.version,
                refCount = state.refs.size,
                updatedAtMs = state.updatedAtMs,
                mode = refsResult.mode,
                hasWebView = hasWebView,
                nodeReliability = nodeReliability,
                rows = rows,
                topActivity = topActivity,
            )
            call.respondText(ok(json.encodeToString(payload)), ContentType.Application.Json)
        }

        get("/mark") {
            val maxMarks = call.request.queryParameters["max_marks"]?.toIntOrNull() ?: 120
            val interactiveOnly = call.request.queryParameters["interactive_only"]?.toBooleanStrictOrNull() ?: true
            val applyOverlay = call.request.queryParameters["apply_overlay"]?.toBooleanStrictOrNull() ?: false
            val snapshot = collectScreenSnapshot()
            val marks = buildMarks(snapshot.windows, interactiveOnly, maxMarks)
            if (applyOverlay) {
                val enabledResult = overlayManager.setEnabled(
                    true,
                    marks,
                    offsetX = overlayConfig.offsetX,
                    offsetY = overlayConfig.offsetY,
                )
                if (enabledResult.isFailure) {
                    call.respondText(
                        err(enabledResult.exceptionOrNull()?.message ?: "Overlay enable failed"),
                        ContentType.Application.Json,
                        HttpStatusCode.ServiceUnavailable,
                    )
                    return@get
                }
            }
            val payload = MarkPayload(
                mode = snapshot.mode,
                degraded = snapshot.degraded,
                interactiveOnly = interactiveOnly,
                maxMarks = maxMarks,
                markCount = marks.size,
                overlay = overlayManager.state().toPayload(),
                marks = marks.map { it.toSerializable() },
            )
            call.respondText(ok(json.encodeToString(payload)), ContentType.Application.Json)
        }

        get("/screenshot") {
            val maxDim = call.request.queryParameters["max_dim"]?.toIntOrNull() ?: 700
            val quality = call.request.queryParameters["quality"]?.toIntOrNull() ?: 80
            val annotate = call.request.queryParameters["annotate"]?.toBooleanStrictOrNull() ?: false
            val hideOverlay = call.request.queryParameters["hide_overlay"]?.toBooleanStrictOrNull() ?: !annotate
            val maxMarks = call.request.queryParameters["max_marks"]?.toIntOrNull() ?: 120
            val interactiveOnly = call.request.queryParameters["interactive_only"]?.toBooleanStrictOrNull() ?: true

            val overlayStateBefore = overlayManager.state()
            val marksBefore = overlayManager.currentMarks()
            var temporarilyEnabledByAnnotate = false
            var temporarilyHiddenOverlay = false
            if (annotate) {
                val snapshot = collectScreenSnapshot()
                val marks = buildMarks(snapshot.windows, interactiveOnly, maxMarks)
                val result = overlayManager.setEnabled(
                    true,
                    marks,
                    offsetX = overlayConfig.offsetX,
                    offsetY = overlayConfig.offsetY,
                )
                if (result.isFailure) {
                    call.respondText(
                        err(result.exceptionOrNull()?.message ?: "Overlay annotate failed"),
                        ContentType.Application.Json,
                        HttpStatusCode.ServiceUnavailable,
                    )
                    return@get
                }
                temporarilyEnabledByAnnotate = !overlayStateBefore.enabled
            }
            if (hideOverlay && overlayManager.state().enabled) {
                val result = overlayManager.setEnabled(false)
                if (result.isSuccess) {
                    temporarilyHiddenOverlay = overlayStateBefore.enabled
                }
            }

            val result = toolRouter.captureScreen(quality = quality, maxWidth = maxDim, maxHeight = maxDim)

            if (temporarilyHiddenOverlay) {
                overlayManager.setEnabled(
                    true,
                    marksBefore,
                    offsetX = overlayConfig.offsetX,
                    offsetY = overlayConfig.offsetY,
                )
            }
            if (temporarilyEnabledByAnnotate && !overlayStateBefore.enabled) {
                overlayManager.setEnabled(false)
            }
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
    @Serializable data class NodeTapRequest(
        val by: String,
        val value: String,
        val exact_match: Boolean = false,
    )

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
            val by = normalizeFindBy(req.by)
                ?: run {
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

        post("/nodes/tap") {
            val req = call.receive<NodeTapRequest>()
            if (req.value.isBlank()) {
                call.respondText(err("value must not be empty"), ContentType.Application.Json, HttpStatusCode.BadRequest)
                return@post
            }
            val by = normalizeSemanticTapBy(req.by)
                ?: run {
                    call.respondText(
                        err("Invalid by: ${req.by}. allowed: text,content_desc,resource_id,ref (aliases: desc,resid)"),
                        ContentType.Application.Json,
                        HttpStatusCode.BadRequest,
                    )
                    return@post
                }
            val byNormalized = findByName(by)
            if (by == SemanticTapBy.REF) {
                val refValue = req.value.trim()
                if (!refValue.matches(Regex("@n\\d+"))) {
                    call.respondText(
                        err("Invalid ref: $refValue (expected format @n<index>)"),
                        ContentType.Application.Json,
                        HttpStatusCode.BadRequest,
                    )
                    return@post
                }
                val token = resolveRecordedTokenForRef(refValue, lastObservedRefTokenMap)
                    ?: run {
                        call.respondText(
                            err("REF_ALIAS_UNOBSERVED: ref=$refValue"),
                            ContentType.Application.Json,
                            HttpStatusCode.Conflict,
                        )
                        return@post
                    }
                val state = resolveRefsState().state
                val refNode = findRefNodeByToken(state.refs, token, ::buildRefToken, ::buildRefIdentityToken)
                    ?: run {
                        call.respondText(
                            err("REF_ALIAS_STALE: ref=$refValue"),
                            ContentType.Application.Json,
                            HttpStatusCode.Conflict,
                        )
                        return@post
                    }
                val cx = (refNode.bounds.left + refNode.bounds.right) / 2f
                val cy = (refNode.bounds.top + refNode.bounds.bottom) / 2f
                toolRouter.tap(cx, cy).respond(
                    call,
                    "Tapped by ref '$refValue' at (${cx.toInt()}, ${cy.toInt()}) node=${refNode.nodeId}",
                )
                return@post
            }
            val snapshot = collectScreenSnapshot()
            if (snapshot.windows.isEmpty()) {
                call.respondText(err("Accessibility not available"), ContentType.Application.Json, HttpStatusCode.ServiceUnavailable)
                return@post
            }
            val orderedWindows = snapshot.windows.sortedWith(
                compareByDescending<WindowData> { it.focused }
                    .thenByDescending { it.layer },
            )
            val matched = orderedWindows.flatMap { window ->
                elementFinder.findElements(window.tree, by.toFindBy(), req.value, req.exact_match)
            }
            val candidates = matched.filter {
                it.enabled &&
                    it.visible &&
                    (it.clickable || it.longClickable) &&
                    area(it.bounds) > 0
            }
            when {
                candidates.isEmpty() -> {
                    call.respondText(
                        err(
                            "ASSERTION_FAILED: no clickable node matched by=$byNormalized value='${req.value}' exact_match=${req.exact_match}; matched_count=${matched.size}, candidate_count=0",
                        ),
                        ContentType.Application.Json,
                        HttpStatusCode.Conflict,
                    )
                    return@post
                }

                candidates.size > 1 -> {
                    call.respondText(
                        err(
                            "ASSERTION_FAILED: multiple clickable nodes matched by=$byNormalized value='${req.value}' exact_match=${req.exact_match}; matched_count=${matched.size}, candidate_count=${candidates.size}",
                        ),
                        ContentType.Application.Json,
                        HttpStatusCode.Conflict,
                    )
                    return@post
                }
            }
            val selected = candidates.first()
            val cx = (selected.bounds.left + selected.bounds.right) / 2f
            val cy = (selected.bounds.top + selected.bounds.bottom) / 2f
            toolRouter.tap(cx, cy).respond(
                call,
                "Tapped by $byNormalized='${req.value}' at (${cx.toInt()}, ${cy.toInt()}) node=${selected.id}",
            )
        }
    }

    private fun normalizeFindBy(raw: String): FindBy? = when (raw.lowercase()) {
        "text" -> FindBy.TEXT
        "content_desc", "desc" -> FindBy.CONTENT_DESC
        "resource_id", "resid", "res_id" -> FindBy.RESOURCE_ID
        "class_name", "class" -> FindBy.CLASS_NAME
        else -> null
    }

    private enum class SemanticTapBy {
        TEXT,
        CONTENT_DESC,
        RESOURCE_ID,
        REF,
    }

    private fun normalizeSemanticTapBy(raw: String): SemanticTapBy? = when (raw.lowercase()) {
        "text" -> SemanticTapBy.TEXT
        "content_desc", "desc" -> SemanticTapBy.CONTENT_DESC
        "resource_id", "resid", "res_id" -> SemanticTapBy.RESOURCE_ID
        "ref" -> SemanticTapBy.REF
        else -> null
    }

    private fun findByName(by: SemanticTapBy): String = when (by) {
        SemanticTapBy.TEXT -> "text"
        SemanticTapBy.CONTENT_DESC -> "content_desc"
        SemanticTapBy.RESOURCE_ID -> "resource_id"
        SemanticTapBy.REF -> "ref"
    }

    private fun SemanticTapBy.toFindBy(): FindBy = when (this) {
        SemanticTapBy.TEXT -> FindBy.TEXT
        SemanticTapBy.CONTENT_DESC -> FindBy.CONTENT_DESC
        SemanticTapBy.RESOURCE_ID -> FindBy.RESOURCE_ID
        SemanticTapBy.REF -> throw IllegalStateException("ref is not a FindBy")
    }

    @Serializable
    data class OverlayRequest(
        val enabled: Boolean,
        val max_marks: Int = 300,
        val interactive_only: Boolean = false,
        val auto_refresh: Boolean = true,
        val refresh_interval_ms: Long = 800L,
        val offset_x: Int? = null,
        val offset_y: Int? = null,
    )

    data class OverlayConfig(
        val enabled: Boolean = false,
        val maxMarks: Int = 300,
        val interactiveOnly: Boolean = false,
        val autoRefresh: Boolean = true,
        val refreshIntervalMs: Long = 800L,
        val offsetX: Int = 0,
        val offsetY: Int = 0,
    )

    @Serializable
    data class OverlayStatePayload(
        val available: Boolean,
        val enabled: Boolean,
        val markCount: Int,
        val autoRefresh: Boolean,
        val refreshIntervalMs: Long,
        val offsetX: Int,
        val offsetY: Int,
    )

    @Serializable
    data class SerializableMark(
        val index: Int,
        val label: String,
        val bounds: String,
        val node_id: String,
        val class_name: String? = null,
        val text: String? = null,
        val desc: String? = null,
        val res_id: String? = null,
    )

    @Serializable
    data class MarkPayload(
        val mode: ToolRouter.Mode,
        val degraded: Boolean,
        val interactiveOnly: Boolean,
        val maxMarks: Int,
        val markCount: Int,
        val overlay: OverlayStatePayload,
        val marks: List<SerializableMark>,
    )

    private fun io.ktor.server.routing.Route.overlayRoutes() {
        get("/overlay") {
            val state = overlayManager.state()
            call.respondText(ok(json.encodeToString(state.toPayload())), ContentType.Application.Json)
        }
        post("/overlay") {
            val req = call.receive<OverlayRequest>()
            if (!req.enabled) {
                stopOverlayAutoRefresh()
                val result = overlayManager.setEnabled(false)
                if (result.isFailure) {
                    call.respondText(
                        err(result.exceptionOrNull()?.message ?: "Overlay disable failed"),
                        ContentType.Application.Json,
                        HttpStatusCode.ServiceUnavailable,
                    )
                    return@post
                }
                overlayConfig = overlayConfig.copy(enabled = false)
                call.respondText(
                    ok(json.encodeToString(overlayManager.state().toPayload())),
                    ContentType.Application.Json,
                )
                return@post
            }
            val snapshot = collectScreenSnapshot()
            val marks = buildMarks(snapshot.windows, req.interactive_only, req.max_marks)
            val resolvedOffsetX = req.offset_x ?: overlayConfig.offsetX
            val resolvedOffsetY = req.offset_y ?: overlayConfig.offsetY
            val result = overlayManager.setEnabled(
                true,
                marks,
                offsetX = resolvedOffsetX,
                offsetY = resolvedOffsetY,
            )
            if (result.isFailure) {
                call.respondText(
                    err(result.exceptionOrNull()?.message ?: "Overlay enable failed"),
                    ContentType.Application.Json,
                    HttpStatusCode.ServiceUnavailable,
                )
                return@post
            }
            overlayConfig = OverlayConfig(
                enabled = true,
                maxMarks = req.max_marks.coerceAtLeast(1),
                interactiveOnly = req.interactive_only,
                autoRefresh = req.auto_refresh,
                refreshIntervalMs = req.refresh_interval_ms.coerceIn(200L, 5_000L),
                offsetX = resolvedOffsetX,
                offsetY = resolvedOffsetY,
            )
            if (overlayConfig.autoRefresh) {
                startOverlayAutoRefresh()
            } else {
                stopOverlayAutoRefresh()
            }
            call.respondText(
                ok(json.encodeToString(overlayManager.state().toPayload())),
                ContentType.Application.Json,
            )
        }
    }

    private data class ScreenSnapshot(
        val mode: ToolRouter.Mode,
        val screenInfo: com.memohai.autofish.services.accessibility.ScreenInfo,
        val windows: List<WindowData>,
        val degraded: Boolean,
    )

    private fun collectScreenSnapshot(): ScreenSnapshot {
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
                    windows.add(
                        WindowData(
                            windowId = 0,
                            windowType = "APPLICATION",
                            packageName = rootNode.packageName?.toString(),
                            focused = true,
                            tree = treeParser.parseTree(rootNode),
                        ),
                    )
                    @Suppress("DEPRECATION") rootNode.recycle()
                }
            }
        } else {
            degraded = true
        }

        return ScreenSnapshot(
            mode = toolRouter.currentMode,
            screenInfo = screenInfo,
            windows = windows,
            degraded = degraded,
        )
    }

    private fun readTopActivityOrNull(): String? = try {
        toolRouter.appController.getTopActivity()
    } catch (_: Exception) {
        null
    }

    private fun flattenNodes(node: AccessibilityNodeData): List<AccessibilityNodeData> {
        val out = mutableListOf(node)
        for (child in node.children) {
            out.addAll(flattenNodes(child))
        }
        return out
    }

    private fun buildNodeFlags(node: AccessibilityNodeData): String {
        val parts = mutableListOf<String>()
        if (node.visible) parts.add("on")
        if (node.clickable) parts.add("clk")
        if (node.longClickable) parts.add("lng")
        if (node.scrollable) parts.add("scr")
        if (node.editable) parts.add("edt")
        if (node.focusable) parts.add("fcs")
        if (node.enabled) parts.add("ena")
        return parts.joinToString(",")
    }

    private fun buildMarks(
        windows: List<WindowData>,
        interactiveOnly: Boolean,
        maxMarks: Int,
    ): List<OverlayMark> {
        val out = mutableListOf<OverlayMark>()
        for (window in windows) {
            if (window.windowType == "ACCESSIBILITY_OVERLAY") {
                continue
            }
            collectMarksRecursive(window.tree, interactiveOnly, out)
        }
        val sorted = out.sortedWith(
            compareBy<OverlayMark> { if (it.interactive) 1 else 0 }
                .thenByDescending { area(it.bounds) },
        )
        val base = sorted
            .take(maxMarks.coerceAtLeast(1))
        if (!refConfig.visible) {
            return base.mapIndexed { idx, mark ->
                val prefix = if (mark.interactive) "C" else "T"
                mark.copy(index = idx + 1, label = "${prefix}${idx + 1}")
            }
        }
        val refMap = refState.refs.associateBy({ it.nodeId }, { it.ref })
        return base.mapIndexed { idx, mark ->
            val refLabel = refMap[mark.nodeId]
            if (refLabel != null) {
                mark.copy(index = idx + 1, label = refLabel)
            } else {
                val prefix = if (mark.interactive) "C" else "T"
                mark.copy(index = idx + 1, label = "${prefix}${idx + 1}")
            }
        }
    }

    private fun collectMarksRecursive(
        node: AccessibilityNodeData,
        interactiveOnly: Boolean,
        out: MutableList<OverlayMark>,
    ) {
        val isInteractive = node.clickable || node.longClickable || node.editable || node.scrollable || node.focusable
        val hasText = !node.text.isNullOrBlank() || !node.contentDescription.isNullOrBlank()
        val shouldInclude = if (interactiveOnly) isInteractive else isInteractive || hasText
        if (shouldInclude && node.visible && area(node.bounds) > 0) {
            out.add(
                OverlayMark(
                    index = 0,
                    label = "",
                    interactive = isInteractive,
                    bounds = node.bounds,
                    nodeId = node.id,
                    className = node.className,
                    text = node.text,
                    desc = node.contentDescription,
                    resId = node.resourceId,
                ),
            )
        }
        for (child in node.children) {
            collectMarksRecursive(child, interactiveOnly, out)
        }
    }

    private fun area(bounds: com.memohai.autofish.services.accessibility.BoundsData): Int {
        val w = (bounds.right - bounds.left).coerceAtLeast(0)
        val h = (bounds.bottom - bounds.top).coerceAtLeast(0)
        return w * h
    }

    private fun OverlayMark.toSerializable(): SerializableMark = SerializableMark(
        index = index,
        label = label,
        bounds = "${bounds.left},${bounds.top},${bounds.right},${bounds.bottom}",
        node_id = nodeId,
        class_name = className,
        text = text,
        desc = desc,
        res_id = resId,
    )

    private fun OverlayState.toPayload(): OverlayStatePayload = OverlayStatePayload(
        available = available,
        enabled = enabled,
        markCount = markCount,
        autoRefresh = overlayConfig.autoRefresh && overlayConfig.enabled,
        refreshIntervalMs = overlayConfig.refreshIntervalMs,
        offsetX = overlayConfig.offsetX,
        offsetY = overlayConfig.offsetY,
    )

    @Synchronized
    private fun startOverlayAutoRefresh() {
        stopOverlayAutoRefresh()
        val scheduler = Executors.newSingleThreadScheduledExecutor()
        overlayScheduler = scheduler
        val interval = overlayConfig.refreshIntervalMs.coerceIn(200L, 5_000L)
        scheduler.scheduleAtFixedRate(
            {
                try {
                    if (!overlayConfig.enabled) return@scheduleAtFixedRate
                    val snapshot = collectScreenSnapshot()
                    val marks = buildMarks(
                        windows = snapshot.windows,
                        interactiveOnly = overlayConfig.interactiveOnly,
                        maxMarks = overlayConfig.maxMarks,
                    )
                    overlayManager.updateMarks(marks)
                } catch (_: Exception) {
                }
            },
            interval,
            interval,
            TimeUnit.MILLISECONDS,
        )
    }

    @Synchronized
    private fun stopOverlayAutoRefresh() {
        overlayScheduler?.shutdownNow()
        overlayScheduler = null
    }

    data class RefPanelStatePayload(
        val version: Long,
        val count: Int,
        val updatedAtMs: Long,
        val visible: Boolean,
        val autoRefresh: Boolean,
        val refreshIntervalMs: Long,
        val refs: List<RefRowPayload>,
    )

    @Synchronized
    fun getRefPanelState(limit: Int = 120): RefPanelStatePayload {
        val state = resolveRefsState().state
        return RefPanelStatePayload(
            version = state.version,
            count = state.refs.size,
            updatedAtMs = state.updatedAtMs,
            visible = refConfig.visible,
            autoRefresh = refConfig.autoRefresh,
            refreshIntervalMs = refConfig.refreshIntervalMs,
            refs = state.refs.take(limit.coerceAtLeast(1)).map {
                RefRowPayload(
                    ref = it.ref,
                    node_id = it.nodeId,
                    class_name = it.className,
                    text = it.text,
                    desc = it.desc,
                    res_id = it.resId,
                    bounds = "${it.bounds.left},${it.bounds.top},${it.bounds.right},${it.bounds.bottom}",
                    flags = buildRefFlags(it),
                )
            },
        )
    }

    @Synchronized
    fun setRefAutoRefresh(enabled: Boolean) {
        refConfig = refConfig.copy(autoRefresh = enabled)
        if (enabled) {
            startRefAutoRefresh()
        } else {
            stopRefAutoRefresh()
        }
    }

    private fun refreshRefState(snapshot: ScreenSnapshot): RefState {
        val refs = buildRefNodes(snapshot.windows, refConfig.maxRefs)
        val digest = buildRefDigest(refs)
        val current = refState
        if (current.digest == digest) {
            return current
        }
        val nextVersion = current.version + 1
        val nextRefs = refs.mapIndexed { idx, r -> r.copy(ref = "@n${idx + 1}") }
        val next = RefState(
            version = nextVersion,
            digest = digest,
            refs = nextRefs,
            updatedAtMs = System.currentTimeMillis(),
        )
        refState = next
        if (overlayConfig.enabled && refConfig.visible) {
            val marks = buildMarks(
                windows = snapshot.windows,
                interactiveOnly = overlayConfig.interactiveOnly,
                maxMarks = overlayConfig.maxMarks,
            )
            overlayManager.updateMarks(marks)
        }
        return next
    }

    private fun buildRefNodes(windows: List<WindowData>, maxRefs: Int): List<RefNode> {
        val out = mutableListOf<RefNode>()
        val orderedWindows = windows
            .filter { it.windowType != "ACCESSIBILITY_OVERLAY" }
            .sortedWith(compareByDescending<WindowData> { it.focused }.thenByDescending { it.layer })
        for (window in orderedWindows) {
            collectRefNodesRecursive(window.tree, window, out)
        }
        return out
            .sortedWith(
                compareBy<RefNode> { if (it.focused) 0 else 1 }
                    .thenByDescending { it.windowLayer }
                    .thenBy { it.bounds.top }
                    .thenBy { it.bounds.left }
                    .thenByDescending { area(it.bounds) },
            )
            .take(maxRefs.coerceAtLeast(1))
    }

    private fun collectRefNodesRecursive(node: AccessibilityNodeData, window: WindowData, out: MutableList<RefNode>) {
        val interactive = node.clickable || node.longClickable || node.editable || node.scrollable
        if (interactive && node.enabled && node.visible && area(node.bounds) > 0) {
            out.add(
                RefNode(
                    ref = "",
                    nodeId = node.id,
                    className = node.className,
                    text = node.text,
                    desc = node.contentDescription,
                    resId = node.resourceId,
                    bounds = node.bounds,
                    clickable = node.clickable,
                    longClickable = node.longClickable,
                    editable = node.editable,
                    scrollable = node.scrollable,
                    enabled = node.enabled,
                    visible = node.visible,
                    focused = window.focused,
                    windowLayer = window.layer,
                ),
            )
        }
        for (child in node.children) {
            collectRefNodesRecursive(child, window, out)
        }
    }

    private fun buildRefDigest(refs: List<RefNode>): String =
        refs.joinToString("|") {
            val coarseLeft = (it.bounds.left / 8) * 8
            val coarseTop = (it.bounds.top / 8) * 8
            val coarseRight = (it.bounds.right / 8) * 8
            val coarseBottom = (it.bounds.bottom / 8) * 8
            listOf(
                it.className ?: "",
                it.text ?: "",
                it.desc ?: "",
                it.resId ?: "",
                "$coarseLeft,$coarseTop,$coarseRight,$coarseBottom",
                if (it.clickable) "1" else "0",
                if (it.longClickable) "1" else "0",
                if (it.editable) "1" else "0",
                if (it.scrollable) "1" else "0",
            ).joinToString("#")
        }

    private fun buildRefToken(node: RefNode): String {
        val coarseLeft = (node.bounds.left / 8) * 8
        val coarseTop = (node.bounds.top / 8) * 8
        val coarseRight = (node.bounds.right / 8) * 8
        val coarseBottom = (node.bounds.bottom / 8) * 8
        val raw = listOf(
            node.resId ?: "",
            node.className ?: "",
            node.text ?: "",
            node.desc ?: "",
            "$coarseLeft,$coarseTop,$coarseRight,$coarseBottom",
            node.windowLayer.toString(),
            if (node.focused) "1" else "0",
        ).joinToString("|")
        return "rt_${raw.hashCode().toUInt().toString(16)}"
    }

    private fun buildRefIdentityToken(node: RefNode): String {
        val width = (node.bounds.right - node.bounds.left).coerceAtLeast(0)
        val height = (node.bounds.bottom - node.bounds.top).coerceAtLeast(0)
        val coarseWidth = (width / 8) * 8
        val coarseHeight = (height / 8) * 8
        val raw = listOf(
            node.resId ?: "",
            node.className ?: "",
            node.text ?: "",
            node.desc ?: "",
            "$coarseWidth,$coarseHeight",
            if (node.clickable) "1" else "0",
            if (node.longClickable) "1" else "0",
            if (node.editable) "1" else "0",
            if (node.scrollable) "1" else "0",
            node.windowLayer.toString(),
            if (node.focused) "1" else "0",
        ).joinToString("|")
        return "ri_${raw.hashCode().toUInt().toString(16)}"
    }

    private fun buildRefFlags(node: RefNode): String = buildString {
        append(if (node.visible) "on" else "off")
        if (node.clickable) append(",clk")
        if (node.longClickable) append(",lclk")
        if (node.scrollable) append(",scr")
        if (node.editable) append(",edt")
        if (node.enabled) append(",ena")
    }

    @Synchronized
    private fun startRefAutoRefresh() {
        stopRefAutoRefresh()
        val scheduler = Executors.newSingleThreadScheduledExecutor()
        refScheduler = scheduler
        val interval = refConfig.refreshIntervalMs.coerceIn(200L, 5_000L)
        scheduler.scheduleAtFixedRate(
            {
                try {
                    if (!refConfig.autoRefresh || !refConfig.visible) return@scheduleAtFixedRate
                    resolveRefsState()
                } catch (_: Exception) {
                }
            },
            interval,
            interval,
            TimeUnit.MILLISECONDS,
        )
    }

    @Synchronized
    private fun stopRefAutoRefresh() {
        refScheduler?.shutdownNow()
        refScheduler = null
    }

    @Serializable
    data class ObservePayload(
        val topActivity: String? = null,
        val mode: ToolRouter.Mode,
        val hasWebView: Boolean,
        val nodeReliability: String,
        val screen: ObserveScreenSlice? = null,
        val refs: ObserveRefsSlice? = null,
    )

    @Serializable
    data class ObserveScreenSlice(
        val rowCount: Int,
        val rows: List<kotlinx.serialization.json.JsonObject>,
    )

    @Serializable
    data class ObserveRefsSlice(
        val refVersion: Long,
        val refCount: Int,
        val updatedAtMs: Long,
        val rows: List<RefRowPayload>,
    )

    @Serializable
    data class RefRowPayload(
        val ref: String,
        val node_id: String,
        val class_name: String? = null,
        val text: String? = null,
        val desc: String? = null,
        val res_id: String? = null,
        val bounds: String,
        val flags: String,
    )

    @Serializable
    data class RefScreenPayload(
        val refVersion: Long,
        val refCount: Int,
        val updatedAtMs: Long,
        val mode: ToolRouter.Mode,
        val hasWebView: Boolean,
        val nodeReliability: String,
        val rows: List<RefRowPayload>,
        val topActivity: String? = null,
    )

    data class RefConfig(
        val autoRefresh: Boolean = true,
        val refreshIntervalMs: Long = 800L,
        val maxRefs: Int = 120,
        val visible: Boolean = false,
    )

    data class RefState(
        val version: Long,
        val digest: String,
        val refs: List<RefNode>,
        val updatedAtMs: Long,
    ) {
        companion object {
            fun empty(): RefState = RefState(version = 0, digest = "", refs = emptyList(), updatedAtMs = 0L)
        }
    }

    data class RefNode(
        val ref: String,
        val nodeId: String,
        val className: String?,
        val text: String?,
        val desc: String?,
        val resId: String?,
        val bounds: BoundsData,
        val clickable: Boolean,
        val longClickable: Boolean,
        val editable: Boolean,
        val scrollable: Boolean,
        val enabled: Boolean,
        val visible: Boolean,
        val focused: Boolean,
        val windowLayer: Int,
    )

    private data class RefsResolveResult(
        val state: RefState,
        val mode: ToolRouter.Mode,
    )

    @Synchronized
    private fun resolveRefsState(): RefsResolveResult {
        val now = System.currentTimeMillis()
        val currentUiSeq = accessibilityProvider.getUiChangeSeq()
        val shouldRebuild = shouldRebuildRefs(
            hasCachedRefs = refState.refs.isNotEmpty(),
            uiSeqChanged = currentUiSeq != lastRefUiChangeSeq,
            nowMs = now,
            lastComputedAtMs = lastRefComputedAtMs,
            forceIntervalMs = forceRefRebuildIntervalMs,
        )
        if (!shouldRebuild) {
            return RefsResolveResult(
                state = refState,
                mode = toolRouter.currentMode,
            )
        }

        val snapshot = collectScreenSnapshot()
        val state = refreshRefState(snapshot)
        lastRefUiChangeSeq = currentUiSeq
        lastRefComputedAtMs = now
        return RefsResolveResult(
            state = state,
            mode = snapshot.mode,
        )
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
