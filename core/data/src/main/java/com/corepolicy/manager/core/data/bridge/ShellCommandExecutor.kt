package com.corepolicy.manager.core.data.bridge

import com.corepolicy.manager.core.domain.CommandExecutor
import com.corepolicy.manager.core.model.CommandRequest
import com.corepolicy.manager.core.model.CommandResult

class ShellCommandExecutor : CommandExecutor {
    override suspend fun execute(request: CommandRequest): CommandResult {
        return CommandResult(
            exitCode = -1,
            stdout = "",
            stderr = "Command execution is intentionally disabled in the base skeleton.",
        )
    }
}
