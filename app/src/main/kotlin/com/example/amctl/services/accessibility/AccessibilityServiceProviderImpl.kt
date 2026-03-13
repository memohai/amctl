package com.example.amctl.services.accessibility

import android.content.Context
import android.view.accessibility.AccessibilityNodeInfo
import android.view.accessibility.AccessibilityWindowInfo
import com.example.amctl.mcp.McpToolException
import javax.inject.Inject

class AccessibilityServiceProviderImpl
    @Inject
    constructor() : AccessibilityServiceProvider {
        override fun getRootNode(): AccessibilityNodeInfo? =
            AmctlAccessibilityService.instance?.rootInActiveWindow

        override fun getAccessibilityWindows(): List<AccessibilityWindowInfo> =
            AmctlAccessibilityService.instance?.windows ?: emptyList()

        override fun getCurrentPackageName(): String? =
            AmctlAccessibilityService.instance?.rootInActiveWindow?.packageName?.toString()

        override fun getCurrentActivityName(): String? = null

        override fun getScreenInfo(): ScreenInfo =
            AmctlAccessibilityService.instance?.getScreenInfo()
                ?: throw McpToolException.PermissionDenied("Accessibility service not available")

        override fun isReady(): Boolean = AmctlAccessibilityService.instance != null

        override fun getContext(): Context? = AmctlAccessibilityService.instance
    }
