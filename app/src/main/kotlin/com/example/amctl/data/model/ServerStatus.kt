package com.example.amctl.data.model

sealed class ServerStatus {
    data object Stopped : ServerStatus()
    data object Starting : ServerStatus()
    data class Running(val port: Int, val address: String) : ServerStatus()
    data object Stopping : ServerStatus()
    data class Error(val message: String) : ServerStatus()
}
