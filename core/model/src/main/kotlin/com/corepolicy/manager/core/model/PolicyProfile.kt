package com.corepolicy.manager.core.model

data class PolicyProfile(
    val mode: PolicyMode,
    val title: String,
    val summary: String,
    val cpuBudget: String,
    val memoryBudget: String,
    val networkBudget: String,
)
