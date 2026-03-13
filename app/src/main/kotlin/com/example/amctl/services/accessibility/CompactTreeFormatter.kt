package com.example.amctl.services.accessibility

import javax.inject.Inject

class CompactTreeFormatter
    @Inject
    constructor() {
        fun formatMultiWindow(result: MultiWindowResult, screenInfo: ScreenInfo): String {
            val sb = StringBuilder()

            if (result.degraded) sb.appendLine(DEGRADATION_NOTE)
            sb.appendLine(NOTE_LINE)
            sb.appendLine(NOTE_LINE_FLAGS_LEGEND)
            sb.appendLine(NOTE_LINE_OFFSCREEN_HINT)
            sb.appendLine("screen:${screenInfo.width}x${screenInfo.height} density:${screenInfo.densityDpi} orientation:${screenInfo.orientation}")

            for (windowData in result.windows) {
                sb.appendLine(buildWindowHeader(windowData))
                sb.appendLine(HEADER)
                val hierarchySb = StringBuilder()
                walkTree(
                    windowData.tree,
                    visitors = listOf(
                        { node, _ -> appendElementRow(sb, node) },
                        { node, depth ->
                            repeat(depth) { hierarchySb.append(HIERARCHY_INDENT) }
                            hierarchySb.appendLine(node.id)
                        },
                    ),
                )
                sb.appendLine(HIERARCHY_HEADER)
                sb.append(hierarchySb)
            }

            return sb.toString().trimEnd('\n')
        }

        private fun walkTree(
            node: AccessibilityNodeData,
            depth: Int = 0,
            visitors: List<(node: AccessibilityNodeData, depth: Int) -> Unit>,
        ) {
            val isKept = shouldKeepNode(node)
            if (isKept) {
                for (visitor in visitors) visitor(node, depth)
            }
            val childDepth = if (isKept) depth + 1 else depth
            for (child in node.children) walkTree(child, childDepth, visitors)
        }

        private fun appendElementRow(sb: StringBuilder, node: AccessibilityNodeData) {
            val id = node.id
            val className = simplifyClassName(node.className)
            val text = sanitizeText(node.text)
            val desc = sanitizeText(node.contentDescription)
            val resId = sanitizeText(node.resourceId)
            val bounds = "${node.bounds.left},${node.bounds.top},${node.bounds.right},${node.bounds.bottom}"
            val flags = buildFlags(node)
            sb.appendLine("$id\t$className\t$text\t$desc\t$resId\t$bounds\t$flags")
        }

        internal fun buildWindowHeader(windowData: WindowData): String = buildString {
            append("--- window:${windowData.windowId} type:${windowData.windowType} ")
            append("pkg:${windowData.packageName ?: "unknown"} ")
            append("title:${windowData.title ?: "unknown"} ")
            if (windowData.activityName != null) append("activity:${windowData.activityName} ")
            append("layer:${windowData.layer} focused:${windowData.focused} ---")
        }

        internal fun shouldKeepNode(node: AccessibilityNodeData): Boolean =
            !node.text.isNullOrEmpty() ||
                !node.contentDescription.isNullOrEmpty() ||
                !node.resourceId.isNullOrEmpty() ||
                node.clickable || node.longClickable || node.scrollable || node.editable

        internal fun simplifyClassName(className: String?): String {
            if (className.isNullOrEmpty()) return NULL_VALUE
            val lastDot = className.lastIndexOf('.')
            return if (lastDot >= 0) className.substring(lastDot + 1) else className
        }

        internal fun sanitizeText(text: String?): String {
            val sanitized = text?.replace('\t', ' ')?.replace('\n', ' ')?.replace('\r', ' ')?.trim()?.ifEmpty { null }
            return when {
                sanitized == null -> NULL_VALUE
                sanitized.length > MAX_TEXT_LENGTH -> sanitized.substring(0, MAX_TEXT_LENGTH) + TRUNCATION_SUFFIX
                else -> sanitized
            }
        }

        internal fun buildFlags(node: AccessibilityNodeData): String = buildString {
            append(if (node.visible) "on" else "off")
            if (node.clickable) append(",clk")
            if (node.longClickable) append(",lclk")
            if (node.focusable) append(",foc")
            if (node.scrollable) append(",scr")
            if (node.editable) append(",edt")
            if (node.enabled) append(",ena")
        }

        companion object {
            const val NULL_VALUE = "-"
            const val MAX_TEXT_LENGTH = 100
            const val TRUNCATION_SUFFIX = "...truncated"
            const val DEGRADATION_NOTE = "note:DEGRADED — multi-window unavailable, only active window reported"
            const val NOTE_LINE = "note:structural-only nodes are omitted from the tree"
            const val NOTE_LINE_FLAGS_LEGEND =
                "note:flags: on=onscreen off=offscreen clk=clickable lclk=longClickable foc=focusable scr=scrollable edt=editable ena=enabled"
            const val NOTE_LINE_OFFSCREEN_HINT = "note:offscreen items require scroll_to_node before interaction"
            const val HIERARCHY_HEADER = "hierarchy:"
            private const val HIERARCHY_INDENT = "  "
            const val HEADER = "node_id\tclass\ttext\tdesc\tres_id\tbounds\tflags"
        }
    }
