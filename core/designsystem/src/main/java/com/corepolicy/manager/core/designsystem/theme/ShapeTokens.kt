package com.corepolicy.manager.core.designsystem.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Shapes
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.unit.dp

val CorePolicyShapes = Shapes(
    extraSmall = RoundedCornerShape(12.dp),
    small = RoundedCornerShape(20.dp),
    medium = RoundedCornerShape(28.dp),
    large = RoundedCornerShape(36.dp),
    extraLarge = RoundedCornerShape(44.dp),
)

@Immutable
data class AppShapeScale(
    val compact: RoundedCornerShape,
    val panel: RoundedCornerShape,
    val slab: RoundedCornerShape,
    val hero: RoundedCornerShape,
)

val LocalAppShapes = staticCompositionLocalOf {
    AppShapeScale(
        compact = RoundedCornerShape(18.dp),
        panel = RoundedCornerShape(30.dp),
        slab = RoundedCornerShape(36.dp),
        hero = RoundedCornerShape(42.dp),
    )
}
