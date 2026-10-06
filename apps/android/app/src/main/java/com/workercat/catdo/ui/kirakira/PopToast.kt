package com.workercat.catdo.ui.kirakira

import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.SnackbarData
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.dp
import com.workercat.catdo.ui.Kirakira

/**
 * Kirakira's Pop Toast for Material's snackbar host: a card that drops in from just above its
 * place and swings to rest (6 deg -> -3 -> 1.5 -> -0.5 -> 0 over 0.55 s). Reduced motion: it fades.
 */
@Composable
fun PopToast(data: SnackbarData, modifier: Modifier = Modifier) {
    val kk = Kirakira.colors
    val reduced = Kirakira.reducedMotion
    val t = remember(data) { Animatable(if (reduced) 1f else 0f) }
    LaunchedEffect(data) { if (!reduced) t.animateTo(1f, tween(550, easing = LinearEasing)) }
    val shape = RoundedCornerShape(20.dp)
    Row(
        modifier
            .padding(horizontal = 16.dp, vertical = 8.dp)
            .widthIn(max = 560.dp)
            .fillMaxWidth()
            .graphicsLayer {
                val p = t.value
                // translate: -1.25rem -> 0.2rem (40 %) -> 0 (62 %); rotate 6 -> -3 -> 1.5 -> -0.5 -> 0.
                translationY = 20.dp.toPx() * when {
                    p < .4f -> -1f + 1.16f * KkEase.CssInOut.transform(p / .4f)
                    p < .62f -> 0.16f * (1f - KkEase.CssInOut.transform((p - .4f) / .22f))
                    else -> 0f
                }
                rotationZ = when {
                    p < .4f -> 6f - 9f * KkEase.CssInOut.transform(p / .4f)
                    p < .62f -> -3f + 4.5f * KkEase.CssInOut.transform((p - .4f) / .22f)
                    p < .82f -> 1.5f - 2f * KkEase.CssInOut.transform((p - .62f) / .2f)
                    else -> -0.5f + 0.5f * KkEase.CssInOut.transform((p - .82f) / .18f)
                }
                alpha = (p / 0.2f).coerceIn(0f, 1f)
            }
            .shadow(16.dp, shape, ambientColor = Color.Black.copy(alpha = 0.15f), spotColor = Color.Black.copy(alpha = 0.15f))
            .background(kk.card, shape)
            .border(1.dp, kk.border, shape)
            .heightIn(min = 56.dp)
            .padding(start = 18.dp, end = 8.dp, top = 6.dp, bottom = 6.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text(data.visuals.message, Modifier.weight(1f).padding(vertical = 8.dp), style = MaterialTheme.typography.bodyMedium, color = kk.ink)
        data.visuals.actionLabel?.let { label ->
            PopButton(onClick = { data.performAction() }, size = PopSize.Small) { Text(label) }
        }
    }
}
