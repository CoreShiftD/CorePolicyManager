package com.corepolicy.manager.core.model

data class OverviewSnapshot(
    val activeProfile: PolicyProfile,
    val daemonState: DaemonState,
    val rustBridge: RustBridgeState,
    val highlights: List<SystemHighlight>,
)
