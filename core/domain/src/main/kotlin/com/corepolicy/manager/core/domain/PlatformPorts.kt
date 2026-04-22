package com.corepolicy.manager.core.domain

import com.corepolicy.manager.core.model.CommandRequest
import com.corepolicy.manager.core.model.CommandResult
import com.corepolicy.manager.core.model.RustBridgeState

interface CommandExecutor {
    suspend fun execute(request: CommandRequest): CommandResult
}

interface DaemonTransport {
    suspend fun requestStatus(): String
}

interface RustBridge {
    suspend fun describe(): RustBridgeState
}
