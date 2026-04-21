package com.corepolicy.manager.ui.components

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.corepolicy.manager.ui.R
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

private data class StaticInfoTile(
    val label: String,
    val value: String,
    val iconRes: Int
)

/**
 * 3x2 static system info grid. Uses the same tonal container token as the
 * metric grid so Overview reads as one cohesive system.
 */
@Composable
fun StaticSystemInfoSection(
    modifier: Modifier = Modifier,
    systemInfo: Triple<String, String, String>,
    runtimeInfo: Triple<String, String, String>
) {
    val tiles = listOf(
        StaticInfoTile(label = "Chip", value = systemInfo.first, iconRes = R.drawable.ic_cpu),
        StaticInfoTile(label = "Architecture", value = systemInfo.second, iconRes = R.drawable.ic_network),
        StaticInfoTile(label = "Kernel", value = systemInfo.third, iconRes = R.drawable.ic_schedule),
        StaticInfoTile(label = "Memory", value = runtimeInfo.first, iconRes = R.drawable.ic_memory),
        StaticInfoTile(label = "Governor", value = runtimeInfo.second, iconRes = R.drawable.ic_performance),
        StaticInfoTile(label = "Android", value = runtimeInfo.third, iconRes = R.drawable.ic_info)
    )
    DashboardPanel(
        modifier = modifier.fillMaxWidth(),
        verticalArrangement = Arrangement.spacedBy(10.dp)
    ) {
        tiles.chunked(2).forEach { rowTiles ->
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(10.dp)
            ) {
                rowTiles.forEach { tile ->
                    StaticInfoTileView(
                        modifier = Modifier.weight(1f),
                        tile = tile
                    )
                }
            }
        }
    }
}

@Composable
private fun StaticInfoTileView(
    modifier: Modifier = Modifier,
    tile: StaticInfoTile
) {
    val palette = LocalCorePolicyPalette.current
    DashboardInsetTile(modifier = modifier) {
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            DashboardIconBadge(iconRes = tile.iconRes, contentDescription = tile.label)
            Column(
                modifier = Modifier.weight(1f),
                verticalArrangement = Arrangement.spacedBy(2.dp)
            ) {
                Text(
                    text = tile.value,
                    style = MaterialTheme.typography.titleSmall,
                    color = palette.onSurface
                )
                Text(
                    text = tile.label,
                    style = MaterialTheme.typography.labelSmall,
                    color = palette.onSurfaceVariant
                )
            }
        }
    }
}
