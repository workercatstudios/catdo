//! Small, shared motion helpers. Every entrance respects the platform's
//! reduced-motion preference through GPUI's `reduce_motion` flag.

use std::time::Duration;

use gpui_kit::component::animation::ease_out_cubic;
use gpui_kit::{Animation, AnimationExt, App, ElementId, IntoElement, Styled, px};

/// Fade and lift an element into place once, when it first appears.
pub fn settle<E: IntoElement + Styled + 'static>(
    element: E,
    id: impl Into<ElementId>,
    cx: &App,
) -> gpui_kit::AnimationElement<E> {
    settle_after(element, id, 0, cx)
}

/// Like [`settle`], with a stagger so a list arrives row by row.
pub fn settle_after<E: IntoElement + Styled + 'static>(
    element: E,
    id: impl Into<ElementId>,
    index: usize,
    cx: &App,
) -> gpui_kit::AnimationElement<E> {
    let reduced = cx.reduce_motion();
    let delay = Duration::from_millis(22 * index.min(12) as u64);
    let base = Duration::from_millis(if reduced { 1 } else { 260 });
    let total = base + delay;
    let delay_share = delay.as_secs_f32() / total.as_secs_f32();
    element.with_animation(id, Animation::new(total), move |el, t| {
        let progress = if reduced {
            1.0
        } else {
            ease_out_cubic(((t - delay_share) / (1.0 - delay_share)).clamp(0.0, 1.0))
        };
        el.opacity(progress)
            .relative()
            .top(px(8.0 * (1.0 - progress)))
    })
}

/// Fade an overlay in.
pub fn fade_in<E: IntoElement + Styled + 'static>(
    element: E,
    id: impl Into<ElementId>,
    cx: &App,
) -> gpui_kit::AnimationElement<E> {
    let reduced = cx.reduce_motion();
    element.with_animation(
        id,
        Animation::new(Duration::from_millis(if reduced { 1 } else { 160 }))
            .with_easing(ease_out_cubic),
        |el, t| el.opacity(t),
    )
}
