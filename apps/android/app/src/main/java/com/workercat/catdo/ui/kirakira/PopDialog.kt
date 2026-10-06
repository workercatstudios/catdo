package com.workercat.catdo.ui.kirakira

import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.EnterExitState
import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.core.FiniteAnimationSpec
import androidx.compose.animation.core.MutableTransitionState
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.keyframes
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalView
import androidx.compose.ui.semantics.paneTitle
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.window.Dialog
import androidx.compose.ui.window.DialogProperties
import androidx.compose.ui.window.DialogWindowProvider
import com.workercat.catdo.ui.Kirakira

/** Lets dialog content close the dialog with its exit animation, then run [then]. */
@Stable
class PopDialogScope internal constructor(private val close: (() -> Unit) -> Unit) {
    fun dismiss(then: () -> Unit) = close(then)
}

/**
 * Kirakira's Pop Dialog. The panel pops in, scale 0.6 -> (1.04, 1.06) -> (0.98, 0.99) -> 1 while it
 * rises 4 % of its height, over a backdrop that fades in; it leaves with a quick squash and fade.
 * Buttons call [PopDialogScope.dismiss] so the exit plays before the caller's state changes.
 * Under reduced motion the panel and backdrop only fade.
 */
@Composable
fun PopDialog(
    onDismissRequest: () -> Unit,
    modifier: Modifier = Modifier,
    title: String? = null,
    dismissible: Boolean = true,
    width: Dp = 400.dp,
    art: (@Composable () -> Unit)? = null,
    buttons: (@Composable RowScope.(PopDialogScope) -> Unit)? = null,
    content: @Composable ColumnScope.(PopDialogScope) -> Unit,
) {
    val kk = Kirakira.colors
    val reduced = Kirakira.reducedMotion
    val visible = remember { MutableTransitionState(false).apply { targetState = true } }
    var after by remember { mutableStateOf<(() -> Unit)?>(null) }
    val scope = remember { PopDialogScope { then -> if (visible.targetState) { after = then; visible.targetState = false } } }
    LaunchedEffect(visible.currentState, visible.targetState) {
        if (!visible.targetState && !visible.currentState) {
            after?.invoke()
            after = null
            // Still here? The caller kept the dialog, so show it again rather than leave an
            // invisible window over the app. (If it was removed, this coroutine is cancelled first.)
            kotlinx.coroutines.delay(100)
            visible.targetState = true
        }
    }

    Dialog(
        onDismissRequest = { if (dismissible) scope.dismiss(onDismissRequest) },
        properties = DialogProperties(usePlatformDefaultWidth = false, decorFitsSystemWindows = false,
            dismissOnBackPress = dismissible, dismissOnClickOutside = false),
    ) {
        // The backdrop and the pop are drawn here, so the window adds neither a dim nor its own animation.
        val window = (LocalView.current.parent as? DialogWindowProvider)?.window
        SideEffect { window?.setDimAmount(0f); window?.setWindowAnimations(0) }
        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
            AnimatedVisibility(visible, enter = fadeIn(tween(200)), exit = fadeOut(tween(150, easing = KkEase.CssIn))) {
                Box(Modifier.fillMaxSize().background(Color.Black.copy(alpha = 0.5f)).clickable(
                    remember { MutableInteractionSource() }, indication = null, enabled = dismissible,
                ) { scope.dismiss(onDismissRequest) })
            }
            AnimatedVisibility(visible, Modifier.safeDrawingPadding().padding(16.dp),
                enter = EnterTransition.None, exit = ExitTransition.None) {
                val opacity by transition.animateFloat({ popSpec(reduced, entering = targetState == EnterExitState.Visible, opacity = true) }, "opacity") {
                    if (it == EnterExitState.Visible) 1f else 0f
                }
                val sx by transition.animateFloat({ popSpec(reduced, targetState == EnterExitState.Visible, peak = 1.04f, settle = 0.98f) }, "sx") {
                    when (it) { EnterExitState.PreEnter -> 0.6f; EnterExitState.Visible -> 1f; EnterExitState.PostExit -> 0.94f }
                }
                val sy by transition.animateFloat({ popSpec(reduced, targetState == EnterExitState.Visible, peak = 1.06f, settle = 0.99f) }, "sy") {
                    when (it) { EnterExitState.PreEnter -> 0.6f; EnterExitState.Visible -> 1f; EnterExitState.PostExit -> 0.9f }
                }
                val rise by transition.animateFloat({ if (reduced) tween(0) else tween(200, easing = KkEase.CssOut) }, "rise") {
                    if (it == EnterExitState.PreEnter) 0.04f else 0f
                }
                val shape = RoundedCornerShape(24.dp)
                Column(
                    modifier
                        .widthIn(max = width)
                        .fillMaxWidth()
                        .graphicsLayer {
                            alpha = opacity
                            if (!reduced) { scaleX = sx; scaleY = sy; translationY = size.height * rise }
                            transformOrigin = TransformOrigin.Center
                        }
                        .shadow(24.dp, shape, ambientColor = Color.Black.copy(alpha = 0.2f), spotColor = Color.Black.copy(alpha = 0.2f))
                        .background(kk.paper, shape)
                        .border(1.dp, kk.border, shape)
                        .semantics { if (title != null) paneTitle = title }
                        .padding(24.dp),
                ) {
                    CompositionLocalProvider(LocalContentColor provides kk.ink) {
                        Column(Modifier.weight(1f, fill = false).heightIn(min = 0.dp).verticalScroll(rememberScrollState())) {
                            if (art != null) Box(Modifier.fillMaxWidth().padding(bottom = 8.dp).popMedia(transition.targetState == EnterExitState.Visible), contentAlignment = Alignment.Center) { art() }
                            if (title != null) Text(title, style = MaterialTheme.typography.titleLarge, modifier = Modifier.padding(bottom = 10.dp))
                            CompositionLocalProvider(LocalContentColor provides kk.mutedInk) {
                                content(scope)
                            }
                        }
                        if (buttons != null) Row(
                            Modifier.fillMaxWidth().padding(top = 20.dp),
                            horizontalArrangement = Arrangement.spacedBy(8.dp, Alignment.End),
                            verticalAlignment = Alignment.CenterVertically,
                        ) { buttons(scope) }
                    }
                }
            }
        }
    }
}

private fun popSpec(reduced: Boolean, entering: Boolean, peak: Float = 1f, settle: Float = 1f, opacity: Boolean = false): FiniteAnimationSpec<Float> = when {
    reduced -> tween(if (entering) 150 else 100)
    !entering -> tween(150, easing = KkEase.CssIn)
    opacity -> tween(120, easing = KkEase.CssOut)
    else -> keyframes {
        durationMillis = 400
        0.6f at 0 using KkEase.CssOut
        peak at 200 using KkEase.CssInOut
        settle at 300 using KkEase.CssInOut
    }
}

/** Alert Dialog's media pop: 0 -> (1.2, 1.25) -> (0.9, 0.95) -> 1 in 0.5 s, a beat after the panel. */
@Composable
private fun Modifier.popMedia(visible: Boolean): Modifier {
    val reduced = Kirakira.reducedMotion
    val s = remember { androidx.compose.animation.core.Animatable(if (reduced) 1f else 0f) }
    LaunchedEffect(visible, reduced) {
        if (!visible) return@LaunchedEffect
        if (reduced) { s.snapTo(1f); return@LaunchedEffect }
        s.animateTo(1f, keyframes {
            durationMillis = 650
            0f at 150 using KkEase.CssInOut
            1.22f at 400 using KkEase.CssInOut
            0.92f at 525 using KkEase.CssInOut
        })
    }
    return graphicsLayer { scaleX = s.value; scaleY = s.value; alpha = (s.value * 5f).coerceIn(0f, 1f) }
}
