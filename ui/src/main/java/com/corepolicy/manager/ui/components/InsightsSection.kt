package com.corepolicy.manager.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import com.corepolicy.manager.ui.theme.CorePolicyPalette
import com.corepolicy.manager.ui.theme.CorePolicySemantics
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

enum class InsightTone { NEUTRAL, POSITIVE, WARNING, CRITICAL }

data class InsightItem(
    val iconRes: Int,
    val summary: String,
    val chip: String? = null,
    val tone: InsightTone = InsightTone.NEUTRAL
)

@Composable
private fun toneColors(
    tone: InsightTone,
    palette: CorePolicyPalette
): Pair<Color, Color> {
    val s = CorePolicySemantics.colors
    return when (tone) {
        InsightTone.NEUTRAL -> palette.surfaceContainerHigh to palette.onSurfaceVariant
        InsightTone.POSITIVE -> s.healthyContainer to s.onHealthyContainer
        InsightTone.WARNING -> s.warningContainer to s.onWarningContainer
        InsightTone.CRITICAL -> s.conflictContainer to s.onConflictContainer
    }
}

/**
 * Compact insight list with hairline separators. Each row carries a semantic
 * chip so the user can scan status at a glance without reading the copy.
 */
@Composable
fun InsightsSection(
    insights: List<InsightItem>,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    val rows = insights.take(3)

    DashboardPanel(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(10.dp)
    ) {
        rows.forEachIndexed { index, insight ->
            InsightRow(insight = insight)
            if (index != rows.lastIndex) {
                Spacer(
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(1.dp)
                        .background(palette.divider)
                )
            }
        }
    }
}

@Composable
private fun InsightRow(insight: InsightItem) {
    val palette = LocalCorePolicyPalette.current
    val (chipBg, chipFg) = toneColors(insight.tone, palette)
    Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        DashboardIconBadge(
            iconRes = insight.iconRes,
            contentDescription = null,
            tintBg = chipBg,
            tintFg = chipFg
        )
        Text(
            text = insight.summary,
            modifier = Modifier.weight(1f),
            style = MaterialTheme.typography.bodyMedium,
            color = palette.onSurface
        )
        insight.chip?.let {
            DashboardStatusChip(
                text = it,
                background = chipBg,
                content = chipFg,
                modifier = Modifier.align(Alignment.CenterVertically)
            )
        }
    }
}
