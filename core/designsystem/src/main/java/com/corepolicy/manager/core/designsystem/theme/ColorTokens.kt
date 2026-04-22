package com.corepolicy.manager.core.designsystem.theme

import android.os.Build
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

object AppThemeTokens {
    val Black = Color(0xFF000000)
    val Night = Color(0xFF05070A)
    val Ink = Color(0xFF0B0F14)
    val Shell = Color(0xFF10151D)
    val ShellHigh = Color(0xFF171D27)
    val ShellSoft = Color(0xFF0E131A)
    val Slate = Color(0xFF8E97A5)
    val SlateBright = Color(0xFFAAB3C1)
    val Frost = Color(0xFFE7ECF3)
    val Halo = Color(0xFF8FB5FF)
    val Bloom = Color(0xFF79E0D0)
    val Cinder = Color(0xFF151B23)
    val backgroundMid = Color(0xFF060A10)
    val backgroundEdge = Color(0xFF020304)
    val vignette = Color(0xFF010203)
    val accentGlow = Color(0xFF89AFFF)
    val secondaryGlow = Color(0xFF5AD7C8)
    val edgeLight = Color(0x33F3F7FF)
    val edgeShadow = Color(0xCC020305)
    val paper = Color(0xFFF5F7FB)
    val mist = Color(0xFFE9EEF5)
    val frostLine = Color(0x66FFFFFF)
    val graphiteLine = Color(0x140D1117)
}

private val FallbackLightColors = lightColorScheme(
    primary = Color(0xFF2D4F90),
    onPrimary = Color.White,
    secondary = Color(0xFF2A6B64),
    onSecondary = Color.White,
    tertiary = Color(0xFF5D5F8F),
    onTertiary = Color.White,
    background = Color(0xFFF7F8FB),
    onBackground = Color(0xFF111418),
    surface = Color(0xFFFFFFFF),
    onSurface = Color(0xFF14181D),
    surfaceVariant = Color(0xFFE6EAF0),
    onSurfaceVariant = Color(0xFF49515C),
    outline = Color(0xFF89919D),
)

private val FallbackDarkColors = darkColorScheme(
    primary = Color(0xFFC5D7FF),
    onPrimary = Color(0xFF0E1C35),
    secondary = Color(0xFFA6F0E5),
    onSecondary = Color(0xFF032B28),
    tertiary = Color(0xFFD7D8FF),
    onTertiary = Color(0xFF20214A),
    background = AppThemeTokens.Black,
    onBackground = AppThemeTokens.Frost,
    surface = AppThemeTokens.Ink,
    onSurface = AppThemeTokens.Frost,
    surfaceVariant = AppThemeTokens.Cinder,
    onSurfaceVariant = AppThemeTokens.Slate,
    outline = Color(0xFF35404F),
)

@Composable
fun rememberAppColorScheme(
    useDarkTheme: Boolean,
    useDynamicColor: Boolean,
    preferAmoled: Boolean,
): ColorScheme {
    val context = LocalContext.current
    val base = when {
        useDynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S && useDarkTheme ->
            dynamicDarkColorScheme(context)
        useDynamicColor && Build.VERSION.SDK_INT >= Build.VERSION_CODES.S ->
            dynamicLightColorScheme(context)
        useDarkTheme -> FallbackDarkColors
        else -> FallbackLightColors
    }
    return if (useDarkTheme) {
        base.refinedDark(preferAmoled = preferAmoled)
    } else {
        base.refinedLight()
    }
}

private fun ColorScheme.refinedDark(
    preferAmoled: Boolean,
): ColorScheme {
    val backgroundBase = if (preferAmoled) AppThemeTokens.Black else AppThemeTokens.Night
    val surfaceBase = if (preferAmoled) Color(0xFF050608) else Color(0xFF0A0E13)
    return copy(
        background = backgroundBase,
        surface = surfaceBase,
        surfaceDim = backgroundBase,
        surfaceBright = AppThemeTokens.ShellHigh,
        surfaceContainerLowest = backgroundBase,
        surfaceContainerLow = AppThemeTokens.ShellSoft,
        surfaceContainer = Color(0xFF0D1218),
        surfaceContainerHigh = Color(0xFF121924),
        surfaceContainerHighest = Color(0xFF18202C),
        surfaceVariant = Color(0xFF161D26),
        inverseSurface = Color(0xFFE3E8F0),
        inverseOnSurface = Color(0xFF10141A),
        onBackground = AppThemeTokens.Frost,
        onSurface = AppThemeTokens.Frost,
        onSurfaceVariant = AppThemeTokens.SlateBright,
        outline = Color(0xFF34404D),
    )
}

private fun ColorScheme.refinedLight(): ColorScheme {
    return copy(
        background = AppThemeTokens.paper,
        surface = Color(0xFFFEFFFF),
        surfaceDim = Color(0xFFDDE5EE),
        surfaceBright = Color(0xFFFFFFFF),
        surfaceContainerLowest = Color(0xFFFFFFFF),
        surfaceContainerLow = Color(0xFFF7F9FC),
        surfaceContainer = Color(0xFFF0F4F9),
        surfaceContainerHigh = Color(0xFFE9EFF6),
        surfaceContainerHighest = AppThemeTokens.mist,
        surfaceVariant = AppThemeTokens.mist,
        onBackground = Color(0xFF111418),
        onSurface = Color(0xFF14181D),
        onSurfaceVariant = Color(0xFF4A5561),
        outline = Color(0xFF8C98A6),
    )
}
