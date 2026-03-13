package com.example.amctl.data.model

enum class BindingAddress(val address: String) {
    LOCALHOST("127.0.0.1"),
    ALL_INTERFACES("0.0.0.0"),
    ;

    companion object {
        fun fromAddress(address: String): BindingAddress =
            entries.firstOrNull { it.address == address } ?: LOCALHOST
    }
}
