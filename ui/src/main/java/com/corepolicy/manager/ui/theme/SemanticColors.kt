package com.corepolicy.manager.ui.theme

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color

/**
 * Semantic role colors that do not live on the M3 ColorScheme.
 *
 * Use these for state signaling (healthy / warning / conflict / info) so that
 * meaning stays consistent across modules, logs, metric cards and chips — and
 * so it stays readable in both dynamic color and AMOLED modes.
 */
@Immutable
data class SemanticColors(
    val healthy: Color,
    val onHealthy: Color,
    val healthyContainer: Color,
    val onHealthyContainer: Color,
    val warning: Color,
    val onWarning: Color,
    val warningContainer: Color,
    val onWarningContainer: Color,
    val conflict: Color,
    val onConflict: Color,
    val conflictContainer: Color,
    val onConflictContainer: Color,
    val info: Color,
    val onInfo: Color,
    val infoContainer: Color,
    val onInfoContainer: Color
)

val LightSemanticColors = SemanticColors(
    healthy = Color(0xFF1E7F3B),
    onHealthy = Color(0xFFFFFFFF),
    healthyContainer = Color(0xFFD7F5DE),
    onHealthyContainer = Color(0xFF0E3F1D),
    warning = Color(0xFF8D5A00),
    onWarning = Color(0xFFFFFFFF),
    warningContainer = Color(0xFFFFE9C2),
    onWarningContainer = Color(0xFF4A2E00),
    conflict = Color(0xFFB3261E),
    onConflict = Color(0xFFFFFFFF),
    conflictContainer = Color(0xFFFCD8D4),
    onConflictContainer = Color(0xFF601410),
    info = Color(0xFF1F6FEB),
    onInfo = Color(0xFFFFFFFF),
    infoContainer = Color(0xFFD9E8FF),
    onInfoContainer = Color(0xFF0C2D66)
)

val DarkSemanticColors = SemanticColors(
    healthy = Color(0xFF7CD893),
    onHealthy = Color(0xFF07351A),
    healthyContainer = Color(0xFF15421F),
    onHealthyContainer = Color(0xFFBFF0C9),
    warning = Color(0xFFFFD478),
    onWarning = Color(0xFF3A2600),
    warningContainer = Color(0xFF4A3A18),
    onWarningContainer = Color(0xFFFFE2A8),
    conflict = Color(0xFFFF8A80),
    onConflict = Color(0xFF4A0E08),
    conflictContainer = Color(0xFF5A1F1A),
    onConflictContainer = Color(0xFFFFDAD6),
    info = Color(0xFF7FB6FF),
    onInfo = Color(0xFF002B66),
    infoContainer = Color(0xFF1E3A66),
    onInfoContainer = Color(0xFFD5E3FF)
)

val LocalSemanticColors = staticCompositionLocalOf { DarkSemanticColors }

object CorePolicySemantics {
    val colors: SemanticColors
        @Composable
        @ReadOnlyComposable
        get() = LocalSemanticColors.current
}
