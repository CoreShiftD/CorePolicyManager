package com.corepolicy.manager.ui.theme

import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.ReadOnlyComposable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Immutable
data class CorePolicySpacing(
    val nano: Dp,
    val xxs: Dp,
    val xs: Dp,
    val sm: Dp,
    val md: Dp,
    val lg: Dp,
    val xl: Dp,
    val xxl: Dp,
    val xxxl: Dp
)

@Immutable
data class CorePolicyRadii(
    val sm: Dp,
    val md: Dp,
    val lg: Dp,
    val xl: Dp,
    val full: Dp
)

@Immutable
data class CorePolicyElevation(
    val low: Dp,
    val medium: Dp,
    val high: Dp
)

@Immutable
data class CorePolicyStroke(
    val hairline: Dp,
    val thin: Dp,
    val medium: Dp
)

@Immutable
data class CorePolicyIconScale(
    val xs: Dp,
    val sm: Dp,
    val md: Dp,
    val lg: Dp,
    val xl: Dp
)

@Immutable
data class CorePolicyMotion(
    val quick: Int,
    val standard: Int,
    val emphasized: Int
)

@Immutable
data class CorePolicyEmphasis(
    val disabled: Float,
    val muted: Float,
    val soft: Float,
    val strong: Float
)

@Immutable
data class CorePolicyLayout(
    val compactBreakpoint: Int,
    val railBreakpoint: Int,
    val contentMaxWidth: Dp,
    val topBarHeight: Dp,
    val navBarHeight: Dp
)

@Immutable
data class CorePolicyTypeScale(
    val heroMetric: TextUnit,
    val technical: TextUnit,
    val micro: TextUnit
)

val LocalCorePolicySpacing = staticCompositionLocalOf {
    CorePolicySpacing(2.dp, 4.dp, 8.dp, 12.dp, 16.dp, 20.dp, 24.dp, 32.dp, 40.dp)
}
val LocalCorePolicyRadii = staticCompositionLocalOf {
    CorePolicyRadii(12.dp, 18.dp, 24.dp, 32.dp, 999.dp)
}
val LocalCorePolicyElevation = staticCompositionLocalOf {
    CorePolicyElevation(2.dp, 10.dp, 20.dp)
}
val LocalCorePolicyStroke = staticCompositionLocalOf {
    CorePolicyStroke(0.5.dp, 1.dp, 1.5.dp)
}
val LocalCorePolicyIconScale = staticCompositionLocalOf {
    CorePolicyIconScale(14.dp, 18.dp, 22.dp, 28.dp, 34.dp)
}
val LocalCorePolicyMotion = staticCompositionLocalOf {
    CorePolicyMotion(120, 240, 420)
}
val LocalCorePolicyEmphasis = staticCompositionLocalOf {
    CorePolicyEmphasis(disabled = 0.38f, muted = 0.62f, soft = 0.8f, strong = 0.92f)
}
val LocalCorePolicyLayout = staticCompositionLocalOf {
    CorePolicyLayout(
        compactBreakpoint = 420,
        railBreakpoint = 920,
        contentMaxWidth = 1320.dp,
        topBarHeight = 64.dp,
        navBarHeight = 76.dp
    )
}
val LocalCorePolicyTypeScale = staticCompositionLocalOf {
    CorePolicyTypeScale(heroMetric = 40.sp, technical = 12.sp, micro = 10.sp)
}

object CorePolicyDesign {
    val spacing: CorePolicySpacing
        @Composable
        @ReadOnlyComposable
        get() = LocalCorePolicySpacing.current

    val radii: CorePolicyRadii
        @Composable
        @ReadOnlyComposable
        get() = LocalCorePolicyRadii.current

    val elevation: CorePolicyElevation
        @Composable
        @ReadOnlyComposable
        get() = LocalCorePolicyElevation.current

    val stroke: CorePolicyStroke
        @Composable
        @ReadOnlyComposable
        get() = LocalCorePolicyStroke.current

    val icons: CorePolicyIconScale
        @Composable
        @ReadOnlyComposable
        get() = LocalCorePolicyIconScale.current

    val motion: CorePolicyMotion
        @Composable
        @ReadOnlyComposable
        get() = LocalCorePolicyMotion.current

    val emphasis: CorePolicyEmphasis
        @Composable
        @ReadOnlyComposable
        get() = LocalCorePolicyEmphasis.current

    val layout: CorePolicyLayout
        @Composable
        @ReadOnlyComposable
        get() = LocalCorePolicyLayout.current

    val type: CorePolicyTypeScale
        @Composable
        @ReadOnlyComposable
        get() = LocalCorePolicyTypeScale.current
}
