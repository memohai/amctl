package com.example.amctl.services.accessibility

import javax.inject.Inject

class ElementFinder
    @Inject
    constructor() {
        fun findElements(
            windows: List<WindowData>,
            by: FindBy,
            value: String,
            exactMatch: Boolean = false,
        ): List<ElementInfo> {
            val results = mutableListOf<ElementInfo>()
            for (windowData in windows) results.addAll(findElements(windowData.tree, by, value, exactMatch))
            return results
        }

        fun findElements(
            tree: AccessibilityNodeData,
            by: FindBy,
            value: String,
            exactMatch: Boolean = false,
        ): List<ElementInfo> {
            val results = mutableListOf<ElementInfo>()
            searchRecursive(tree, by, value, exactMatch, results)
            return results
        }

        fun findNodeById(windows: List<WindowData>, nodeId: String): AccessibilityNodeData? {
            for (windowData in windows) {
                val found = findNodeById(windowData.tree, nodeId)
                if (found != null) return found
            }
            return null
        }

        fun findNodeById(tree: AccessibilityNodeData, nodeId: String): AccessibilityNodeData? {
            if (tree.id == nodeId) return tree
            for (child in tree.children) {
                val found = findNodeById(child, nodeId)
                if (found != null) return found
            }
            return null
        }

        private fun searchRecursive(
            node: AccessibilityNodeData,
            by: FindBy,
            value: String,
            exactMatch: Boolean,
            results: MutableList<ElementInfo>,
        ) {
            val nodeValue = when (by) {
                FindBy.TEXT -> node.text
                FindBy.CONTENT_DESC -> node.contentDescription
                FindBy.RESOURCE_ID -> node.resourceId
                FindBy.CLASS_NAME -> node.className
            }

            if (matchesValue(nodeValue, value, exactMatch)) results.add(toElementInfo(node))
            for (child in node.children) searchRecursive(child, by, value, exactMatch, results)
        }

        internal fun matchesValue(nodeValue: String?, searchValue: String, exactMatch: Boolean): Boolean {
            if (nodeValue == null) return false
            return if (exactMatch) nodeValue == searchValue else nodeValue.contains(searchValue, ignoreCase = true)
        }

        private fun toElementInfo(node: AccessibilityNodeData): ElementInfo = ElementInfo(
            id = node.id,
            text = node.text,
            contentDescription = node.contentDescription,
            resourceId = node.resourceId,
            className = node.className,
            bounds = node.bounds,
            clickable = node.clickable,
            longClickable = node.longClickable,
            scrollable = node.scrollable,
            editable = node.editable,
            enabled = node.enabled,
            visible = node.visible,
        )
    }
