package com.corepolicy.manager.core.model

data class CommandRequest(
    val executable: String,
    val args: List<String>,
)

data class CommandResult(
    val exitCode: Int,
    val stdout: String,
    val stderr: String,
)
