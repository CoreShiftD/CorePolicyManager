package com.corepolicy.manager.core.model

data class PoliciesState(
    val availableProfiles: List<PolicyProfile>,
    val selectedMode: PolicyMode,
)
