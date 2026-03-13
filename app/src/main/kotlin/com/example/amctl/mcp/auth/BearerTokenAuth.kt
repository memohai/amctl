package com.example.amctl.mcp.auth

import io.ktor.http.HttpStatusCode
import io.ktor.server.application.createApplicationPlugin
import io.ktor.server.response.respond

val BearerTokenAuth = createApplicationPlugin(name = "BearerTokenAuth", createConfiguration = ::BearerTokenAuthConfig) {
    val expectedToken = pluginConfig.token

    onCall { call ->
        val path = call.request.local.uri
        if (path == "/health") return@onCall

        val authHeader = call.request.headers["Authorization"]
        if (authHeader == null || !authHeader.startsWith("Bearer ")) {
            call.respond(HttpStatusCode.Unauthorized, "Missing or invalid Authorization header")
            return@onCall
        }

        val token = authHeader.removePrefix("Bearer ")
        if (!constantTimeEquals(token, expectedToken)) {
            call.respond(HttpStatusCode.Unauthorized, "Invalid bearer token")
            return@onCall
        }
    }
}

class BearerTokenAuthConfig {
    var token: String = ""
}

private fun constantTimeEquals(a: String, b: String): Boolean {
    val aBytes = a.toByteArray()
    val bBytes = b.toByteArray()
    if (aBytes.size != bBytes.size) {
        // Still iterate to avoid timing leaks on length
        var dummy = 0
        for (i in aBytes.indices) dummy = dummy or aBytes[i].toInt()
        return false
    }
    var result = 0
    for (i in aBytes.indices) {
        result = result or (aBytes[i].toInt() xor bBytes[i].toInt())
    }
    return result == 0
}
