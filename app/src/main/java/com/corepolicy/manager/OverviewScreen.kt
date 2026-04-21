package com.corepolicy.manager

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.background
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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.windowInsetsPadding
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
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.ui.R
import com.corepolicy.manager.ui.components.DynamicMetric
import com.corepolicy.manager.ui.components.InsightItem
import com.corepolicy.manager.ui.components.InsightTone
import com.corepolicy.manager.ui.components.SystemProfile
import com.corepolicy.manager.ui.theme.CorePolicyDesign
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

@OptIn(ExperimentalLayoutApi::class)
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
    val spacing = CorePolicyDesign.spacing
    Column(
        modifier = modifier
            .windowInsetsPadding(WindowInsets.statusBars)
            .padding(top = spacing.sm)
            .verticalScroll(scroll),
        verticalArrangement = Arrangement.spacedBy(spacing.lg)
    ) {
        OverviewHeader(
            selectedProfile = selectedProfile,
            daemonStatus = daemonStatus,
            onProfileClick = onProfileClick
        )

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

        SectionHeader("Live telemetry", "Signal density without filler metrics")
        OverviewMetricsSection(metrics = metrics)

        SectionHeader("Operating notes", "Current behavior worth attention")
        InsightList(insights = insights)

        SectionHeader("System envelope", "Platform, kernel, runtime")
        StaticInfoGrid(systemInfo = systemInfo, runtimeInfo = runtimeInfo)

        Spacer(Modifier.height(spacing.sm))
    }
}

@Composable
private fun OverviewHeader(
    selectedProfile: SystemProfile,
    daemonStatus: DaemonOverviewStatus,
    onProfileClick: () -> Unit
) {
    val spacing = CorePolicyDesign.spacing
    PageHeader(
        eyebrow = "CorePolicy",
        title = "System control center",
        subtitle = "Daemon state, module enforcement, and thermal posture in one surface.",
        trailing = {
            Column(
                horizontalAlignment = Alignment.End,
                verticalArrangement = Arrangement.spacedBy(spacing.xs)
            ) {
                StatusChip(
                    text = formatDaemonStateLabel(daemonStatus.state, daemonStatus.disconnected),
                    tone = overviewStatusTone(daemonStatus),
                    leadingDot = true
                )
                SecondaryButton(text = selectedProfile.title, onClick = onProfileClick)
            }
        }
    )
}

@Composable
private fun OverviewSignalRow(
    daemonStatus: DaemonOverviewStatus,
    managedAppsCount: Int,
    modulesCount: Int,
    warningCount: Int
) {
    val spacing = CorePolicyDesign.spacing
    FlowRow(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.spacedBy(spacing.sm),
        verticalArrangement = Arrangement.spacedBy(spacing.sm)
    ) {
        OverviewInlineBadge(label = "Modules online", value = modulesCount.toString(), tone = ChipTone.ACTIVE)
        OverviewInlineBadge(label = "Managed targets", value = managedAppsCount.toString(), tone = ChipTone.INFO)
        OverviewInlineBadge(
            label = "Attention items",
            value = warningCount.toString(),
            tone = if (warningCount > 0) ChipTone.WARNING else if (daemonStatus.disconnected) ChipTone.ERROR else ChipTone.NEUTRAL
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
    val spacing = CorePolicyDesign.spacing
    val stateLabel = formatDaemonStateLabel(status.state, status.disconnected)
    val statusSubtitle = when {
        status.disconnected -> "Offline • Awaiting reconnect"
        status.restartInProgress -> "Restarting • Sync paused"
        status.state == DaemonState.DEGRADED -> "Degraded • Review required"
        status.lastSyncTimestampMs > 0L -> "Running • Synced"
        else -> "Healthy • Enforcing profile"
    }

    SectionCard(elevated = true) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(spacing.sm),
            verticalAlignment = Alignment.Top
        ) {
            Row(
                modifier = Modifier.weight(1f),
                verticalAlignment = Alignment.Top,
                horizontalArrangement = Arrangement.spacedBy(spacing.sm)
            ) {
                IconBadge(
                    iconRes = R.drawable.ic_cpu,
                    contentDescription = "Daemon",
                    tone = ChipTone.ACTIVE,
                    size = CorePolicyDesign.icons.xl
                )
                Column(verticalArrangement = Arrangement.spacedBy(spacing.nano)) {
                    Text(
                        "Policy daemon",
                        style = MaterialTheme.typography.headlineSmall,
                        color = palette.onSurface
                    )
                    Text(
                        statusSubtitle,
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

        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(spacing.sm)
        ) {
            DaemonMetaItem(label = "Uptime", value = formatDuration(status.uptimeMs), modifier = Modifier.weight(1f))
            DaemonMetaItem(label = "Last sync", value = formatRelativeTime(status.lastSyncTimestampMs), modifier = Modifier.weight(1f))
        }

        OverviewOperationalBanner(status = status)

        HorizontalDivider(color = palette.divider, thickness = 1.dp)

        Row(horizontalArrangement = Arrangement.spacedBy(spacing.sm)) {
            PrimaryButton(text = "Restart daemon", onClick = onRestartDaemon, modifier = Modifier.weight(1f))
            SecondaryButton(text = "View logs", onClick = onOpenLogs, modifier = Modifier.weight(1f))
            SecondaryButton(text = "Open modules", onClick = onManageModules, modifier = Modifier.weight(1f))
        }
    }
}

@Composable
private fun OverviewMetricsSection(metrics: List<DynamicMetric>) {
    val spacing = CorePolicyDesign.spacing
    Column(verticalArrangement = Arrangement.spacedBy(spacing.sm)) {
        metrics.chunked(2).forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(spacing.sm)) {
                row.forEach { metric ->
                    MetricCell(metric = metric, modifier = Modifier.weight(1f))
                }
                if (row.size == 1) {
                    Spacer(modifier = Modifier.weight(1f))
                }
            }
        }
    }
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
        verticalArrangement = Arrangement.spacedBy(CorePolicyDesign.spacing.nano)
    ) {
        Text(text = label, style = MaterialTheme.typography.labelSmall, color = palette.onSurfaceVariant)
        Text(
            text = value,
            style = MaterialTheme.typography.titleSmall,
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
    val spacing = CorePolicyDesign.spacing
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
            .clip(RoundedCornerShape(CorePolicyDesign.radii.full))
            .background(accentBackgroundFor(tone))
            .padding(horizontal = spacing.md, vertical = spacing.sm),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(spacing.xs)
    ) {
        Box(
            modifier = Modifier
                .size(6.dp)
                .clip(CircleShape)
                .background(dotColor)
                .graphicsLayer { alpha = dotAlpha }
        )
        Column(verticalArrangement = Arrangement.spacedBy(spacing.nano)) {
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

@Composable
private fun MetricCell(
    metric: DynamicMetric,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    val spacing = CorePolicyDesign.spacing
    val tone = when (metric.state) {
        com.corepolicy.manager.ui.components.MetricState.CALM -> ChipTone.SUCCESS
        com.corepolicy.manager.ui.components.MetricState.WARNING,
        com.corepolicy.manager.ui.components.MetricState.HIGH -> ChipTone.WARNING
        com.corepolicy.manager.ui.components.MetricState.CRITICAL -> ChipTone.ERROR
        com.corepolicy.manager.ui.components.MetricState.NEUTRAL -> ChipTone.INFO
    }
    SectionCard(modifier = modifier) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Row(horizontalArrangement = Arrangement.spacedBy(spacing.xs), verticalAlignment = Alignment.CenterVertically) {
                IconBadge(iconRes = metric.iconRes, contentDescription = metric.label, tone = tone, size = CorePolicyDesign.icons.lg)
                Text(metric.label, style = MaterialTheme.typography.labelMedium, color = palette.onSurfaceVariant)
            }
            if (metric.trend.isNotBlank()) {
                StatusChip(metric.trend, tone = tone)
            }
        }
        AnimatedContent(
            targetState = metric.value,
            transitionSpec = {
                (fadeIn(tween(140)) + slideInVertically(tween(140)) { it / 3 }) togetherWith
                    (fadeOut(tween(100)) + slideOutVertically(tween(100)) { -it / 3 })
            },
            label = "overviewMetricValue"
        ) { value ->
            Text(
                text = value,
                style = MaterialTheme.typography.displaySmall,
                color = palette.onSurface,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
        }
        Text(
            text = metric.secondary,
            style = MaterialTheme.typography.bodySmall,
            color = palette.onSurfaceVariant
        )
    }
}

@Composable
private fun InsightList(insights: List<InsightItem>) {
    val palette = LocalCorePolicyPalette.current
    val spacing = CorePolicyDesign.spacing
    SectionCard {
        insights.take(3).forEachIndexed { index, insight ->
            val tone = when (insight.tone) {
                InsightTone.POSITIVE -> ChipTone.SUCCESS
                InsightTone.WARNING -> ChipTone.WARNING
                InsightTone.CRITICAL -> ChipTone.ERROR
                InsightTone.NEUTRAL -> ChipTone.INFO
            }
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(spacing.sm),
                verticalAlignment = Alignment.CenterVertically
            ) {
                IconBadge(iconRes = insight.iconRes, contentDescription = null, tone = tone, size = CorePolicyDesign.icons.lg)
                Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(spacing.nano)) {
                    Text(insight.summary, style = MaterialTheme.typography.bodyMedium, color = palette.onSurface)
                    insight.chip?.let { Text(it, style = MaterialTheme.typography.labelSmall, color = palette.onSurfaceVariant) }
                }
                insight.chip?.let { StatusChip(text = it, tone = tone) }
            }
            if (index != insights.take(3).lastIndex) {
                HorizontalDivider(color = palette.divider, thickness = 1.dp)
            }
        }
    }
}

@Composable
private fun StaticInfoGrid(
    systemInfo: Triple<String, String, String>,
    runtimeInfo: Triple<String, String, String>
) {
    val items = listOf(
        Triple("Chipset", systemInfo.first, R.drawable.ic_cpu),
        Triple("Architecture", systemInfo.second, R.drawable.ic_network),
        Triple("Kernel", systemInfo.third, R.drawable.ic_schedule),
        Triple("Memory", runtimeInfo.first, R.drawable.ic_memory),
        Triple("Governor", runtimeInfo.second, R.drawable.ic_performance),
        Triple("Android", runtimeInfo.third, R.drawable.ic_info)
    )
    val spacing = CorePolicyDesign.spacing
    Column(verticalArrangement = Arrangement.spacedBy(spacing.sm)) {
        items.chunked(2).forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(spacing.sm)) {
                row.forEach { (label, value, icon) ->
                    SectionCard(modifier = Modifier.weight(1f)) {
                        Row(horizontalArrangement = Arrangement.spacedBy(spacing.sm), verticalAlignment = Alignment.CenterVertically) {
                            IconBadge(iconRes = icon, contentDescription = label, tone = ChipTone.NEUTRAL, size = CorePolicyDesign.icons.lg)
                            Column(verticalArrangement = Arrangement.spacedBy(spacing.nano)) {
                                Text(value, style = MaterialTheme.typography.titleSmall, color = LocalCorePolicyPalette.current.onSurface)
                                Text(label, style = MaterialTheme.typography.labelSmall, color = LocalCorePolicyPalette.current.onSurfaceVariant)
                            }
                        }
                    }
                }
            }
        }
    }
}
