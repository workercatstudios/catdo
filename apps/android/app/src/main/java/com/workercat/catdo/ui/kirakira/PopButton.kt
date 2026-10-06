package com.workercat.catdo.ui.kirakira

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.defaultMinSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Icon
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ProvideTextStyle
import androidx.compose.material3.Text
import androidx.compose.material3.minimumInteractiveComponentSize
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.workercat.catdo.ui.Kirakira
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch

enum class PopVariant { Default, Secondary, Outline, Ghost, Destructive, Soft }

enum class PopSize(val height: Dp, val padding: Dp) { Small(36.dp, 14.dp), Default(40.dp, 18.dp), Large(48.dp, 22.dp) }

/**
 * Kirakira's Pop Button: shadcn's variants with a sink on press and a pop on release.
 * Small sizes still get a 48dp touch target.
 */
@Composable
fun PopButton(
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    variant: PopVariant = PopVariant.Default,
    size: PopSize = PopSize.Default,
    enabled: Boolean = true,
    icon: ImageVector? = null,
    elevated: Boolean = false,
    content: @Composable RowScope.() -> Unit,
) {
    val kk = Kirakira.colors
    val interaction = remember { MutableInteractionSource() }
    val pressed by interaction.collectIsPressedAsState()
    val focused by interaction.collectIsFocusedAsState()
    val (base, ink) = when (variant) {
        PopVariant.Default -> kk.primary to kk.onPrimary
        PopVariant.Secondary -> kk.secondary to kk.ink
        PopVariant.Outline -> kk.paper to kk.ink
        PopVariant.Ghost -> Color.Transparent to kk.ink
        PopVariant.Soft -> kk.blush to (if (kk.dark) kk.primary else MaterialTheme.colorScheme.primary)
        PopVariant.Destructive -> (if (kk.dark) kk.destructive.copy(alpha = 0.6f) else kk.destructive) to Color.White
    }
    val active = when (variant) {
        PopVariant.Default, PopVariant.Destructive -> base.copy(alpha = base.alpha * 0.9f)
        PopVariant.Secondary -> base.copy(alpha = 0.8f)
        PopVariant.Outline, PopVariant.Ghost -> kk.blush
        PopVariant.Soft -> kk.blush.copy(alpha = 0.7f)
    }
    val fill by animateColorAsState(if (pressed || focused) active else base, tween(150, easing = KkEase.CssOut), label = "buttonFill")
    val shape = RoundedCornerShape(if (size == PopSize.Large) 16.dp else 14.dp)
    Box(
        modifier
            .minimumInteractiveComponentSize()
            .popPress(interaction)
            .then(if (elevated) Modifier.shadow(10.dp, shape, ambientColor = kk.primary, spotColor = kk.primary) else Modifier)
            .background(fill, shape)
            .then(if (variant == PopVariant.Outline) Modifier.border(1.dp, kk.input, shape) else Modifier)
            .clickable(interaction, indication = null, enabled = enabled, role = Role.Button, onClick = onClick)
            .defaultMinSize(minWidth = size.height)
            .height(size.height)
            .alpha(if (enabled) 1f else 0.5f)
            .padding(horizontal = if (icon != null) size.padding - 4.dp else size.padding),
        contentAlignment = Alignment.Center,
    ) {
        CompositionLocalProvider(LocalContentColor provides ink) {
            ProvideTextStyle(if (size == PopSize.Small) MaterialTheme.typography.labelMedium else MaterialTheme.typography.labelLarge) {
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp), verticalAlignment = Alignment.CenterVertically) {
                    if (icon != null) Icon(icon, null, Modifier.size(if (size == PopSize.Small) 16.dp else 18.dp))
                    content()
                }
            }
        }
    }
}

/** A round, quiet icon button. The glyph squashes while held and springs back, like the dialog close button. */
@Composable
fun PopIconButton(
    icon: ImageVector,
    contentDescription: String?,
    onClick: () -> Unit,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    tint: Color = LocalContentColor.current,
    iconSize: Dp = 22.dp,
) {
    val kk = Kirakira.colors
    val reduced = Kirakira.reducedMotion
    val interaction = remember { MutableInteractionSource() }
    val pressed by interaction.collectIsPressedAsState()
    val focused by interaction.collectIsFocusedAsState()
    val fill by animateColorAsState(if (pressed || focused) kk.blush else Color.Transparent, tween(160), label = "iconFill")
    val sx = remember { Animatable(1f) }
    val sy = remember { Animatable(1f) }
    LaunchedEffect(interaction, reduced) {
        if (reduced) return@LaunchedEffect
        interaction.interactions.collectLatest { event ->
            val press = event is PressInteraction.Press
            val spec = if (press) tween<Float>(80, easing = KkEase.CssOut) else tween(350, easing = KkEase.Spring)
            coroutineScope {
                launch { sx.animateTo(if (press) 1.2f else 1f, spec) }
                launch { sy.animateTo(if (press) 0.8f else 1f, spec) }
            }
        }
    }
    Box(
        modifier
            .size(48.dp)
            .background(fill, CircleShape)
            .clickable(interaction, indication = null, enabled = enabled, role = Role.Button, onClick = onClick)
            .alpha(if (enabled) 1f else 0.4f),
        contentAlignment = Alignment.Center,
    ) {
        Icon(icon, contentDescription, Modifier.size(iconSize).graphicsLayer { scaleX = sx.value; scaleY = sy.value }, tint = tint)
    }
}

/** Pop Badge: a small pill, pink by default, for counts. */
@Composable
fun PopBadge(text: String, modifier: Modifier = Modifier, selected: Boolean = true, color: Color? = null) {
    val kk = Kirakira.colors
    val background = color ?: if (selected) kk.primary else kk.muted
    val ink = if (color != null) kk.ink else if (selected) kk.onPrimary else kk.mutedInk
    Text(
        text, modifier.background(background, CircleShape).padding(horizontal = 8.dp, vertical = 1.dp),
        style = MaterialTheme.typography.labelMedium, color = ink,
    )
}
