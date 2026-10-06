//! CatDo's motion, on Kirakira's curves and keyframes. Entrances settle on Kirakira's `out`
//! curve, dialogs pop in like Kirakira's Pop Dialog, and a completed task's check pops before its
//! row leaves. Every helper honours the platform's reduced-motion preference through GPUI's
//! `reduce_motion` flag: entrances land at once and only fades remain.

use std::time::Duration;

use gpui_kit::{Animation, AnimationExt, App, ElementId, IntoElement, Styled, Window, px};
use kirakira::ActiveKira as _;
use kirakira::motion::{Clock, Easing, Keyframes, Pose, Track, Transform, Trigger, ms, transform};

/// How long a completed task stays in its list: the check pops and bursts, then the row fades.
pub const LEAVE: Duration = ms(850);
/// When the leaving row starts to fade.
const LEAVE_FADE: Duration = ms(480);

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
    let out = cx.curves().out;
    let delay = Duration::from_millis(28 * index.min(12) as u64);
    let base = Duration::from_millis(if reduced { 1 } else { 320 });
    let total = base + delay;
    let delay_share = delay.as_secs_f32() / total.as_secs_f32();
    element.with_animation(id, Animation::new(total), move |el, t| {
        let progress = if reduced {
            1.0
        } else {
            out.sample(((t - delay_share) / (1.0 - delay_share)).clamp(0.0, 1.0))
        };
        el.opacity(progress.clamp(0.0, 1.0))
            .relative()
            .top(px(10.0 * (1.0 - progress)))
    })
}

/// Fade an overlay's backdrop in (0.2 s, ease-out), as Kirakira's dialogs do.
pub fn fade_in<E: IntoElement + Styled + 'static>(
    element: E,
    id: impl Into<ElementId>,
    cx: &App,
) -> gpui_kit::AnimationElement<E> {
    let reduced = cx.reduce_motion();
    element.with_animation(
        id,
        Animation::new(Duration::from_millis(if reduced { 1 } else { 200 })),
        |el, t| el.opacity(Easing::EaseOut.sample(t)),
    )
}

/// Kirakira's Pop Dialog entrance (`kk-pop-dialog-in`, 0.4 s): the panel grows from 0.6 past full
/// size and settles, rising the last 4 % of its height, opaque by 30 %. GPUI scales a subtree
/// evenly through its rem size, so each two-axis keyframe uses its geometric mean, as Kirakira's
/// own dialog does; size the panel in rems so it scales with its text. Under reduced motion it
/// fades in where it rests (0.15 s).
pub fn pop_in(
    id: impl Into<ElementId>,
    child: impl IntoElement,
    window: &mut Window,
    cx: &mut App,
) -> Transform {
    let id = id.into();
    let clock = Clock::new(id.clone(), Trigger::Mount, window, cx);
    let elapsed = clock.elapsed();
    let pose = if cx.reduce_motion() {
        let t = progress(clock_time(&clock), ms(150));
        Pose::new().opacity(Easing::EaseOut.sample(t))
    } else {
        clock.animate(Some(ms(400)), window);
        let p = progress(elapsed, ms(400));
        let scale = pop_scale().sample(p);
        Pose::new()
            .scale(scale)
            .yp(pop_rise().sample(p) * scale)
            .opacity(pop_opacity().sample(p))
    };
    transform(id, pose, child)
}

/// A reduced-motion clock reports entrances as finished; fades still read the real time.
fn clock_time(clock: &Clock) -> Duration {
    if clock.reduced() {
        ms(150)
    } else {
        clock.elapsed()
    }
}

fn progress(elapsed: Duration, duration: Duration) -> f32 {
    (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}

/// `kk-pop-dialog-in`'s scale: 0.6 → 1.05 (50 %) → 0.985 (75 %) → 1, ease-out into the overshoot.
fn pop_scale() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.6_f32, Easing::EaseOut)
        .at(0.5, 1.05)
        .at(0.75, 0.985)
        .at(1.0, 1.0)
        .build()
}

/// `kk-pop-dialog-in`'s rise, as a fraction of the panel's height: 4 % → 0 by 50 %.
fn pop_rise() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.04_f32, Easing::EaseOut)
        .at(0.5, 0.0)
        .at(1.0, 0.0)
        .build()
}

/// `kk-pop-dialog-in`'s opacity: 0 → 1 by 30 %.
fn pop_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.0_f32, Easing::EaseOut)
        .at(0.3, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// Kirakira Checkbox's rebound as a task is checked: 0.85 → 1.08 → 0.97 → 1 in 0.34 s.
pub fn check_pop() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.85_f32)
        .at(0.4, 1.08)
        .at(0.7, 0.97)
        .at(1.0, 1.0)
        .build()
}

/// How far a leaving row has faded at `t` of [`LEAVE`]: it holds while the check pops and bursts,
/// then fades and drifts aside on Kirakira's `in` curve.
pub fn leave_progress(t: f32, ease: &Easing) -> f32 {
    let start = LEAVE_FADE.as_secs_f32() / LEAVE.as_secs_f32();
    ease.sample(((t - start) / (1.0 - start)).clamp(0.0, 1.0))
}
