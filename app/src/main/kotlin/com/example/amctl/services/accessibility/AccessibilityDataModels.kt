package com.example.amctl.services.accessibility

import kotlinx.serialization.Serializable

@Serializable
data class BoundsData(
    val left: Int,
    val top: Int,
    val right: Int,
    val bottom: Int,
)

@Serializable
data class AccessibilityNodeData(
    val id: String,
    val className: String? = null,
    val text: String? = null,
    val contentDescription: String? = null,
    val resourceId: String? = null,
    val bounds: BoundsData,
    val clickable: Boolean = false,
    val longClickable: Boolean = false,
    val focusable: Boolean = false,
    val scrollable: Boolean = false,
    val editable: Boolean = false,
    val enabled: Boolean = false,
    val visible: Boolean = false,
    val children: List<AccessibilityNodeData> = emptyList(),
)

@Serializable
data class WindowData(
    val windowId: Int,
    val windowType: String,
    val packageName: String? = null,
    val title: String? = null,
    val activityName: String? = null,
    val layer: Int = 0,
    val focused: Boolean = false,
    val tree: AccessibilityNodeData,
)

@Serializable
data class MultiWindowResult(
    val windows: List<WindowData>,
    val degraded: Boolean = false,
)

data class ScreenInfo(
    val width: Int,
    val height: Int,
    val densityDpi: Int,
    val orientation: String,
)

enum class FindBy {
    TEXT,
    CONTENT_DESC,
    RESOURCE_ID,
    CLASS_NAME,
}

@Serializable
data class ElementInfo(
    val id: String,
    val text: String? = null,
    val contentDescription: String? = null,
    val resourceId: String? = null,
    val className: String? = null,
    val bounds: BoundsData,
    val clickable: Boolean = false,
    val longClickable: Boolean = false,
    val scrollable: Boolean = false,
    val editable: Boolean = false,
    val enabled: Boolean = false,
    val visible: Boolean = false,
)

enum class ScrollDirection {
    UP,
    DOWN,
    LEFT,
    RIGHT,
}

@Suppress("MagicNumber")
enum class ScrollAmount(val screenPercentage: Float) {
    SMALL(0.25f),
    MEDIUM(0.50f),
    LARGE(0.75f),
}
