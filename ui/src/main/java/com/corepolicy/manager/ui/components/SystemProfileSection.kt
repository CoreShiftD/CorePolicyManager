package com.corepolicy.manager.ui.components

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
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
import com.corepolicy.manager.ui.R
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

enum class SystemProfile(val title: String, val description: String, val iconRes: Int) {
    PERFORMANCE("Performance", "Max speed", R.drawable.ic_performance),
    BALANCED("Balanced", "Adaptive", R.drawable.ic_balanced),
    EFFICIENCY("Efficiency", "Battery save", R.drawable.ic_efficiency)
}

private fun profileIdentityColors(profile: SystemProfile, palette: com.corepolicy.manager.ui.theme.CorePolicyPalette): Pair<androidx.compose.ui.graphics.Color, androidx.compose.ui.graphics.Color> =
    when (profile) {
        SystemProfile.PERFORMANCE -> palette.performanceContainer to palette.onPerformanceContainer
        SystemProfile.BALANCED -> palette.balancedContainer to palette.onBalancedContainer
        SystemProfile.EFFICIENCY -> palette.efficiencyContainer to palette.onEfficiencyContainer
    }


@Composable
fun SystemProfileSection(
    modifier: Modifier = Modifier,
    selectedProfile: SystemProfile,
    onProfileSelected: (SystemProfile) -> Unit
) {
    val palette = LocalCorePolicyPalette.current

    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            modifier = Modifier.padding(start = 4.dp)
        ) {
            Text(
                text = "SYSTEM PROFILE",
                style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.SemiBold),
                color = palette.primary
            )
        }

        DashboardPanel(
            modifier = Modifier.fillMaxWidth(),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 8.dp, vertical = 10.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp)
        ) {
            SystemProfile.values().forEach { profile ->
                ProfileItem(
                    profile = profile,
                    isSelected = profile == selectedProfile,
                    onClick = { onProfileSelected(profile) }
                )
            }
        }
    }
}

@Composable
fun ProfileItem(
    profile: SystemProfile,
    isSelected: Boolean,
    onClick: () -> Unit
) {
    val palette = LocalCorePolicyPalette.current

    val (profileContainer, profileOnContainer) = profileIdentityColors(profile, palette)
    DashboardInsetTile(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(16.dp))
            .background(if (isSelected) palette.selectedRowSurface else palette.rowSurface.copy(alpha = 0.82f))
            .clickable(onClick = onClick),
        contentPadding = androidx.compose.foundation.layout.PaddingValues(horizontal = 12.dp, vertical = 10.dp)
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Box(
                modifier = Modifier
                    .size(34.dp)
                    .background(if (isSelected) palette.selectedIconContainer else profileContainer, CircleShape),
                contentAlignment = Alignment.Center
            ) {
                Image(
                    painter = painterResource(id = profile.iconRes),
                    contentDescription = profile.title,
                    colorFilter = ColorFilter.tint(
                        if (isSelected) palette.onPrimaryContainer else profileOnContainer.copy(alpha = 0.88f)
                    ),
                    modifier = Modifier.size(20.dp)
                )
            }

            Spacer(modifier = Modifier.width(16.dp))

            Column(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(2.dp)
            ) {
                Text(
                    text = profile.title,
                    style = MaterialTheme.typography.titleLarge.copy(
                        fontWeight = if (isSelected) FontWeight.SemiBold else FontWeight.Medium
                    ),
                    color = if (isSelected) palette.onPrimaryContainer else palette.onSurface
                )
                Text(
                    text = profile.description,
                    style = MaterialTheme.typography.bodySmall,
                    color = if (isSelected) {
                        palette.onPrimaryContainer.copy(alpha = 0.8f)
                    } else {
                        palette.onSurfaceVariant.copy(alpha = 0.8f)
                    }
                )
            }

            if (isSelected) {
                Text(
                    text = "Active",
                    style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.SemiBold),
                    color = palette.onPrimaryContainer
                )
            }
        }
    }
}
