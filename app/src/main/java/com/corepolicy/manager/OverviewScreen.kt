package com.corepolicy.manager

import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
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
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.ui.R
import com.corepolicy.manager.ui.components.DynamicMetric
import com.corepolicy.manager.ui.components.InfoCardGrid
import com.corepolicy.manager.ui.components.InsightItem
import com.corepolicy.manager.ui.components.InsightsSection
import com.corepolicy.manager.ui.components.StaticSystemInfoSection
import com.corepolicy.manager.ui.components.SystemProfile
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

@Composable
fun OverviewScreen(
    metrics: List<DynamicMetric>,
    insights: List<InsightItem>,
    systemInfo: Triple<String, String, String>,
    runtimeInfo: Triple<String, String, String>,
    selectedProfile: SystemProfile,
    daemonStatus: DaemonOverviewStatus,
    managedAppsCount: Int,
    onProfileClick: () -> Unit,
    onRestartDaemon: () -> Unit,
    onOpenLogs: () -> Unit,
    onManageModules: () -> Unit,
    modifier: Modifier = Modifier
) {
    val scroll = rememberScrollState()
    Column(
        modifier = modifier.verticalScroll(scroll),
        verticalArrangement = Arrangement.spacedBy(CorePolicyDimens.sectionGap)
    ) {
        OverviewHeroHeader(
            daemonStatus = daemonStatus,
            managedAppsCount = managedAppsCount
        )

        DaemonHeroCard(
            status = daemonStatus,
            managedAppsCount = managedAppsCount,
            onRestartDaemon = onRestartDaemon,
            onOpenLogs = onOpenLogs,
            onManageModules = onManageModules
        )

        OverviewMetricsSection(
            metrics = metrics,
            daemonStatus = daemonStatus
        )

        Column(verticalArrangement = Arrangement.spacedBy(CorePolicyDimens.cardGap)) {
            SectionHeader(
                title = "Policy posture",
                subtitle = "Current profile ownership and operational health"
            )
            ActiveProfileCard(
                selectedProfile = selectedProfile,
                warningCount = daemonStatus.warningCount,
                managedAppsCount = managedAppsCount,
                onClick = onProfileClick
            )
        }

        Column(verticalArrangement = Arrangement.spacedBy(CorePolicyDimens.cardGap)) {
            SectionHeader(
                title = "Insights",
                subtitle = "Daemon behavior and policy health",
                trailing = {
                    StatusChip(
                        text = "${insights.size} signals",
                        tone = ChipTone.INFO
                    )
                }
            )
            InsightsSection(insights = insights)
        }

        Column(verticalArrangement = Arrangement.spacedBy(CorePolicyDimens.cardGap)) {
            SectionHeader(
                title = "Device info",
                subtitle = "Technical diagnostics and platform context"
            )
            StaticSystemInfoSection(systemInfo = systemInfo, runtimeInfo = runtimeInfo)
        }

        Spacer(Modifier.height(4.dp))
    }
}

@Composable
private fun OverviewHeroHeader(
    daemonStatus: DaemonOverviewStatus,
    managedAppsCount: Int
) {
    Column(verticalArrangement = Arrangement.spacedBy(14.dp)) {
        PageHeader(
            eyebrow = "CorePolicy Manager",
            title = "Control Center",
            subtitle = when {
                daemonStatus.disconnected -> "Daemon communication is offline. The last known policy state is still visible."
                daemonStatus.restartInProgress -> "Daemon restart is in flight. Control surfaces remain available while policy sync recovers."
                daemonStatus.warningCount > 0 -> "Policy control remains available with active warnings that need attention."
                else -> "A live operational view of policy health, modules, and managed application coverage."
            },
            trailing = {
                StatusChip(
                    text = overviewStatusText(daemonStatus),
                    tone = overviewStatusTone(daemonStatus),
                    leadingDot = !daemonStatus.disconnected
                )
            }
        )

        OverviewSignalRow(
            managedAppsCount = managedAppsCount,
            modulesCount = daemonStatus.enabledModules,
            warningCount = daemonStatus.warningCount
        )
    }
}

@Composable
private fun DaemonHeroCard(
    status: DaemonOverviewStatus,
    managedAppsCount: Int,
    onRestartDaemon: () -> Unit,
    onOpenLogs: () -> Unit,
    onManageModules: () -> Unit
) {
    val palette = LocalCorePolicyPalette.current
    val stateLabel = when (status.state) {
        DaemonState.RUNNING -> "Running"
        DaemonState.STOPPED -> "Stopped"
        DaemonState.DEGRADED -> "Degraded"
    }
    val summaryLine = when {
        status.disconnected -> "Reconnect the daemon service to resume policy synchronization and action feedback."
        status.restartInProgress -> "Restart in progress. Existing controls remain visible while the daemon comes back."
        status.state == DaemonState.DEGRADED -> "The daemon is live, but one or more modules need review."
        else -> "The daemon is healthy and actively enforcing the selected device profile."
    }

    SectionCard(elevated = true) {
        Text(
            text = "Operational status",
            style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.SemiBold),
            color = palette.primary
        )
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Row(
                modifier = Modifier.weight(1f),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                IconBadge(
                    iconRes = R.drawable.ic_cpu,
                    contentDescription = "Daemon",
                    tone = ChipTone.ACTIVE,
                    size = 34.dp
                )
                Column(verticalArrangement = Arrangement.spacedBy(1.dp)) {
                    Text(
                        "CorePolicy Daemon",
                        style = MaterialTheme.typography.titleSmall,
                        color = palette.onSurface
                    )
                    Text(
                        summaryLine,
                        style = MaterialTheme.typography.bodySmall,
                        color = palette.onSurfaceVariant
                    )
                }
            }
            StatusChip(
                text = if (status.disconnected) "Offline" else if (status.restartInProgress) "Restarting" else stateLabel,
                tone = overviewStatusTone(status),
                leadingDot = !status.disconnected
            )
        }

        Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                DaemonMetaItem(label = "Profile", value = status.activeProfile.title, modifier = Modifier.weight(1f))
                DaemonMetaItem(label = "Uptime", value = formatDuration(status.uptimeMs), modifier = Modifier.weight(1f))
            }
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                DaemonMetaItem(label = "Last sync", value = formatRelativeTime(status.lastSyncTimestampMs), modifier = Modifier.weight(1f))
                DaemonMetaItem(label = "Managed apps", value = managedAppsCount.toString(), modifier = Modifier.weight(1f))
            }
        }

        HorizontalDivider(color = palette.divider, thickness = 1.dp)

        OverviewSummaryRow(
            modulesActive = status.enabledModules,
            managedApps = managedAppsCount,
            currentProfile = status.activeProfile.title,
            warnings = status.warningCount
        )

        OverviewOperationalBanner(status = status)

        HorizontalDivider(color = palette.divider, thickness = 1.dp)

        Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
            ActionTile(
                iconRes = R.drawable.ic_performance,
                label = "Restart",
                onClick = onRestartDaemon,
                modifier = Modifier.weight(1f)
            )
            ActionTile(
                iconRes = R.drawable.ic_schedule,
                label = "Logs",
                onClick = onOpenLogs,
                modifier = Modifier.weight(1f)
            )
            ActionTile(
                iconRes = R.drawable.ic_network,
                label = "Modules",
                onClick = onManageModules,
                modifier = Modifier.weight(1f)
            )
        }
    }
}

@Composable
private fun OverviewMetricsSection(
    metrics: List<DynamicMetric>,
    daemonStatus: DaemonOverviewStatus
) {
    Column(verticalArrangement = Arrangement.spacedBy(CorePolicyDimens.cardGap)) {
        SectionHeader(
            title = "System snapshot",
            subtitle = "Live runtime telemetry across device and daemon surfaces",
            trailing = {
                StatusChip(
                    text = if (daemonStatus.disconnected) "Paused" else "Live",
                    tone = if (daemonStatus.disconnected) ChipTone.ERROR else ChipTone.ACTIVE,
                    leadingDot = !daemonStatus.disconnected
                )
            }
        )
        InfoCardGrid(metrics = metrics)
    }
}

@Composable
private fun OverviewOperationalBanner(status: DaemonOverviewStatus) {
    val title: String
    val message: String
    val tone: ChipTone
    when {
        status.disconnected -> {
            title = "Daemon unreachable"
            message = "Controls remain visible, but policy application feedback is temporarily unavailable."
            tone = ChipTone.ERROR
        }
        status.restartInProgress -> {
            title = "Restart underway"
            message = "The daemon is cycling now. Recent actions may briefly report as pending."
            tone = ChipTone.WARNING
        }
        status.warningCount > 0 || status.errorCount > 0 -> {
            title = "Attention needed"
            message = status.lastAction.ifBlank { "Review warnings and recent daemon activity for the current profile." }
            tone = if (status.errorCount > 0) ChipTone.ERROR else ChipTone.WARNING
        }
        else -> {
            title = "Last action"
            message = status.lastAction.ifBlank { "Daemon synchronization is current and no corrective action is required." }
            tone = ChipTone.INFO
        }
    }
    ErrorBanner(title = title, message = message, tone = tone)
}

@Composable
private fun DaemonMetaItem(label: String, value: String, modifier: Modifier = Modifier) {
    val palette = LocalCorePolicyPalette.current
    Row(
        modifier = modifier,
        horizontalArrangement = Arrangement.spacedBy(6.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Text(text = label, style = MaterialTheme.typography.labelSmall, color = palette.onSurfaceVariant)
        Text(
            text = value,
            style = MaterialTheme.typography.bodySmall.copy(fontWeight = FontWeight.SemiBold),
            color = palette.onSurface
        )
    }
}

@Composable
private fun OverviewSummaryRow(
    modulesActive: Int,
    managedApps: Int,
    currentProfile: String,
    warnings: Int
) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            AnimatedStatPill(
                label = "Modules",
                value = modulesActive.toString(),
                modifier = Modifier.weight(1f),
                tone = ChipTone.ACTIVE
            )
            AnimatedStatPill(
                label = "Managed apps",
                value = managedApps.toString(),
                modifier = Modifier.weight(1f),
                tone = ChipTone.INFO
            )
        }
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            AnimatedStatPill(
                label = "Profile",
                value = currentProfile,
                modifier = Modifier.weight(1f),
                tone = ChipTone.ACTIVE
            )
            AnimatedStatPill(
                label = "Warnings",
                value = warnings.toString(),
                tone = if (warnings > 0) ChipTone.WARNING else ChipTone.NEUTRAL,
                modifier = Modifier.weight(1f)
            )
        }
    }
}

@Composable
private fun OverviewSignalRow(
    managedAppsCount: Int,
    modulesCount: Int,
    warningCount: Int
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        OverviewInlineBadge(
            label = "Modules",
            value = modulesCount.toString(),
            tone = ChipTone.ACTIVE,
            modifier = Modifier.weight(1f)
        )
        OverviewInlineBadge(
            label = "Managed apps",
            value = managedAppsCount.toString(),
            tone = ChipTone.INFO,
            modifier = Modifier.weight(1f)
        )
        OverviewInlineBadge(
            label = "Warnings",
            value = warningCount.toString(),
            tone = if (warningCount > 0) ChipTone.WARNING else ChipTone.NEUTRAL,
            modifier = Modifier.weight(1f)
        )
    }
}

@Composable
private fun ActiveProfileCard(
    selectedProfile: SystemProfile,
    warningCount: Int,
    managedAppsCount: Int,
    onClick: () -> Unit
) {
    val palette = LocalCorePolicyPalette.current
    SectionCard(onClick = onClick) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Row(
                modifier = Modifier.weight(1f),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp)
            ) {
                IconBadge(
                    iconRes = selectedProfile.iconRes,
                    contentDescription = selectedProfile.title,
                    tone = ChipTone.ACTIVE,
                    size = 40.dp
                )
                Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(
                        "Active profile",
                        style = MaterialTheme.typography.labelMedium,
                        color = palette.onSurfaceVariant
                    )
                    Text(
                        selectedProfile.title,
                        style = MaterialTheme.typography.titleMedium,
                        color = palette.onSurface
                    )
                    Text(
                        "$managedAppsCount managed apps · $warningCount warnings in current posture",
                        style = MaterialTheme.typography.bodySmall,
                        color = palette.onSurfaceVariant
                    )
                }
            }
            Box(
                modifier = Modifier
                    .clip(RoundedCornerShape(CorePolicyDimens.chipRadius))
                    .background(palette.primaryContainer)
                    .padding(horizontal = 14.dp, vertical = 6.dp)
            ) {
                Text(
                    "Change",
                    style = MaterialTheme.typography.labelLarge.copy(fontWeight = FontWeight.SemiBold),
                    color = palette.onPrimaryContainer
                )
            }
        }
    }
}

@Composable
fun OverviewInlineBadge(
    label: String,
    value: String,
    tone: ChipTone,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    val dotColor = accentForegroundFor(tone).takeIf { tone != ChipTone.NEUTRAL } ?: palette.onSurfaceVariant
    val infiniteTransition = rememberInfiniteTransition(label = "overviewInlineBadge")
    val dotAlpha by infiniteTransition.animateFloat(
        initialValue = 1f,
        targetValue = if (tone == ChipTone.ACTIVE) 0.45f else 1f,
        animationSpec = infiniteRepeatable(
            animation = tween(1000),
            repeatMode = RepeatMode.Reverse
        ),
        label = "overviewInlineBadgeAlpha"
    )
    Row(
        modifier = modifier
            .clip(RoundedCornerShape(CorePolicyDimens.cardRadiusTight))
            .background(accentBackgroundFor(tone))
            .padding(horizontal = 10.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp)
    ) {
        Box(
            modifier = Modifier
                .size(6.dp)
                .clip(CircleShape)
                .background(dotColor)
                .graphicsLayer { alpha = dotAlpha }
        )
        Column(verticalArrangement = Arrangement.spacedBy(0.dp)) {
            Text(
                text = label,
                style = MaterialTheme.typography.labelSmall,
                color = dotColor.copy(alpha = 0.85f)
            )
            Text(
                text = value,
                style = MaterialTheme.typography.labelLarge,
                color = dotColor
            )
        }
    }
}

@Composable
private fun overviewStatusTone(status: DaemonOverviewStatus): ChipTone = when {
    status.disconnected -> ChipTone.ERROR
    status.restartInProgress -> ChipTone.WARNING
    status.state == DaemonState.RUNNING && status.warningCount == 0 && status.errorCount == 0 -> ChipTone.SUCCESS
    status.state == DaemonState.DEGRADED || status.warningCount > 0 -> ChipTone.WARNING
    else -> ChipTone.NEUTRAL
}

private fun overviewStatusText(status: DaemonOverviewStatus): String = when {
    status.disconnected -> "Offline"
    status.restartInProgress -> "Restarting"
    status.state == DaemonState.RUNNING && status.warningCount == 0 && status.errorCount == 0 -> "Nominal"
    status.state == DaemonState.DEGRADED || status.warningCount > 0 -> "Needs review"
    else -> "Stopped"
}
