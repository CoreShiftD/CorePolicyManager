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
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.blur
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
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
                .padding(horizontal = spacing.medium, vertical = spacing.large),
            verticalArrangement = Arrangement.spacedBy(spacing.medium),
        ) {
            MinimalWordmark()
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
private fun MinimalWordmark() {
    val spacing = LocalSpacing.current
    Column(
        verticalArrangement = Arrangement.spacedBy(spacing.xSmall),
    ) {
        Text(
            text = "CORE POLICY",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.54f),
            fontWeight = FontWeight.Medium,
        )
        Text(
            text = "Visual foundation",
            style = MaterialTheme.typography.displaySmall,
            color = MaterialTheme.colorScheme.onBackground,
        )
    }
}

@Composable
private fun HeroSlab() {
    FoundationSurface(
        modifier = Modifier
            .fillMaxWidth()
            .height(300.dp),
        shape = RoundedCornerShape(42.dp),
        brush = Brush.linearGradient(
            colors = listOf(
                MaterialTheme.colorScheme.surfaceBright.copy(alpha = 0.98f),
                MaterialTheme.colorScheme.surfaceContainerHigh.copy(alpha = 0.90f),
                MaterialTheme.colorScheme.surfaceContainer.copy(alpha = 0.74f),
            ),
            start = Offset.Zero,
            end = Offset(1280f, 960f),
        ),
        innerGlow = AppThemeTokens.accentGlow.copy(alpha = 0.16f),
        highlightAlpha = 0.08f,
        baseShadowAlpha = 0.16f,
    )
}

@Composable
private fun FloatingGlassPanel(
    offsetY: androidx.compose.ui.unit.Dp,
) {
    FoundationSurface(
        modifier = Modifier
            .fillMaxWidth(0.72f)
            .height(132.dp)
            .offset(y = offsetY)
            .blur(0.35.dp),
        shape = RoundedCornerShape(32.dp),
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
        baseShadowAlpha = 0.06f,
        edgeHighlightAlpha = 0.16f,
        floating = true,
    )
}

@Composable
private fun MatteStage(
    offsetX: androidx.compose.ui.unit.Dp,
) {
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
        baseShadowAlpha = 0.18f,
        edgeHighlightAlpha = 0.08f,
    )
}

@Composable
private fun FoundationSurface(
    modifier: Modifier,
    shape: RoundedCornerShape,
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
    val graphiteLine = if (isDark) AppThemeTokens.graphiteLine else Color(0x120B1118)
    Surface(
        modifier = modifier
            .drawBehind {
                drawRoundRect(
                    brush = Brush.radialGradient(
                        colors = listOf(innerGlow, Color.Transparent),
                        center = Offset(size.width * 0.68f, size.height * 0.18f),
                        radius = size.maxDimension * 0.9f,
                    ),
                )
                drawRoundRect(
                    brush = Brush.verticalGradient(
                        colors = listOf(
                            Color.Black.copy(alpha = if (floating) baseShadowAlpha * 0.75f else baseShadowAlpha),
                            Color.Transparent,
                        ),
                        startY = size.height * 0.62f,
                    ),
                )
            },
        shape = shape,
        color = Color.Transparent,
        tonalElevation = 0.dp,
        shadowElevation = if (floating) 18.dp else 12.dp,
        border = BorderStroke(
            width = 1.dp,
            color = outline,
        ),
    ) {
        Box(
            modifier = Modifier
                .fillMaxSize()
                .background(brush)
                .drawBehind {
                    drawRect(
                        brush = Brush.verticalGradient(
                            colors = listOf(
                                Color.White.copy(alpha = highlightAlpha),
                                Color.Transparent,
                                Color.Black.copy(alpha = if (isDark) 0.10f else 0.04f),
                            ),
                        ),
                    )
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
                        topLeft = Offset(0f, 0f),
                    )
                    drawRect(
                        brush = Brush.verticalGradient(
                            colors = listOf(
                                frostLine.copy(alpha = edgeHighlightAlpha * 0.65f),
                                Color.Transparent,
                            ),
                            startY = 0f,
                            endY = size.height * 0.18f,
                        ),
                    )
                    drawRect(
                        brush = Brush.verticalGradient(
                            colors = listOf(
                                Color.Transparent,
                                graphiteLine.copy(alpha = 0.9f),
                            ),
                            startY = size.height * 0.78f,
                            endY = size.height,
                        ),
                    )
                },
            contentAlignment = Alignment.Center,
        ) {}
    }
}
