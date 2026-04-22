package com.corepolicy.manager.core.data.repository

import android.content.Context
import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.emptyPreferences
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStoreFile
import com.corepolicy.manager.core.domain.CommandExecutor
import com.corepolicy.manager.core.domain.CorePolicyRepository
import com.corepolicy.manager.core.domain.DaemonTransport
import com.corepolicy.manager.core.domain.RustBridge
import com.corepolicy.manager.core.model.DaemonRunState
import com.corepolicy.manager.core.model.DaemonState
import com.corepolicy.manager.core.model.OverviewSnapshot
import com.corepolicy.manager.core.model.PoliciesState
import com.corepolicy.manager.core.model.PolicyMode
import com.corepolicy.manager.core.model.PolicyProfile
import com.corepolicy.manager.core.model.RustBridgeState
import com.corepolicy.manager.core.model.SettingsState
import com.corepolicy.manager.core.model.SystemHighlight
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.catch
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.map

private object PreferenceKeys {
    val selectedMode = stringPreferencesKey("selected_mode")
    val dynamicColor = booleanPreferencesKey("dynamic_color")
    val compactDensity = booleanPreferencesKey("compact_density")
    val confirmBeforePolicyApply = booleanPreferencesKey("confirm_before_policy_apply")
}

class DefaultCorePolicyRepository(
    context: Context,
    private val commandExecutor: CommandExecutor,
    private val daemonTransport: DaemonTransport,
    private val rustBridge: RustBridge,
) : CorePolicyRepository {

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private val dataStore = PreferenceDataStoreFactory.create(
        scope = scope,
        produceFile = { context.preferencesDataStoreFile("core_policy.preferences_pb") },
    )

    private val preferences = dataStore.data.catch { emit(emptyPreferences()) }

    private val selectedMode: Flow<PolicyMode> = preferences.map { prefs ->
        runCatching {
            prefs[PreferenceKeys.selectedMode]
                ?.let(PolicyMode::valueOf)
                ?: PolicyMode.Balanced
        }.getOrDefault(PolicyMode.Balanced)
    }

    private val settingsState: Flow<SettingsState> = preferences.map { prefs ->
        SettingsState(
            dynamicColor = prefs[PreferenceKeys.dynamicColor] ?: false,
            compactDensity = prefs[PreferenceKeys.compactDensity] ?: false,
            confirmBeforePolicyApply = prefs[PreferenceKeys.confirmBeforePolicyApply] ?: true,
        )
    }

    private val rustBridgeState: Flow<RustBridgeState> = preferences.map {
        rustBridge.describe()
    }

    private val daemonState: Flow<DaemonState> = selectedMode.map { mode ->
        val endpointLabel = when (mode) {
            PolicyMode.Balanced -> "Foreground service seam"
            PolicyMode.Efficiency -> "Deferred worker seam"
            PolicyMode.Performance -> "Dedicated daemon seam"
        }
        DaemonState(
            runState = DaemonRunState.Idle,
            endpointLabel = endpointLabel,
            lastHandshake = daemonTransport.requestStatus(),
            transport = "Command / binder / socket ready",
        )
    }

    override fun observeOverview(): Flow<OverviewSnapshot> {
        return combine(
            selectedMode,
            daemonState,
            rustBridgeState,
        ) { mode, daemon, rust ->
            val profile = profileFor(mode)
            OverviewSnapshot(
                activeProfile = profile,
                daemonState = daemon,
                rustBridge = rust,
                highlights = listOf(
                    SystemHighlight(
                        label = "CPU Envelope",
                        value = profile.cpuBudget,
                        supportingText = "Mapped to policy selection rather than hard-coded commands.",
                    ),
                    SystemHighlight(
                        label = "Memory Guardrails",
                        value = profile.memoryBudget,
                        supportingText = "Reserved for future cgroup or service tuning adapters.",
                    ),
                    SystemHighlight(
                        label = "Network Posture",
                        value = profile.networkBudget,
                        supportingText = "Designed to route through daemon or Rust bridge later.",
                    ),
                ),
            )
        }
    }

    override fun observeDaemonState(): Flow<DaemonState> = daemonState

    override fun observePolicies(): Flow<PoliciesState> {
        return selectedMode.map { mode ->
            PoliciesState(
                availableProfiles = PolicyMode.entries.map(::profileFor),
                selectedMode = mode,
            )
        }
    }

    override fun observeSettings(): Flow<SettingsState> = settingsState

    override suspend fun selectPolicyMode(mode: PolicyMode) {
        dataStore.edit { prefs ->
            prefs[PreferenceKeys.selectedMode] = mode.name
        }
    }

    override suspend fun setDynamicColor(enabled: Boolean) {
        dataStore.edit { prefs ->
            prefs[PreferenceKeys.dynamicColor] = enabled
        }
    }

    override suspend fun setCompactDensity(enabled: Boolean) {
        dataStore.edit { prefs ->
            prefs[PreferenceKeys.compactDensity] = enabled
        }
    }

    override suspend fun setConfirmBeforePolicyApply(enabled: Boolean) {
        commandExecutor.execute(
            request = com.corepolicy.manager.core.model.CommandRequest(
                executable = "policy-preview",
                args = listOf(enabled.toString()),
            ),
        )
        dataStore.edit { prefs ->
            prefs[PreferenceKeys.confirmBeforePolicyApply] = enabled
        }
    }

    private fun profileFor(mode: PolicyMode): PolicyProfile {
        return when (mode) {
            PolicyMode.Balanced -> PolicyProfile(
                mode = mode,
                title = "Balanced",
                summary = "Stable daily profile with clear seams for daemon mediation.",
                cpuBudget = "Adaptive",
                memoryBudget = "Moderate",
                networkBudget = "Measured",
            )
            PolicyMode.Efficiency -> PolicyProfile(
                mode = mode,
                title = "Efficiency",
                summary = "Bias toward lower wakeups and lighter background pressure.",
                cpuBudget = "Conservative",
                memoryBudget = "Lean",
                networkBudget = "Restricted",
            )
            PolicyMode.Performance -> PolicyProfile(
                mode = mode,
                title = "Performance",
                summary = "Reserves headroom for real-time daemon and Rust-backed execution.",
                cpuBudget = "Expanded",
                memoryBudget = "Generous",
                networkBudget = "Priority",
            )
        }
    }
}
