package com.corepolicy.manager

import com.corepolicy.manager.ui.components.SystemProfile
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update

enum class DaemonState { RUNNING, STOPPED, DEGRADED }
enum class ModuleHealth { HEALTHY, DEGRADED, DISABLED, CONFLICT }

data class DaemonOverviewStatus(
    val state: DaemonState,
    val activeProfile: SystemProfile,
    val enabledModules: Int,
    val lastAction: String,
    val disconnected: Boolean,
    val uptimeMs: Long,
    val lastSyncTimestampMs: Long,
    val warningCount: Int,
    val errorCount: Int,
    val restartInProgress: Boolean
)

data class ModuleStatus(
    val id: String,
    val title: String,
    val description: String,
    val enabled: Boolean,
    val health: ModuleHealth,
    val dependencyNote: String? = null,
    val conflictNote: String? = null,
    val lastAction: String = "No recent action",
    val hasSettings: Boolean = true
)

enum class LogCategory { POLICY, DAEMON, MODULE, ERROR }
enum class LogSeverity { INFO, WARNING, ERROR }

data class LogEntry(
    val category: LogCategory,
    val sourceId: String,
    val message: String,
    val timestampMs: Long,
    val severity: LogSeverity
)

interface DaemonPolicyService {
    fun observeOverview(): Flow<DaemonOverviewStatus>
    fun observeModules(): Flow<List<ModuleStatus>>
    fun observeLogs(): Flow<List<LogEntry>>
    suspend fun setModuleEnabled(moduleId: String, enabled: Boolean)
    suspend fun applyProfile(profile: SystemProfile)
    suspend fun applyAppPolicy(policy: AppPolicy)
    suspend fun restartDaemon()
}

class MockDaemonPolicyService : DaemonPolicyService {
    private val modules = MutableStateFlow(
        listOf(
            ModuleStatus("battery", "Battery Addon", "Power policy integration", true, ModuleHealth.HEALTHY, lastAction = "Enabled at boot"),
            ModuleStatus("preload", "Preload Addon", "App warmup and launch cache", true, ModuleHealth.HEALTHY, "Depends on process controller", lastAction = "Warmup cache refreshed"),
            ModuleStatus("process", "Process Controller", "Foreground/background daemon rules", true, ModuleHealth.HEALTHY, lastAction = "Policy sync complete")
        )
    )
    private val startTimestamp = System.currentTimeMillis() - 4 * 60 * 60 * 1000
    private val logs = MutableStateFlow(listOf(LogEntry(LogCategory.DAEMON, "daemon", "Boot sync complete", System.currentTimeMillis(), LogSeverity.INFO)))
    private val overview = MutableStateFlow(
        DaemonOverviewStatus(
            state = DaemonState.RUNNING,
            activeProfile = SystemProfile.BALANCED,
            enabledModules = modules.value.count { it.enabled },
            lastAction = "Boot sync complete",
            disconnected = false,
            uptimeMs = System.currentTimeMillis() - startTimestamp,
            lastSyncTimestampMs = System.currentTimeMillis(),
            warningCount = 0,
            errorCount = 0,
            restartInProgress = false
        )
    )

    override fun observeOverview(): Flow<DaemonOverviewStatus> = overview.asStateFlow()
    override fun observeModules(): Flow<List<ModuleStatus>> = modules.asStateFlow()
    override fun observeLogs(): Flow<List<LogEntry>> = logs.asStateFlow()

    override suspend fun setModuleEnabled(moduleId: String, enabled: Boolean) {
        modules.update { list ->
            list.map {
                if (it.id == moduleId) it.copy(
                    enabled = enabled,
                    health = if (enabled) ModuleHealth.HEALTHY else ModuleHealth.DISABLED,
                    lastAction = "${if (enabled) "Enabled" else "Disabled"} manually"
                ) else it
            }
        }
        val enabledCount = modules.value.count { it.enabled }
        overview.update {
            it.copy(
                enabledModules = enabledCount,
                lastAction = "${if (enabled) "Enabled" else "Disabled"} $moduleId",
                lastSyncTimestampMs = System.currentTimeMillis(),
                warningCount = modules.value.count { module -> module.health == ModuleHealth.DEGRADED || module.health == ModuleHealth.CONFLICT }
            )
        }
        logs.update {
            listOf(
                LogEntry(
                    category = LogCategory.MODULE,
                    sourceId = moduleId,
                    message = "${if (enabled) "Enabled" else "Disabled"} ${moduleId.replaceFirstChar { c -> c.uppercase() }} Addon",
                    timestampMs = System.currentTimeMillis(),
                    severity = LogSeverity.INFO
                )
            ) + it.take(24)
        }
    }

    override suspend fun applyProfile(profile: SystemProfile) {
        overview.update { it.copy(activeProfile = profile, lastAction = "Applied profile ${profile.title}", lastSyncTimestampMs = System.currentTimeMillis()) }
        logs.update {
            listOf(LogEntry(LogCategory.POLICY, "policy", "Applied profile ${profile.title}", System.currentTimeMillis(), LogSeverity.INFO)) + it.take(24)
        }
    }

    override suspend fun applyAppPolicy(policy: AppPolicy) {
        logs.update {
            listOf(LogEntry(LogCategory.POLICY, policy.packageName, "${policy.appName} policy updated", System.currentTimeMillis(), LogSeverity.INFO)) + it.take(24)
        }
    }

    override suspend fun restartDaemon() {
        overview.update { it.copy(restartInProgress = true, lastAction = "Restart in progress…") }
        overview.update {
            it.copy(
                state = DaemonState.RUNNING,
                lastAction = "Daemon restarted",
                restartInProgress = false,
                lastSyncTimestampMs = System.currentTimeMillis(),
                uptimeMs = System.currentTimeMillis() - startTimestamp
            )
        }
        logs.update { listOf(LogEntry(LogCategory.DAEMON, "daemon", "Daemon restarted", System.currentTimeMillis(), LogSeverity.WARNING)) + it.take(24) }
    }
}
