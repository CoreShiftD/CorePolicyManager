package com.corepolicy.manager.app.navigation

import androidx.compose.runtime.Immutable
import com.corepolicy.manager.core.designsystem.component.AppIconGlyph

@Immutable
enum class AppDestination(
    val label: String,
    val icon: AppIconGlyph,
) {
    Overview(label = "Overview", icon = AppIconGlyph.Overview),
    Daemon(label = "Daemon", icon = AppIconGlyph.Daemon),
    Policies(label = "Policies", icon = AppIconGlyph.Policies),
    Settings(label = "Settings", icon = AppIconGlyph.Settings),
}
