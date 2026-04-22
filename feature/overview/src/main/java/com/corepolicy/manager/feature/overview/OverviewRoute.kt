package com.corepolicy.manager.feature.overview

import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.core.designsystem.component.AppCard
import com.corepolicy.manager.core.designsystem.component.AppListItem
import com.corepolicy.manager.core.designsystem.component.AppTopBar
import com.corepolicy.manager.core.designsystem.component.ScreenContainer
import com.corepolicy.manager.core.designsystem.component.SectionTitle
import com.corepolicy.manager.core.domain.usecase.ObserveOverviewUseCase
import com.corepolicy.manager.core.model.DaemonRunState
import com.corepolicy.manager.core.model.DaemonState
import com.corepolicy.manager.core.model.OverviewSnapshot
import com.corepolicy.manager.core.model.PolicyMode
import com.corepolicy.manager.core.model.PolicyProfile
import com.corepolicy.manager.core.model.RustBridgeState
import com.corepolicy.manager.core.model.RustBridgeStatus
import com.corepolicy.manager.core.model.SystemHighlight

@Composable
fun OverviewRoute(
    observeOverview: ObserveOverviewUseCase,
) {
    val state by observeOverview().collectAsState(initial = overviewPreview())

    ScreenContainer {
        AppTopBar(
            title = "Overview",
            subtitle = "Thin shell, clear boundaries, and room for real daemon execution.",
        )
        AppCard(
            title = state.activeProfile.title,
            supportingText = state.activeProfile.summary,
        ) {
            AppListItem(
                label = "Daemon",
                value = state.daemonState.runState.name,
                supportingText = state.daemonState.endpointLabel,
            )
            Spacer(modifier = Modifier.height(12.dp))
            AppListItem(
                label = "Rust Bridge",
                value = state.rustBridge.status.name,
                supportingText = state.rustBridge.notes,
            )
        }
        SectionTitle(
            title = "Execution Posture",
            subtitle = "Current guardrails exposed as reusable UI state.",
        )
        state.highlights.forEach { highlight ->
            AppListItem(
                label = highlight.label,
                value = highlight.value,
                supportingText = highlight.supportingText,
            )
        }
    }
}

private fun overviewPreview() = OverviewSnapshot(
    activeProfile = PolicyProfile(
        mode = PolicyMode.Balanced,
        title = "Balanced",
        summary = "Stable orchestration baseline.",
        cpuBudget = "Adaptive",
        memoryBudget = "Moderate",
        networkBudget = "Measured",
    ),
    daemonState = DaemonState(
        runState = DaemonRunState.Idle,
        endpointLabel = "Foreground service seam",
        lastHandshake = "Pending",
        transport = "Command / binder / socket ready",
    ),
    rustBridge = RustBridgeState(
        status = RustBridgeStatus.Planned,
        crateName = "CoreShift",
        executionPath = "rust/",
        notes = "JNI and FFI seams are reserved.",
    ),
    highlights = listOf(
        SystemHighlight("CPU Envelope", "Adaptive", "Mapped to policy selection."),
        SystemHighlight("Memory Guardrails", "Moderate", "Held in repository state."),
        SystemHighlight("Network Posture", "Measured", "Ready for daemon mediation."),
    ),
)
