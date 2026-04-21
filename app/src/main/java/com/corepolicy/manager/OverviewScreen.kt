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
import androidx.compose.foundation.layout.statusBarsPadding
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
        modifier = modifier
            .statusBarsPadding()
            .padding(top = 4.dp)
            .verticalScroll(scroll),
        verticalArrangement = Arrangement.spacedBy(CorePolicyDimens.sectionGap)
    ) {
        OverviewSignalRow(
            daemonStatus = daemonStatus,
            managedAppsCount = managedAppsCount,
            modulesCount = daemonStatus.enabledModules,
            warningCount = daemonStatus.warningCount
        )

        DaemonHeroCard(
            status = daemonStatus,
            onRestartDaemon = onRestartDaemon,
            onOpenLogs = onOpenLogs,
            onManageModules = onManageModules
        )

        OverviewMetricsSection(
            metrics = metrics
        )

        InsightsSection(insights = insights)

        StaticSystemInfoSection(systemInfo = systemInfo, runtimeInfo = runtimeInfo)

        Spacer(Modifier.height(4.dp))
    }
}

@Composable
private fun OverviewSignalRow(
    daemonStatus: DaemonOverviewStatus,
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
            label = "Targets",
            value = managedAppsCount.toString(),
            tone = ChipTone.INFO,
            modifier = Modifier.weight(1f)
        )
        OverviewInlineBadge(
            label = "Warnings",
            value = warningCount.toString(),
            tone = if (warningCount > 0) ChipTone.WARNING else if (daemonStatus.disconnected) ChipTone.ERROR else ChipTone.NEUTRAL,
            modifier = Modifier.weight(1f)
        )
    }
}

@Composable
private fun DaemonHeroCard(
    status: DaemonOverviewStatus,
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
    val statusSubtitle = when {
        status.disconnected -> "Offline • Awaiting reconnect"
        status.restartInProgress -> "Restarting • Sync paused"
        status.state == DaemonState.DEGRADED -> "Degraded • Review required"
        status.lastSyncTimestampMs > 0L -> "Running • Synced"
        else -> "Healthy • Enforcing profile"
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
            verticalAlignment = Alignment.Top
        ) {
            Row(
                modifier = Modifier.weight(1f),
                verticalAlignment = Alignment.Top,
                horizontalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                IconBadge(
                    iconRes = R.drawable.ic_cpu,
                    contentDescription = "Daemon",
                    tone = ChipTone.ACTIVE,
                    size = 34.dp
                )
                Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                    Text(
                        "CorePolicy Daemon",
                        style = MaterialTheme.typography.titleSmall,
                        color = palette.onSurface
                    )
                    Text(
                        statusSubtitle,
                        style = MaterialTheme.typography.labelSmall,
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

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            DaemonMetaItem(label = "Uptime", value = formatDuration(status.uptimeMs), modifier = Modifier.weight(1f))
            DaemonMetaItem(label = "Last sync", value = formatRelativeTime(status.lastSyncTimestampMs), modifier = Modifier.weight(1f))
        }

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
private fun OverviewMetricsSection(metrics: List<DynamicMetric>) {
    InfoCardGrid(metrics = metrics)
}

@Composable
private fun OverviewOperationalBanner(status: DaemonOverviewStatus) {
    val title: String
    val message: String
    val tone: ChipTone
    when {
        status.disconnected -> {
            title = "Last action"
            message = status.lastAction.ifBlank { "Daemon unreachable. Action feedback is temporarily unavailable." }
            tone = ChipTone.ERROR
        }
        status.restartInProgress -> {
            title = "Last action"
            message = status.lastAction.ifBlank { "Restart underway. Recent commands may report as pending." }
            tone = ChipTone.WARNING
        }
        status.warningCount > 0 || status.errorCount > 0 -> {
            title = "Last action"
            message = status.lastAction.ifBlank { "Review the latest daemon activity and warning state." }
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
    Column(
        modifier = modifier,
        verticalArrangement = Arrangement.spacedBy(2.dp)
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
