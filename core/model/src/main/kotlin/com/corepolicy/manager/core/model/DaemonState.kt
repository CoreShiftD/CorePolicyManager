package com.corepolicy.manager.core.model

enum class DaemonRunState {
    Running,
    Idle,
    Disconnected,
}

data class DaemonState(
    val runState: DaemonRunState,
    val endpointLabel: String,
    val lastHandshake: String,
    val transport: String,
)
