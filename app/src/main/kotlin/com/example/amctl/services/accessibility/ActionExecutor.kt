package com.example.amctl.services.accessibility

interface ActionExecutor {
    suspend fun clickNode(nodeId: String, windows: List<WindowData>): Result<Unit>
    suspend fun longClickNode(nodeId: String, windows: List<WindowData>): Result<Unit>
    suspend fun setTextOnNode(nodeId: String, text: String, windows: List<WindowData>): Result<Unit>
    suspend fun scrollNode(nodeId: String, direction: ScrollDirection, windows: List<WindowData>): Result<Unit>
    suspend fun tap(x: Float, y: Float): Result<Unit>
    suspend fun longPress(x: Float, y: Float, duration: Long = DEFAULT_LONG_PRESS_DURATION_MS): Result<Unit>
    suspend fun doubleTap(x: Float, y: Float): Result<Unit>
    suspend fun swipe(x1: Float, y1: Float, x2: Float, y2: Float, duration: Long = DEFAULT_SWIPE_DURATION_MS): Result<Unit>
    suspend fun scroll(direction: ScrollDirection, amount: ScrollAmount = ScrollAmount.MEDIUM): Result<Unit>
    suspend fun pressBack(): Result<Unit>
    suspend fun pressHome(): Result<Unit>

    companion object {
        internal const val DEFAULT_LONG_PRESS_DURATION_MS = 1000L
        internal const val DEFAULT_SWIPE_DURATION_MS = 300L
    }
}
