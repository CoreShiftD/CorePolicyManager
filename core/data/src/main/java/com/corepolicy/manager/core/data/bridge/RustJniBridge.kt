package com.corepolicy.manager.core.data.bridge

import com.corepolicy.manager.core.domain.RustBridge
import com.corepolicy.manager.core.model.RustBridgeState
import com.corepolicy.manager.core.model.RustBridgeStatus

class RustJniBridge : RustBridge {
    override suspend fun describe(): RustBridgeState {
        return RustBridgeState(
            status = RustBridgeStatus.Planned,
            crateName = "CoreShift",
            executionPath = "rust/",
            notes = "JNI, FFI, and daemon handoff seams are reserved without locking in a transport yet.",
        )
    }
}
