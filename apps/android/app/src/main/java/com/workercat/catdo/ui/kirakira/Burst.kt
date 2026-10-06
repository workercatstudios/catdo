package com.workercat.catdo.ui.kirakira

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Easing
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.State
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.withFrameMillis
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithContent
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.graphics.vector.PathParser
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.workercat.catdo.ui.Accents
import com.workercat.catdo.ui.Kirakira
import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.sin

/*
 * Kirakira's Burst (ring + confetti, here four-point stars) and Sparkles (stars that twinkle
 * around their content), drawn on a Canvas. The numbers are burst.tsx's and sparkles.tsx's.
 * Both are decorative: nothing to read, and gone under reduced motion.
 */

/** Sparkles' star: a four-point star whose sides curve in towards the centre, in a 24-unit box. */
private val starPath = PathParser().parsePathString("M12 0Q13.4 10.6 24 12 13.4 13.4 12 24 10.6 13.4 0 12 10.6 10.6 12 0Z").toPath()

private fun DrawScope.star(center: Offset, size: Float, color: Color, degrees: Float = 0f) {
    if (size <= 0f) return
    translate(center.x - 12f, center.y - 12f) {
        scale(size / 24f, size / 24f, Offset(12f, 12f)) {
            rotate(degrees, Offset(12f, 12f)) { drawPath(starPath, color) }
        }
    }
}

private fun progress(ms: Float, start: Float, length: Float, easing: Easing): Float =
    easing.transform(((ms - start) / length).coerceIn(0f, 1f))

private const val BURST_MILLIS = 800

private class Spoke(val x: Float, val y: Float, val weight: Float, val delay: Float)

// Ten on 36° spokes in burst.tsx; eight on 45° here, so they clear a small round check.
// Index rules vary size, reach and delay so the ring of stars breaks up.
private val spokes = List(8) { i ->
    val angle = (i * 45 - 90) * PI / 180
    val reach = if (i % 3 == 2) 0.8f else if (i % 2 == 1) 0.92f else 1.05f
    Spoke(
        (cos(angle) * reach).toFloat(), (sin(angle) * reach).toFloat(),
        if (i % 3 == 2) 3f else if (i % 2 == 1) 4f else 6f,
        if (i % 3 == 2) 100f else if (i % 2 == 1) 50f else 0f,
    )
}

private fun DrawScope.burst(ms: Float, radius: Float, ring: Color, colors: List<Color>) {
    val center = Offset(size.width / 2, size.height / 2)
    // Ring: a disc grows, then a hole opens a beat behind it and eats it from the inside.
    val outer = radius * progress(ms, 0f, 450f, KkEase.Out)
    val inner = radius * progress(ms, 70f, 450f, KkEase.Out)
    if (outer - inner > 0.5f) drawCircle(ring, (outer + inner) / 2, center, style = Stroke(outer - inner))
    // Stars fly out along their spokes, scaling in on the way and out at the end.
    spokes.forEachIndexed { i, spoke ->
        val fly = progress(ms, spoke.delay, 700f, KkEase.Out)
        val grow = progress(ms, spoke.delay, 300f, KkEase.CssOut)
        val shrink = progress(ms, spoke.delay + 450f, 300f, KkEase.In)
        val scale = grow * (1f - shrink)
        val reach = radius * 1.25f
        val starSize = (spoke.weight * 2.2f + 3f) * density * scale
        star(center + Offset(spoke.x * reach * fly, spoke.y * reach * fly), starSize, colors[i % colors.size], 45f * fly)
    }
}

/** Fires a burst behind the content each time [fire] changes (and is above zero). */
@Composable
fun Modifier.burst(fire: Int, radius: Dp = 28.dp, ring: Color = Kirakira.colors.primary, colors: List<Color> = burstColors()): Modifier {
    val reduced = Kirakira.reducedMotion
    val time = remember { Animatable(BURST_MILLIS.toFloat()) }
    LaunchedEffect(fire) {
        if (fire > 0 && !reduced) {
            time.snapTo(0f)
            time.animateTo(BURST_MILLIS.toFloat(), tween(BURST_MILLIS, easing = LinearEasing))
        }
    }
    return drawWithContent {
        if (time.value < BURST_MILLIS) burst(time.value, radius.toPx(), ring, colors)
        drawContent()
    }
}

@Composable
fun burstColors(): List<Color> = listOf(Accents.kk("pink"), Accents.kk("yellow"), Accents.kk("pink"), Kirakira.colors.primary)

/** A clock in milliseconds that ticks every frame while composed, for draw-phase loops. */
@Composable
fun rememberFrameClock(running: Boolean = true): State<Long> {
    val clock = remember { mutableLongStateOf(0L) }
    LaunchedEffect(running) {
        if (!running) return@LaunchedEffect
        val start = withFrameMillis { it } - clock.longValue
        while (true) withFrameMillis { clock.longValue = it - start }
    }
    return clock
}

private fun hash(i: Int, seed: Float) = ((i + 1) * seed) % 1f

/**
 * Kirakira's Sparkles: [count] stars scattered on an ellipse just outside the content, each
 * twinkling (scale 0 -> 1 -> 0 with a turn) in the first 40 % of its cycle, staggered.
 */
@Composable
fun Modifier.sparkles(
    count: Int = 6,
    colors: List<Color> = listOf(Accents.kk("yellow"), Accents.kk("pink")),
    size: Dp = 14.dp,
    duration: Int = 600,
    stagger: Int = 200,
): Modifier {
    if (Kirakira.reducedMotion) return this
    val clock = rememberFrameClock()
    val cycle = duration * 2.5f
    return drawWithContent {
        drawContent()
        val now = clock.value.toFloat()
        for (i in 0 until count) {
            val angle = (i * 137.508 - 30) * PI / 180
            val rx = (48 + 14 * hash(i, 0.618034f)) / 100f
            val ry = (58 + 30 * hash(i, 0.754878f)) / 100f
            val center = Offset(
                this.size.width * (0.5f + cos(angle).toFloat() * rx),
                this.size.height * (0.5f + sin(angle).toFloat() * ry),
            )
            val local = now - i * stagger
            if (local < 0) continue
            val phase = (local % cycle) / cycle
            val (scale, turn) = when {
                phase < 0.2f -> KkEase.CssInOut.transform(phase / 0.2f).let { it to -30f + 30f * it }
                phase < 0.4f -> KkEase.CssInOut.transform((phase - 0.2f) / 0.2f).let { 1f - it to 30f * it }
                else -> 0f to 0f
            }
            star(center, size.toPx() * (0.55f + 0.45f * hash(i, 0.414214f)) * scale, colors[i % colors.size], turn)
        }
    }
}
