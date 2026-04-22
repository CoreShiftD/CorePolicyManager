package com.corepolicy.manager.core.model

enum class RustBridgeStatus {
    Ready,
    Planned,
    Offline,
}

data class RustBridgeState(
    val status: RustBridgeStatus,
    val crateName: String,
    val executionPath: String,
    val notes: String,
)
