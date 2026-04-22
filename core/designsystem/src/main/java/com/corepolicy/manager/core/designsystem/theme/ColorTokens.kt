package com.corepolicy.manager.core.designsystem.theme

import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.ui.graphics.Color

private val Ink = Color(0xFF161A1D)
private val Bone = Color(0xFFF7F3EE)
private val Clay = Color(0xFFE4D6C8)
private val Slate = Color(0xFF5F6873)
private val Moss = Color(0xFF4F6B53)
private val Ember = Color(0xFFC96A3D)
private val Mist = Color(0xFFEDF1F4)
private val Night = Color(0xFF121416)

val CorePolicyLightColors = lightColorScheme(
    primary = Ink,
    onPrimary = Bone,
    secondary = Moss,
    onSecondary = Bone,
    tertiary = Ember,
    onTertiary = Bone,
    background = Bone,
    onBackground = Ink,
    surface = Color(0xFFFFFCF8),
    onSurface = Ink,
    surfaceVariant = Mist,
    onSurfaceVariant = Slate,
    outline = Color(0xFFD0C3B7),
)

val CorePolicyDarkColors = darkColorScheme(
    primary = Bone,
    onPrimary = Night,
    secondary = Color(0xFFA8C4A9),
    onSecondary = Night,
    tertiary = Color(0xFFFFB289),
    onTertiary = Night,
    background = Night,
    onBackground = Bone,
    surface = Color(0xFF181D21),
    onSurface = Bone,
    surfaceVariant = Color(0xFF222A31),
    onSurfaceVariant = Color(0xFFB4C0C7),
    outline = Color(0xFF3A454D),
)
