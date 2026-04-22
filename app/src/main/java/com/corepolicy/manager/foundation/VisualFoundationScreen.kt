package com.corepolicy.manager.foundation

import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
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

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.background),
    ) {
        AtmosphericBackdrop(glowScale = drift.value)
        AmbientRim(
            modifier = Modifier.align(Alignment.TopCenter),
            alpha = 0.18f,
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
            FloatingGlassPanel()
            MatteStage()
        }
        AmbientRim(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .offset(y = 96.dp),
            alpha = 0.10f,
            height = 260.dp,
        )
    }
}

@Composable
private fun AtmosphericBackdrop(
    glowScale: Float,
) {
    val tokens = AppThemeTokens
    val background = MaterialTheme.colorScheme.background
    Canvas(modifier = Modifier.fillMaxSize()) {
        drawRect(
            brush = Brush.verticalGradient(
                colors = listOf(
                    background,
                    tokens.backgroundMid,
                    tokens.backgroundEdge,
                ),
            ),
        )
        drawCircle(
            brush = Brush.radialGradient(
                colors = listOf(
                    tokens.accentGlow.copy(alpha = 0.34f),
                    Color.Transparent,
                ),
                center = Offset(size.width * 0.82f, size.height * 0.16f),
                radius = size.minDimension * 0.42f * glowScale,
            ),
        )
        drawCircle(
            brush = Brush.radialGradient(
                colors = listOf(
                    tokens.secondaryGlow.copy(alpha = 0.24f),
                    Color.Transparent,
                ),
                center = Offset(size.width * 0.16f, size.height * 0.78f),
                radius = size.minDimension * 0.50f,
            ),
        )
        drawRect(
            brush = Brush.radialGradient(
                colors = listOf(
                    tokens.edgeLight.copy(alpha = 0.08f),
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
                    tokens.edgeShadow.copy(alpha = 0.22f),
                    tokens.vignette.copy(alpha = 0.82f),
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
            color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.58f),
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
private fun FloatingGlassPanel() {
    FoundationSurface(
        modifier = Modifier
            .fillMaxWidth(0.72f)
            .height(132.dp)
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
        borderAlpha = 0.40f,
        innerGlow = Color.White.copy(alpha = 0.08f),
        highlightAlpha = 0.11f,
        baseShadowAlpha = 0.08f,
    )
}

@Composable
private fun MatteStage() {
    FoundationSurface(
        modifier = Modifier
            .fillMaxWidth()
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
) {
    val outline = MaterialTheme.colorScheme.outline.copy(alpha = borderAlpha)
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
                            Color.Black.copy(alpha = baseShadowAlpha),
                            Color.Transparent,
                        ),
                        startY = size.height * 0.62f,
                    ),
                )
            },
        shape = shape,
        color = Color.Transparent,
        tonalElevation = 0.dp,
        shadowElevation = 10.dp,
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
                                Color.Black.copy(alpha = 0.10f),
                            ),
                        ),
                    )
                    drawRect(
                        brush = Brush.horizontalGradient(
                            colors = listOf(
                                Color.White.copy(alpha = highlightAlpha * 0.85f),
                                Color.Transparent,
                                Color.Transparent,
                            ),
                            startX = 0f,
                            endX = size.width * 0.56f,
                        ),
                        topLeft = Offset(0f, 0f),
                    )
                },
            contentAlignment = Alignment.Center,
        ) {}
    }
}
