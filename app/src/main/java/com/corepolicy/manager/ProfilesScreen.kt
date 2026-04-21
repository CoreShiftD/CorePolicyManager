package com.corepolicy.manager

import androidx.compose.foundation.clickable
import androidx.compose.foundation.background
import androidx.compose.foundation.Image
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBars
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.ui.components.SystemProfile
import com.corepolicy.manager.ui.theme.CorePolicyDesign
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun ProfilesScreen(
    selectedProfile: SystemProfile,
    onSelect: (SystemProfile) -> Unit,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    val spacing = CorePolicyDesign.spacing
    Column(
        modifier = modifier
            .windowInsetsPadding(WindowInsets.statusBars)
            .padding(top = spacing.sm)
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(spacing.lg)
    ) {
        PageHeader(
            eyebrow = "Profiles",
            title = "Runtime posture",
            subtitle = "Swap daemon bias instantly based on workload, thermals, and battery goals."
        )

        FlowRow(
            horizontalArrangement = Arrangement.spacedBy(spacing.sm),
            verticalArrangement = Arrangement.spacedBy(spacing.sm)
        ) {
            OverviewInlineBadge("Active", selectedProfile.title, ChipTone.ACTIVE)
            OverviewInlineBadge("Mode", selectedProfile.description, ChipTone.INFO)
        }

        SectionCard(elevated = true) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(spacing.sm),
                verticalAlignment = Alignment.Top
            ) {
                IconBadge(
                    iconRes = selectedProfile.iconRes,
                    contentDescription = null,
                    tone = ChipTone.INFO,
                    size = CorePolicyDesign.icons.xl
                )
                Column(verticalArrangement = Arrangement.spacedBy(spacing.nano)) {
                    Text(
                        "Current profile: ${selectedProfile.title}",
                        style = MaterialTheme.typography.headlineSmall,
                        color = palette.onSurface
                    )
                    Text(
                        profileHint(selectedProfile),
                        style = MaterialTheme.typography.bodySmall,
                        color = palette.onSurfaceVariant
                    )
                }
            }
        }

        Column(verticalArrangement = Arrangement.spacedBy(spacing.sm)) {
            SystemProfile.values().forEach { profile ->
                ProfileRow(
                    profile = profile,
                    selected = profile == selectedProfile,
                    onClick = { onSelect(profile) }
                )
            }
        }
        Spacer(Modifier.height(spacing.sm))
    }
}

private fun profileHint(profile: SystemProfile): String = when (profile) {
    SystemProfile.PERFORMANCE -> "Daemon is biasing CPU governor toward higher frequencies. Expect increased power draw and heat."
    SystemProfile.BALANCED -> "Daemon is using adaptive scaling. Recommended for daily use — good balance of speed and battery life."
    SystemProfile.EFFICIENCY -> "Daemon is restricting background activity and lowering CPU ceilings. Best for extended battery life."
}

@Composable
private fun ProfileRow(
    profile: SystemProfile,
    selected: Boolean,
    onClick: () -> Unit
) {
    val palette = LocalCorePolicyPalette.current
    val spacing = CorePolicyDesign.spacing

    val summary = when (profile) {
        SystemProfile.PERFORMANCE -> "Max speed · CPU biased high · aggressive preload"
        SystemProfile.BALANCED -> "Adaptive scaling · safe thermals"
        SystemProfile.EFFICIENCY -> "Battery-first · low background load"
    }

    SectionCard(
        elevated = selected,
        onClick = onClick
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(spacing.sm)
        ) {
            Box(
                modifier = Modifier
                    .size(46.dp)
                    .clip(CircleShape)
                    .background(
                        if (selected) palette.onPrimaryContainer.copy(alpha = 0.15f)
                        else palette.surfaceRaised
                    ),
                contentAlignment = Alignment.Center
            ) {
                Image(
                    painter = painterResource(id = profile.iconRes),
                    contentDescription = profile.title,
                    modifier = Modifier.size(CorePolicyDesign.icons.lg),
                    colorFilter = ColorFilter.tint(
                        if (selected) palette.onPrimaryContainer else palette.onSurfaceVariant
                    )
                )
            }
            Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(spacing.nano)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        profile.title,
                        style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.SemiBold),
                        color = if (selected) palette.onPrimaryContainer else palette.onSurface
                    )
                    if (selected) {
                        StatusChip(
                            text = if (profile == SystemProfile.BALANCED) "Active · Recommended" else "Active",
                            tone = ChipTone.ACTIVE,
                            leadingDot = true
                        )
                    }
                }
                Text(
                    summary,
                    style = MaterialTheme.typography.bodySmall,
                    color = if (selected) palette.onPrimaryContainer.copy(alpha = 0.8f) else palette.onSurfaceVariant
                )
            }
        }
    }
}
