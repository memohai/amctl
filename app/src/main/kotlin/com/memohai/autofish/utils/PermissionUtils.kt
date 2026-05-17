package com.memohai.autofish.utils

import android.accessibilityservice.AccessibilityServiceInfo
import android.content.ComponentName
import android.content.Context
import android.view.accessibility.AccessibilityManager
import android.provider.Settings

object PermissionUtils {
    fun isAccessibilityServiceEnabled(context: Context, serviceClass: Class<*>): Boolean =
        isEnabledInAccessibilityManager(context, serviceClass) ||
            isEnabledInSecureSettings(context, serviceClass)

    private fun isEnabledInAccessibilityManager(context: Context, serviceClass: Class<*>): Boolean {
        val serviceClassName = serviceClass.name
        val packageName = context.packageName
        val am = context.getSystemService(Context.ACCESSIBILITY_SERVICE) as? AccessibilityManager ?: return false
        val enabled = am.getEnabledAccessibilityServiceList(AccessibilityServiceInfo.FEEDBACK_ALL_MASK)
        return enabled.any { info ->
            info.resolveInfo?.serviceInfo?.packageName == packageName &&
                info.resolveInfo?.serviceInfo?.name == serviceClassName
        }
    }

    private fun isEnabledInSecureSettings(context: Context, serviceClass: Class<*>): Boolean {
        val serviceClassName = serviceClass.name
        val packageName = context.packageName
        val enabledServices =
            Settings.Secure.getString(
                context.contentResolver,
                Settings.Secure.ENABLED_ACCESSIBILITY_SERVICES,
            ) ?: return false

        return enabledServices
            .split(':')
            .asSequence()
            .map { it.trim() }
            .filter { it.isNotEmpty() }
            .mapNotNull { ComponentName.unflattenFromString(it) }
            .any { it.packageName == packageName && it.className == serviceClassName }
    }
}
