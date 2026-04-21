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
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.corepolicy.manager.ui.theme.CorePolicySemantics
import com.corepolicy.manager.ui.theme.LocalCorePolicyPalette

/* -------------------------------------------------------------------------- */
/*  Dimensions                                                                */
/* -------------------------------------------------------------------------- */

object CorePolicyDimens {
    val screenHorizontal = 20.dp
    val screenTop = 8.dp
    val screenBottom = 112.dp
    val sectionGap = 20.dp
    val cardGap = 12.dp
    val cardPaddingH = 16.dp
    val cardPaddingV = 14.dp
    val cardRadius = 20.dp
    val cardRadiusTight = 16.dp
    val chipRadius = 999.dp
    val iconBadge = 32.dp
}

// Kept for backward compatibility with existing screens (AppManagerScreen uses these).
object ControlCenterDimens {
    val sectionGap = CorePolicyDimens.cardGap
    val cardRadius = CorePolicyDimens.cardRadius
    val chipRadius = CorePolicyDimens.chipRadius
    val screenHorizontal = CorePolicyDimens.screenHorizontal
    val screenBottom = CorePolicyDimens.screenBottom
}

/* -------------------------------------------------------------------------- */
/*  PageHeader                                                                */
/* -------------------------------------------------------------------------- */

@Composable
fun PageHeader(
    title: String,
    subtitle: String,
    modifier: Modifier = Modifier,
    eyebrow: String? = null,
    trailing: @Composable (() -> Unit)? = null
) {
    val palette = LocalCorePolicyPalette.current
    Row(
        modifier = modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.Top
    ) {
        Column(
            modifier = Modifier.weight(1f),
            verticalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            eyebrow?.let {
                Text(
                    text = it.uppercase(),
                    style = MaterialTheme.typography.labelSmall.copy(fontWeight = FontWeight.SemiBold),
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

/* -------------------------------------------------------------------------- */
/*  Tones                                                                     */
/* -------------------------------------------------------------------------- */

enum class ChipTone { NEUTRAL, SUCCESS, WARNING, ERROR, INFO, ACTIVE }

@Composable
private fun chipColors(tone: ChipTone): Pair<Color, Color> {
    val palette = LocalCorePolicyPalette.current
    val s = CorePolicySemantics.colors
    return when (tone) {
        ChipTone.NEUTRAL -> palette.surfaceContainerHigh to palette.onSurfaceVariant
        ChipTone.SUCCESS -> s.healthyContainer to s.onHealthyContainer
        ChipTone.WARNING -> s.warningContainer to s.onWarningContainer
        ChipTone.ERROR -> s.conflictContainer to s.onConflictContainer
        ChipTone.INFO -> s.infoContainer to s.onInfoContainer
        ChipTone.ACTIVE -> palette.primaryContainer to palette.onPrimaryContainer
    }
}

@Composable
fun accentBackgroundFor(tone: ChipTone): Color = chipColors(tone).first

@Composable
fun accentForegroundFor(tone: ChipTone): Color = chipColors(tone).second

/* -------------------------------------------------------------------------- */
/*  SectionHeader                                                             */
/* -------------------------------------------------------------------------- */

@Composable
fun SectionHeader(
    title: String,
    subtitle: String? = null,
    trailing: @Composable (() -> Unit)? = null,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    Row(
        modifier = modifier.fillMaxWidth(),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(
                text = title,
                style = MaterialTheme.typography.headlineSmall,
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

/* -------------------------------------------------------------------------- */
/*  StatusChip                                                                */
/* -------------------------------------------------------------------------- */

@Composable
fun StatusChip(
    text: String,
    tone: ChipTone,
    modifier: Modifier = Modifier,
    leadingDot: Boolean = false
) {
    val (bg, fg) = chipColors(tone)
    Row(
        modifier = modifier
            .clip(RoundedCornerShape(CorePolicyDimens.chipRadius))
            .background(bg)
            .padding(horizontal = 10.dp, vertical = 5.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp)
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
            style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.SemiBold)
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
    val bg = if (selected) palette.primaryContainer else palette.surfaceContainerHigh
    val fg = if (selected) palette.onPrimaryContainer else palette.onSurfaceVariant
    val border = if (selected) palette.primary.copy(alpha = 0.32f) else palette.divider
    val shape = RoundedCornerShape(CorePolicyDimens.chipRadius)
    Row(
        modifier = modifier
            .clip(shape)
            .background(bg)
            .border(1.dp, border, shape)
            .clickable(onClick = onClick)
            .padding(horizontal = 14.dp, vertical = 7.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(6.dp)
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
            style = MaterialTheme.typography.labelMedium.copy(fontWeight = FontWeight.SemiBold)
        )
    }
}

/* -------------------------------------------------------------------------- */
/*  SectionCard — the primary container                                       */
/* -------------------------------------------------------------------------- */

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
    val interaction = remember { MutableInteractionSource() }
    val pressed by interaction.collectIsPressedAsState()
    val scale by animateFloatAsState(
        targetValue = if (pressed && onClick != null) 0.985f else 1f,
        animationSpec = spring(dampingRatio = Spring.DampingRatioMediumBouncy, stiffness = Spring.StiffnessLow),
        label = "sectionCardScale"
    )
    val bg = if (elevated) palette.surfaceContainerHigh else palette.surfaceContainer
    val shape = RoundedCornerShape(CorePolicyDimens.cardRadius)

    var base: Modifier = modifier
        .fillMaxWidth()
        .scale(scale)
        .shadow(
            elevation = if (elevated) 6.dp else 0.dp,
            shape = shape,
            clip = false,
            ambientColor = Color.Black.copy(alpha = 0.25f),
            spotColor = Color.Black.copy(alpha = 0.25f)
        )
        .clip(shape)
        .background(bg)
        .border(1.dp, palette.divider, shape)
    if (onClick != null) {
        base = base.clickable(
            interactionSource = interaction,
            indication = ripple(bounded = true),
            onClick = onClick
        )
    }
    Column(
        modifier = base.padding(contentPadding),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        content = content
    )
}

/** Backwards-compatible alias used by existing code paths. */
@Composable
fun ControlCard(
    modifier: Modifier = Modifier,
    onClick: (() -> Unit)? = null,
    content: @Composable ColumnScope.() -> Unit
) = SectionCard(modifier = modifier, onClick = onClick, content = content)

/* -------------------------------------------------------------------------- */
/*  ModernSwitchRow                                                           */
/* -------------------------------------------------------------------------- */

@Composable
fun ModernSwitchRow(
    title: String,
    subtitle: String? = null,
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    modifier: Modifier = Modifier
) {
    val palette = LocalCorePolicyPalette.current
    Row(
        modifier = modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(14.dp))
            .clickable { onCheckedChange(!checked) }
            .padding(vertical = 6.dp),
        horizontalArrangement = Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically
    ) {
        Column(modifier = Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(2.dp)) {
            Text(title, style = MaterialTheme.typography.titleSmall, color = palette.onSurface)
            subtitle?.let {
                Text(it, style = MaterialTheme.typography.bodySmall, color = palette.onSurfaceVariant)
            }
        }
        Spacer(Modifier.width(12.dp))
        Switch(
            checked = checked,
            onCheckedChange = onCheckedChange,
            colors = SwitchDefaults.colors(
                checkedThumbColor = palette.onPrimaryContainer,
                checkedTrackColor = palette.primary,
                uncheckedThumbColor = palette.onSurfaceVariant,
                uncheckedTrackColor = palette.surfaceContainerHigh,
                uncheckedBorderColor = palette.divider
            )
        )
    }
}

/** Backwards-compatible name used by AppManagerScreen. */
@Composable
fun PolicyToggleRow(label: String, checked: Boolean, onChanged: (Boolean) -> Unit) {
    ModernSwitchRow(title = label, checked = checked, onCheckedChange = onChanged)
}

/* -------------------------------------------------------------------------- */
/*  MetadataLine                                                              */
/* -------------------------------------------------------------------------- */

@Composable
fun MetadataLine(label: String, value: String) {
    val palette = LocalCorePolicyPalette.current
    Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
        Text(label, style = MaterialTheme.typography.labelMedium, color = palette.onSurfaceVariant)
        Text(value, style = MaterialTheme.typography.bodySmall, color = palette.onSurfaceVariant)
    }
}

/* -------------------------------------------------------------------------- */
/*  PressableCardBox — generic press-scale wrapper                            */
/* -------------------------------------------------------------------------- */

@Composable
fun PressableCardBox(
    modifier: Modifier = Modifier,
    onClick: () -> Unit,
    content: @Composable () -> Unit
) {
    val interaction = remember { MutableInteractionSource() }
    val pressed by interaction.collectIsPressedAsState()
    val scale by animateFloatAsState(if (pressed) 0.97f else 1f, label = "pressScale")
    val elev by animateDpAsState(if (pressed) 1.dp else 4.dp, label = "pressElev")
    Box(
        modifier = modifier
            .scale(scale)
            .shadow(elev, RoundedCornerShape(CorePolicyDimens.cardRadius), clip = false)
            .clip(RoundedCornerShape(CorePolicyDimens.cardRadius))
            .clickable(
                interactionSource = interaction,
                indication = ripple(bounded = true),
                onClick = onClick
            )
    ) { content() }
}
