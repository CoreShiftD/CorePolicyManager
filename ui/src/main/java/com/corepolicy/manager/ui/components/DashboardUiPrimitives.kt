package com.corepolicy.manager.ui.components

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.ColorFilter
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

private val DashboardIconShape = RoundedCornerShape(10.dp)
private val DashboardChipShape = RoundedCornerShape(999.dp)
private val DashboardPanelShape = RoundedCornerShape(20.dp)
private val DashboardTileShape = RoundedCornerShape(16.dp)

/**
 * Small rounded-square icon badge used across the dashboard.
 *
 * Accepts optional [tintBg]/[tintFg] so callers can theme it semantically
 * (e.g. healthy-green for a CPU-calm badge). When omitted it falls back to
 * neutral surface + onSurfaceVariant.
 */
@Composable
fun DashboardIconBadge(
    iconRes: Int,
    contentDescription: String?,
    modifier: Modifier = Modifier,
    tintBg: Color? = null,
    tintFg: Color? = null
) {
    val palette = LocalCorePolicyPalette.current
    val bg = tintBg ?: palette.surfaceContainerHigh
    val fg = tintFg ?: palette.onSurfaceVariant
    Box(
        modifier = modifier
            .size(28.dp)
            .clip(DashboardIconShape)
            .background(bg),
        contentAlignment = Alignment.Center
    ) {
        Image(
            painter = painterResource(id = iconRes),
            contentDescription = contentDescription,
            modifier = Modifier.size(14.dp),
            colorFilter = ColorFilter.tint(fg)
        )
    }
}

@Composable
fun DashboardStatusChip(
    text: String,
    background: Color,
    content: Color,
    modifier: Modifier = Modifier
) {
    Text(
        text = text,
        modifier = modifier
            .clip(DashboardChipShape)
            .background(background)
            .padding(PaddingValues(horizontal = 10.dp, vertical = 4.dp)),
        color = content,
        style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.SemiBold)
    )
}

@Composable
fun DashboardPanel(
    modifier: Modifier = Modifier,
    contentPadding: PaddingValues = PaddingValues(horizontal = 16.dp, vertical = 14.dp),
    verticalArrangement: Arrangement.Vertical = Arrangement.spacedBy(10.dp),
    content: @Composable ColumnScope.() -> Unit
) {
    val palette = LocalCorePolicyPalette.current
    Column(
        modifier = modifier
            .clip(DashboardPanelShape)
            .background(palette.surfaceContainer)
            .border(1.dp, palette.divider, DashboardPanelShape)
            .padding(contentPadding),
        verticalArrangement = verticalArrangement,
        content = content
    )
}

@Composable
fun DashboardInsetTile(
    modifier: Modifier = Modifier,
    contentPadding: PaddingValues = PaddingValues(horizontal = 12.dp, vertical = 10.dp),
    verticalArrangement: Arrangement.Vertical = Arrangement.spacedBy(2.dp),
    content: @Composable ColumnScope.() -> Unit
) {
    val palette = LocalCorePolicyPalette.current
    Column(
        modifier = modifier
            .clip(DashboardTileShape)
            .background(palette.surfaceContainerHigh)
            .padding(contentPadding),
        verticalArrangement = verticalArrangement,
        content = content
    )
}
