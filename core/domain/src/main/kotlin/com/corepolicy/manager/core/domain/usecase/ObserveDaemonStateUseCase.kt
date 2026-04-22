package com.corepolicy.manager.core.domain.usecase

import com.corepolicy.manager.core.domain.CorePolicyRepository

class ObserveDaemonStateUseCase(
    private val repository: CorePolicyRepository,
) {
    operator fun invoke() = repository.observeDaemonState()
}
