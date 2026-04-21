package com.corepolicy.manager.ui.theme

import android.app.Activity
import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.lerp
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalView
import androidx.core.view.WindowCompat

/**
 * CorePolicy palette — curated surface tokens derived from the active M3 [ColorScheme]
 * so Material defaults stay correct while our own composables get consistent surfaces.
 *
 * Legacy fields ([cardSurface], [metricCardSurface], [rowSurface], [selectedRowSurface])
 * are kept so existing screens compile, but they now resolve to proper tonal
 * surface-container levels instead of alpha-faded guesses.
 */
@Immutable
data class CorePolicyPalette(
    val isDark: Boolean,
    val isAmoled: Boolean,
    val background: Color,
    val surface: Color,
    val surfaceContainerLow: Color,
    val surfaceContainer: Color,
    val surfaceContainerHigh: Color,
    val surfaceContainerHighest: Color,
    val cardSurface: Color,
    val metricCardSurface: Color,
    val rowSurface: Color,
    val selectedRowSurface: Color,
    val stroke: Color,
    val divider: Color,
    val primary: Color,
    val onPrimary: Color,
    val primaryContainer: Color,
    val onPrimaryContainer: Color,
    val secondaryContainer: Color,
    val onSecondaryContainer: Color,
    val tertiaryContainer: Color,
    val onTertiaryContainer: Color,
    val errorContainer: Color,
    val onErrorContainer: Color,
    val performanceContainer: Color,
    val onPerformanceContainer: Color,
    val balancedContainer: Color,
    val onBalancedContainer: Color,
    val efficiencyContainer: Color,
    val onEfficiencyContainer: Color,
    val onSurface: Color,
    val onSurfaceVariant: Color,
    val iconContainer: Color,
    val selectedIconContainer: Color,
    val onIconContainer: Color
)

val LocalCorePolicyPalette = staticCompositionLocalOf {
    CorePolicyPalette(
        isDark = true,
        isAmoled = false,
        background = Color(0xFF0B0F17),
        surface = Color(0xFF0B0F17),
        surfaceContainerLow = Color(0xFF11161F),
        surfaceContainer = Color(0xFF151B26),
        surfaceContainerHigh = Color(0xFF1B2330),
        surfaceContainerHighest = Color(0xFF222B3A),
        cardSurface = Color(0xFF151B26),
        metricCardSurface = Color(0xFF1B2330),
        rowSurface = Color(0xFF1B2330),
        selectedRowSurface = Color(0xFF223A55),
        stroke = Color(0x22FFFFFF),
        divider = Color(0x14FFFFFF),
        primary = Color(0xFF8FB7FF),
        onPrimary = Color(0xFF002B66),
        primaryContainer = Color(0xFF1E3A66),
        onPrimaryContainer = Color(0xFFD5E3FF),
        secondaryContainer = Color(0xFF2A3E5A),
        onSecondaryContainer = Color(0xFFD9E7FF),
        tertiaryContainer = Color(0xFF2B4A44),
        onTertiaryContainer = Color(0xFFD8F4EE),
        errorContainer = Color(0xFF5A2B33),
        onErrorContainer = Color(0xFFFFDAD6),
        performanceContainer = Color(0xFF2F4968),
        onPerformanceContainer = Color(0xFFE8F2FF),
        balancedContainer = Color(0xFF2A3E5A),
        onBalancedContainer = Color(0xFFD9E7FF),
        efficiencyContainer = Color(0xFF2B4A44),
        onEfficiencyContainer = Color(0xFFD8F4EE),
        onSurface = Color(0xFFF3F5FA),
        onSurfaceVariant = Color(0xFFB5BECF),
        iconContainer = Color(0xFF2A3950),
        selectedIconContainer = Color(0xFF2F4968),
        onIconContainer = Color(0xFFE2EAF7)
    )
}

/** Brand fallback schemes used when dynamic color is unavailable (API < 31). */
private val FallbackDarkScheme = darkColorScheme(
    primary = Color(0xFF8FB7FF),
    onPrimary = Color(0xFF002B66),
    primaryContainer = Color(0xFF1E3A66),
    onPrimaryContainer = Color(0xFFD5E3FF),
    secondary = Color(0xFF9EC3D8),
    onSecondary = Color(0xFF003547),
    secondaryContainer = Color(0xFF1E4B5E),
    onSecondaryContainer = Color(0xFFCDE5F3),
    tertiary = Color(0xFF9FD8C5),
    onTertiary = Color(0xFF003828),
    tertiaryContainer = Color(0xFF1B4A3C),
    onTertiaryContainer = Color(0xFFBFF0DF),
    background = Color(0xFF0B0F17),
    onBackground = Color(0xFFE8ECF4),
    surface = Color(0xFF0B0F17),
    onSurface = Color(0xFFE8ECF4),
    surfaceVariant = Color(0xFF1B2330),
    onSurfaceVariant = Color(0xFFB5BECF),
    outline = Color(0xFF3A4557),
    outlineVariant = Color(0xFF222B3A)
)

private val FallbackLightScheme = lightColorScheme(
    primary = Color(0xFF1F5FD1),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFFD9E4FF),
    onPrimaryContainer = Color(0xFF001A47),
    secondary = Color(0xFF1E5973),
    onSecondary = Color(0xFFFFFFFF),
    secondaryContainer = Color(0xFFCEE6F3),
    onSecondaryContainer = Color(0xFF051F2C),
    tertiary = Color(0xFF1C6B53),
    onTertiary = Color(0xFFFFFFFF),
    tertiaryContainer = Color(0xFFBFF0DF),
    onTertiaryContainer = Color(0xFF002418),
    background = Color(0xFFF6F8FB),
    onBackground = Color(0xFF0D1220),
    surface = Color(0xFFF6F8FB),
    onSurface = Color(0xFF0D1220),
    surfaceVariant = Color(0xFFE2E6EF),
    onSurfaceVariant = Color(0xFF47505F),
    outline = Color(0xFF8B94A4),
    outlineVariant = Color(0xFFD6DCE6)
)

private fun ColorScheme.toAmoled(): ColorScheme = copy(
    background = Color.Black,
    surface = Color.Black
)

private fun buildPalette(scheme: ColorScheme, darkTheme: Boolean, amoled: Boolean): CorePolicyPalette {
    val tint = scheme.surfaceTint
    val base = scheme.surface
    val sLow = lerp(base, tint, if (darkTheme) 0.05f else 0.04f)
    val sContainer = lerp(base, tint, if (darkTheme) 0.09f else 0.07f)
    val sHigh = lerp(base, tint, if (darkTheme) 0.13f else 0.11f)
    val sHighest = lerp(base, tint, if (darkTheme) 0.18f else 0.15f)

    val selectedRow = if (darkTheme) {
        lerp(scheme.primaryContainer, scheme.surface, 0.25f)
    } else {
        lerp(scheme.primaryContainer, scheme.surface, 0.15f)
    }

    return CorePolicyPalette(
        isDark = darkTheme,
        isAmoled = amoled,
        background = scheme.background,
        surface = scheme.surface,
        surfaceContainerLow = if (amoled) Color(0xFF0A0A0A) else sLow,
        surfaceContainer = if (amoled) Color(0xFF101010) else sContainer,
        surfaceContainerHigh = if (amoled) Color(0xFF161616) else sHigh,
        surfaceContainerHighest = if (amoled) Color(0xFF1D1D1D) else sHighest,
        cardSurface = if (amoled) Color(0xFF101010) else sContainer,
        metricCardSurface = if (amoled) Color(0xFF141414) else sHigh,
        rowSurface = if (amoled) Color(0xFF121212) else sContainer,
        selectedRowSurface = selectedRow,
        stroke = scheme.outlineVariant.copy(alpha = if (darkTheme) 0.22f else 0.35f),
        divider = scheme.outlineVariant.copy(alpha = if (darkTheme) 0.18f else 0.32f),
        primary = scheme.primary,
        onPrimary = scheme.onPrimary,
        primaryContainer = scheme.primaryContainer,
        onPrimaryContainer = scheme.onPrimaryContainer,
        secondaryContainer = scheme.secondaryContainer,
        onSecondaryContainer = scheme.onSecondaryContainer,
        tertiaryContainer = scheme.tertiaryContainer,
        onTertiaryContainer = scheme.onTertiaryContainer,
        errorContainer = scheme.errorContainer,
        onErrorContainer = scheme.onErrorContainer,
        performanceContainer = scheme.primaryContainer,
        onPerformanceContainer = scheme.onPrimaryContainer,
        balancedContainer = scheme.secondaryContainer,
        onBalancedContainer = scheme.onSecondaryContainer,
        efficiencyContainer = scheme.tertiaryContainer,
        onEfficiencyContainer = scheme.onTertiaryContainer,
        onSurface = scheme.onSurface,
        onSurfaceVariant = if (darkTheme) scheme.onSurfaceVariant else lerp(scheme.onSurfaceVariant, scheme.onSurface, 0.2f),
        iconContainer = scheme.secondaryContainer,
        selectedIconContainer = scheme.primaryContainer,
        onIconContainer = scheme.onSecondaryContainer
    )
}

@Composable
fun CorePolicyTheme(
    darkTheme: Boolean = isSystemInDarkTheme(),
    dynamicColor: Boolean = true,
    amoled: Boolean = false,
    content: @Composable () -> Unit
) {
    val context = LocalContext.current
    val baseScheme = when {
        dynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S -> {
            if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        }
        darkTheme -> FallbackDarkScheme
        else -> FallbackLightScheme
    }

    val amoledActive = amoled && darkTheme
    val scheme = if (amoledActive) baseScheme.toAmoled() else baseScheme
    val palette = buildPalette(scheme, darkTheme, amoledActive)
    val semantic = if (darkTheme) DarkSemanticColors else LightSemanticColors

    val view = LocalView.current
    if (!view.isInEditMode) {
        SideEffect {
            val window = (view.context as? Activity)?.window
            if (window != null) {
                WindowCompat.setDecorFitsSystemWindows(window, false)
                val controller = WindowCompat.getInsetsController(window, view)
                val surfaceLum = scheme.surface.luminance()
                controller.isAppearanceLightStatusBars = surfaceLum > 0.5f
                controller.isAppearanceLightNavigationBars = surfaceLum > 0.5f
            }
        }
    }

    MaterialTheme(colorScheme = scheme, typography = CorePolicyTypography) {
        CompositionLocalProvider(
            LocalCorePolicyPalette provides palette,
            LocalSemanticColors provides semantic,
            content = content
        )
    }
}
