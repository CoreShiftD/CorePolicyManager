package com.corepolicy.manager.core.designsystem.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider

@Composable
fun AppTheme(
    useDarkTheme: Boolean = isSystemInDarkTheme(),
    useDynamicColor: Boolean = true,
    preferAmoled: Boolean = true,
    content: @Composable () -> Unit,
) {
    val colorScheme = rememberAppColorScheme(
        useDarkTheme = useDarkTheme,
        useDynamicColor = useDynamicColor,
        preferAmoled = preferAmoled,
    )
    CompositionLocalProvider(
        LocalSpacing provides ComfortableSpacing,
        LocalAppShapes provides LocalAppShapes.current,
    ) {
        MaterialTheme(
            colorScheme = colorScheme,
            typography = CorePolicyTypography,
            shapes = CorePolicyShapes,
            content = content,
        )
    }
}
