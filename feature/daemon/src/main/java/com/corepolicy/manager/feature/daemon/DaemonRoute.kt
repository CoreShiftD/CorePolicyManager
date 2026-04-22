package com.corepolicy.manager.feature.daemon

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
import com.corepolicy.manager.core.domain.usecase.ObserveDaemonStateUseCase
import com.corepolicy.manager.core.model.DaemonRunState
import com.corepolicy.manager.core.model.DaemonState

@Composable
fun DaemonRoute(
    observeDaemonState: ObserveDaemonStateUseCase,
) {
    val state by observeDaemonState().collectAsState(
        initial = DaemonState(
            runState = DaemonRunState.Idle,
            endpointLabel = "Daemon seam not bound yet",
            lastHandshake = "Pending",
            transport = "Command / binder / socket ready",
        ),
    )

    ScreenContainer {
        AppTopBar(
            title = "Daemon",
            subtitle = "Reserved for long-running service, IPC, or external process coordination.",
        )
        AppCard(
            title = "Transport Status",
            supportingText = "These fields stay compile-safe while the real transport is still being selected.",
        ) {
            AppListItem(
                label = "Run State",
                value = state.runState.name,
                supportingText = state.lastHandshake,
            )
            Spacer(modifier = Modifier.height(12.dp))
            AppListItem(
                label = "Endpoint",
                value = state.endpointLabel,
                supportingText = state.transport,
            )
        }
    }
}
