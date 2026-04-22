package com.corepolicy.manager.core.designsystem.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider

@Composable
fun AppTheme(
    useDarkTheme: Boolean = isSystemInDarkTheme(),
    compactMode: Boolean = false,
    content: @Composable () -> Unit,
) {
    CompositionLocalProvider(
        LocalSpacing provides if (compactMode) CompactSpacing else ComfortableSpacing,
        LocalAppShapes provides LocalAppShapes.current,
    ) {
        MaterialTheme(
            colorScheme = if (useDarkTheme) CorePolicyDarkColors else CorePolicyLightColors,
            typography = CorePolicyTypography,
            shapes = CorePolicyShapes,
            content = content,
        )
    }
}
