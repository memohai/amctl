package com.example.amctl.utils

import java.net.Inet4Address
import java.net.NetworkInterface

object NetworkUtils {
    fun getDeviceIpAddress(): String? {
        return try {
            NetworkInterface.getNetworkInterfaces()?.toList()
                ?.flatMap { it.inetAddresses.toList() }
                ?.firstOrNull { !it.isLoopbackAddress && it is Inet4Address }
                ?.hostAddress
        } catch (_: Exception) {
            null
        }
    }
}
