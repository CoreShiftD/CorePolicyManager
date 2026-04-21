package com.corepolicy.manager.ui.theme

import android.app.Activity
import android.os.Build
import android.view.WindowManager
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
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsControllerCompat

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
    val backgroundAccent: Color,
    val surface: Color,
    val surfaceContainerLow: Color,
    val surfaceContainer: Color,
    val surfaceContainerHigh: Color,
    val surfaceContainerHighest: Color,
    val surfaceRaised: Color,
    val surfaceOverlay: Color,
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
        backgroundAccent = Color(0xFF132033),
        surface = Color(0xFF0B0F17),
        surfaceContainerLow = Color(0xFF11161F),
        surfaceContainer = Color(0xFF151B26),
        surfaceContainerHigh = Color(0xFF1B2330),
        surfaceContainerHighest = Color(0xFF222B3A),
        surfaceRaised = Color(0xFF202A39),
        surfaceOverlay = Color(0xFF243246),
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

private val FallbackDarkScheme = darkColorScheme(
    primary = Color(0xFF7DB1FF),
    onPrimary = Color(0xFF00295C),
    primaryContainer = Color(0xFF1A3B69),
    onPrimaryContainer = Color(0xFFD8E7FF),
    secondary = Color(0xFF7FD3C6),
    onSecondary = Color(0xFF003730),
    secondaryContainer = Color(0xFF184B46),
    onSecondaryContainer = Color(0xFFC9F4ED),
    tertiary = Color(0xFFE0C16F),
    onTertiary = Color(0xFF3B2D00),
    tertiaryContainer = Color(0xFF584300),
    onTertiaryContainer = Color(0xFFFFEAB1),
    background = Color(0xFF09111C),
    onBackground = Color(0xFFE9EEF8),
    surface = Color(0xFF09111C),
    onSurface = Color(0xFFE9EEF8),
    surfaceVariant = Color(0xFF172332),
    onSurfaceVariant = Color(0xFFB5C2D6),
    outline = Color(0xFF415069),
    outlineVariant = Color(0xFF233142)
)

private val FallbackLightScheme = lightColorScheme(
    primary = Color(0xFF0D63D7),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFFDCE8FF),
    onPrimaryContainer = Color(0xFF001D4D),
    secondary = Color(0xFF10655C),
    onSecondary = Color(0xFFFFFFFF),
    secondaryContainer = Color(0xFFC5F0E7),
    onSecondaryContainer = Color(0xFF00201C),
    tertiary = Color(0xFF7A5900),
    onTertiary = Color(0xFFFFFFFF),
    tertiaryContainer = Color(0xFFFFE7A9),
    onTertiaryContainer = Color(0xFF261A00),
    background = Color(0xFFF2F6FB),
    onBackground = Color(0xFF101721),
    surface = Color(0xFFF2F6FB),
    onSurface = Color(0xFF101721),
    surfaceVariant = Color(0xFFDEE7F2),
    onSurfaceVariant = Color(0xFF4A5567),
    outline = Color(0xFF7A879A),
    outlineVariant = Color(0xFFD0DAE5)
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
    val sRaised = lerp(sHighest, tint, if (darkTheme) 0.08f else 0.04f)
    val sOverlay = lerp(sHighest, tint, if (darkTheme) 0.16f else 0.08f)

    val selectedRow = if (darkTheme) {
        lerp(scheme.primaryContainer, scheme.surface, 0.25f)
    } else {
        lerp(scheme.primaryContainer, scheme.surface, 0.15f)
    }

    return CorePolicyPalette(
        isDark = darkTheme,
        isAmoled = amoled,
        background = scheme.background,
        backgroundAccent = lerp(scheme.primaryContainer, scheme.background, if (darkTheme) 0.55f else 0.78f),
        surface = scheme.surface,
        surfaceContainerLow = if (amoled) Color(0xFF0A0A0A) else sLow,
        surfaceContainer = if (amoled) Color(0xFF101010) else sContainer,
        surfaceContainerHigh = if (amoled) Color(0xFF161616) else sHigh,
        surfaceContainerHighest = if (amoled) Color(0xFF1D1D1D) else sHighest,
        surfaceRaised = if (amoled) Color(0xFF181818) else sRaised,
        surfaceOverlay = if (amoled) Color(0xFF202020) else sOverlay,
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
                window.statusBarColor = android.graphics.Color.TRANSPARENT
                window.navigationBarColor = android.graphics.Color.TRANSPARENT
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    window.isNavigationBarContrastEnforced = false
                }
                window.clearFlags(WindowManager.LayoutParams.FLAG_TRANSLUCENT_STATUS)
                val controller = WindowInsetsControllerCompat(window, view)
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
            LocalCorePolicySpacing provides CorePolicySpacing(2.dp, 4.dp, 8.dp, 12.dp, 16.dp, 20.dp, 24.dp, 32.dp, 40.dp),
            LocalCorePolicyRadii provides CorePolicyRadii(12.dp, 18.dp, 24.dp, 32.dp, 999.dp),
            LocalCorePolicyElevation provides CorePolicyElevation(2.dp, 10.dp, 20.dp),
            LocalCorePolicyStroke provides CorePolicyStroke(0.5.dp, 1.dp, 1.5.dp),
            LocalCorePolicyIconScale provides CorePolicyIconScale(14.dp, 18.dp, 22.dp, 28.dp, 34.dp),
            LocalCorePolicyMotion provides CorePolicyMotion(120, 240, 420),
            LocalCorePolicyEmphasis provides CorePolicyEmphasis(0.38f, 0.62f, 0.8f, 0.92f),
            LocalCorePolicyLayout provides CorePolicyLayout(
                compactBreakpoint = 420,
                railBreakpoint = 920,
                contentMaxWidth = 1320.dp,
                topBarHeight = 64.dp,
                navBarHeight = 76.dp
            ),
            LocalCorePolicyTypeScale provides CorePolicyTypeScale(heroMetric = 40.sp, technical = 12.sp, micro = 10.sp),
            content = content
        )
    }
}
