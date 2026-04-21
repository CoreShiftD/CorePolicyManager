package com.corepolicy.manager

import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.ripple
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.ui.theme.CorePolicyDesign
import com.corepolicy.manager.ui.theme.CorePolicySemantics
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

object CorePolicyDimens {
    val screenHorizontal = 20.dp
    val screenTop = 8.dp
    val screenBottom = 104.dp
    val sectionGap = 20.dp
    val cardGap = 12.dp
    val cardPaddingH = 16.dp
    val cardPaddingV = 14.dp
    val cardRadius = 24.dp
    val cardRadiusTight = 18.dp
    val chipRadius = 999.dp
    val iconBadge = 34.dp
}

object ControlCenterDimens {
    val sectionGap = CorePolicyDimens.cardGap
    val cardRadius = CorePolicyDimens.cardRadius
    val chipRadius = CorePolicyDimens.chipRadius
    val screenHorizontal = CorePolicyDimens.screenHorizontal
    val screenBottom = CorePolicyDimens.screenBottom
}

enum class ChipTone { NEUTRAL, SUCCESS, WARNING, ERROR, INFO, ACTIVE }

@Composable
private fun chipColors(tone: ChipTone): Pair<Color, Color> {
    val palette = LocalCorePolicyPalette.current
    val semantic = CorePolicySemantics.colors
    return when (tone) {
        ChipTone.NEUTRAL -> palette.surfaceRaised to palette.onSurfaceVariant
        ChipTone.SUCCESS -> semantic.healthyContainer to semantic.onHealthyContainer
        ChipTone.WARNING -> semantic.warningContainer to semantic.onWarningContainer
        ChipTone.ERROR -> semantic.conflictContainer to semantic.onConflictContainer
        ChipTone.INFO -> semantic.infoContainer to semantic.onInfoContainer
        ChipTone.ACTIVE -> palette.primaryContainer to palette.onPrimaryContainer
    }
}

@Composable
fun accentBackgroundFor(tone: ChipTone): Color = chipColors(tone).first

@Composable
fun accentForegroundFor(tone: ChipTone): Color = chipColors(tone).second

@Composable
fun AppBackdrop(modifier: Modifier = Modifier) {
    val palette = LocalCorePolicyPalette.current
    Box(
        modifier = modifier.background(
            brush = Brush.verticalGradient(
                colors = listOf(
                    palette.backgroundAccent.copy(alpha = 0.38f),
                    palette.background,
                    palette.background
                )
            )
        )
    )
}

@Composable
fun PageHeader(
    title: String,
    subtitle: String,
    modifier: Modifier = Modifier,
    eyebrow: String? = null,
    trailing: @Composable (() -> Unit)? = null
) {
    val palette = LocalCorePolicyPalette.current
    val spacing = CorePolicyDesign.spacing
    Row(
        modifier = modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.Top
    ) {
        Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(spacing.xs)
        ) {
            eyebrow?.let {
                Text(
                    text = it.uppercase(),
                    style = MaterialTheme.typography.labelMedium,
                    color = palette.primary
                )
            }
            Text(
                text = title,
                style = MaterialTheme.typography.headlineLarge,
                color = palette.onSurface
            )
            Text(
                text = subtitle,
                style = MaterialTheme.typography.bodyMedium,
                color = palette.onSurfaceVariant
            )
        }
        trailing?.invoke()
    }
}

@Composable
fun SectionHeader(
    title: String,
    subtitle: String? = null,
    trailing: @Composable (() -> Unit)? = null,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    val spacing = CorePolicyDesign.spacing
    Row(
        modifier = modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(spacing.nano)
        ) {
            Text(
                text = title,
                style = MaterialTheme.typography.titleLarge,
                color = palette.onSurface
            )
            subtitle?.let {
                Text(
                    text = it,
                    style = MaterialTheme.typography.bodySmall,
                    color = palette.onSurfaceVariant
                )
            }
        }
        trailing?.invoke()
    }
}

@Composable
fun StatusChip(
    text: String,
    tone: ChipTone,
    modifier: Modifier = Modifier,
    leadingDot: Boolean = false
) {
    val spacing = CorePolicyDesign.spacing
    val radii = CorePolicyDesign.radii
    val (bg, fg) = chipColors(tone)
    Row(
        modifier = modifier
            .clip(RoundedCornerShape(radii.full))
            .background(bg)
            .padding(horizontal = spacing.sm, vertical = spacing.xxs),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(spacing.xxs + spacing.nano)
    ) {
        if (leadingDot) {
            Box(
                modifier = Modifier
                    .size(6.dp)
                    .background(fg, shape = RoundedCornerShape(50))
            )
        }
        Text(
            text = text,
            color = fg,
            style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.SemiBold)
        )
    }
}

@Composable
fun SelectableFilterChip(
    label: String,
    selected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    val spacing = CorePolicyDesign.spacing
    val radii = CorePolicyDesign.radii
    val bg = if (selected) palette.primaryContainer else palette.surfaceRaised
    val fg = if (selected) palette.onPrimaryContainer else palette.onSurfaceVariant
    val border = if (selected) palette.primary.copy(alpha = 0.32f) else palette.divider
    Row(
        modifier = modifier
            .clip(RoundedCornerShape(radii.full))
            .background(bg)
            .border(1.dp, border, RoundedCornerShape(radii.full))
            .clickable(onClick = onClick)
            .padding(horizontal = spacing.md, vertical = spacing.xs),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(spacing.xxs + spacing.nano)
    ) {
        if (selected) {
            Box(
                modifier = Modifier
                    .size(6.dp)
                    .background(fg, shape = RoundedCornerShape(50))
            )
        }
        Text(
            text = label,
            color = fg,
            style = MaterialTheme.typography.labelMedium
        )
    }
}

@Composable
fun SectionCard(
    modifier: Modifier = Modifier,
    elevated: Boolean = false,
    onClick: (() -> Unit)? = null,
    contentPadding: PaddingValues = PaddingValues(
        horizontal = CorePolicyDimens.cardPaddingH,
        vertical = CorePolicyDimens.cardPaddingV
    ),
    content: @Composable ColumnScope.() -> Unit
) {
    val palette = LocalCorePolicyPalette.current
    val radii = CorePolicyDesign.radii
    val elevation = CorePolicyDesign.elevation
    val interaction = remember { MutableInteractionSource() }
    val pressed by interaction.collectIsPressedAsState()
    val scale by animateFloatAsState(
        targetValue = if (pressed && onClick != null) 0.988f else 1f,
        animationSpec = spring(dampingRatio = Spring.DampingRatioNoBouncy, stiffness = Spring.StiffnessMediumLow),
        label = "sectionCardScale"
    )
    val shape = RoundedCornerShape(radii.lg)
    var cardModifier = modifier
        .fillMaxWidth()
        .scale(scale)
        .shadow(
            elevation = if (elevated) elevation.medium else elevation.low,
            shape = shape,
            clip = false,
            ambientColor = Color.Black.copy(alpha = 0.18f),
            spotColor = Color.Black.copy(alpha = 0.18f)
        )
        .clip(shape)
        .background(if (elevated) palette.surfaceRaised else palette.surfaceContainer)
        .border(1.dp, palette.divider, shape)
    if (onClick != null) {
        cardModifier = cardModifier.clickable(
            interactionSource = interaction,
            indication = ripple(bounded = true),
            onClick = onClick
        )
    }
    Column(
        modifier = cardModifier.padding(contentPadding),
        verticalArrangement = Arrangement.spacedBy(CorePolicyDesign.spacing.sm),
        content = content
    )
}

@Composable
fun ControlCard(
    modifier: Modifier = Modifier,
    onClick: (() -> Unit)? = null,
    content: @Composable ColumnScope.() -> Unit
) = SectionCard(modifier = modifier, onClick = onClick, content = content)

@Composable
fun PrimaryButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    val spacing = CorePolicyDesign.spacing
    val radii = CorePolicyDesign.radii
    Box(
        modifier = modifier
            .clip(RoundedCornerShape(radii.md))
            .background(palette.primaryContainer)
            .clickable(onClick = onClick)
            .padding(horizontal = spacing.md, vertical = spacing.sm),
        contentAlignment = Alignment.Center
    ) {
        Text(text = text, style = MaterialTheme.typography.labelLarge, color = palette.onPrimaryContainer)
    }
}

@Composable
fun SecondaryButton(
    text: String,
    onClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    val spacing = CorePolicyDesign.spacing
    val radii = CorePolicyDesign.radii
    Box(
        modifier = modifier
            .clip(RoundedCornerShape(radii.md))
            .background(palette.surfaceRaised)
            .border(1.dp, palette.divider, RoundedCornerShape(radii.md))
            .clickable(onClick = onClick)
            .padding(horizontal = spacing.md, vertical = spacing.sm),
        contentAlignment = Alignment.Center
    ) {
        Text(text = text, style = MaterialTheme.typography.labelLarge, color = palette.onSurface)
    }
}

@Composable
fun ModernSwitchRow(
    title: String,
    subtitle: String? = null,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    val radii = CorePolicyDesign.radii
    val spacing = CorePolicyDesign.spacing
    Row(
        modifier = modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(radii.md))
            .clickable { onCheckedChange(!checked) }
            .padding(vertical = spacing.xs),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(spacing.nano)) {
            Text(title, style = MaterialTheme.typography.titleSmall, color = palette.onSurface)
            subtitle?.let {
                Text(it, style = MaterialTheme.typography.bodySmall, color = palette.onSurfaceVariant)
            }
        }
        Spacer(Modifier.width(spacing.sm))
        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange,
            colors = SwitchDefaults.colors(
                checkedThumbColor = palette.onPrimaryContainer,
                checkedTrackColor = palette.primary,
                uncheckedThumbColor = palette.onSurfaceVariant,
                uncheckedTrackColor = palette.surfaceRaised,
                uncheckedBorderColor = palette.divider
            )
        )
    }
}

@Composable
fun PolicyToggleRow(label: String, checked: Boolean, onChanged: (Boolean) -> Unit) {
    ModernSwitchRow(title = label, checked = checked, onCheckedChange = onChanged)
}

@Composable
fun MetadataLine(label: String, value: String) {
    val palette = LocalCorePolicyPalette.current
    val spacing = CorePolicyDesign.spacing
    Row(horizontalArrangement = Arrangement.spacedBy(spacing.xxs)) {
        Text(label, style = MaterialTheme.typography.labelSmall, color = palette.onSurfaceVariant)
        Text(value, style = MaterialTheme.typography.bodySmall, color = palette.onSurfaceVariant)
    }
}

@Composable
fun PressableCardBox(
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
    content: @Composable () -> Unit
) {
    val interaction = remember { MutableInteractionSource() }
    val pressed by interaction.collectIsPressedAsState()
    val scale by animateFloatAsState(if (pressed) 0.98f else 1f, label = "pressScale")
    val elevation by animateDpAsState(if (pressed) 2.dp else 8.dp, label = "pressElev")
    Box(
        modifier = modifier
            .scale(scale)
            .shadow(elevation, RoundedCornerShape(CorePolicyDimens.cardRadius), clip = false)
            .clip(RoundedCornerShape(CorePolicyDimens.cardRadius))
            .clickable(
                interactionSource = interaction,
                indication = ripple(bounded = true),
                onClick = onClick
            )
    ) { content() }
}
