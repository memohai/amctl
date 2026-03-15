package com.example.amctl.data.model

data class ServerConfig(
    val port: Int = DEFAULT_MCP_PORT,
    val bindingAddress: BindingAddress = BindingAddress.LOCALHOST,
    val bearerToken: String = "",
    val autoStartOnBoot: Boolean = false,
    val restPort: Int = DEFAULT_REST_PORT,
    val restBearerToken: String = "",
) {
    companion object {
        const val DEFAULT_MCP_PORT = 8080
        const val DEFAULT_REST_PORT = 8081
        const val DEFAULT_PORT = DEFAULT_MCP_PORT
        const val MIN_PORT = 1
        const val MAX_PORT = 65535
    }
}
