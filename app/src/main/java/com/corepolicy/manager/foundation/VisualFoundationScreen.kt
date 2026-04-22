package com.corepolicy.manager.foundation

import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.asPaddingValues
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Outline
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathFillType
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.core.designsystem.theme.AppThemeTokens
import com.corepolicy.manager.core.designsystem.theme.LocalSpacing

@Composable
fun VisualFoundationScreen() {
    val spacing = LocalSpacing.current
    val isDark = isSystemInDarkTheme()
    val safePadding = WindowInsets.safeDrawing.asPaddingValues()
    val transition = rememberInfiniteTransition(label = "corepolicy")
    val drift = transition.animateFloat(
        initialValue = 0.96f,
        targetValue = 1.04f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 12000, easing = FastOutSlowInEasing),
        ),
        label = "drift",
    )

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background),
    ) {
        AtmosphericField(
            drift = drift.value,
            isDark = isDark,
        )
        TopCanopy(
            modifier = Modifier.fillMaxWidth(),
            isDark = isDark,
        )
        Column(
            modifier = Modifier
                .fillMaxSize()
                .verticalScroll(rememberScrollState())
                .padding(
                    start = spacing.medium,
                    end = spacing.medium,
                    top = safePadding.calculateTopPadding() + spacing.small,
                    bottom = safePadding.calculateBottomPadding() + spacing.large,
                ),
            verticalArrangement = Arrangement.spacedBy(spacing.medium),
        ) {
            IdentityZone()
            HeroLane()
            CapabilityLane(
                title = "Trust posture",
                summary = "Identity, governance, and runtime trust prepared as one calm operational surface.",
            )
            CapabilityLane(
                title = "Policy engine",
                summary = "A deliberate lane for authoring, staging, and enforcing execution boundaries without dashboard clutter.",
            )
            CapabilityLane(
                title = "Audit lane",
                summary = "Evidence and review space held back until real events arrive, instead of filling the screen with synthetic telemetry.",
            )
        }
    }
}

@Composable
private fun AtmosphericField(
    drift: Float,
    isDark: Boolean,
) {
    val background = MaterialTheme.colorScheme.background
    val upperGlow = if (isDark) AppThemeTokens.accentGlow.copy(alpha = 0.24f) else Color(0xFFB9D2FF).copy(alpha = 0.18f)
    val lowerGlow = if (isDark) AppThemeTokens.secondaryGlow.copy(alpha = 0.16f) else Color(0xFFAEE7E1).copy(alpha = 0.12f)
    Canvas(modifier = Modifier.fillMaxSize()) {
        drawRect(
            brush = Brush.verticalGradient(
                colors = listOf(
                    background,
                    if (isDark) AppThemeTokens.backgroundMid else Color(0xFFF0F5FB),
                    if (isDark) AppThemeTokens.backgroundEdge else Color(0xFFE6EDF5),
                ),
            ),
        )
        drawCircle(
            brush = Brush.radialGradient(
                colors = listOf(upperGlow, Color.Transparent),
                center = Offset(size.width * 0.82f, size.height * 0.12f),
                radius = size.minDimension * 0.44f * drift,
            ),
        )
        drawCircle(
            brush = Brush.radialGradient(
                colors = listOf(lowerGlow, Color.Transparent),
                center = Offset(size.width * 0.16f, size.height * 0.78f),
                radius = size.minDimension * 0.50f,
            ),
        )
    }
}

@Composable
private fun TopCanopy(
    modifier: Modifier = Modifier,
    isDark: Boolean,
) {
    val fillTop = MaterialTheme.colorScheme.surface.copy(alpha = if (isDark) 0.92f else 0.95f)
    val fillBottom = MaterialTheme.colorScheme.surfaceContainerHigh.copy(alpha = if (isDark) 0.80f else 0.88f)
    val outline = MaterialTheme.colorScheme.outline.copy(alpha = if (isDark) 0.10f else 0.08f)
    Box(
        modifier = modifier
            .height(252.dp)
            .drawBehind {
                val path = Path().apply {
                    fillType = PathFillType.NonZero
                    moveTo(-size.width * 0.08f, -size.height * 0.24f)
                    lineTo(size.width * 1.08f, -size.height * 0.24f)
                    lineTo(size.width * 1.08f, size.height * 0.78f)
                    cubicTo(
                        size.width * 1.03f,
                        size.height * 0.92f,
                        size.width * 0.94f,
                        size.height * 0.92f,
                        size.width * 0.82f,
                        size.height * 0.90f,
                    )
                    lineTo(size.width * 0.18f, size.height * 0.90f)
                    cubicTo(
                        size.width * 0.06f,
                        size.height * 0.92f,
                        -size.width * 0.03f,
                        size.height * 0.92f,
                        -size.width * 0.08f,
                        size.height * 0.78f,
                    )
                    close()
                }
                clipPath(path) {
                    drawRect(
                        brush = Brush.verticalGradient(
                            colors = listOf(fillTop, fillBottom),
                        ),
                    )
                    drawRect(
                        brush = Brush.verticalGradient(
                            colors = listOf(
                                Color.White.copy(alpha = if (isDark) 0.04f else 0.08f),
                                Color.Transparent,
                            ),
                            startY = size.height * 0.14f,
                            endY = size.height * 0.40f,
                        ),
                    )
                }
                drawPath(
                    path = path,
                    color = outline,
                    style = Stroke(width = 1.dp.toPx()),
                )
            },
    )
}

@Composable
private fun IdentityZone() {
    val spacing = LocalSpacing.current
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .padding(top = spacing.small),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Row(
            modifier = Modifier.weight(1f),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            BrandSeal()
            Spacer(modifier = Modifier.width(spacing.small))
            Column(
                verticalArrangement = Arrangement.spacedBy(2.dp),
            ) {
                Text(
                    text = "CorePolicy",
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.66f),
                    fontWeight = FontWeight.Medium,
                )
                Text(
                    text = "Manager",
                    style = MaterialTheme.typography.displaySmall,
                    color = MaterialTheme.colorScheme.onBackground,
                    fontWeight = FontWeight.SemiBold,
                )
            }
        }
        BuildStateChip()
    }
}

@Composable
private fun BrandSeal() {
    val isDark = isSystemInDarkTheme()
    val line = MaterialTheme.colorScheme.primary.copy(alpha = if (isDark) 0.92f else 0.74f)
    Surface(
        modifier = Modifier.size(46.dp),
        shape = CircleShape,
        color = MaterialTheme.colorScheme.surfaceContainerHigh.copy(alpha = if (isDark) 0.72f else 0.78f),
        border = BorderStroke(
            1.dp,
            MaterialTheme.colorScheme.outline.copy(alpha = if (isDark) 0.16f else 0.10f),
        ),
        shadowElevation = 0.dp,
        tonalElevation = 0.dp,
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .clip(CircleShape)
                .drawBehind {
                    val stroke = 1.7.dp.toPx()
                    val x1 = size.width * 0.34f
                    val x2 = size.width * 0.66f
                    val y1 = size.height * 0.34f
                    val y2 = size.height * 0.66f
                    drawLine(line, Offset(x1, y1), Offset(x2, y1), strokeWidth = stroke, cap = StrokeCap.Round)
                    drawLine(line, Offset(x1, y1), Offset(x1, y2), strokeWidth = stroke, cap = StrokeCap.Round)
                    drawLine(line, Offset(x2, y1), Offset(x2, y2), strokeWidth = stroke, cap = StrokeCap.Round)
                    drawLine(line, Offset(x1, y2), Offset(x2, y2), strokeWidth = stroke, cap = StrokeCap.Round)
                    drawLine(line, Offset(size.width * 0.50f, y1), Offset(size.width * 0.50f, y2), strokeWidth = stroke, cap = StrokeCap.Round)
                    drawLine(line, Offset(x1, size.height * 0.50f), Offset(x2, size.height * 0.50f), strokeWidth = stroke, cap = StrokeCap.Round)
                },
        )
    }
}

@Composable
private fun BuildStateChip() {
    val isDark = isSystemInDarkTheme()
    Surface(
        shape = RoundedCornerShape(999.dp),
        color = MaterialTheme.colorScheme.surfaceContainerHighest.copy(alpha = if (isDark) 0.58f else 0.68f),
        border = BorderStroke(
            1.dp,
            MaterialTheme.colorScheme.outline.copy(alpha = if (isDark) 0.14f else 0.10f),
        ),
        shadowElevation = 0.dp,
        tonalElevation = 0.dp,
    ) {
        Text(
            text = "PRIVATE BUILD",
            modifier = Modifier.padding(horizontal = 14.dp, vertical = 8.dp),
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.primary.copy(alpha = if (isDark) 0.90f else 0.76f),
            fontWeight = FontWeight.SemiBold,
        )
    }
}

@Composable
private fun HeroLane() {
    SurfaceLane(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(32.dp),
    ) {
        Column(
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Text(
                text = "A calmer governance lane for execution trust and runtime boundaries.",
                style = MaterialTheme.typography.displaySmall,
                color = MaterialTheme.colorScheme.onSurface,
                fontWeight = FontWeight.SemiBold,
            )
            Text(
                text = "This first generation is deliberately quiet. It leads with identity, posture, and future scale instead of synthetic telemetry or dashboard filler.",
                style = MaterialTheme.typography.bodyLarge,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }
}

@Composable
private fun CapabilityLane(
    title: String,
    summary: String,
) {
    val spacing = LocalSpacing.current
    SurfaceLane(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(28.dp),
    ) {
        Row(
            horizontalArrangement = Arrangement.spacedBy(spacing.medium),
            verticalAlignment = Alignment.Top,
        ) {
            SkeletonStem()
            Column(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Text(
                    text = title,
                    style = MaterialTheme.typography.titleLarge,
                    color = MaterialTheme.colorScheme.onSurface,
                    fontWeight = FontWeight.SemiBold,
                )
                Text(
                    text = summary,
                    style = MaterialTheme.typography.bodyLarge,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                SkeletonTextLine(widthFraction = 0.82f)
                SkeletonTextLine(widthFraction = 0.60f)
            }
        }
    }
}

@Composable
private fun SurfaceLane(
    modifier: Modifier,
    shape: RoundedCornerShape,
    content: @Composable ColumnScope.() -> Unit,
) {
    val isDark = isSystemInDarkTheme()
    val accent = AppThemeTokens.accentGlow.copy(alpha = if (isDark) 0.08f else 0.04f)
    val highlight = Color.White.copy(alpha = if (isDark) 0.035f else 0.055f)
    Surface(
        modifier = modifier,
        shape = shape,
        color = MaterialTheme.colorScheme.surfaceContainer.copy(alpha = if (isDark) 0.92f else 0.96f),
        border = BorderStroke(
            1.dp,
            MaterialTheme.colorScheme.outline.copy(alpha = if (isDark) 0.12f else 0.08f),
        ),
        tonalElevation = 0.dp,
        shadowElevation = if (isDark) 4.dp else 1.dp,
    ) {
        Column(
            modifier = Modifier
                .clip(shape)
                .drawBehind {
                    val path = createRoundedPath(shape)
                    clipPath(path) {
                        drawRect(
                            brush = Brush.radialGradient(
                                colors = listOf(accent, Color.Transparent),
                                center = Offset(size.width * 0.88f, size.height * 0.18f),
                                radius = size.maxDimension * 0.56f,
                            ),
                        )
                        drawRect(
                            brush = Brush.verticalGradient(
                                colors = listOf(
                                    highlight,
                                    Color.Transparent,
                                ),
                                startY = 0f,
                                endY = size.height * 0.18f,
                            ),
                        )
                    }
                }
                .padding(24.dp),
        ) {
            content()
        }
    }
}

@Composable
private fun SkeletonStem() {
    val isDark = isSystemInDarkTheme()
    Surface(
        modifier = Modifier
            .padding(top = 4.dp)
            .size(width = 12.dp, height = 72.dp),
        shape = RoundedCornerShape(999.dp),
        color = MaterialTheme.colorScheme.primary.copy(alpha = if (isDark) 0.36f else 0.22f),
        shadowElevation = 0.dp,
        tonalElevation = 0.dp,
    ) {}
}

@Composable
private fun SkeletonTextLine(
    widthFraction: Float,
) {
    val isDark = isSystemInDarkTheme()
    Surface(
        modifier = Modifier
            .fillMaxWidth(widthFraction)
            .height(10.dp),
        shape = RoundedCornerShape(999.dp),
        color = MaterialTheme.colorScheme.surfaceContainerHighest.copy(alpha = if (isDark) 0.70f else 0.86f),
        shadowElevation = 0.dp,
        tonalElevation = 0.dp,
    ) {}
}

private fun androidx.compose.ui.graphics.drawscope.DrawScope.createRoundedPath(
    shape: RoundedCornerShape,
): Path {
    return Path().apply {
        when (val outline = shape.createOutline(size, LayoutDirection.Ltr, this@createRoundedPath)) {
            is Outline.Generic -> addPath(outline.path)
            is Outline.Rectangle -> addRect(Rect(0f, 0f, size.width, size.height))
            is Outline.Rounded -> addRoundRect(outline.roundRect)
        }
    }
}
