package com.corepolicy.manager.core.domain.usecase

import com.corepolicy.manager.core.domain.CorePolicyRepository

class SetDynamicColorUseCase(
    private val repository: CorePolicyRepository,
) {
    suspend operator fun invoke(enabled: Boolean) {
        repository.setDynamicColor(enabled)
    }
}

class SetCompactDensityUseCase(
    private val repository: CorePolicyRepository,
) {
    suspend operator fun invoke(enabled: Boolean) {
        repository.setCompactDensity(enabled)
    }
}

class SetConfirmBeforePolicyApplyUseCase(
    private val repository: CorePolicyRepository,
) {
    suspend operator fun invoke(enabled: Boolean) {
        repository.setConfirmBeforePolicyApply(enabled)
    }
}
