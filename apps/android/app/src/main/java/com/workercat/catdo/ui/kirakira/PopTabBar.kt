package com.workercat.catdo.ui.kirakira

import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.launch
import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.keyframes
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import com.workercat.catdo.ui.Kirakira

class PopTab(val label: String, val icon: ImageVector, val tint: Color)

/**
 * A bottom tab bar with Pop Tabs' pill: it slides to the chosen tab on Kirakira's spring curve
 * (0.42 s) and lands with a stretch, squash and rebound (0.46 s). With nothing selected it shrinks
 * away. Reduced motion: the pill jumps.
 */
@Composable
fun PopTabBar(tabs: List<PopTab>, selected: Int, onSelect: (Int) -> Unit, modifier: Modifier = Modifier) {
    val kk = Kirakira.colors
    val reduced = Kirakira.reducedMotion
    var last by remember { mutableIntStateOf(selected.coerceAtLeast(0)) }
    if (selected >= 0) last = selected
    val position by animateFloatAsState(last.toFloat(), if (reduced) tween(0) else tween(420, easing = KkEase.Spring), label = "pill")
    val shown by animateFloatAsState(if (selected >= 0) 1f else 0f, tween(150), label = "pillShown")
    val landX = remember { Animatable(1f) }
    val landY = remember { Animatable(1f) }
    LaunchedEffect(last) {
        if (reduced) return@LaunchedEffect
        coroutineScope {
            launch { landX.animateTo(1f, keyframes { durationMillis = 460; 1f at 0; 1.06f at 138; 0.96f at 276; 1.015f at 377 }) }
            launch { landY.animateTo(1f, keyframes { durationMillis = 460; 1f at 0; 0.92f at 138; 1.06f at 276; 0.985f at 377 }) }
        }
    }
    Column(modifier.fillMaxWidth().background(kk.paper).navigationBarsPadding()) {
        HorizontalDivider(color = kk.border)
        BoxWithConstraints(Modifier.fillMaxWidth().height(68.dp)) {
            val itemWidth = maxWidth / tabs.size
            Box(
                Modifier
                    .offset { IntOffset((itemWidth * position + (itemWidth - 60.dp) / 2).roundToPx(), 8.dp.roundToPx()) }
                    .size(60.dp, 32.dp)
                    .graphicsLayer {
                        alpha = shown
                        val s = 0.85f + 0.15f * shown
                        scaleX = s * landX.value; scaleY = s * landY.value
                    }
                    .background(kk.blush, CircleShape),
            )
            Row(Modifier.fillMaxWidth().fillMaxHeight().selectableGroup(), horizontalArrangement = Arrangement.SpaceEvenly) {
                tabs.forEachIndexed { index, tab ->
                    val isSelected = index == selected
                    val interaction = remember { MutableInteractionSource() }
                    val ink by animateColorAsState(if (isSelected) kk.ink else kk.mutedInk, tween(150), label = "tabInk")
                    val iconTint by animateColorAsState(if (isSelected) tab.tint else kk.mutedInk, tween(150), label = "tabIcon")
                    Column(
                        Modifier.weight(1f).fillMaxHeight()
                            .selectable(isSelected, interaction, indication = null, role = Role.Tab) { onSelect(index) }
                            .popPress(interaction, 0.92f),
                        horizontalAlignment = Alignment.CenterHorizontally,
                    ) {
                        Box(Modifier.padding(top = 8.dp).size(60.dp, 32.dp), contentAlignment = Alignment.Center) {
                            Icon(tab.icon, null, Modifier.size(22.dp), tint = iconTint)
                        }
                        Text(tab.label, Modifier.padding(top = 3.dp), style = MaterialTheme.typography.labelMedium,
                            color = ink, fontWeight = if (isSelected) FontWeight.ExtraBold else FontWeight.Medium)
                    }
                }
            }
        }
    }
}
