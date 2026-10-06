package com.workercat.catdo.ui.kirakira

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import com.workercat.catdo.ui.Kirakira

/**
 * Kirakira's Pop Input: a rounded field whose primary underline runs out from the left on focus
 * (0.25 s, snap) and back towards the right on blur (0.15 s), with the focus ring around it.
 * Under reduced motion the underline appears and goes at once.
 */
@Composable
fun PopTextField(
    value: String,
    onValueChange: (String) -> Unit,
    label: String,
    modifier: Modifier = Modifier,
    placeholder: String = label,
    icon: ImageVector? = null,
    singleLine: Boolean = true,
    keyboardOptions: KeyboardOptions = KeyboardOptions.Default,
    keyboardActions: KeyboardActions = KeyboardActions.Default,
) {
    val kk = Kirakira.colors
    val reduced = Kirakira.reducedMotion
    val interaction = remember { MutableInteractionSource() }
    val focused by interaction.collectIsFocusedAsState()
    val line by animateFloatAsState(
        if (focused) 1f else 0f,
        when { reduced -> tween(0); focused -> tween(250, easing = KkEase.Snap); else -> tween(150, easing = KkEase.In) },
        label = "underline",
    )
    val border by animateColorAsState(if (focused) kk.primary else kk.input, tween(150), label = "border")
    val halo by animateFloatAsState(if (focused) 1f else 0f, tween(150), label = "halo")
    val shape = RoundedCornerShape(16.dp)
    val textStyle = MaterialTheme.typography.bodyLarge.copy(color = kk.ink)
    BasicTextField(
        value, onValueChange,
        modifier.fillMaxWidth().semantics { contentDescription = label },
        singleLine = singleLine, textStyle = textStyle, cursorBrush = SolidColor(kk.primary),
        keyboardOptions = keyboardOptions, keyboardActions = keyboardActions, interactionSource = interaction,
        decorationBox = { inner ->
            Row(
                Modifier
                    .drawBehind {
                        // The focus ring: 3dp of ring at half strength just outside the border.
                        if (halo > 0f) {
                            val grow = 2.dp.toPx()
                            drawRoundRect(kk.primary.copy(alpha = 0.3f * halo), Offset(-grow, -grow),
                                Size(size.width + grow * 2, size.height + grow * 2), CornerRadius(18.dp.toPx()), Stroke(3.dp.toPx()))
                        }
                    }
                    .background(if (kk.dark) kk.input.copy(alpha = 0.3f) else kk.card, shape)
                    .border(1.dp, border, shape)
                    .heightIn(min = 52.dp)
                    .drawBehind {
                        // The underline grows from the left and leaves towards the right.
                        if (line > 0f) {
                            val inset = 16.dp.toPx()
                            val full = size.width - inset * 2
                            val width = full * line
                            val x = if (focused) inset else inset + full - width
                            val thick = 2.dp.toPx()
                            drawRoundRect(kk.primary, Offset(x, size.height - 6.dp.toPx() - thick), Size(width, thick), CornerRadius(thick))
                        }
                    }
                    .padding(horizontal = 16.dp, vertical = 14.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                if (icon != null) Icon(icon, null, Modifier.padding(end = 10.dp).size(20.dp), tint = kk.mutedInk)
                Box(Modifier.weight(1f)) {
                    if (value.isEmpty()) Text(placeholder, style = textStyle, color = kk.mutedInk)
                    inner()
                }
            }
        },
    )
}
