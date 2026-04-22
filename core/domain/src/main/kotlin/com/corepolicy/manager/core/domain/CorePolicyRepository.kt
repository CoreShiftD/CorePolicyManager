package com.corepolicy.manager.core.domain

import com.corepolicy.manager.core.model.DaemonState
import com.corepolicy.manager.core.model.OverviewSnapshot
import com.corepolicy.manager.core.model.PoliciesState
import com.corepolicy.manager.core.model.PolicyMode
import com.corepolicy.manager.core.model.SettingsState
import kotlinx.coroutines.flow.Flow

interface CorePolicyRepository {
    fun observeOverview(): Flow<OverviewSnapshot>
    fun observeDaemonState(): Flow<DaemonState>
    fun observePolicies(): Flow<PoliciesState>
    fun observeSettings(): Flow<SettingsState>

    suspend fun selectPolicyMode(mode: PolicyMode)
    suspend fun setDynamicColor(enabled: Boolean)
    suspend fun setCompactDensity(enabled: Boolean)
    suspend fun setConfirmBeforePolicyApply(enabled: Boolean)
}
