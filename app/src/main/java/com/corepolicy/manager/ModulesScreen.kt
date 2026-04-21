package com.corepolicy.manager

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
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
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.rotate
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.ui.R
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

@Composable
fun ModulesScreen(
    modules: List<ModuleStatus>,
    onToggle: (String, Boolean) -> Unit,
    onOpenLogs: () -> Unit,
    modifier: Modifier = Modifier
) {
    var expandedId by remember { mutableStateOf<String?>(null) }

    Column(
        modifier = modifier.verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(CorePolicyDimens.sectionGap)
    ) {
        if (modules.isEmpty()) {
            EmptyStateCard(
                title = "No modules installed",
                message = "Install a module to extend daemon behavior. Modules can add battery, preload, and process control rules.",
                iconRes = R.drawable.ic_network
            )
        } else {
            Column(verticalArrangement = Arrangement.spacedBy(CorePolicyDimens.cardGap)) {
                modules.forEach { module ->
                    ExpandableModuleCard(
                        module = module,
                        expanded = expandedId == module.id,
                        onToggle = { onToggle(module.id, it) },
                        onOpen = { expandedId = if (expandedId == module.id) null else module.id },
                        onOpenLogs = onOpenLogs
                    )
                }
            }
        }

        Spacer(Modifier.height(4.dp))
    }
}

@Composable
private fun ExpandableModuleCard(
    module: ModuleStatus,
    expanded: Boolean,
    onToggle: (Boolean) -> Unit,
    onOpen: () -> Unit,
    onOpenLogs: () -> Unit
) {
    val palette = LocalCorePolicyPalette.current
    val healthTone = when (module.health) {
        ModuleHealth.HEALTHY -> ChipTone.SUCCESS
        ModuleHealth.DISABLED -> ChipTone.NEUTRAL
        ModuleHealth.CONFLICT -> ChipTone.ERROR
        ModuleHealth.DEGRADED -> ChipTone.WARNING
    }
    val shape = RoundedCornerShape(CorePolicyDimens.cardRadius)
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(shape)
            .background(palette.surfaceContainer)
            .border(1.dp, palette.divider, shape)
            .clickable(onClick = onOpen)
            .padding(horizontal = 12.dp, vertical = 10.dp),
        verticalArrangement = Arrangement.spacedBy(6.dp)
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp)
        ) {
            IconBadge(
                iconRes = R.drawable.ic_network,
                contentDescription = module.title,
                tone = if (module.enabled) ChipTone.ACTIVE else ChipTone.NEUTRAL,
                size = 32.dp
            )
            Column(modifier = Modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(1.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(module.title, style = MaterialTheme.typography.titleSmall, color = palette.onSurface)
                    Switch(
                        checked = module.enabled,
                        onCheckedChange = onToggle,
                        colors = SwitchDefaults.colors(
                            checkedThumbColor = palette.onPrimaryContainer,
                            checkedTrackColor = palette.primary,
                            uncheckedThumbColor = palette.onSurfaceVariant,
                            uncheckedTrackColor = palette.surfaceContainerHigh,
                            uncheckedBorderColor = palette.divider
                        )
                    )
                }
                Text(module.description, style = MaterialTheme.typography.bodySmall, color = palette.onSurfaceVariant)
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(5.dp)) {
            StatusChip(formatModuleHealthLabel(module.health), healthTone, leadingDot = true)
            if (module.enabled) StatusChip("Active", ChipTone.ACTIVE) else StatusChip("Disabled", ChipTone.NEUTRAL)
        }
        module.dependencyNote?.let {
            MetadataLine("Dependency", it)
        }
        AnimatedVisibility(
            visible = expanded,
            enter = fadeIn(tween(200)) + expandVertically(tween(200)),
            exit = fadeOut(tween(160)) + shrinkVertically(tween(160))
        ) {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(12.dp))
                    .background(palette.surfaceContainerHigh)
                    .padding(10.dp),
                verticalArrangement = Arrangement.spacedBy(3.dp)
            ) {
                MetadataLine("Last action", module.lastAction)
                MetadataLine("Dependency", module.dependencyNote ?: "None")
                MetadataLine("Conflict", module.conflictNote ?: "No conflicts")
                MetadataLine("Settings", if (module.hasSettings) "Available" else "Not available")
                Spacer(Modifier.height(2.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    StatusChip("Open logs", ChipTone.INFO, modifier = Modifier.clickable(onClick = onOpenLogs))
                }
            }
        }
    }
}
