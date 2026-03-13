package com.example.amctl.services.accessibility

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.GestureDescription
import android.graphics.Path
import android.os.Bundle
import android.util.Log
import android.view.accessibility.AccessibilityNodeInfo
import kotlinx.coroutines.delay
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withTimeoutOrNull
import javax.inject.Inject
import kotlin.coroutines.resume

@Suppress("TooManyFunctions")
class ActionExecutorImpl
    @Inject
    constructor(
        private val treeParser: AccessibilityTreeParser,
    ) : ActionExecutor {
        override suspend fun clickNode(nodeId: String, windows: List<WindowData>): Result<Unit> =
            performNodeAction(nodeId, windows) { realNode ->
                if (!realNode.isClickable) return@performNodeAction Result.failure(IllegalStateException("Node not clickable"))
                if (realNode.performAction(AccessibilityNodeInfo.ACTION_CLICK)) Result.success(Unit)
                else Result.failure(RuntimeException("ACTION_CLICK failed"))
            }

        override suspend fun longClickNode(nodeId: String, windows: List<WindowData>): Result<Unit> =
            performNodeAction(nodeId, windows) { realNode ->
                if (!realNode.isLongClickable) return@performNodeAction Result.failure(IllegalStateException("Node not long-clickable"))
                if (realNode.performAction(AccessibilityNodeInfo.ACTION_LONG_CLICK)) Result.success(Unit)
                else Result.failure(RuntimeException("ACTION_LONG_CLICK failed"))
            }

        override suspend fun setTextOnNode(nodeId: String, text: String, windows: List<WindowData>): Result<Unit> =
            performNodeAction(nodeId, windows) { realNode ->
                if (!realNode.isEditable) return@performNodeAction Result.failure(IllegalStateException("Node not editable"))
                val args = Bundle().apply {
                    putCharSequence(AccessibilityNodeInfo.ACTION_ARGUMENT_SET_TEXT_CHARSEQUENCE, text)
                }
                if (realNode.performAction(AccessibilityNodeInfo.ACTION_SET_TEXT, args)) Result.success(Unit)
                else Result.failure(RuntimeException("ACTION_SET_TEXT failed"))
            }

        override suspend fun scrollNode(nodeId: String, direction: ScrollDirection, windows: List<WindowData>): Result<Unit> =
            performNodeAction(nodeId, windows) { realNode ->
                val scrollable = findScrollableAncestor(realNode) ?: realNode.takeIf { it.isScrollable }
                if (scrollable == null) return@performNodeAction Result.failure(IllegalStateException("No scrollable ancestor found"))
                val action = when (direction) {
                    ScrollDirection.UP, ScrollDirection.LEFT -> AccessibilityNodeInfo.ACTION_SCROLL_BACKWARD
                    ScrollDirection.DOWN, ScrollDirection.RIGHT -> AccessibilityNodeInfo.ACTION_SCROLL_FORWARD
                }
                if (scrollable.performAction(action)) Result.success(Unit)
                else Result.failure(RuntimeException("Scroll failed"))
            }

        private fun findScrollableAncestor(node: AccessibilityNodeInfo): AccessibilityNodeInfo? {
            var current = node.parent
            while (current != null) {
                if (current.isScrollable) return current
                current = current.parent
            }
            return null
        }

        override suspend fun tap(x: Float, y: Float): Result<Unit> {
            val path = Path().apply { moveTo(x, y) }
            val stroke = GestureDescription.StrokeDescription(path, 0L, TAP_DURATION_MS)
            return dispatchGesture(stroke)
        }

        override suspend fun longPress(x: Float, y: Float, duration: Long): Result<Unit> {
            val path = Path().apply { moveTo(x, y) }
            val stroke = GestureDescription.StrokeDescription(path, 0L, duration)
            return dispatchGesture(stroke)
        }

        @Suppress("ReturnCount")
        override suspend fun doubleTap(x: Float, y: Float): Result<Unit> {
            val first = tap(x, y)
            if (first.isFailure) return first
            delay(DOUBLE_TAP_GAP_MS)
            return tap(x, y)
        }

        override suspend fun swipe(x1: Float, y1: Float, x2: Float, y2: Float, duration: Long): Result<Unit> {
            val path = Path().apply { moveTo(x1, y1); lineTo(x2, y2) }
            val stroke = GestureDescription.StrokeDescription(path, 0L, duration)
            return dispatchGesture(stroke)
        }

        override suspend fun scroll(direction: ScrollDirection, amount: ScrollAmount): Result<Unit> {
            val service = AmctlAccessibilityService.instance
                ?: return Result.failure(IllegalStateException("Accessibility service not available"))
            val screenInfo = service.getScreenInfo()
            val w = screenInfo.width.toFloat()
            val h = screenInfo.height.toFloat()
            val cx = w / 2f
            val cy = h / 2f
            val dist = when (direction) {
                ScrollDirection.UP, ScrollDirection.DOWN -> h * amount.screenPercentage
                ScrollDirection.LEFT, ScrollDirection.RIGHT -> w * amount.screenPercentage
            }
            val half = dist / 2f
            return when (direction) {
                ScrollDirection.UP -> swipe(cx, cy - half, cx, cy + half)
                ScrollDirection.DOWN -> swipe(cx, cy + half, cx, cy - half)
                ScrollDirection.LEFT -> swipe(cx - half, cy, cx + half, cy)
                ScrollDirection.RIGHT -> swipe(cx + half, cy, cx - half, cy)
            }
        }

        override suspend fun pressBack(): Result<Unit> =
            performGlobalAction(AccessibilityService.GLOBAL_ACTION_BACK)

        override suspend fun pressHome(): Result<Unit> =
            performGlobalAction(AccessibilityService.GLOBAL_ACTION_HOME)

        @Suppress("ReturnCount")
        private suspend fun performNodeAction(
            nodeId: String,
            windows: List<WindowData>,
            action: (AccessibilityNodeInfo) -> Result<Unit>,
        ): Result<Unit> {
            val service = AmctlAccessibilityService.instance
                ?: return Result.failure(IllegalStateException("Accessibility service not available"))
            val realWindows = service.windows
            if (realWindows.isNotEmpty()) {
                val realWindowById = realWindows.associateBy { it.id }
                for (windowData in windows) {
                    val realWindow = realWindowById[windowData.windowId] ?: continue
                    val realRootNode = realWindow.root ?: continue
                    val realNode = findNodeByWalk(realRootNode, windowData.tree, nodeId)
                    if (realNode != null) {
                        return try {
                            action(realNode)
                        } finally {
                            if (realNode !== realRootNode) {
                                @Suppress("DEPRECATION") realNode.recycle()
                            }
                            @Suppress("DEPRECATION") realRootNode.recycle()
                        }
                    }
                    @Suppress("DEPRECATION") realRootNode.recycle()
                }
            }
            val rootNode = service.rootInActiveWindow
                ?: return Result.failure(IllegalStateException("No root node available"))
            for (windowData in windows) {
                val realNode = findNodeByWalk(rootNode, windowData.tree, nodeId)
                if (realNode != null) {
                    return try { action(realNode) } finally {
                        if (realNode !== rootNode) { @Suppress("DEPRECATION") realNode.recycle() }
                        @Suppress("DEPRECATION") rootNode.recycle()
                    }
                }
            }
            @Suppress("DEPRECATION") rootNode.recycle()
            return Result.failure(NoSuchElementException("Node '$nodeId' not found"))
        }

        @Suppress("ReturnCount")
        private fun findNodeByWalk(
            realNode: AccessibilityNodeInfo,
            parsedNode: AccessibilityNodeData,
            targetNodeId: String,
            recycleOnMismatch: Boolean = false,
        ): AccessibilityNodeInfo? {
            if (parsedNode.id == targetNodeId) return realNode
            val minCount = minOf(realNode.childCount, parsedNode.children.size)
            for (i in 0 until minCount) {
                val realChild = realNode.getChild(i) ?: continue
                val found = findNodeByWalk(realChild, parsedNode.children[i], targetNodeId, recycleOnMismatch = true)
                if (found != null) {
                    if (recycleOnMismatch) { @Suppress("DEPRECATION") realNode.recycle() }
                    return found
                }
            }
            if (recycleOnMismatch) { @Suppress("DEPRECATION") realNode.recycle() }
            return null
        }

        private fun performGlobalAction(action: Int): Result<Unit> {
            val service = AmctlAccessibilityService.instance
                ?: return Result.failure(IllegalStateException("Accessibility service not available"))
            return if (service.performGlobalAction(action)) Result.success(Unit)
            else Result.failure(RuntimeException("Global action failed"))
        }

        private suspend fun dispatchGesture(stroke: GestureDescription.StrokeDescription): Result<Unit> {
            val service = AmctlAccessibilityService.instance
                ?: return Result.failure(IllegalStateException("Accessibility service not available"))
            val gesture = GestureDescription.Builder().addStroke(stroke).build()
            return withTimeoutOrNull(GESTURE_TIMEOUT_MS) {
                suspendCancellableCoroutine { continuation ->
                    val callback = object : AccessibilityService.GestureResultCallback() {
                        override fun onCompleted(desc: GestureDescription?) {
                            if (continuation.isActive) continuation.resume(Result.success(Unit))
                        }
                        override fun onCancelled(desc: GestureDescription?) {
                            if (continuation.isActive) continuation.resume(Result.failure(RuntimeException("Gesture cancelled")))
                        }
                    }
                    if (!service.dispatchGesture(gesture, callback, null)) {
                        if (continuation.isActive) continuation.resume(Result.failure(RuntimeException("Failed to dispatch gesture")))
                    }
                }
            } ?: Result.failure(RuntimeException("Gesture timed out"))
        }

        companion object {
            private const val TAP_DURATION_MS = 50L
            private const val DOUBLE_TAP_GAP_MS = 100L
            private const val GESTURE_TIMEOUT_MS = 10_000L
        }
    }
