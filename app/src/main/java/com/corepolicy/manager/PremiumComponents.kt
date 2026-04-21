package com.corepolicy.manager

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
import androidx.compose.foundation.Image
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
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.ui.theme.CorePolicySemantics
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

/* -------------------------------------------------------------------------- */
/*  IconBadge — rounded square icon with tonal background                     */
/* -------------------------------------------------------------------------- */

@Composable
fun IconBadge(
    iconRes: Int,
    contentDescription: String?,
    tone: ChipTone = ChipTone.NEUTRAL,
    size: androidx.compose.ui.unit.Dp = CorePolicyDimens.iconBadge,
    modifier: Modifier = Modifier
) {
    val bg = accentBackgroundFor(tone)
    val fg = accentForegroundFor(tone)
    Box(
        modifier = modifier
            .size(size)
            .clip(RoundedCornerShape(10.dp))
            .background(bg),
        contentAlignment = Alignment.Center
    ) {
        Image(
            painter = painterResource(id = iconRes),
            contentDescription = contentDescription,
            colorFilter = ColorFilter.tint(fg),
            modifier = Modifier.size((size.value * 0.5f).dp)
        )
    }
}

/* -------------------------------------------------------------------------- */
/*  ActionTile — small square tile for quick actions                          */
/* -------------------------------------------------------------------------- */

@Composable
fun ActionTile(
    iconRes: Int,
    label: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    tone: ChipTone = ChipTone.NEUTRAL
) {
    val palette = LocalCorePolicyPalette.current
    val shape = RoundedCornerShape(CorePolicyDimens.cardRadiusTight)
    Column(
        modifier = modifier
            .clip(shape)
            .background(palette.surfaceContainerHigh)
            .border(1.dp, palette.divider, shape)
            .clickable(onClick = onClick)
            .padding(horizontal = 12.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp)
    ) {
        IconBadge(iconRes = iconRes, contentDescription = label, tone = tone, size = 28.dp)
        Text(
            text = label,
            style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.SemiBold),
            color = palette.onSurface
        )
    }
}

/* -------------------------------------------------------------------------- */
/*  AnimatedStatPill — value that animates when it changes                    */
/* -------------------------------------------------------------------------- */

@Composable
fun AnimatedStatPill(
    label: String,
    value: String,
    modifier: Modifier = Modifier,
    tone: ChipTone = ChipTone.NEUTRAL
) {
    val palette = LocalCorePolicyPalette.current
    val shape = RoundedCornerShape(CorePolicyDimens.cardRadiusTight)
    Column(
        modifier = modifier
            .clip(shape)
            .background(palette.surfaceContainerHigh)
            .border(1.dp, palette.divider, shape)
            .padding(horizontal = 12.dp, vertical = 10.dp),
        verticalArrangement = Arrangement.spacedBy(4.dp)
    ) {
        Text(label, style = MaterialTheme.typography.labelSmall, color = palette.onSurfaceVariant)
        AnimatedContent(
            targetState = value,
            transitionSpec = {
                (slideInVertically(tween(200)) { it / 2 } + fadeIn(tween(200))) togetherWith
                    (slideOutVertically(tween(200)) { -it / 2 } + fadeOut(tween(200)))
            },
            label = "statPillValue"
        ) { v ->
            Text(
                text = v,
                style = MaterialTheme.typography.titleMedium,
                color = accentForegroundFor(tone).takeIf { tone != ChipTone.NEUTRAL } ?: palette.onSurface
            )
        }
    }
}

/* -------------------------------------------------------------------------- */
/*  MetricCard — the hero dashboard stat                                      */
/* -------------------------------------------------------------------------- */

data class MetricCardData(
    val label: String,
    val value: String,
    val secondary: String,
    val trend: String,
    val progress: Float,
    val tone: ChipTone,
    val iconRes: Int,
    val showProgress: Boolean
)

@Composable
fun MetricCard(
    data: MetricCardData,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    val accent = accentForegroundFor(data.tone)
    val trackColor = palette.divider
    val animatedProgress by animateFloatAsState(
        targetValue = data.progress.coerceIn(0f, 1f),
        animationSpec = spring(stiffness = Spring.StiffnessMediumLow, dampingRatio = Spring.DampingRatioNoBouncy),
        label = "metricProgress"
    )
    val shape = RoundedCornerShape(CorePolicyDimens.cardRadius)

    Column(
        modifier = modifier
            .clip(shape)
            .background(palette.surfaceContainer)
            .border(1.dp, palette.divider, shape)
            .padding(horizontal = 14.dp, vertical = 12.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp)
    ) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically
        ) {
            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                IconBadge(iconRes = data.iconRes, contentDescription = data.label, tone = data.tone, size = 26.dp)
                Text(data.label, style = MaterialTheme.typography.labelMedium, color = palette.onSurfaceVariant)
            }
            if (data.trend.isNotBlank()) {
                Text(
                    text = data.trend,
                    style = MaterialTheme.typography.labelSmall,
                    color = palette.onSurfaceVariant
                )
            }
        }
        AnimatedContent(
            targetState = data.value,
            transitionSpec = {
                (fadeIn(tween(180)) + slideInVertically(tween(180)) { it / 3 }) togetherWith
                    (fadeOut(tween(120)) + slideOutVertically(tween(120)) { -it / 3 })
            },
            label = "metricValue"
        ) { v ->
            Text(
                text = v,
                style = MaterialTheme.typography.headlineMedium.copy(fontWeight = FontWeight.SemiBold),
                color = palette.onSurface
            )
        }
        Text(
            text = data.secondary,
            style = MaterialTheme.typography.bodySmall,
            color = palette.onSurfaceVariant
        )
        if (data.showProgress) {
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(4.dp)
                    .clip(RoundedCornerShape(50))
                    .background(trackColor)
            ) {
                Box(
                    modifier = Modifier
                        .fillMaxWidth(animatedProgress)
                        .height(4.dp)
                        .clip(RoundedCornerShape(50))
                        .background(accent)
                )
            }
        }
    }
}

/* -------------------------------------------------------------------------- */
/*  EmptyStateCard                                                            */
/* -------------------------------------------------------------------------- */

@Composable
fun EmptyStateCard(
    title: String,
    message: String,
    iconRes: Int? = null,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    val shape = RoundedCornerShape(CorePolicyDimens.cardRadius)
    Column(
        modifier = modifier
            .fillMaxWidth()
            .clip(shape)
            .background(palette.surfaceContainer)
            .border(1.dp, palette.divider, shape)
            .padding(24.dp),
        verticalArrangement = Arrangement.spacedBy(10.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        iconRes?.let {
            Box(
                modifier = Modifier
                    .size(44.dp)
                    .clip(CircleShape)
                    .background(palette.surfaceContainerHigh),
                contentAlignment = Alignment.Center
            ) {
                Image(
                    painter = painterResource(id = it),
                    contentDescription = null,
                    modifier = Modifier.size(22.dp),
                    colorFilter = ColorFilter.tint(palette.onSurfaceVariant)
                )
            }
        }
        Text(title, style = MaterialTheme.typography.titleMedium, color = palette.onSurface)
        Text(message, style = MaterialTheme.typography.bodyMedium, color = palette.onSurfaceVariant)
    }
}

/* -------------------------------------------------------------------------- */
/*  ErrorBanner                                                               */
/* -------------------------------------------------------------------------- */

@Composable
fun ErrorBanner(
    title: String,
    message: String,
    tone: ChipTone = ChipTone.ERROR,
    modifier: Modifier = Modifier,
    action: (@Composable () -> Unit)? = null
) {
    val bg = accentBackgroundFor(tone)
    val fg = accentForegroundFor(tone)
    val shape = RoundedCornerShape(CorePolicyDimens.cardRadiusTight)
    Row(
        modifier = modifier
            .fillMaxWidth()
            .clip(shape)
            .background(bg)
            .padding(horizontal = 14.dp, vertical = 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp)
    ) {
        Box(
            modifier = Modifier
                .size(8.dp)
                .background(fg, shape = RoundedCornerShape(50))
        )
        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(title, style = MaterialTheme.typography.titleSmall, color = fg)
            Text(message, style = MaterialTheme.typography.bodySmall, color = fg.copy(alpha = 0.8f))
        }
        action?.invoke()
    }
}
