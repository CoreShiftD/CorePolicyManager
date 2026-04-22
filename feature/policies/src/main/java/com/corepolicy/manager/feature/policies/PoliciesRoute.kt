package com.corepolicy.manager.feature.policies

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.Spacer
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.core.designsystem.component.AppCard
import com.corepolicy.manager.core.designsystem.component.AppPrimaryButton
import com.corepolicy.manager.core.designsystem.component.AppTopBar
import com.corepolicy.manager.core.designsystem.component.ScreenContainer
import com.corepolicy.manager.core.domain.usecase.ObservePoliciesUseCase
import com.corepolicy.manager.core.domain.usecase.UpdatePolicyModeUseCase
import com.corepolicy.manager.core.model.PoliciesState
import com.corepolicy.manager.core.model.PolicyMode
import com.corepolicy.manager.core.model.PolicyProfile
import kotlinx.coroutines.launch

@Composable
fun PoliciesRoute(
    observePolicies: ObservePoliciesUseCase,
    updatePolicyMode: UpdatePolicyModeUseCase,
) {
    val state by observePolicies().collectAsState(initial = policiesPreview())
    val scope = rememberCoroutineScope()

    ScreenContainer {
        AppTopBar(
            title = "Policies",
            subtitle = "Policy definitions stay explicit so future rule engines can slot in without UI churn.",
        )
        state.availableProfiles.forEach { profile ->
            AppCard(
                title = profile.title,
                supportingText = profile.summary,
            ) {
                Column(
                    modifier = Modifier.fillMaxWidth(),
                    verticalArrangement = Arrangement.spacedBy(12.dp),
                ) {
                    FilterChip(
                        selected = state.selectedMode == profile.mode,
                        onClick = {
                            scope.launch { updatePolicyMode(profile.mode) }
                        },
                        label = { Text(text = profile.mode.name) },
                        colors = FilterChipDefaults.filterChipColors(
                            selectedContainerColor = MaterialTheme.colorScheme.secondary.copy(alpha = 0.18f),
                        ),
                    )
                    Text(
                        text = "CPU ${profile.cpuBudget} • Memory ${profile.memoryBudget} • Network ${profile.networkBudget}",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
        Spacer(modifier = Modifier.height(4.dp))
        AppPrimaryButton(
            text = "Apply ${state.selectedMode.name}",
            onClick = {
                scope.launch { updatePolicyMode(state.selectedMode) }
            },
        )
    }
}

private fun policiesPreview() = PoliciesState(
    availableProfiles = PolicyMode.entries.map { mode ->
        PolicyProfile(
            mode = mode,
            title = mode.name,
            summary = "Profile placeholder reserved for policy contracts.",
            cpuBudget = "Adaptive",
            memoryBudget = "Moderate",
            networkBudget = "Measured",
        )
    },
    selectedMode = PolicyMode.Balanced,
)
