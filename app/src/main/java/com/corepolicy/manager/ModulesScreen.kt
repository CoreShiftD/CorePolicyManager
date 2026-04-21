package com.corepolicy.manager

import androidx.compose.animation.AnimatedVisibility
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.ui.R
import com.corepolicy.manager.ui.theme.CorePolicyDesign
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun ModulesScreen(
    modules: List<ModuleStatus>,
    onToggle: (String, Boolean) -> Unit,
    onOpenLogs: () -> Unit,
    modifier: Modifier = Modifier
) {
    var expandedId by remember { mutableStateOf<String?>(null) }
    val spacing = CorePolicyDesign.spacing

    Column(
        modifier = modifier
            .windowInsetsPadding(WindowInsets.statusBars)
            .padding(top = spacing.sm)
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(spacing.lg)
    ) {
        PageHeader(
            eyebrow = "Modules",
            title = "Enforcement modules",
            subtitle = "Daemon capabilities grouped by health, actionability, and dependency risk."
        )

        FlowRow(
            horizontalArrangement = Arrangement.spacedBy(spacing.sm),
            verticalArrangement = Arrangement.spacedBy(spacing.sm)
        ) {
            OverviewInlineBadge("Installed", modules.size.toString(), ChipTone.INFO)
            OverviewInlineBadge("Enabled", modules.count { it.enabled }.toString(), ChipTone.ACTIVE)
            OverviewInlineBadge(
                "Needs review",
                modules.count { it.health == ModuleHealth.DEGRADED || it.health == ModuleHealth.CONFLICT }.toString(),
                ChipTone.WARNING
            )
        }

        if (modules.isEmpty()) {
            EmptyStateCard(
                title = "No modules installed",
                message = "Install a module to extend daemon behavior. Modules can add battery, preload, and process control rules.",
                iconRes = R.drawable.ic_network
            )
        } else {
            Column(verticalArrangement = Arrangement.spacedBy(spacing.sm)) {
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

        Spacer(Modifier.height(spacing.sm))
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
    val spacing = CorePolicyDesign.spacing
    val healthTone = when (module.health) {
        ModuleHealth.HEALTHY -> ChipTone.SUCCESS
        ModuleHealth.DISABLED -> ChipTone.NEUTRAL
        ModuleHealth.CONFLICT -> ChipTone.ERROR
        ModuleHealth.DEGRADED -> ChipTone.WARNING
    }
    SectionCard(onClick = onOpen) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(spacing.sm)
        ) {
            IconBadge(
                iconRes = R.drawable.ic_network,
                contentDescription = module.title,
                tone = if (module.enabled) ChipTone.ACTIVE else ChipTone.NEUTRAL,
                size = CorePolicyDesign.icons.xl
            )
            Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(spacing.nano)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(module.title, style = MaterialTheme.typography.titleLarge, color = palette.onSurface)
                    StatusChip(if (module.enabled) "Enabled" else "Disabled", if (module.enabled) ChipTone.ACTIVE else ChipTone.NEUTRAL)
                }
                Text(module.description, style = MaterialTheme.typography.bodySmall, color = palette.onSurfaceVariant)
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(spacing.xs)) {
            StatusChip(formatModuleHealthLabel(module.health), healthTone, leadingDot = true)
            SecondaryButton(
                text = if (module.enabled) "Disable" else "Enable",
                onClick = { onToggle(!module.enabled) }
            )
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
                    .clip(RoundedCornerShape(CorePolicyDesign.radii.md))
                    .background(palette.surfaceRaised)
                    .padding(spacing.sm),
                verticalArrangement = Arrangement.spacedBy(spacing.nano)
            ) {
                MetadataLine("Last action", module.lastAction)
                MetadataLine("Dependency", module.dependencyNote ?: "None")
                MetadataLine("Conflict", module.conflictNote ?: "No conflicts")
                MetadataLine("Settings", if (module.hasSettings) "Available" else "Not available")
                Spacer(Modifier.height(spacing.nano))
                Row(horizontalArrangement = Arrangement.spacedBy(spacing.sm)) {
                    SecondaryButton(text = "Open logs", onClick = onOpenLogs)
                }
            }
        }
    }
}
