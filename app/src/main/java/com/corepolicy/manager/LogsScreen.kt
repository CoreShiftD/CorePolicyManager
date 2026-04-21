package com.corepolicy.manager

import androidx.compose.foundation.background
import androidx.compose.foundation.horizontalScroll
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
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
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
import com.corepolicy.manager.ui.theme.CorePolicySemantics
import com.corepolicy.manager.ui.theme.CorePolicyDesign
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

private val LogFilters = listOf("All", "Daemon", "Policy", "Module", "Errors")

@OptIn(ExperimentalLayoutApi::class)
@Composable
fun LogsScreen(
    logs: List<LogEntry>,
    modifier: Modifier = Modifier
) {
    val spacing = CorePolicyDesign.spacing
    var filter by remember { mutableStateOf("All") }
    val filtered = logs.filter {
        when (filter) {
            "Daemon" -> it.category == LogCategory.DAEMON
            "Policy" -> it.category == LogCategory.POLICY
            "Module" -> it.category == LogCategory.MODULE
            "Errors" -> it.severity == LogSeverity.ERROR
            else -> true
        }
    }
    val grouped = filtered.groupBy { formatDateHeading(it.timestampMs) }

    Column(
        modifier = modifier
            .windowInsetsPadding(WindowInsets.statusBars)
            .padding(top = spacing.sm)
            .verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(spacing.lg)
    ) {
        PageHeader(
            eyebrow = "Diagnostics",
            title = "Event timeline",
            subtitle = "Daemon, policy, and module activity compressed into a readable operational log."
        )

        FlowRow(
            horizontalArrangement = Arrangement.spacedBy(spacing.sm),
            verticalArrangement = Arrangement.spacedBy(spacing.sm)
        ) {
            OverviewInlineBadge("Events", filtered.size.toString(), ChipTone.INFO)
            OverviewInlineBadge("Errors", filtered.count { it.severity == LogSeverity.ERROR }.toString(), ChipTone.ERROR)
        }

        Row(
            modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(spacing.sm)
        ) {
            LogFilters.forEach { label ->
                SelectableFilterChip(
                    label = label,
                    selected = filter == label,
                    onClick = { filter = label }
                )
            }
        }

        if (filtered.isEmpty()) {
            EmptyStateCard(
                title = "No log entries",
                message = "Nothing to show for this filter yet. Daemon and policy events will appear here as they happen.",
                iconRes = R.drawable.ic_schedule
            )
        } else {
            Column(verticalArrangement = Arrangement.spacedBy(spacing.sm)) {
                grouped.forEach { (date, dayLogs) ->
                    LogDayGroup(date = date, entries = dayLogs)
                }
            }
        }
        Spacer(Modifier.height(spacing.sm))
    }
}

/* -------------------------------------------------------------------------- */
/*  Day group card with sticky-style date header                              */
/* -------------------------------------------------------------------------- */

@Composable
private fun LogDayGroup(date: String, entries: List<LogEntry>) {
    val palette = LocalCorePolicyPalette.current
    val spacing = CorePolicyDesign.spacing
    SectionCard {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(bottom = spacing.nano),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(spacing.xs)
        ) {
            Box(
                modifier = Modifier
                    .size(6.dp)
                    .background(palette.primary, CircleShape)
            )
            Text(
                date,
                style = MaterialTheme.typography.labelLarge.copy(fontWeight = FontWeight.SemiBold),
                color = palette.onSurface
            )
            Spacer(Modifier.weight(1f))
            Text(
                "${entries.size} entries",
                style = MaterialTheme.typography.labelSmall,
                color = palette.onSurfaceVariant
            )
        }
        HorizontalDivider(color = palette.divider, thickness = 1.dp)

        entries.forEachIndexed { index, entry ->
            LogRow(entry)
            if (index != entries.lastIndex) {
                HorizontalDivider(color = palette.divider, thickness = 1.dp)
            }
        }
    }
}

/* -------------------------------------------------------------------------- */
/*  Individual log row                                                        */
/* -------------------------------------------------------------------------- */

@Composable
private fun LogRow(entry: LogEntry) {
    val palette = LocalCorePolicyPalette.current
    val spacing = CorePolicyDesign.spacing
    val s = CorePolicySemantics.colors
    val severityTone = when (entry.severity) {
        LogSeverity.INFO -> ChipTone.INFO
        LogSeverity.WARNING -> ChipTone.WARNING
        LogSeverity.ERROR -> ChipTone.ERROR
    }
    val accentColor = when (entry.severity) {
        LogSeverity.INFO -> s.info
        LogSeverity.WARNING -> s.warning
        LogSeverity.ERROR -> s.conflict
    }

    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(vertical = spacing.sm),
        horizontalArrangement = Arrangement.spacedBy(spacing.sm),
        verticalAlignment = Alignment.Top
    ) {
        Box(
            modifier = Modifier
                .height(40.dp)
                .size(width = 3.dp, height = 40.dp)
                .clip(CircleShape)
                .background(accentColor.copy(alpha = 0.7f))
        )
        Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(spacing.xs)
        ) {
            Row(
                horizontalArrangement = Arrangement.spacedBy(spacing.xs),
                verticalAlignment = Alignment.CenterVertically
            ) {
                StatusChip(
                    text = entry.category.name.lowercase().replaceFirstChar { it.uppercase() },
                    tone = severityTone,
                    leadingDot = true
                )
                Text(
                    text = formatRelativeTime(entry.timestampMs),
                    style = MaterialTheme.typography.labelMedium,
                    color = palette.onSurfaceVariant
                )
                Text("·", style = MaterialTheme.typography.labelMedium, color = palette.onSurfaceVariant)
                Text(
                    text = entry.sourceId,
                    style = MaterialTheme.typography.labelMedium,
                    color = palette.onSurfaceVariant
                )
            }
            Text(
                entry.message,
                style = MaterialTheme.typography.bodyMedium,
                color = palette.onSurface
            )
        }
    }
}
