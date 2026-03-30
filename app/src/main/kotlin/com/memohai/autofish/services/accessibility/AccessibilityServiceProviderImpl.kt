package com.memohai.autofish.services.accessibility

import android.content.Context
import android.view.accessibility.AccessibilityNodeInfo
import android.view.accessibility.AccessibilityWindowInfo
import com.memohai.autofish.core.ToolException
import javax.inject.Inject

class AccessibilityServiceProviderImpl
    @Inject
    constructor() : AccessibilityServiceProvider {
        override fun getRootNode(): AccessibilityNodeInfo? =
            AutoFishAccessibilityService.instance?.rootInActiveWindow

        override fun getAccessibilityWindows(): List<AccessibilityWindowInfo> =
            AutoFishAccessibilityService.instance?.windows ?: emptyList()

        override fun getCurrentPackageName(): String? =
            AutoFishAccessibilityService.instance?.rootInActiveWindow?.packageName?.toString()

        override fun getCurrentActivityName(): String? = null

        override fun getScreenInfo(): ScreenInfo =
            AutoFishAccessibilityService.instance?.getScreenInfo()
                ?: throw ToolException.PermissionDenied("Accessibility service not available")

        override fun isReady(): Boolean = AutoFishAccessibilityService.instance != null

        override fun getContext(): Context? = AutoFishAccessibilityService.instance

        override fun getUiChangeSeq(): Long = AutoFishAccessibilityService.uiChangeSeq
    }
