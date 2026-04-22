package com.corepolicy.manager.core.designsystem.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Shapes
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.unit.dp

val CorePolicyShapes = Shapes(
    extraSmall = RoundedCornerShape(10.dp),
    small = RoundedCornerShape(16.dp),
    medium = RoundedCornerShape(22.dp),
    large = RoundedCornerShape(28.dp),
    extraLarge = RoundedCornerShape(36.dp),
)

@Immutable
data class AppShapeScale(
    val chip: RoundedCornerShape,
    val panel: RoundedCornerShape,
    val hero: RoundedCornerShape,
)

val LocalAppShapes = staticCompositionLocalOf {
    AppShapeScale(
        chip = RoundedCornerShape(16.dp),
        panel = RoundedCornerShape(24.dp),
        hero = RoundedCornerShape(32.dp),
    )
}
