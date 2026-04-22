package com.corepolicy.manager.core.designsystem.theme

import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

@Immutable
data class SpacingScale(
    val xSmall: Dp,
    val small: Dp,
    val medium: Dp,
    val large: Dp,
    val xLarge: Dp,
    val xxLarge: Dp,
)

val ComfortableSpacing = SpacingScale(
    xSmall = 8.dp,
    small = 16.dp,
    medium = 24.dp,
    large = 32.dp,
    xLarge = 44.dp,
    xxLarge = 64.dp,
)

val LocalSpacing = staticCompositionLocalOf { ComfortableSpacing }
