package com.example.amctl.data.model

data class ServerConfig(
    val port: Int = DEFAULT_PORT,
    val bindingAddress: BindingAddress = BindingAddress.LOCALHOST,
    val bearerToken: String = "",
    val autoStartOnBoot: Boolean = false,
) {
    companion object {
        const val DEFAULT_PORT = 8080
        const val MIN_PORT = 1
        const val MAX_PORT = 65535
    }
}
