package com.corepolicy.manager

import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import com.corepolicy.manager.app.navigation.AppShell
import com.corepolicy.manager.core.data.CorePolicyContainer
import com.corepolicy.manager.core.designsystem.theme.AppTheme
import com.corepolicy.manager.core.model.SettingsState

@Composable
fun CorePolicyApp(
    container: CorePolicyContainer,
) {
    val settings by container.observeSettings().collectAsState(
        initial = SettingsState(
            dynamicColor = false,
            compactDensity = false,
            confirmBeforePolicyApply = true,
        ),
    )

    AppTheme(compactMode = settings.compactDensity) {
        AppShell(container = container)
    }
}
