package com.workercat.catdo.ui.kirakira

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.keyframes
import androidx.compose.animation.core.tween
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.foundation.interaction.collectIsPressedAsState
import androidx.compose.foundation.background
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.RectangleShape
import androidx.compose.ui.graphics.Shape
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.Role
import com.workercat.catdo.ui.Kirakira
import kotlinx.coroutines.flow.collectLatest

/*
 * Kirakira's motion vocabulary for Compose. The curves and keyframes are the web components'
 * numbers (src/registry/kirakira/ui/pop-*.tsx), so a press or a check feels the same everywhere.
 */
object KkEase {
    val Snap = CubicBezierEasing(0.85f, 0f, 0.15f, 1f)
    val Out = CubicBezierEasing(0.05f, 0.3f, 0.1f, 1f)
    val In = CubicBezierEasing(0.8f, 0f, 1f, 1f)
    val Spring = CubicBezierEasing(0.34f, 1.56f, 0.64f, 1f)

    // CSS's named curves, which the keyframes are written against.
    val CssEase = CubicBezierEasing(0.25f, 0.1f, 0.25f, 1f)
    val CssIn = CubicBezierEasing(0.42f, 0f, 1f, 1f)
    val CssOut = CubicBezierEasing(0f, 0f, 0.58f, 1f)
    val CssInOut = CubicBezierEasing(0.42f, 0f, 0.58f, 1f)
}

/** Pressing sinks to [sink] in 0.1 s; letting go pops back 0.95 -> 1.05 -> 0.98 -> 1 in 0.3 s (pop-button). */
@Composable
fun Modifier.popPress(interaction: MutableInteractionSource, sink: Float = 0.95f, pop: Boolean = true): Modifier {
    val reduced = Kirakira.reducedMotion
    val scale = remember { Animatable(1f) }
    LaunchedEffect(interaction, reduced, sink, pop) {
        if (reduced) { scale.snapTo(1f); return@LaunchedEffect }
        interaction.interactions.collectLatest { event ->
            when (event) {
                is PressInteraction.Press -> scale.animateTo(sink, tween(100, easing = KkEase.CssOut))
                is PressInteraction.Release -> if (pop) scale.animateTo(1f, keyframes {
                    durationMillis = 300
                    sink at 0 using KkEase.CssInOut
                    (2f - sink) at 135 using KkEase.CssInOut
                    (1f - (1f - sink) * 0.4f) at 225 using KkEase.CssInOut
                }) else scale.animateTo(1f, tween(300, easing = KkEase.Spring))
                is PressInteraction.Cancel -> scale.animateTo(1f, tween(200, easing = KkEase.Spring))
            }
        }
    }
    return graphicsLayer { scaleX = scale.value; scaleY = scale.value }
}

/**
 * A Kirakira press for rows and tiles: no ripple; the shape fills with [pressed] while held and
 * the whole thing sinks a little and pops back.
 */
@Composable
fun Modifier.popClickable(
    onClick: () -> Unit,
    shape: Shape = RectangleShape,
    enabled: Boolean = true,
    role: Role? = Role.Button,
    onClickLabel: String? = null,
    sink: Float = 0.98f,
    selected: Color = Color.Transparent,
    pressed: Color = Kirakira.colors.muted,
): Modifier {
    val interaction = remember { MutableInteractionSource() }
    val isPressed by interaction.collectIsPressedAsState()
    val isFocused by interaction.collectIsFocusedAsState()
    val fill by animateColorAsState(if (isPressed || isFocused) pressed else selected, tween(150, easing = KkEase.CssOut), label = "pressFill")
    return this
        .popPress(interaction, sink)
        .background(fill, shape)
        .clickable(interaction, indication = null, enabled = enabled, role = role, onClickLabel = onClickLabel, onClick = onClick)
}
