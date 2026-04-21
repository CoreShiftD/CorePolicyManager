package com.corepolicy.manager.ui.components

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
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
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.ui.theme.CorePolicySemantics
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

enum class MetricState { CALM, WARNING, HIGH, CRITICAL, NEUTRAL }

enum class MetricType { CAPACITY, STATE }

data class DynamicMetric(
    val label: String,
    val value: String,
    val secondary: String,
    val trend: String,
    val progress: Float,
    val type: MetricType,
    val state: MetricState,
    val iconRes: Int
)

/**
 * Map a [MetricState] to a semantic accent color pair. This is the single
 * source of truth the metric grid uses so all severity coloring stays
 * consistent with the rest of the app (chips, banners, logs).
 */
@Composable
private fun accentColorsFor(state: MetricState): Pair<Color, Color> {
    val palette = LocalCorePolicyPalette.current
    val s = CorePolicySemantics.colors
    return when (state) {
        MetricState.CALM -> s.healthyContainer to s.onHealthyContainer
        MetricState.WARNING -> s.warningContainer to s.onWarningContainer
        MetricState.HIGH -> s.warningContainer to s.onWarningContainer
        MetricState.CRITICAL -> s.conflictContainer to s.onConflictContainer
        MetricState.NEUTRAL -> palette.surfaceContainerHigh to palette.onSurfaceVariant
    }
}

@Composable
private fun progressColorFor(state: MetricState): Color {
    val palette = LocalCorePolicyPalette.current
    val s = CorePolicySemantics.colors
    return when (state) {
        MetricState.CALM -> s.healthy
        MetricState.WARNING, MetricState.HIGH -> s.warning
        MetricState.CRITICAL -> s.conflict
        MetricState.NEUTRAL -> palette.primary
    }
}

/**
 * Premium 2x2 metric grid. Cards use tonal surface-container backgrounds (no
 * alpha cheats), the value animates when it changes, and the progress bar
 * springs in with the state-aware accent color.
 */
@Composable
fun InfoCardGrid(
    metrics: List<DynamicMetric>,
    modifier: Modifier = Modifier
) {
    val trimmed = metrics.take(4)
    val fallbackIcon = trimmed.firstOrNull()?.iconRes ?: 0
    val cells = if (trimmed.size == 4) {
        trimmed
    } else {
        trimmed + List(4 - trimmed.size) {
            DynamicMetric("—", "—", "—", "", 0f, MetricType.STATE, MetricState.NEUTRAL, fallbackIcon)
        }
    }
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(12.dp)) {
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            DynamicInfoCard(modifier = Modifier.weight(1f), metric = cells[0])
            DynamicInfoCard(modifier = Modifier.weight(1f), metric = cells[1])
        }
        Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            DynamicInfoCard(modifier = Modifier.weight(1f), metric = cells[2])
            DynamicInfoCard(modifier = Modifier.weight(1f), metric = cells[3])
        }
    }
}

@Composable
private fun DynamicInfoCard(
    modifier: Modifier = Modifier,
    metric: DynamicMetric
) {
    val palette = LocalCorePolicyPalette.current
    val (accentBg, accentFg) = accentColorsFor(metric.state)
    val progressColor = progressColorFor(metric.state)
    val animatedProgress by animateFloatAsState(
        targetValue = metric.progress.coerceIn(0f, 1f),
        animationSpec = spring(stiffness = Spring.StiffnessMediumLow, dampingRatio = Spring.DampingRatioNoBouncy),
        label = "metricProgress"
    )

    DashboardPanel(
        modifier = modifier,
        contentPadding = PaddingValues(horizontal = 14.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                DashboardIconBadge(iconRes = metric.iconRes, contentDescription = metric.label, tintBg = accentBg, tintFg = accentFg)
                Text(
                    text = metric.label,
                    style = MaterialTheme.typography.labelMedium,
                    color = palette.onSurfaceVariant
                )
            }
            if (metric.trend.isNotBlank()) {
                Text(
                    text = metric.trend,
                    style = MaterialTheme.typography.labelSmall,
                    color = palette.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis
                )
            }
        }
        AnimatedContent(
            targetState = metric.value,
            transitionSpec = {
                (fadeIn(tween(160)) + slideInVertically(tween(160)) { it / 3 }) togetherWith
                    (fadeOut(tween(120)) + slideOutVertically(tween(120)) { -it / 3 })
            },
            label = "metricValue"
        ) { v ->
            Text(
                text = v,
                style = MaterialTheme.typography.headlineMedium.copy(fontWeight = FontWeight.SemiBold),
                color = palette.onSurface,
                maxLines = 1,
                overflow = TextOverflow.Ellipsis
            )
        }
        Text(
            text = metric.secondary,
            style = MaterialTheme.typography.bodySmall,
            color = palette.onSurfaceVariant,
            maxLines = 1,
            overflow = TextOverflow.Ellipsis
        )
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .height(4.dp)
                .clip(RoundedCornerShape(50))
                .background(palette.divider)
        ) {
            val fill = if (metric.type == MetricType.CAPACITY) animatedProgress else 1f
            Box(
                modifier = Modifier
                    .fillMaxWidth(fill)
                    .height(4.dp)
                    .clip(RoundedCornerShape(50))
                    .background(progressColor)
            )
        }
    }
}
