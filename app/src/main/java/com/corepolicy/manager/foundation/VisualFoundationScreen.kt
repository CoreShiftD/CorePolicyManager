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
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
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

@Composable
fun VisualFoundationScreen() {
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
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = 24.dp, vertical = 30.dp),
            verticalArrangement = Arrangement.spacedBy(22.dp),
        ) {
            MinimalWordmark()
            Spacer(modifier = Modifier.height(10.dp))
            HeroSlab()
            FloatingGlassPanel()
            MatteStage()
        }
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
            brush = Brush.verticalGradient(
                colors = listOf(
                    Color.Transparent,
                    Color.Black.copy(alpha = 0.16f),
                    Color.Black.copy(alpha = 0.34f),
                ),
                startY = size.height * 0.32f,
            ),
        )
    }
}

@Composable
private fun MinimalWordmark() {
    Column(
        verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
        Text(
            text = "CORE POLICY",
            style = MaterialTheme.typography.labelSmall,
            color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.66f),
            fontWeight = FontWeight.Medium,
        )
        Text(
            text = "Visual foundation",
            style = MaterialTheme.typography.headlineMedium,
            color = MaterialTheme.colorScheme.onBackground,
        )
    }
}

@Composable
private fun HeroSlab() {
    FoundationSurface(
        modifier = Modifier
            .fillMaxWidth()
            .height(280.dp),
        shape = RoundedCornerShape(38.dp),
        brush = Brush.linearGradient(
            colors = listOf(
                MaterialTheme.colorScheme.surface.copy(alpha = 0.96f),
                MaterialTheme.colorScheme.surfaceContainerHigh.copy(alpha = 0.82f),
                MaterialTheme.colorScheme.surfaceContainer.copy(alpha = 0.66f),
            ),
            start = Offset.Zero,
            end = Offset(1200f, 900f),
        ),
        innerGlow = AppThemeTokens.accentGlow.copy(alpha = 0.14f),
    )
}

@Composable
private fun FloatingGlassPanel() {
    FoundationSurface(
        modifier = Modifier
            .fillMaxWidth(0.74f)
            .height(126.dp)
            .blur(0.2.dp),
        shape = RoundedCornerShape(30.dp),
        brush = Brush.linearGradient(
            colors = listOf(
                MaterialTheme.colorScheme.surfaceContainerHighest.copy(alpha = 0.60f),
                MaterialTheme.colorScheme.surface.copy(alpha = 0.30f),
            ),
            start = Offset.Zero,
            end = Offset(800f, 500f),
        ),
        borderAlpha = 0.32f,
        innerGlow = Color.White.copy(alpha = 0.06f),
    )
}

@Composable
private fun MatteStage() {
    FoundationSurface(
        modifier = Modifier
            .fillMaxWidth()
            .height(176.dp),
        shape = RoundedCornerShape(
            topStart = 32.dp,
            topEnd = 32.dp,
            bottomEnd = 48.dp,
            bottomStart = 24.dp,
        ),
        brush = Brush.verticalGradient(
            colors = listOf(
                MaterialTheme.colorScheme.surfaceContainer.copy(alpha = 0.82f),
                MaterialTheme.colorScheme.surfaceDim.copy(alpha = 0.94f),
            ),
        ),
        borderAlpha = 0.18f,
        innerGlow = AppThemeTokens.secondaryGlow.copy(alpha = 0.08f),
    )
}

@Composable
private fun FoundationSurface(
    modifier: Modifier,
    shape: RoundedCornerShape,
    brush: Brush,
    innerGlow: Color,
    borderAlpha: Float = 0.22f,
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
            },
        shape = shape,
        color = Color.Transparent,
        tonalElevation = 0.dp,
        shadowElevation = 0.dp,
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
                                Color.White.copy(alpha = 0.04f),
                                Color.Transparent,
                                Color.Black.copy(alpha = 0.10f),
                            ),
                        ),
                    )
                },
            contentAlignment = Alignment.Center,
        ) {}
    }
}
