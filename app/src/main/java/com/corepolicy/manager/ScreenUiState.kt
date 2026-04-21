package com.corepolicy.manager

import com.corepolicy.manager.ui.components.DynamicMetric
import com.corepolicy.manager.ui.components.InsightItem
import com.corepolicy.manager.ui.components.SystemProfile

data class OverviewUiState(
    val metrics: List<DynamicMetric>,
    val insights: List<InsightItem>,
    val daemonStatus: DaemonOverviewStatus,
    val selectedProfile: SystemProfile,
    val managedAppsCount: Int
)

data class ModulesUiState(
    val modules: List<ModuleStatus>
)

data class AppManagerUiState(
    val policiesCount: Int
)

data class LogsUiState(
    val logs: List<LogEntry>
)
