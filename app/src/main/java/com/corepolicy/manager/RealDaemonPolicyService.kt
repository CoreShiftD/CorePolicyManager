package com.corepolicy.manager

import com.corepolicy.manager.ui.components.SystemProfile
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.flow

/**
 * Placeholder seam for real daemon integration.
 *
 * Planned transport options:
 * - privileged shell command bridge
 * - local socket daemon
 * - binder/service IPC
 * - file-based command channel
 */
class RealDaemonPolicyService : DaemonPolicyService {
    override fun observeOverview(): Flow<DaemonOverviewStatus> = flow { error("Real daemon transport not implemented yet") }
    override fun observeModules(): Flow<List<ModuleStatus>> = flow { error("Real daemon transport not implemented yet") }
    override fun observeLogs(): Flow<List<LogEntry>> = flow { error("Real daemon transport not implemented yet") }
    override suspend fun setModuleEnabled(moduleId: String, enabled: Boolean) = error("Real daemon transport not implemented yet")
    override suspend fun applyProfile(profile: SystemProfile) = error("Real daemon transport not implemented yet")
    override suspend fun applyAppPolicy(policy: AppPolicy) = error("Real daemon transport not implemented yet")
    override suspend fun restartDaemon() = error("Real daemon transport not implemented yet")
}
