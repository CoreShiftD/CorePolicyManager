package com.corepolicy.manager.foundation

import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.BoxScope
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.graphics.Outline
import androidx.compose.ui.graphics.Shape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.PathFillType
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.addOutline
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.clipPath
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.core.designsystem.theme.AppThemeTokens
import com.corepolicy.manager.core.designsystem.theme.LocalSpacing

@Composable
fun VisualFoundationScreen() {
    val spacing = LocalSpacing.current
    val isDark = isSystemInDarkTheme()
    val transition = rememberInfiniteTransition(label = "foundation")
    val drift = transition.animateFloat(
        initialValue = 0.92f,
        targetValue = 1.08f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 9000, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "drift",
    )
    val panelFloat = transition.animateFloat(
        initialValue = -4f,
        targetValue = 6f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 7000, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "panelFloat",
    )
    val matteShift = transition.animateFloat(
        initialValue = 0f,
        targetValue = 10f,
        animationSpec = infiniteRepeatable(
            animation = tween(durationMillis = 10000, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "matteShift",
    )

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background),
    ) {
        AtmosphericBackdrop(
            glowScale = drift.value,
            isDark = isDark,
        )
        AmbientRim(
            modifier = Modifier.align(Alignment.TopCenter),
            alpha = if (isDark) 0.18f else 0.10f,
            height = 220.dp,
        )
        Column(
            modifier = Modifier
                .fillMaxSize()
                .statusBarsPadding()
                .padding(horizontal = spacing.medium, vertical = spacing.small),
            verticalArrangement = Arrangement.spacedBy(spacing.medium),
        ) {
            IdentityHeader()
            Spacer(modifier = Modifier.height(spacing.small))
            HeroSlab()
            FloatingGlassPanel(offsetY = panelFloat.value.dp)
            MatteStage(offsetX = matteShift.value.dp)
        }
        AmbientRim(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .offset(y = 96.dp),
            alpha = if (isDark) 0.10f else 0.06f,
            height = 260.dp,
        )
    }
}

@Composable
private fun AtmosphericBackdrop(
    glowScale: Float,
    isDark: Boolean,
) {
    val tokens = AppThemeTokens
    val background = MaterialTheme.colorScheme.background
    Canvas(modifier = Modifier.fillMaxSize()) {
        drawRect(
            brush = Brush.verticalGradient(
                colors = listOf(
                    background,
                    if (isDark) tokens.backgroundMid else Color(0xFFEFF4FA),
                    if (isDark) tokens.backgroundEdge else Color(0xFFE3EAF3),
                ),
            ),
        )
        drawCircle(
            brush = Brush.radialGradient(
                colors = listOf(
                    tokens.accentGlow.copy(alpha = if (isDark) 0.34f else 0.14f),
                    Color.Transparent,
                ),
                center = Offset(size.width * 0.82f, size.height * 0.16f),
                radius = size.minDimension * 0.42f * glowScale,
            ),
        )
        drawCircle(
            brush = Brush.radialGradient(
                colors = listOf(
                    tokens.secondaryGlow.copy(alpha = if (isDark) 0.24f else 0.10f),
                    Color.Transparent,
                ),
                center = Offset(size.width * 0.16f, size.height * 0.78f),
                radius = size.minDimension * 0.50f,
            ),
        )
        drawRect(
            brush = Brush.radialGradient(
                colors = listOf(
                    tokens.edgeLight.copy(alpha = if (isDark) 0.08f else 0.10f),
                    Color.Transparent,
                ),
                center = Offset(size.width * 0.50f, size.height * 0.10f),
                radius = size.width * 0.80f,
            ),
        )
        drawRect(
            brush = Brush.verticalGradient(
                colors = listOf(
                    Color.Transparent,
                    if (isDark) tokens.edgeShadow.copy(alpha = 0.22f) else Color(0x140B1017),
                    if (isDark) tokens.vignette.copy(alpha = 0.82f) else Color(0x0A0D1117),
                ),
                startY = size.height * 0.32f,
            ),
        )
    }
}

@Composable
private fun BoxScope.AmbientRim(
    modifier: Modifier,
    alpha: Float,
    height: androidx.compose.ui.unit.Dp,
) {
    Box(
        modifier = modifier
            .fillMaxWidth(0.92f)
            .height(height)
            .blur(42.dp)
            .background(
                brush = Brush.verticalGradient(
                    colors = listOf(
                        AppThemeTokens.edgeLight.copy(alpha = alpha),
                        Color.Transparent,
                    ),
                ),
                shape = RoundedCornerShape(120.dp),
            ),
    )
}

@Composable
private fun IdentityHeader() {
    val spacing = LocalSpacing.current
    BoxWithConstraints(
        modifier = Modifier
            .fillMaxWidth()
            .height(156.dp),
    ) {
        HeaderAnchorSurface(
            modifier = Modifier
                .align(Alignment.TopCenter),
            width = maxWidth + (spacing.large * 2),
        )
        Box(modifier = Modifier.fillMaxSize()) {
            Row(
                modifier = Modifier
                    .align(Alignment.Center)
                    .fillMaxWidth()
                    .padding(
                        start = spacing.small,
                        end = spacing.xSmall,
                        top = spacing.medium,
                    ),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                BrandGlyph()
                Spacer(modifier = Modifier.width(spacing.small))
                Column(
                    modifier = Modifier.weight(1f),
                    verticalArrangement = Arrangement.spacedBy(2.dp),
                ) {
                    Text(
                        text = "CorePolicy",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.60f),
                        fontWeight = FontWeight.Medium,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    Text(
                        text = "Manager",
                        style = MaterialTheme.typography.headlineLarge,
                        color = MaterialTheme.colorScheme.onBackground,
                        fontWeight = FontWeight.SemiBold,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                }
                PrivateBuildChip()
            }
        }
    }
}

@Composable
private fun HeaderAnchorSurface(
    modifier: Modifier = Modifier,
    width: androidx.compose.ui.unit.Dp,
) {
    val isDark = isSystemInDarkTheme()
    val slabPrimary = MaterialTheme.colorScheme.surfaceContainerHigh.copy(alpha = if (isDark) 0.84f else 0.88f)
    val slabSecondary = MaterialTheme.colorScheme.surface.copy(alpha = if (isDark) 0.90f else 0.94f)
    val topHighlight = Color.White.copy(alpha = if (isDark) 0.045f else 0.085f)
    val outline = MaterialTheme.colorScheme.outline.copy(alpha = if (isDark) 0.12f else 0.10f)
    Box(
        modifier = modifier
            .width(width)
            .height(146.dp)
            .offset(y = (-spacingForHeader()).dp)
            .drawBehind {
                val path = Path().apply {
                    fillType = PathFillType.NonZero
                    moveTo(0f, -size.height * 0.45f)
                    lineTo(size.width, -size.height * 0.45f)
                    lineTo(size.width, size.height * 0.72f)
                    quadraticTo(
                        size.width * 0.98f,
                        size.height * 0.88f,
                        size.width * 0.88f,
                        size.height * 0.88f,
                    )
                    lineTo(size.width * 0.12f, size.height * 0.88f)
                    quadraticTo(
                        size.width * 0.02f,
                        size.height * 0.88f,
                        0f,
                        size.height * 0.72f,
                    )
                    close()
                }
                clipPath(path) {
                    drawRect(
                        brush = Brush.verticalGradient(
                            colors = listOf(
                                slabSecondary,
                                slabPrimary,
                            ),
                        ),
                    )
                    drawRect(
                        brush = Brush.verticalGradient(
                            colors = listOf(
                                topHighlight,
                                Color.Transparent,
                            ),
                            startY = size.height * 0.24f,
                            endY = size.height * 0.52f,
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
private fun BrandGlyph() {
    val isDark = isSystemInDarkTheme()
    val shape = RoundedCornerShape(20.dp)
    val line = MaterialTheme.colorScheme.onSurface.copy(alpha = if (isDark) 0.88f else 0.70f)
    val dot = MaterialTheme.colorScheme.onSurface.copy(alpha = if (isDark) 0.96f else 0.78f)
    Surface(
        modifier = Modifier.size(42.dp),
        shape = shape,
        color = MaterialTheme.colorScheme.surfaceContainerHigh.copy(alpha = if (isDark) 0.58f else 0.74f),
        border = BorderStroke(
            1.dp,
            MaterialTheme.colorScheme.outline.copy(alpha = if (isDark) 0.22f else 0.14f),
        ),
        tonalElevation = 0.dp,
        shadowElevation = if (isDark) 6.dp else 2.dp,
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .clip(shape)
                .drawBehind {
                    val p1 = Offset(size.width * 0.34f, size.height * 0.30f)
                    val p2 = Offset(size.width * 0.68f, size.height * 0.34f)
                    val p3 = Offset(size.width * 0.30f, size.height * 0.68f)
                    val p4 = Offset(size.width * 0.66f, size.height * 0.70f)
                    val path = Path().apply {
                        moveTo(p1.x, p1.y)
                        lineTo(p2.x, p2.y)
                        lineTo(p4.x, p4.y)
                        lineTo(p3.x, p3.y)
                        close()
                    }
                    drawPath(
                        path = path,
                        color = line,
                        style = Stroke(width = 1.6.dp.toPx(), cap = StrokeCap.Round),
                    )
                    drawCircle(color = dot, radius = 2.4.dp.toPx(), center = p1)
                    drawCircle(color = dot, radius = 2.4.dp.toPx(), center = p2)
                    drawCircle(color = dot, radius = 2.4.dp.toPx(), center = p3)
                    drawCircle(color = dot, radius = 2.4.dp.toPx(), center = p4)
                },
        )
    }
}

@Composable
private fun PrivateBuildChip(
    modifier: Modifier = Modifier,
) {
    val isDark = isSystemInDarkTheme()
    Surface(
        modifier = modifier,
        shape = RoundedCornerShape(999.dp),
        color = MaterialTheme.colorScheme.surfaceContainerHigh.copy(alpha = if (isDark) 0.48f else 0.64f),
        border = BorderStroke(
            1.dp,
            MaterialTheme.colorScheme.outline.copy(alpha = if (isDark) 0.20f else 0.12f),
        ),
        tonalElevation = 0.dp,
        shadowElevation = 0.dp,
    ) {
        Text(
            text = "PRIVATE BUILD",
            modifier = Modifier.padding(horizontal = 12.dp, vertical = 8.dp),
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.primary.copy(alpha = if (isDark) 0.86f else 0.72f),
            fontWeight = FontWeight.SemiBold,
        )
    }
}

@Composable
private fun HeroSlab() {
    val radius = 42.dp
    FoundationSurface(
        modifier = Modifier
            .fillMaxWidth()
            .height(300.dp),
        shape = RoundedCornerShape(radius),
        brush = Brush.linearGradient(
            colors = listOf(
                MaterialTheme.colorScheme.surfaceBright.copy(alpha = 0.99f),
                MaterialTheme.colorScheme.surfaceContainerHigh.copy(alpha = 0.92f),
                MaterialTheme.colorScheme.surfaceContainer.copy(alpha = 0.78f),
            ),
            start = Offset.Zero,
            end = Offset(1280f, 960f),
        ),
        innerGlow = AppThemeTokens.accentGlow.copy(alpha = 0.16f),
        highlightAlpha = 0.08f,
        baseShadowAlpha = 0.10f,
        )
}

@Composable
private fun FloatingGlassPanel(
    offsetY: androidx.compose.ui.unit.Dp,
) {
    val radius = 32.dp
    FoundationSurface(
        modifier = Modifier
            .fillMaxWidth(0.72f)
            .height(132.dp)
            .offset(y = offsetY),
        shape = RoundedCornerShape(radius),
        brush = Brush.linearGradient(
            colors = listOf(
                MaterialTheme.colorScheme.surfaceBright.copy(alpha = 0.24f),
                MaterialTheme.colorScheme.surfaceContainerHighest.copy(alpha = 0.44f),
                MaterialTheme.colorScheme.surface.copy(alpha = 0.22f),
            ),
            start = Offset.Zero,
            end = Offset(840f, 560f),
        ),
        borderAlpha = 0.34f,
        innerGlow = Color.White.copy(alpha = 0.08f),
        highlightAlpha = 0.11f,
        baseShadowAlpha = 0.03f,
        edgeHighlightAlpha = 0.16f,
        floating = true,
    )
}

@Composable
private fun MatteStage(
    offsetX: androidx.compose.ui.unit.Dp,
) {
    val radius = 34.dp
    FoundationSurface(
        modifier = Modifier
            .fillMaxWidth()
            .offset(x = offsetX)
            .height(188.dp),
        shape = RoundedCornerShape(
            topStart = 34.dp,
            topEnd = 34.dp,
            bottomEnd = 54.dp,
            bottomStart = 26.dp,
        ),
        brush = Brush.verticalGradient(
            colors = listOf(
                MaterialTheme.colorScheme.surfaceContainer.copy(alpha = 0.86f),
                MaterialTheme.colorScheme.surfaceContainerLow.copy(alpha = 0.98f),
            ),
        ),
        borderAlpha = 0.24f,
        innerGlow = AppThemeTokens.secondaryGlow.copy(alpha = 0.10f),
        highlightAlpha = 0.06f,
        baseShadowAlpha = 0.10f,
        edgeHighlightAlpha = 0.08f,
    )
}

@Composable
private fun FoundationSurface(
    modifier: Modifier,
    shape: Shape,
    brush: Brush,
    innerGlow: Color,
    borderAlpha: Float = 0.22f,
    highlightAlpha: Float = 0.05f,
    baseShadowAlpha: Float = 0.12f,
    edgeHighlightAlpha: Float = 0.10f,
    floating: Boolean = false,
) {
    val isDark = isSystemInDarkTheme()
    val outline = MaterialTheme.colorScheme.outline.copy(alpha = borderAlpha)
    val frostLine = if (isDark) AppThemeTokens.frostLine else Color(0x40FFFFFF)
    val surfaceShadow = if (floating) {
        if (isDark) 10.dp else 4.dp
    } else {
        if (isDark) 6.dp else 2.dp
    }
    Surface(
        modifier = modifier
            .clip(shape),
        shape = shape,
        color = Color.Transparent,
        tonalElevation = 0.dp,
        shadowElevation = surfaceShadow,
        border = BorderStroke(
            width = 1.dp,
            color = outline,
        ),
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .clip(shape)
                .drawBehind {
                    val path = createShapePath(shape)
                    clipPath(path) {
                        drawRect(brush = brush)
                        drawRect(
                            brush = Brush.horizontalGradient(
                                colors = listOf(
                                    frostLine.copy(alpha = edgeHighlightAlpha),
                                    Color.Transparent,
                                    Color.Transparent,
                                ),
                                startX = 0f,
                                endX = size.width * 0.56f,
                            ),
                        )
                        drawRect(
                            brush = Brush.radialGradient(
                                colors = listOf(
                                    innerGlow.copy(alpha = if (isDark) 0.08f else 0.04f),
                                    Color.Transparent,
                                ),
                                center = Offset(size.width * 0.70f, size.height * 0.16f),
                                radius = size.maxDimension * 0.72f,
                            ),
                        )
                        drawRect(
                            brush = Brush.verticalGradient(
                                colors = listOf(
                                    Color.White.copy(alpha = highlightAlpha),
                                    Color.Transparent,
                                ),
                                startY = 0f,
                                endY = size.height * 0.18f,
                            ),
                        )
                    }
                },
            contentAlignment = Alignment.Center,
        ) {}
    }
}

private fun androidx.compose.ui.graphics.drawscope.DrawScope.createShapePath(shape: Shape): Path {
    val outline = shape.createOutline(
        size = Size(size.width, size.height),
        layoutDirection = layoutDirection,
        density = this,
    )
    return Path().apply {
        addOutline(outline)
    }
}

private fun spacingForHeader(): Int = 28
