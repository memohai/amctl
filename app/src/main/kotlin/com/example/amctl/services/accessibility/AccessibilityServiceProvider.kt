package com.example.amctl.services.accessibility

import android.content.Context
import android.view.accessibility.AccessibilityNodeInfo
import android.view.accessibility.AccessibilityWindowInfo

interface AccessibilityServiceProvider {
    fun getRootNode(): AccessibilityNodeInfo?
    fun getAccessibilityWindows(): List<AccessibilityWindowInfo>
    fun getCurrentPackageName(): String?
    fun getCurrentActivityName(): String?
    fun getScreenInfo(): ScreenInfo
    fun isReady(): Boolean
    fun getContext(): Context?
}
