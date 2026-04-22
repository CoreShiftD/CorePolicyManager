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
    xSmall = 6.dp,
    small = 12.dp,
    medium = 18.dp,
    large = 24.dp,
    xLarge = 32.dp,
    xxLarge = 40.dp,
)

val CompactSpacing = SpacingScale(
    xSmall = 4.dp,
    small = 10.dp,
    medium = 14.dp,
    large = 20.dp,
    xLarge = 28.dp,
    xxLarge = 36.dp,
)

val LocalSpacing = staticCompositionLocalOf { ComfortableSpacing }
