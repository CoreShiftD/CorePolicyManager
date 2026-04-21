package com.corepolicy.manager

import androidx.compose.animation.animateColorAsState
import androidx.compose.runtime.getValue
import androidx.compose.animation.core.spring
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.Alignment
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.ui.R
import com.corepolicy.manager.ui.theme.CorePolicyDesign
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

enum class AppSection(val title: String, val iconRes: Int) {
    OVERVIEW("Overview", R.drawable.ic_info),
    MODULES("Modules", R.drawable.ic_network),
    APP_MANAGER("Apps", R.drawable.ic_cpu),
    PROFILES("Profiles", R.drawable.ic_balanced),
    LOGS("Logs", R.drawable.ic_schedule)
}

enum class NavigationShellLayout { BOTTOM_BAR, NAV_RAIL }

@Composable
fun NavigationShell(
    selectedSection: AppSection,
    onSectionSelected: (AppSection) -> Unit,
    layout: NavigationShellLayout = NavigationShellLayout.BOTTOM_BAR,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    val spacing = CorePolicyDesign.spacing
    val radii = CorePolicyDesign.radii
    val isRail = layout == NavigationShellLayout.NAV_RAIL
    if (isRail) {
        Column(
            modifier = modifier
                .width(96.dp)
                .shadow(CorePolicyDesign.elevation.medium, RoundedCornerShape(radii.xl), clip = false)
                .clip(RoundedCornerShape(radii.xl))
                .background(palette.surfaceRaised.copy(alpha = 0.94f))
                .border(1.dp, palette.divider, RoundedCornerShape(radii.xl))
                .padding(horizontal = spacing.xs, vertical = spacing.lg),
            verticalArrangement = Arrangement.spacedBy(spacing.xs),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            NavigationShellHeader(compact = false, modifier = Modifier.padding(bottom = spacing.sm))
            AppSection.values().forEach { section ->
                PremiumNavItem(
                    section = section,
                    selected = section == selectedSection,
                    onClick = { onSectionSelected(section) },
                    layout = layout,
                    modifier = Modifier.fillMaxWidth()
                )
            }
        }
    } else {
        Row(
            modifier = modifier
                .fillMaxWidth()
                .shadow(
                    elevation = CorePolicyDesign.elevation.medium,
                    shape = RoundedCornerShape(radii.lg),
                    clip = false
                )
                .clip(RoundedCornerShape(radii.lg))
                .background(palette.surfaceRaised.copy(alpha = 0.96f))
                .border(1.dp, palette.divider, RoundedCornerShape(radii.lg))
                .padding(horizontal = spacing.xs, vertical = spacing.xs),
            horizontalArrangement = Arrangement.spacedBy(spacing.nano)
        ) {
            NavigationShellHeader(
                compact = true,
                modifier = Modifier.padding(horizontal = spacing.nano, vertical = spacing.sm)
            )
            AppSection.values().forEach { section ->
                PremiumNavItem(
                    section = section,
                    selected = section == selectedSection,
                    onClick = { onSectionSelected(section) },
                    layout = layout,
                    modifier = Modifier.weight(1f)
                )
            }
        }
    }
}

@Composable
private fun NavigationShellHeader(
    compact: Boolean,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    val radii = CorePolicyDesign.radii
    if (compact) {
        Box(
            modifier = modifier
                .size(width = 5.dp, height = 28.dp)
                .clip(RoundedCornerShape(radii.full))
                .background(palette.primary)
        )
    } else {
        Column(
            modifier = modifier,
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(CorePolicyDesign.spacing.xs)
        ) {
            Box(
                modifier = Modifier
                    .size(CorePolicyDesign.icons.xl)
                    .clip(RoundedCornerShape(radii.md))
                    .background(palette.primaryContainer),
                contentAlignment = Alignment.Center
            ) {
                Text(
                    text = "C",
                    style = MaterialTheme.typography.titleSmall,
                    color = palette.onPrimaryContainer
                )
            }
            Text(
                text = "CTRL",
                style = MaterialTheme.typography.labelSmall,
                color = palette.primary
            )
        }
    }
}

@Composable
private fun PremiumNavItem(
    section: AppSection,
    selected: Boolean,
    onClick: () -> Unit,
    layout: NavigationShellLayout,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    val spacing = CorePolicyDesign.spacing
    val radii = CorePolicyDesign.radii
    val isRail = layout == NavigationShellLayout.NAV_RAIL
    val containerColor by animateColorAsState(
        targetValue = if (selected) palette.primaryContainer else Color.Transparent,
        animationSpec = spring(),
        label = "navItemContainer"
    )
    val borderColor by animateColorAsState(
        targetValue = if (selected) palette.primary.copy(alpha = 0.16f) else Color.Transparent,
        animationSpec = spring(),
        label = "navItemBorder"
    )
    Column(
        modifier = modifier
            .clip(RoundedCornerShape(radii.md))
            .background(containerColor)
            .border(width = 1.dp, color = borderColor, shape = RoundedCornerShape(radii.md))
            .clickable(onClick = onClick)
            .padding(horizontal = spacing.xs, vertical = if (isRail) spacing.sm else spacing.xs)
            .semantics { contentDescription = section.title },
        verticalArrangement = Arrangement.spacedBy(spacing.nano),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        IconBadge(
            iconRes = section.iconRes,
            contentDescription = null,
            tone = if (selected) ChipTone.ACTIVE else ChipTone.NEUTRAL,
            size = if (isRail) CorePolicyDesign.icons.lg else CorePolicyDesign.icons.md
        )
        Text(
            text = section.title,
            style = MaterialTheme.typography.labelSmall,
            color = if (selected) palette.onPrimaryContainer else palette.onSurfaceVariant,
            maxLines = 1
        )
        Box(
            modifier = Modifier
                .size(width = if (selected) 18.dp else 8.dp, height = 2.dp)
                .clip(RoundedCornerShape(radii.full))
                .background(
                    if (selected) palette.primary else palette.onSurfaceVariant.copy(alpha = 0.12f)
                )
        )
    }
}
