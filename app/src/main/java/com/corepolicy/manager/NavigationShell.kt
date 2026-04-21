package com.corepolicy.manager

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateDpAsState
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.ui.R
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
    val isRail = layout == NavigationShellLayout.NAV_RAIL
    val shellShape = RoundedCornerShape(if (isRail) 30.dp else 28.dp)
    if (isRail) {
        Column(
            modifier = modifier
                .padding(start = 12.dp, top = 16.dp, bottom = 16.dp)
                .width(108.dp)
                .shadow(18.dp, shellShape, clip = false)
                .clip(shellShape)
                .background(palette.surfaceContainerHigh)
                .border(1.dp, palette.divider, shellShape)
                .padding(horizontal = 10.dp, vertical = 16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            NavigationShellHeader(compact = false)
            AppSection.values().forEach { section ->
                PremiumNavItem(
                    section = section,
                    selected = section == selectedSection,
                    onClick = { onSectionSelected(section) },
                    layout = layout,
                    modifier = Modifier.fillMaxWidth()
                )
            }
            Box(
                modifier = Modifier
                    .padding(top = 6.dp)
                    .size(width = 28.dp, height = 3.dp)
                    .clip(RoundedCornerShape(50))
                    .background(palette.primary.copy(alpha = 0.35f))
            )
        }
    } else {
        Row(
            modifier = modifier
                .padding(horizontal = 16.dp, vertical = 12.dp)
                .fillMaxWidth()
                .shadow(
                    elevation = 18.dp,
                    shape = shellShape,
                    clip = false
                )
                .clip(shellShape)
                .background(palette.surfaceContainerHigh)
                .border(1.dp, palette.divider, shellShape)
                .padding(horizontal = 10.dp, vertical = 10.dp),
            horizontalArrangement = Arrangement.spacedBy(6.dp)
        ) {
            NavigationShellHeader(
                compact = true,
                modifier = Modifier.padding(start = 2.dp, end = 4.dp)
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
    if (compact) {
        Box(
            modifier = modifier
                .padding(vertical = 8.dp)
                .size(6.dp)
                .clip(RoundedCornerShape(50))
                .background(palette.primary)
        )
    } else {
        Column(
            modifier = modifier.padding(bottom = 6.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(4.dp)
        ) {
            Box(
                modifier = Modifier
                    .size(28.dp)
                    .clip(RoundedCornerShape(10.dp))
                    .background(palette.primaryContainer),
                contentAlignment = Alignment.Center
            ) {
                Text(
                    text = "C",
                    style = MaterialTheme.typography.labelLarge.copy(fontWeight = FontWeight.SemiBold),
                    color = palette.onPrimaryContainer
                )
            }
            Text(
                text = "CORE",
                style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.SemiBold),
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
    val tone = if (selected) ChipTone.ACTIVE else ChipTone.NEUTRAL
    val isRail = layout == NavigationShellLayout.NAV_RAIL
    val shape = RoundedCornerShape(if (isRail) 20.dp else 22.dp)
    val containerColor by animateColorAsState(
        targetValue = if (selected) palette.primaryContainer else palette.surfaceContainerHigh.copy(alpha = 0.55f),
        animationSpec = spring(),
        label = "navItemContainer"
    )
    val borderColor by animateColorAsState(
        targetValue = if (selected) palette.primary.copy(alpha = 0.18f) else palette.divider.copy(alpha = 0.45f),
        animationSpec = spring(),
        label = "navItemBorder"
    )
    val indicatorWidth by animateDpAsState(
        targetValue = if (isRail) 3.dp else if (selected) 18.dp else 10.dp,
        animationSpec = spring(),
        label = "navItemIndicatorWidth"
    )
    val indicatorHeight by animateDpAsState(
        targetValue = if (isRail) {
            if (selected) 18.dp else 10.dp
        } else {
            3.dp
        },
        animationSpec = spring(),
        label = "navItemIndicatorHeight"
    )
    Column(
        modifier = modifier
            .clip(shape)
            .background(containerColor)
            .border(width = 1.dp, color = borderColor, shape = shape)
            .clickable(onClick = onClick)
            .padding(horizontal = 6.dp, vertical = if (isRail) 12.dp else 10.dp)
            .semantics { contentDescription = section.title },
        verticalArrangement = Arrangement.spacedBy(6.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        IconBadge(
            iconRes = section.iconRes,
            contentDescription = null,
            tone = tone,
            size = 28.dp
        )
        Text(
            text = section.title,
            style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.SemiBold),
            color = if (selected) palette.onPrimaryContainer else palette.onSurfaceVariant,
            maxLines = 1
        )
        if (isRail) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.Center
            ) {
                Box(
                    modifier = Modifier
                        .size(width = indicatorWidth, height = indicatorHeight)
                        .clip(RoundedCornerShape(50))
                        .background(
                            if (selected) palette.primary else palette.onSurfaceVariant.copy(alpha = 0.12f)
                        )
                )
            }
        } else {
            Box(
                modifier = Modifier
                    .size(width = indicatorWidth, height = indicatorHeight)
                    .clip(RoundedCornerShape(50))
                    .background(
                        if (selected) palette.primary else palette.onSurfaceVariant.copy(alpha = 0.12f)
                    )
            )
        }
    }
}
