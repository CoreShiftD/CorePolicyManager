package com.corepolicy.manager

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
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
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

private val LogFilters = listOf("All", "Daemon", "Policy", "Module", "Errors")

@Composable
fun LogsScreen(
    logs: List<LogEntry>,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
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
        modifier = modifier.verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(CorePolicyDimens.sectionGap)
    ) {
        PageHeader(
            eyebrow = "Observability",
            title = "Logs",
            subtitle = "${filtered.size} entries visible. Filter by domain to isolate daemon, policy, module, or error activity."
        )

        // Filter chip row
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .horizontalScroll(rememberScrollState()),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
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
            Column(verticalArrangement = Arrangement.spacedBy(CorePolicyDimens.cardGap)) {
                grouped.forEach { (date, dayLogs) ->
                    LogDayGroup(date = date, entries = dayLogs)
                }
            }
        }
        Spacer(Modifier.height(4.dp))
    }
}

/* -------------------------------------------------------------------------- */
/*  Day group card with sticky-style date header                              */
/* -------------------------------------------------------------------------- */

@Composable
private fun LogDayGroup(date: String, entries: List<LogEntry>) {
    val palette = LocalCorePolicyPalette.current
    val shape = RoundedCornerShape(CorePolicyDimens.cardRadius)
    Column(
        modifier = Modifier
            .fillMaxWidth()
            .clip(shape)
            .background(palette.surfaceContainer)
            .border(1.dp, palette.divider, shape)
    ) {
        // Sticky-style date header
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .background(palette.surfaceContainerHigh)
                .padding(horizontal = CorePolicyDimens.cardPaddingH, vertical = 10.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp)
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

        // Log rows
        entries.forEachIndexed { index, entry ->
            LogRow(entry)
            if (index != entries.lastIndex) {
                HorizontalDivider(
                    color = palette.divider,
                    thickness = 1.dp,
                    modifier = Modifier.padding(horizontal = CorePolicyDimens.cardPaddingH)
                )
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
            .padding(horizontal = CorePolicyDimens.cardPaddingH, vertical = 12.dp),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.Top
    ) {
        // Severity accent bar
        Box(
            modifier = Modifier
                .width(3.dp)
                .height(40.dp)
                .clip(RoundedCornerShape(CorePolicyDimens.chipRadius))
                .background(accentColor.copy(alpha = 0.7f))
        )
        Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(5.dp)
        ) {
            // Meta row
            Row(
                horizontalArrangement = Arrangement.spacedBy(6.dp),
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
            // Message
            Text(
                entry.message,
                style = MaterialTheme.typography.bodyMedium,
                color = palette.onSurface
            )
        }
    }
}
