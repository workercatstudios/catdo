package com.workercat.catdo.ui.kirakira

import androidx.compose.animation.core.CubicBezierEasing
import androidx.compose.animation.core.Easing
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Paint
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import androidx.compose.ui.graphics.drawscope.DrawScope
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.drawscope.rotate
import androidx.compose.ui.graphics.drawscope.scale
import androidx.compose.ui.graphics.drawscope.translate
import androidx.compose.ui.graphics.drawscope.withTransform
import androidx.compose.ui.graphics.vector.PathParser
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.drawText
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.workercat.catdo.ui.Accents
import com.workercat.catdo.ui.Kirakira

/*
 * Kirakira UI's Idle Cat (apps/kirakira/src/registry/kirakira/ui/idle-cat.tsx, and idle_cat.rs on
 * GPUI), ported to a Compose Canvas. Same 140 x 120 drawing box, ground line and path strings, and
 * the cast's shared breath, blink, hop and z's with the cat's own ear twitch and tail wag.
 * It's an original WorkerCat character drawn for Kirakira, which was made for CatDo.
 */

enum class IdleCatMood { Idle, Happy, Sleepy }

private fun path(d: String): Path = PathParser().parsePathString(d).toPath()

private val tail = path("M94 101C116 103 129 92 127 77C125.5 66 113 64.5 112 73")
private val ears = listOf(
    path("M30 60L31 19L60 40Z") to path("M35 47L35.5 28L50 39Z"),
    path("M98 60L97 19L68 40Z") to path("M93 47L92.5 28L78 39Z"),
)
// Ears turn about the middle of their fill box, 76 % of the way down.
private val earPivots = listOf(Offset(45f, 50.16f), Offset(83f, 50.16f))
private val body = path("M20 86C20 52 40 34 64 34C88 34 108 52 108 86C108 104 94 112 64 112C34 112 20 104 20 86Z")
private val stripes = path("M58 41V47M64 39V47.5M70 41V47")
private val nose = path("M61.6 75.2h4.8l-2.4 2.6z")
private val mouth = path("M64 77.6V79.6M58.5 79.6Q61.25 83.4 64 79.6Q66.75 83.4 69.5 79.6")
private val whiskers = path("M32 76L15 73.5M32 80.5L16 83M96 76L113 73.5M96 80.5L112 83")
private fun eyeArc(x: Float, y: Float, bend: Float) = path("M${x - 5} $y Q$x ${y + bend} ${x + 5} $y")
private val happyEyes = listOf(eyeArc(48f, 74f, -7.5f), eyeArc(80f, 74f, -7.5f))
private val sleepyEyes = listOf(eyeArc(48f, 71f, 5.5f), eyeArc(80f, 71f, 5.5f))

private val CatInk = Color(0xFF4B3832)
private val EaseInOut = KkEase.CssInOut
private val RiseEase = CubicBezierEasing(0.33f, 1f, 0.68f, 1f)
private val FallEase = CubicBezierEasing(0.32f, 0f, 0.67f, 0f)

/** A CSS keyframe track: [stops] are (offset, value) pairs from 0 to 1, eased per segment. */
private fun track(ms: Float, period: Float, delay: Float, easing: Easing, vararg stops: Pair<Float, Float>): Float {
    val phase = (((ms - delay) % period) + period) % period / period
    for (i in 1 until stops.size) {
        val (end, to) = stops[i]
        val (start, from) = stops[i - 1]
        if (phase <= end) {
            val f = if (end == start) 1f else easing.transform((phase - start) / (end - start))
            return from + (to - from) * f
        }
    }
    return stops.last().second
}

private fun DrawScope.layer(alpha: Float, block: DrawScope.() -> Unit) {
    drawIntoCanvas { it.saveLayer(Rect(Offset.Zero, Size(140f, 120f)), Paint().apply { this.alpha = alpha }) }
    block()
    drawIntoCanvas { it.restore() }
}

/**
 * The Idle Cat: breathes, blinks, twitches its ears and wags its tail. Happy hops; sleepy breathes
 * deeply and dozes off with z's. [label] names it for accessibility; leave it null where it's
 * decoration. It holds still under reduced motion or when not [animated] (offscreen, say).
 */
@Composable
fun IdleCat(
    modifier: Modifier = Modifier,
    size: Dp = 160.dp,
    mood: IdleCatMood = IdleCatMood.Idle,
    color: Color = Accents.kk("orange"),
    label: String? = null,
    animated: Boolean = true,
) {
    val kk = Kirakira.colors
    val pink = Accents.kk("pink")
    val still = Kirakira.reducedMotion || !animated
    val clock = rememberFrameClock(running = !still)
    val measurer = rememberTextMeasurer()
    val zStyle = TextStyle(fontWeight = FontWeight.ExtraBold, color = kk.markInk)
    Canvas(
        modifier.size(size, size * 120f / 140f).then(
            if (label == null) Modifier else Modifier.semantics { contentDescription = label; role = Role.Image },
        ),
    ) {
        val ms = if (still) 0f else clock.value.toFloat()
        val happy = mood == IdleCatMood.Happy
        val sleepy = mood == IdleCatMood.Sleepy
        val unit = this.size.width / 140f
        scale(unit, unit, Offset.Zero) {
            val feet = Offset(64f, 112f)
            // Shadow: shrinks and fades while a happy cat is up.
            val shadowScale = if (happy) track(ms, 1200f, 0f, EaseInOut, 0f to 1f, .22f to 1f, .44f to .7f, .66f to 1f, .7f to 1.12f, 1f to 1f) else 1f
            val shadowScaleY = if (happy) track(ms, 1200f, 0f, EaseInOut, 0f to 1f, .22f to 1f, .44f to .7f, .66f to 1f, 1f to 1f) else 1f
            val shadowAlpha = if (happy) track(ms, 1200f, 0f, EaseInOut, 0f to 1f, .22f to 1f, .44f to .5f, .66f to 1f, 1f to 1f) else 1f
            scale(shadowScale, shadowScaleY, Offset(64f, 113f)) {
                drawOval(CatInk.copy(alpha = 0.12f * shadowAlpha), Offset(24f, 108.5f), Size(80f, 9f))
            }
            // Hop: rise on an ease-out, fall on an ease-in, like a thrown ball.
            val hop = if (!happy) 0f else {
                val phase = (ms % 1200f) / 1200f
                when {
                    phase < .22f || phase > .66f -> 0f
                    phase < .44f -> -20f * RiseEase.transform((phase - .22f) / .22f)
                    else -> -20f * (1f - FallEase.transform((phase - .44f) / .22f))
                }
            }
            val squashX = if (happy) track(ms, 1200f, 0f, EaseInOut, 0f to 1f, .14f to 1.16f, .26f to .88f, .44f to .98f, .62f to .92f, .7f to 1.18f, .8f to .97f, .9f to 1f, 1f to 1f) else 1f
            val squashY = if (happy) track(ms, 1200f, 0f, EaseInOut, 0f to 1f, .14f to .84f, .26f to 1.14f, .44f to 1.02f, .62f to 1.08f, .7f to .8f, .8f to 1.03f, .9f to 1f, 1f to 1f) else 1f
            translate(0f, hop) {
                scale(squashX, squashY, feet) {
                    val wagPeriod = if (happy) 1200f else if (sleepy) 6400f else 3200f
                    val wag = track(ms, wagPeriod, 0f, EaseInOut, 0f to 0f, .12f to -14f, .26f to 6f, .4f to -10f, .52f to 3f, .62f to 0f, 1f to 0f)
                    rotate(wag, Offset(96f, 101f)) {
                        drawPath(tail, color, style = Stroke(10f, cap = StrokeCap.Round))
                    }
                    val breathePeriod = if (sleepy) 2600f else 1100f
                    val bx = track(ms, breathePeriod, 0f, EaseInOut, 0f to 1f, .5f to (if (sleepy) 1.036f else 1.024f), 1f to 1f)
                    val by = track(ms, breathePeriod, 0f, EaseInOut, 0f to 1f, .5f to (if (sleepy) 1.06f else 1.04f), 1f to 1f)
                    scale(bx, by, feet) {
                        val earPeriod = if (sleepy) 7000f else 4400f
                        ears.forEachIndexed { i, (outer, inner) ->
                            val side = if (i == 0) -1f else 1f
                            val twitch = track(ms, earPeriod, if (i == 0) 0f else -2200f, EaseInOut,
                                0f to 0f, .56f to 0f, .585f to 16f * side, .61f to 0f, .635f to 9f * side, .66f to 0f, 1f to 0f)
                            rotate(twitch, earPivots[i]) {
                                drawPath(outer, color)
                                drawPath(outer, color, style = Stroke(8f, join = StrokeJoin.Round))
                                layer(0.5f) {
                                    drawPath(inner, pink)
                                    drawPath(inner, pink, style = Stroke(3f, join = StrokeJoin.Round))
                                }
                            }
                        }
                        drawPath(body, color)
                        drawOval(Color.White.copy(alpha = 0.3f), Offset(44f, 92f), Size(40f, 18f))
                        drawPath(stripes, CatInk.copy(alpha = 0.14f), style = Stroke(3.2f, cap = StrokeCap.Round))
                        when (mood) {
                            IdleCatMood.Idle -> {
                                val blink = track(ms, 5000f, 0f, EaseInOut, 0f to 1f, .3f to 1f, .315f to .1f, .33f to 1f,
                                    .86f to 1f, .875f to .1f, .89f to 1f, .92f to 1f, .935f to .1f, .95f to 1f, 1f to 1f)
                                withTransform({ scale(1f, blink, Offset(64f, 72f)) }) {
                                    for (x in listOf(48f, 80f)) {
                                        drawOval(CatInk, Offset(x - 4.6f, 66.4f), Size(9.2f, 11.2f))
                                        drawCircle(Color.White, 1.6f, Offset(x + 1.6f, 70f))
                                    }
                                }
                            }
                            else -> (if (happy) happyEyes else sleepyEyes).forEach {
                                drawPath(it, CatInk, style = Stroke(3f, cap = StrokeCap.Round))
                            }
                        }
                        for (x in listOf(38f, 90f)) drawOval(pink.copy(alpha = 0.45f), Offset(x - 6.5f, 78.2f), Size(13f, 7.6f))
                        drawPath(nose, pink)
                        drawPath(nose, pink, style = Stroke(1.6f, join = StrokeJoin.Round))
                        drawPath(mouth, CatInk, style = Stroke(2f, cap = StrokeCap.Round, join = StrokeJoin.Round))
                        drawPath(whiskers, CatInk.copy(alpha = 0.4f), style = Stroke(1.6f, cap = StrokeCap.Round))
                        for (x in listOf(50f, 78f)) {
                            drawOval(Color(0xFFFFFAF3), Offset(x - 8.5f, 104f), Size(17f, 10f))
                            drawOval(CatInk.copy(alpha = 0.12f), Offset(x - 8.5f, 104f), Size(17f, 10f), style = Stroke(1.4f))
                        }
                    }
                }
            }
            if (sleepy) listOf(Triple(102f, 38f, 13f), Triple(113f, 25f, 10f)).forEachIndexed { i, (x, y, fontSize) ->
                val delay = if (i == 0) 0f else -1400f
                val t = track(ms, 2800f, delay, KkEase.CssOut, 0f to 0f, 1f to 1f)
                val alpha = if (still) 0.6f else track(ms, 2800f, delay, KkEase.CssOut, 0f to 0f, .3f to .6f, 1f to 0f)
                val zScale = if (still) 1f else 0.6f + 0.4f * t
                val dx = if (still) 0f else -2f + 8f * t
                val dy = if (still) 0f else 6f - 16f * t
                val layout = measurer.measure("z", zStyle.copy(fontSize = (fontSize / density / fontScale).sp))
                withTransform({
                    translate(x + dx, y + dy - fontSize)
                    scale(zScale, zScale, Offset(layout.size.width / 2f, layout.size.height / 2f))
                }) { drawText(layout, alpha = alpha) }
            }
        }
    }
}
