package com.corepolicy.manager

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.ui.components.SystemProfile
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

@Composable
fun ProfilesScreen(
    selectedProfile: SystemProfile,
    onSelect: (SystemProfile) -> Unit,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    Column(
        modifier = modifier.verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(CorePolicyDimens.sectionGap)
    ) {
        // Page header
        Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Text(
                "Profiles",
                style = MaterialTheme.typography.headlineLarge,
                color = palette.onSurface
            )
            Text(
                "Tune daemon behavior by workload target.",
                style = MaterialTheme.typography.bodyMedium,
                color = palette.onSurfaceVariant
            )
        }

        // Profile cards
        Column(verticalArrangement = Arrangement.spacedBy(CorePolicyDimens.cardGap)) {
            SystemProfile.values().forEach { profile ->
                ProfileRow(
                    profile = profile,
                    selected = profile == selectedProfile,
                    onClick = { onSelect(profile) }
                )
            }
        }

        // Contextual hint
        SectionCard {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalAlignment = Alignment.Top
            ) {
                IconBadge(
                    iconRes = selectedProfile.iconRes,
                    contentDescription = null,
                    tone = ChipTone.INFO,
                    size = 32.dp
                )
                Column(verticalArrangement = Arrangement.spacedBy(3.dp)) {
                    Text(
                        "Active: ${selectedProfile.title}",
                        style = MaterialTheme.typography.titleSmall,
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

        Spacer(Modifier.height(4.dp))
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
    val shape = RoundedCornerShape(CorePolicyDimens.cardRadius)

    val summary = when (profile) {
        SystemProfile.PERFORMANCE -> "Max speed · CPU biased high · aggressive preload"
        SystemProfile.BALANCED -> "Adaptive scaling · safe thermals"
        SystemProfile.EFFICIENCY -> "Battery-first · low background load"
    }

    // Animate background and border color transitions
    val bgColor by animateColorAsState(
        targetValue = if (selected) palette.primaryContainer else palette.surfaceContainer,
        animationSpec = spring(stiffness = Spring.StiffnessMediumLow),
        label = "profileRowBg"
    )
    val borderColor by animateColorAsState(
        targetValue = if (selected) palette.primary.copy(alpha = 0.5f) else palette.divider,
        animationSpec = spring(stiffness = Spring.StiffnessMediumLow),
        label = "profileRowBorder"
    )
    val textColor by animateColorAsState(
        targetValue = if (selected) palette.onPrimaryContainer else palette.onSurface,
        animationSpec = spring(stiffness = Spring.StiffnessMediumLow),
        label = "profileRowText"
    )
    val subtextColor by animateColorAsState(
        targetValue = if (selected) palette.onPrimaryContainer.copy(alpha = 0.8f) else palette.onSurfaceVariant,
        animationSpec = spring(stiffness = Spring.StiffnessMediumLow),
        label = "profileRowSubtext"
    )

    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(shape)
            .background(bgColor)
            .border(1.dp, borderColor, shape)
            .clickable(onClick = onClick)
    ) {
        Row(
            modifier = Modifier.padding(
                horizontal = CorePolicyDimens.cardPaddingH,
                vertical = CorePolicyDimens.cardPaddingV
            ),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(14.dp)
        ) {
            Box(
                modifier = Modifier
                    .size(46.dp)
                    .clip(CircleShape)
                    .background(
                        if (selected) palette.onPrimaryContainer.copy(alpha = 0.15f)
                        else palette.surfaceContainerHigh
                    ),
                contentAlignment = Alignment.Center
            ) {
                Image(
                    painter = painterResource(id = profile.iconRes),
                    contentDescription = profile.title,
                    modifier = Modifier.size(24.dp),
                    colorFilter = ColorFilter.tint(
                        if (selected) palette.onPrimaryContainer else palette.onSurfaceVariant
                    )
                )
            }
            Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(3.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        profile.title,
                        style = MaterialTheme.typography.titleMedium.copy(fontWeight = FontWeight.SemiBold),
                        color = textColor
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
                    color = subtextColor
                )
            }
        }

        // Bottom accent strip for selected profile
        if (selected) {
            HorizontalDivider(
                color = palette.primary.copy(alpha = 0.35f),
                thickness = 2.dp
            )
        }
    }
}
