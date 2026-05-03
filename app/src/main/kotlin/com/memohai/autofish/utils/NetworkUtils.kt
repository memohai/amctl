package com.memohai.autofish.utils

import java.net.Inet4Address
import java.net.NetworkInterface
import java.util.Locale

object NetworkUtils {
    fun getDeviceIpAddress(): String? =
        try {
            val candidates =
                NetworkInterface.getNetworkInterfaces()
                    ?.toList()
                    ?.filter { it.isUsableInterface() }
                    ?.flatMap { networkInterface ->
                        networkInterface.inetAddresses
                            .toList()
                            .filterIsInstance<Inet4Address>()
                            .filter { it.isUsableAddress() }
                            .map { address -> networkInterface to address }
                    }
                    .orEmpty()

            (
                candidates.firstOrNull { it.first.isPreferredLanInterface() }
                    ?: candidates.firstOrNull()
            )?.second
                ?.hostAddress
        } catch (_: Exception) {
            null
        }

    private fun NetworkInterface.isUsableInterface(): Boolean {
        val lowerName = name.lowercase(Locale.ROOT)
        return isUp &&
            !isLoopback &&
            !isVirtual &&
            !isPointToPoint &&
            ignoredInterfacePrefixes.none { lowerName.startsWith(it) }
    }

    private fun NetworkInterface.isPreferredLanInterface(): Boolean {
        val lowerName = name.lowercase(Locale.ROOT)
        return preferredLanInterfacePrefixes.any { lowerName.startsWith(it) }
    }

    private fun Inet4Address.isUsableAddress(): Boolean =
        !isAnyLocalAddress &&
            !isLoopbackAddress &&
            !isLinkLocalAddress &&
            !isMulticastAddress &&
            hostAddress != "0.0.0.0"

    private val preferredLanInterfacePrefixes =
        listOf(
            "wlan",
            "wifi",
            "eth",
            "en",
        )

    private val ignoredInterfacePrefixes =
        listOf(
            "rmnet",
            "dummy",
            "docker",
            "veth",
            "virbr",
            "br-",
            "tun",
            "tap",
            "ip6tnl",
            "sit",
        )
}
