package com.workercat.catdo.ui.kirakira

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.keyframes
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.selection.toggleable
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.PathMeasure
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.workercat.catdo.ui.Kirakira
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.launch

/** pop-checkbox's tick, in a 24-unit box. */
private val tickPath = androidx.compose.ui.graphics.vector.PathParser().parsePathString("M6.5 12.5l3.75 3.75L17.5 8.5").toPath()

/**
 * Kirakira's Pop Checkbox. With a null [onCheckedChange] the parent row owns the toggle and the
 * press (pass its [interaction] so the squash still plays).
 *
 * Kirakira's Pop Checkbox as CatDo's round task check. Pressing squashes it to 0.85; checking
 * rebounds it 0.85 -> 1.08 -> 0.97 -> 1 in 0.34 s, fills it pink, draws the tick in over 0.25 s a
 * beat later (0.08 s) and fires a pink burst. Unchecking fades the tick out in 0.12 s. Nothing
 * plays on first composition; under reduced motion the fill and tick change at once.
 */
@Composable
fun PopCheck(
    checked: Boolean,
    onCheckedChange: ((Boolean) -> Unit)?,
    contentDescription: String?,
    modifier: Modifier = Modifier,
    size: Dp = 24.dp,
    color: Color = Kirakira.colors.primary,
    round: Boolean = true,
    interaction: MutableInteractionSource = remember { MutableInteractionSource() },
) {
    val kk = Kirakira.colors
    val reduced = Kirakira.reducedMotion
    val scale = remember { Animatable(1f) }
    val tick = remember { Animatable(if (checked) 1f else 0f) }
    val tickAlpha = remember { Animatable(if (checked) 1f else 0f) }
    var seen by remember { mutableStateOf(checked) }
    var fire by remember { mutableIntStateOf(0) }

    LaunchedEffect(interaction, reduced) {
        if (reduced) return@LaunchedEffect
        interaction.interactions.collectLatest { event ->
            when (event) {
                is PressInteraction.Press -> scale.animateTo(0.85f, tween(100, easing = KkEase.CssOut))
                is PressInteraction.Cancel -> scale.animateTo(1f, tween(300, easing = KkEase.Spring))
                else -> Unit
            }
        }
    }
    LaunchedEffect(checked, reduced) {
        if (checked == seen) return@LaunchedEffect
        seen = checked
        if (reduced) {
            scale.snapTo(1f); tick.snapTo(if (checked) 1f else 0f); tickAlpha.snapTo(if (checked) 1f else 0f)
            return@LaunchedEffect
        }
        coroutineScope {
            if (checked) {
                fire++
                launch {
                    scale.animateTo(1f, keyframes {
                        durationMillis = 340
                        0.85f at 0 using KkEase.CssInOut
                        1.08f at 136 using KkEase.CssInOut
                        0.97f at 245 using KkEase.CssInOut
                    })
                }
                launch {
                    tickAlpha.snapTo(1f)
                    tick.snapTo(0f)
                    tick.animateTo(1f, tween(250, delayMillis = 80, easing = KkEase.CssInOut))
                }
            } else {
                launch { scale.animateTo(1f, tween(300, easing = KkEase.Spring)) }
                launch { tickAlpha.animateTo(0f, tween(120, easing = KkEase.CssIn)); tick.snapTo(0f) }
            }
        }
    }

    val fill by animateColorAsState(if (checked) color else Color.Transparent, tween(if (reduced) 0 else 150, easing = KkEase.CssOut), label = "checkFill")
    val ring by animateColorAsState(if (checked) color else kk.mutedInk.copy(alpha = 0.7f), tween(if (reduced) 0 else 150), label = "checkRing")
    val mark = if (kk.dark) kk.onPrimary else Color.White

    Box(
        modifier
            .size(48.dp)
            .then(
                if (onCheckedChange == null) Modifier
                else Modifier.toggleable(checked, interaction, indication = null, role = Role.Checkbox, onValueChange = onCheckedChange)
            )
            .then(if (contentDescription == null) Modifier else Modifier.semantics { this.contentDescription = contentDescription }),
        contentAlignment = Alignment.Center,
    ) {
        Canvas(
            Modifier.size(size).burst(if (round) fire else 0, radius = size * 1.25f, ring = color.copy(alpha = 0.35f))
                .graphicsLayer { scaleX = scale.value; scaleY = scale.value },
        ) {
            val stroke = 2.dp.toPx()
            val r = this.size.minDimension / 2
            if (round) {
                drawCircle(fill, r)
                drawCircle(ring, r - stroke / 2, style = Stroke(stroke))
            } else {
                // pop-checkbox's box: rounded to 30 %.
                val corner = CornerRadius(this.size.minDimension * 0.3f)
                drawRoundRect(fill, cornerRadius = corner)
                drawRoundRect(ring, Offset(stroke / 2, stroke / 2), Size(this.size.width - stroke, this.size.height - stroke), corner, Stroke(stroke))
            }
            if (tickAlpha.value > 0f && tick.value > 0f) {
                val measure = PathMeasure().apply { setPath(tickPath, false) }
                val part = Path()
                measure.getSegment(0f, measure.length * tick.value, part, true)
                scale(this.size.width / 24f, this.size.height / 24f, Offset.Zero) {
                    drawPath(part, mark.copy(alpha = tickAlpha.value),
                        style = Stroke(3f, cap = StrokeCap.Round, join = StrokeJoin.Round))
                }
            }
        }
    }
}
