package com.corepolicy.manager.core.designsystem.component

import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Dashboard
import androidx.compose.material.icons.rounded.Dns
import androidx.compose.material.icons.rounded.Policy
import androidx.compose.material.icons.rounded.Settings
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector

enum class AppIconGlyph(val vector: ImageVector) {
    Overview(Icons.Rounded.Dashboard),
    Daemon(Icons.Rounded.Dns),
    Policies(Icons.Rounded.Policy),
    Settings(Icons.Rounded.Settings),
}

@Composable
fun AppIcon(
    glyph: AppIconGlyph,
    contentDescription: String?,
    modifier: Modifier = Modifier,
) {
    Icon(
        imageVector = glyph.vector,
        contentDescription = contentDescription,
        tint = MaterialTheme.colorScheme.onSurface,
        modifier = modifier,
    )
}
