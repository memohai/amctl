package com.example.amctl.services.accessibility

import android.graphics.Rect
import android.util.Log
import android.view.accessibility.AccessibilityNodeInfo
import android.view.accessibility.AccessibilityWindowInfo
import javax.inject.Inject

class AccessibilityTreeParser
    @Inject
    constructor() {
        fun parseTree(
            rootNode: AccessibilityNodeInfo,
            rootParentId: String = ROOT_PARENT_ID,
        ): AccessibilityNodeData =
            parseNode(node = rootNode, depth = 0, index = 0, parentId = rootParentId, recycleNode = false)

        @Suppress("LongParameterList")
        internal fun parseNode(
            node: AccessibilityNodeInfo,
            depth: Int,
            index: Int,
            parentId: String,
            recycleNode: Boolean = true,
        ): AccessibilityNodeData {
            val rect = Rect()
            node.getBoundsInScreen(rect)
            val bounds = BoundsData(left = rect.left, top = rect.top, right = rect.right, bottom = rect.bottom)
            val nodeId = generateNodeId(node, bounds, depth, index, parentId)

            if (depth >= MAX_TREE_DEPTH) {
                Log.w(TAG, "Maximum tree depth ($MAX_TREE_DEPTH) reached, truncating subtree")
                if (recycleNode) {
                    @Suppress("DEPRECATION")
                    node.recycle()
                }
                return AccessibilityNodeData(
                    id = nodeId,
                    className = node.className?.toString(),
                    bounds = bounds,
                    visible = node.isVisibleToUser,
                )
            }

            val children = mutableListOf<AccessibilityNodeData>()
            for (i in 0 until node.childCount) {
                val childNode = node.getChild(i) ?: continue
                if (childNode.viewIdResourceName == null) childNode.refresh()
                children.add(parseNode(node = childNode, depth = depth + 1, index = i, parentId = nodeId))
            }

            val nodeData = AccessibilityNodeData(
                id = nodeId,
                className = node.className?.toString(),
                text = node.text?.toString(),
                contentDescription = node.contentDescription?.toString(),
                resourceId = node.viewIdResourceName,
                bounds = bounds,
                clickable = node.isClickable,
                longClickable = node.isLongClickable,
                focusable = node.isFocusable,
                scrollable = node.isScrollable,
                editable = node.isEditable,
                enabled = node.isEnabled,
                visible = node.isVisibleToUser,
                children = children,
            )

            if (recycleNode) {
                @Suppress("DEPRECATION")
                node.recycle()
            }

            return nodeData
        }

        internal fun generateNodeId(
            node: AccessibilityNodeInfo,
            bounds: BoundsData,
            depth: Int,
            index: Int,
            parentId: String,
        ): String {
            val resourceId = node.viewIdResourceName ?: ""
            val className = node.className?.toString() ?: ""
            val hashInput =
                "$resourceId|$className|${bounds.left},${bounds.top}," +
                    "${bounds.right},${bounds.bottom}|$depth|$index|$parentId"
            val hash = hashInput.hashCode().toUInt().toString(HASH_RADIX)
            return "node_$hash"
        }

        companion object {
            private const val TAG = "amctl:TreeParser"
            private const val ROOT_PARENT_ID = "root"
            private const val HASH_RADIX = 16
            internal const val MAX_TREE_DEPTH = 100

            fun mapWindowType(type: Int): String =
                when (type) {
                    AccessibilityWindowInfo.TYPE_APPLICATION -> "APPLICATION"
                    AccessibilityWindowInfo.TYPE_INPUT_METHOD -> "INPUT_METHOD"
                    AccessibilityWindowInfo.TYPE_SYSTEM -> "SYSTEM"
                    AccessibilityWindowInfo.TYPE_ACCESSIBILITY_OVERLAY -> "ACCESSIBILITY_OVERLAY"
                    AccessibilityWindowInfo.TYPE_SPLIT_SCREEN_DIVIDER -> "SPLIT_SCREEN_DIVIDER"
                    AccessibilityWindowInfo.TYPE_MAGNIFICATION_OVERLAY -> "MAGNIFICATION_OVERLAY"
                    else -> "UNKNOWN($type)"
                }
        }
    }
