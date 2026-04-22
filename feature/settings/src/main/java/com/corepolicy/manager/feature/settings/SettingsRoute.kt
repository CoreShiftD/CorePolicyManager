package com.corepolicy.manager.feature.settings

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.core.designsystem.component.AppCard
import com.corepolicy.manager.core.designsystem.component.AppTopBar
import com.corepolicy.manager.core.designsystem.component.ScreenContainer
import com.corepolicy.manager.core.domain.usecase.ObserveSettingsUseCase
import com.corepolicy.manager.core.domain.usecase.SetCompactDensityUseCase
import com.corepolicy.manager.core.domain.usecase.SetConfirmBeforePolicyApplyUseCase
import com.corepolicy.manager.core.domain.usecase.SetDynamicColorUseCase
import com.corepolicy.manager.core.model.SettingsState
import kotlinx.coroutines.launch

@Composable
fun SettingsRoute(
    observeSettings: ObserveSettingsUseCase,
    setDynamicColor: SetDynamicColorUseCase,
    setCompactDensity: SetCompactDensityUseCase,
    setConfirmBeforePolicyApply: SetConfirmBeforePolicyApplyUseCase,
) {
    val state by observeSettings().collectAsState(
        initial = SettingsState(
            dynamicColor = false,
            compactDensity = false,
            confirmBeforePolicyApply = true,
        ),
    )
    val scope = rememberCoroutineScope()

    ScreenContainer {
        AppTopBar(
            title = "Settings",
            subtitle = "Global preferences stay in data contracts so the app shell remains thin.",
        )
        AppCard(
            title = "Interface",
            supportingText = "UI decisions are persisted centrally and can later be synced or migrated.",
        ) {
            SettingsToggle(
                title = "Dynamic Color",
                summary = "Keep theme tokens ready for device palette adoption later.",
                checked = state.dynamicColor,
                onCheckedChange = { enabled -> scope.launch { setDynamicColor(enabled) } },
            )
            Spacer(modifier = Modifier.height(12.dp))
            SettingsToggle(
                title = "Compact Density",
                summary = "Switch to the tighter spacing scale across all surfaces.",
                checked = state.compactDensity,
                onCheckedChange = { enabled -> scope.launch { setCompactDensity(enabled) } },
            )
            Spacer(modifier = Modifier.height(12.dp))
            SettingsToggle(
                title = "Confirm Policy Apply",
                summary = "Reserve a safety checkpoint before touching runtime policy adapters.",
                checked = state.confirmBeforePolicyApply,
                onCheckedChange = { enabled -> scope.launch { setConfirmBeforePolicyApply(enabled) } },
            )
        }
    }
}

@Composable
private fun SettingsToggle(
    title: String,
    summary: String,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Column(modifier = Modifier.weight(1f)) {
            Text(text = title, style = MaterialTheme.typography.titleMedium)
            Text(
                text = summary,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange,
        )
    }
}
