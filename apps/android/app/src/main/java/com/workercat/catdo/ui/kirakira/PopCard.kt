package com.workercat.catdo.ui.kirakira

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.keyframes
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.foundation.shape.RoundedCornerShape
import com.workercat.catdo.ui.Kirakira

/** Pop Card's surface: white card on the paper, a hairline border and a small soft shadow. */
@Composable
fun Modifier.popCard(radius: Dp = 20.dp, elevation: Dp = 2.dp): Modifier {
    val kk = Kirakira.colors
    val shape = RoundedCornerShape(radius)
    return this
        .shadow(if (kk.dark) 0.dp else elevation, shape, ambientColor = Color.Black.copy(alpha = 0.1f), spotColor = Color.Black.copy(alpha = 0.1f))
        .background(kk.card, shape)
        .border(1.dp, kk.border, shape)
}

/**
 * Pop Card's entrance: up from a little below, past full size, a small dip, rest
 * (translateY 16 -> -2 -> 0, scale 0.75 -> 1.03 -> 0.99 -> 1, opacity done by 30 %), after [delay] ms.
 * Reduced motion: it's simply there.
 */
@Composable
fun Modifier.popIn(delay: Int = 0, duration: Int = 500, from: Float = 0.75f): Modifier {
    val reduced = Kirakira.reducedMotion
    val t = remember { Animatable(if (reduced) 1f else 0f) }
    LaunchedEffect(Unit) {
        if (reduced) return@LaunchedEffect
        t.animateTo(1f, keyframes {
            durationMillis = duration + delay
            0f at delay using KkEase.CssInOut
        })
    }
    return graphicsLayer {
        val p = t.value
        fun seg(a: Float, b: Float) = KkEase.CssInOut.transform(((p - a) / (b - a)).coerceIn(0f, 1f))
        val s = when {
            p < .55f -> from + (1.03f - from) * seg(0f, .55f)
            p < .8f -> 1.03f - 0.04f * seg(.55f, .8f)
            else -> 0.99f + 0.01f * seg(.8f, 1f)
        }
        val y = when {
            p < .55f -> 16f - 18f * seg(0f, .55f)
            p < .8f -> -2f + 2f * seg(.55f, .8f)
            else -> 0f
        }
        scaleX = s; scaleY = s
        translationY = y * density
        alpha = (p / .3f).coerceIn(0f, 1f)
    }
}
