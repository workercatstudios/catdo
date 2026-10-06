package com.workercat.catdo.ui.kirakira

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.keyframes
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.PressInteraction
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.workercat.catdo.ui.Kirakira
import kotlinx.coroutines.flow.collectLatest

/**
 * Kirakira's Pop Switch. The thumb slides in 0.25 s on Kirakira's strong ease-out and pops on its
 * own scale: pressing shrinks it to 0.85, a toggle ducks it to 0.78 while it travels and pops it
 * to 1.12 as it lands, 0.97, then 1, over 0.4 s. Under reduced motion it jumps across.
 */
@Composable
fun PopSwitch(
    checked: Boolean,
    onCheckedChange: (Boolean) -> Unit,
    contentDescription: String,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
) {
    val kk = Kirakira.colors
    val reduced = Kirakira.reducedMotion
    val interaction = remember { MutableInteractionSource() }
    val thumbScale = remember { Animatable(1f) }
    var seen by remember { mutableStateOf(checked) }
    LaunchedEffect(interaction, reduced) {
        if (reduced) return@LaunchedEffect
        interaction.interactions.collectLatest { event ->
            when (event) {
                is PressInteraction.Press -> thumbScale.animateTo(0.85f, tween(100, easing = KkEase.CssOut))
                is PressInteraction.Cancel -> thumbScale.animateTo(1f, tween(200))
                else -> Unit
            }
        }
    }
    LaunchedEffect(checked, reduced) {
        if (checked == seen) return@LaunchedEffect
        seen = checked
        if (reduced) { thumbScale.snapTo(1f); return@LaunchedEffect }
        thumbScale.animateTo(1f, keyframes {
            durationMillis = 400
            0.85f at 0 using KkEase.CssInOut
            0.78f at 80 using KkEase.CssInOut
            1.12f at 220 using KkEase.CssInOut
            0.97f at 320 using KkEase.CssInOut
        })
    }
    val travel by animateDpAsState(if (checked) 20.dp else 0.dp, if (reduced) tween(0) else tween(250, easing = KkEase.Out), label = "thumb")
    val track by animateColorAsState(if (checked) kk.primary else if (kk.dark) kk.input.copy(alpha = 0.8f) else Color(0xFFE6D5C9), tween(200), label = "track")
    val thumb by animateColorAsState(if (kk.dark) (if (checked) kk.onPrimary else kk.ink) else kk.card, tween(200), label = "thumbColor")
    Box(
        modifier
            .size(width = 64.dp, height = 48.dp)
            .toggleable(checked, interaction, indication = null, enabled = enabled, role = Role.Switch, onValueChange = onCheckedChange)
            .semantics { this.contentDescription = contentDescription }
            .alpha(if (enabled) 1f else 0.5f),
        contentAlignment = Alignment.Center,
    ) {
        Box(Modifier.size(width = 48.dp, height = 28.dp).background(track, CircleShape).padding(3.dp)) {
            Box(
                Modifier.offset { IntOffset(travel.roundToPx(), 0) }.size(22.dp)
                    .graphicsLayer { scaleX = thumbScale.value; scaleY = thumbScale.value }
                    .shadow(1.5.dp, CircleShape)
                    .background(thumb, CircleShape),
            )
        }
    }
}
