package com.corepolicy.manager

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.ui.R
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

/* -------------------------------------------------------------------------- */
/*  SearchBar                                                                 */
/* -------------------------------------------------------------------------- */

/**
 * Pill-shaped search field with a leading search icon and an animated clear
 * button that appears when the query is non-empty. Uses [BasicTextField] so
 * the cursor and selection colors follow the palette primary.
 */
@Composable
fun SearchBar(
    query: String,
    onQueryChange: (String) -> Unit,
    placeholder: String = "Search",
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    val shape = RoundedCornerShape(CorePolicyDimens.chipRadius)
    Row(
        modifier = modifier
            .fillMaxWidth()
            .clip(shape)
            .background(palette.surfaceContainerHigh)
            .border(1.dp, palette.divider, shape)
            .padding(horizontal = 14.dp, vertical = 11.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            painter = painterResource(id = R.drawable.ic_info),
            contentDescription = null,
            tint = palette.onSurfaceVariant,
            modifier = Modifier.size(18.dp)
        )
        Spacer(Modifier.width(10.dp))
        BasicTextField(
            value = query,
            onValueChange = onQueryChange,
            singleLine = true,
            textStyle = MaterialTheme.typography.bodyLarge.copy(color = palette.onSurface),
            cursorBrush = SolidColor(palette.primary),
            modifier = Modifier.weight(1f),
            decorationBox = { inner ->
                Box {
                    if (query.isEmpty()) {
                        Text(
                            text = placeholder,
                            style = MaterialTheme.typography.bodyLarge,
                            color = palette.onSurfaceVariant
                        )
                    }
                    inner()
                }
            }
        )
        AnimatedVisibility(
            visible = query.isNotEmpty(),
            enter = fadeIn(tween(150)),
            exit = fadeOut(tween(150))
        ) {
            Box(
                modifier = Modifier
                    .size(20.dp)
                    .clip(CircleShape)
                    .background(palette.onSurfaceVariant.copy(alpha = 0.18f))
                    .clickable { onQueryChange("") }
                    .semantics { contentDescription = "Clear search" },
                contentAlignment = Alignment.Center
            ) {
                Text(
                    text = "×",
                    style = MaterialTheme.typography.labelLarge,
                    color = palette.onSurfaceVariant
                )
            }
        }
    }
}

/* -------------------------------------------------------------------------- */
/*  Shimmer — simple loading state modifier                                   */
/* -------------------------------------------------------------------------- */

@Composable
fun ShimmerPlaceholder(
    modifier: Modifier = Modifier,
    shape: Shape = RoundedCornerShape(8.dp)
) {
    val palette = LocalCorePolicyPalette.current
    val transition = rememberInfiniteTransition(label = "shimmer")
    val progress by transition.animateFloat(
        initialValue = 0f,
        targetValue = 1f,
        animationSpec = infiniteRepeatable(
            animation = tween(1400, easing = LinearEasing)
        ),
        label = "shimmerProgress"
    )
    val base = palette.surfaceContainerHigh
    val highlight = palette.surfaceContainerHighest
    val start = Offset(x = -600f + progress * 1800f, y = 0f)
    val end = Offset(x = start.x + 600f, y = 0f)
    val brush = Brush.linearGradient(
        colors = listOf(base, highlight, base),
        start = start,
        end = end
    )
    Box(
        modifier = modifier
            .clip(shape)
            .background(brush)
    )
}

@Composable
fun ShimmerRow(modifier: Modifier = Modifier) {
    Row(modifier = modifier.fillMaxWidth()) {
        ShimmerPlaceholder(modifier = Modifier.size(36.dp))
        androidx.compose.foundation.layout.Spacer(Modifier.width(12.dp))
        androidx.compose.foundation.layout.Column(
            modifier = Modifier.fillMaxWidth(),
            verticalArrangement = androidx.compose.foundation.layout.Arrangement.spacedBy(6.dp)
        ) {
            ShimmerPlaceholder(modifier = Modifier
                .fillMaxWidth(0.6f)
                .height(12.dp))
            ShimmerPlaceholder(modifier = Modifier
                .fillMaxWidth(0.3f)
                .height(10.dp))
        }
    }
}
