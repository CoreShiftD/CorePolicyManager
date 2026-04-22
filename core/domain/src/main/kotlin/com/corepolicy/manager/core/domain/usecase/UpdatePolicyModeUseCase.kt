package com.corepolicy.manager.core.domain.usecase

import com.corepolicy.manager.core.domain.CorePolicyRepository
import com.corepolicy.manager.core.model.PolicyMode

class UpdatePolicyModeUseCase(
    private val repository: CorePolicyRepository,
) {
    suspend operator fun invoke(mode: PolicyMode) {
        repository.selectPolicyMode(mode)
    }
}
