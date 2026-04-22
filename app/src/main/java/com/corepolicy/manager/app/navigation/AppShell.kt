package com.corepolicy.manager.app.navigation

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.Modifier
import com.corepolicy.manager.core.data.CorePolicyContainer
import com.corepolicy.manager.core.designsystem.component.AppIcon
import com.corepolicy.manager.core.designsystem.component.AppScaffold
import com.corepolicy.manager.feature.daemon.DaemonRoute
import com.corepolicy.manager.feature.overview.OverviewRoute
import com.corepolicy.manager.feature.policies.PoliciesRoute
import com.corepolicy.manager.feature.settings.SettingsRoute

@Composable
fun AppShell(
    container: CorePolicyContainer,
) {
    val navigationState = rememberAppNavigationState()

    AppScaffold(
        bottomBar = {
            NavigationBar(
                containerColor = Color.Transparent,
            ) {
                AppDestination.entries.forEach { destination ->
                    NavigationBarItem(
                        selected = navigationState.currentDestination == destination,
                        onClick = { navigationState.navigateTo(destination) },
                        icon = {
                            AppIcon(
                                glyph = destination.icon,
                                contentDescription = destination.label,
                            )
                        },
                        label = { Text(text = destination.label) },
                    )
                }
            }
        },
    ) { paddingValues ->
        Box(modifier = Modifier.padding(paddingValues)) {
            when (navigationState.currentDestination) {
                AppDestination.Overview -> OverviewRoute(
                    observeOverview = container.observeOverview,
                )
                AppDestination.Daemon -> DaemonRoute(
                    observeDaemonState = container.observeDaemonState,
                )
                AppDestination.Policies -> PoliciesRoute(
                    observePolicies = container.observePolicies,
                    updatePolicyMode = container.updatePolicyMode,
                )
                AppDestination.Settings -> SettingsRoute(
                    observeSettings = container.observeSettings,
                    setDynamicColor = container.setDynamicColor,
                    setCompactDensity = container.setCompactDensity,
                    setConfirmBeforePolicyApply = container.setConfirmBeforePolicyApply,
                )
            }
        }
    }
}
