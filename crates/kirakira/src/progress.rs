//! Progress: a bar that springs to each new value, with optional moving stripes.
//!
//! Replaces `gpui_kit::component::progress`, rebuilt on gpui-base with GPUI Component's structure,
//! sizes and colours; `ProgressCircle` is re-exported unchanged.
//!
//! The fill grows on the spring curve over 0.5 s, so each new value overshoots a little and
//! settles while its left end stays put. [`Progress::striped`] adds diagonal stripes that slide one
//! period (1 rem) every 0.8 s and fade out in 0.3 s once the bar is full. `loading` keeps GPUI
//! Component's indeterminate slide, on a [`Clock`] so a [`Timeline`](crate::timeline::Timeline)
//! or a screenshot can seek it. Under reduced motion the bar eases to its value in 0.2 s without
//! overshoot, and the stripes and the indeterminate bar hold still.
//!
//! Differences: GPUI clips children to rectangles, not rounded ones, so the stripes are paths in a
//! `canvas`, each one clipped to the fill's pill shape (a convex polygon clip) rather than a CSS
//! gradient under `overflow: hidden`.

use std::time::Duration;

use gpui_kit::base::{Progress as BaseProgress, ProgressIndicator, ProgressTrack};
use gpui_kit::component::{ActiveTheme as _, Sizable, Size, StyledExt as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, Background, Bounds, ElementId, Hsla, IntoElement, IsZero as _, ParentElement as _,
    PathBuilder, Pixels, RenderOnce, SharedString, StyleRefinement, Styled, Window, canvas,
    ease_in_out, point, px, relative,
};

#[allow(unused_imports)]
pub use gpui_kit::component::progress::*;

use crate::glide::glide_always;
use crate::motion::{Clock, Easing, Keyframes, Track, Trigger, ms};
use crate::theme::ActiveKira as _;

const GROW: Duration = ms(500);
const GROW_REDUCED: Duration = ms(200);
const STRIPE_LOOP: Duration = ms(800);
const STRIPE_FADE: Duration = ms(300);
/// GPUI Component's indeterminate slide: one pass a second, linear.
const LOADING_LOOP: Duration = ms(1000);
/// The stripes' colour: the primary foreground at 28 %.
const STRIPE_ALPHA: f32 = 0.28;

/// `kk-pop-progress-stripes`: one period (1 rem) to the right per loop, linear.
fn stripe_track() -> Keyframes<f32> {
    Track::new(Easing::Linear).at(0.0, 0.0).at(1.0, 1.0).build()
}

/// Clips a convex polygon to a convex `clip` polygon (both counter-clockwise or both clockwise),
/// Sutherland–Hodgman style.
pub(crate) fn clip_convex(subject: &[(f32, f32)], clip: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut output = subject.to_vec();
    // The clip polygon's orientation decides which side is inside.
    let area: f32 = clip
        .iter()
        .zip(clip.iter().cycle().skip(1))
        .map(|(a, b)| a.0 * b.1 - b.0 * a.1)
        .sum();
    let sign = area.signum();
    for (i, &a) in clip.iter().enumerate() {
        let b = clip[(i + 1) % clip.len()];
        let side = |p: (f32, f32)| sign * ((b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0));
        let input = std::mem::take(&mut output);
        for (j, &p) in input.iter().enumerate() {
            let q = input[(j + 1) % input.len()];
            let (sp, sq) = (side(p), side(q));
            if sp >= 0.0 {
                output.push(p);
            }
            if (sp >= 0.0) != (sq >= 0.0) {
                let t = sp / (sp - sq);
                output.push((p.0 + (q.0 - p.0) * t, p.1 + (q.1 - p.1) * t));
            }
        }
        if output.is_empty() {
            break;
        }
    }
    output
}

/// A pill (rounded rectangle) of `w` × `h` with corner radius `r`, as a polygon.
pub(crate) fn pill(w: f32, h: f32, r: f32) -> Vec<(f32, f32)> {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let steps = 8;
    let corners = [
        (w - r, r, -90.0_f32),
        (w - r, h - r, 0.0),
        (r, h - r, 90.0),
        (r, r, 180.0),
    ];
    let mut points = Vec::new();
    for (cx, cy, start) in corners {
        for i in 0..=steps {
            let angle = (start + 90.0 * i as f32 / steps as f32).to_radians();
            points.push((cx + r * angle.cos(), cy + r * angle.sin()));
        }
    }
    points
}

/// The stripes over a fill of `w` × `h`: "/" bands half a `period` wide, shifted right by
/// `phase` periods, each clipped to the pill.
pub(crate) fn stripes(w: f32, h: f32, r: f32, period: f32, phase: f32) -> Vec<Vec<(f32, f32)>> {
    let shape = pill(w, h, r);
    let half = period / 2.0;
    let offset = phase.fract() * period;
    let mut bands = Vec::new();
    let mut start = offset - period * ((h / period).ceil() + 1.0);
    while start < w + h {
        let band = [
            (start, 0.0),
            (start + half, 0.0),
            (start + half - h, h),
            (start - h, h),
        ];
        let clipped = clip_convex(&band, &shape);
        if clipped.len() >= 3 {
            bands.push(clipped);
        }
        start += period;
    }
    bands
}

fn paint_stripes(
    bounds: Bounds<Pixels>,
    radius: Pixels,
    period: Pixels,
    phase: f32,
    color: Hsla,
    window: &mut Window,
) {
    let (w, h) = (f32::from(bounds.size.width), f32::from(bounds.size.height));
    for band in stripes(w, h, f32::from(radius), f32::from(period), phase) {
        let points: Vec<_> = band
            .iter()
            .map(|&(x, y)| point(bounds.origin.x + px(x), bounds.origin.y + px(y)))
            .collect();
        let mut path = PathBuilder::fill();
        path.add_polygon(&points, true);
        if let Ok(path) = path.build() {
            window.paint_path(path, color);
        }
    }
}

/// A linear progress bar.
#[derive(IntoElement)]
pub struct Progress {
    id: ElementId,
    style: StyleRefinement,
    color: Option<Hsla>,
    value: f32,
    accessibility_label: Option<SharedString>,
    size: Size,
    loading: bool,
    striped: bool,
}

impl Progress {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: Default::default(),
            color: None,
            accessibility_label: None,
            style: StyleRefinement::default(),
            size: Size::default(),
            loading: false,
            striped: false,
        }
    }

    /// The indeterminate loading slide, ignoring `value`.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// The percentage, 0 to 100.
    pub fn value(mut self, value: f32) -> Self {
        self.value = value.clamp(0., 100.);
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Diagonal stripes that move while the bar fills. Not in GPUI Component.
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = striped;
        self
    }
}

impl Styled for Progress {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Progress {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

struct Frozen(Option<f32>);

impl RenderOnce for Progress {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let bg = self
            .color
            .map(Background::from)
            .unwrap_or(cx.theme().tokens.progress_bar.into());
        let value = self.value;
        let loading = self.loading;
        let reduce_motion = cx.reduce_motion();
        let id = self.id.clone();

        let radius = self.style.corner_radii.clone();
        let inner_style = StyleRefinement {
            corner_radii: radius,
            ..Default::default()
        };

        let (height, pill_radius) = match self.size {
            Size::XSmall => (px(4.), px(2.)),
            Size::Small => (px(6.), px(3.)),
            Size::Medium => (px(8.), px(4.)),
            Size::Large => (px(10.), px(5.)),
            Size::Size(s) => (s, s / 2.),
        };
        let radius = if cx.theme().radius.is_zero() {
            px(0.)
        } else {
            pill_radius
        };

        let (duration, easing) = if reduce_motion {
            (GROW_REDUCED, Easing::EaseOut)
        } else {
            (GROW, cx.curves().spring)
        };
        let animated = glide_always((id.clone(), "kk-fill"), value, duration, easing, window, cx);

        // The stripes slide on a loop and hold still once the bar is full (or under reduced
        // motion), fading out over 0.3 s.
        let complete = value >= 100.;
        let stripe_alpha = crate::glide::glide(
            (id.clone(), "kk-stripes"),
            if complete { 0.0 } else { 1.0 },
            STRIPE_FADE,
            Easing::EaseOut,
            window,
            cx,
        );
        let phase = if self.striped && !loading {
            let clock = Clock::new((id.clone(), "kk-stripe-clock"), Trigger::Mount, window, cx);
            let running = !complete && !clock.reduced();
            let live = if clock.reduced() {
                0.0
            } else {
                let elapsed = clock.elapsed().as_secs_f32() / STRIPE_LOOP.as_secs_f32();
                stripe_track().sample(elapsed.fract())
            };
            let frozen = window.use_keyed_state((id.clone(), "kk-frozen"), cx, |_, _| Frozen(None));
            let held = frozen.read(cx).0;
            match (complete, held) {
                (true, Some(phase)) => phase,
                (true, None) => {
                    frozen.update(cx, |frozen, _| frozen.0 = Some(live));
                    live
                }
                (false, held) => {
                    if held.is_some() {
                        frozen.update(cx, |frozen, _| frozen.0 = None);
                    }
                    if running {
                        clock.animate(None, window);
                    }
                    live
                }
            }
        } else {
            0.0
        };
        // GPUI Component's indeterminate slide, as a loop on the clock: the right end runs to the
        // far side, then the left end follows it in the second half.
        let loading_delta = (loading && !reduce_motion).then(|| {
            let clock = Clock::new((id.clone(), "kk-loading-clock"), Trigger::Mount, window, cx);
            clock.animate(None, window);
            (clock.elapsed().as_secs_f32() / LOADING_LOOP.as_secs_f32()).fract()
        });
        let show_stripes = self.striped && !loading && stripe_alpha > 0.0;
        let stripe_color = cx
            .theme()
            .primary_foreground
            .opacity(STRIPE_ALPHA * stripe_alpha);
        let period = window.rem_size();

        BaseProgress::new(self.id)
            .value(value)
            .indeterminate(loading)
            .when_some(self.accessibility_label, |this, label| {
                this.accessibility_label(label)
            })
            .w_full()
            .relative()
            .h(height)
            .rounded(radius)
            .refine_style(&self.style)
            .child(
                ProgressTrack::new()
                    .absolute()
                    .size_full()
                    .bg(bg.opacity(0.2))
                    .rounded(radius)
                    .refine_style(&inner_style),
            )
            .child(
                ProgressIndicator::new()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h_full()
                    .bg(bg)
                    .rounded(radius)
                    .refine_style(&inner_style)
                    .when(show_stripes, |this| {
                        this.child(
                            canvas(
                                |_, _, _| {},
                                move |bounds, _, window, _| {
                                    paint_stripes(
                                        bounds,
                                        radius,
                                        period,
                                        phase,
                                        stripe_color,
                                        window,
                                    )
                                },
                            )
                            .absolute()
                            .size_full(),
                        )
                    })
                    .map(|this| {
                        if let Some(delta) = loading_delta {
                            let start = relative(ease_in_out(((delta - 0.5) / 0.5).clamp(0., 1.)));
                            let end = relative(ease_in_out(1.0 - delta));
                            this.when(delta > 0.5, |this| this.left(start)).right(end)
                        } else if loading {
                            this.left(relative(0.325)).right(relative(0.325))
                        } else {
                            // The spring overshoots past the value; never past the track.
                            this.w(relative((animated / 100.).clamp(0., 1.)))
                        }
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stripes_match_the_web() {
        // The web slides `translate: 1rem 0`; the track is in rems.
        crate::parity::assert_number_track(
            "pop-progress",
            "kk-pop-progress-stripes",
            "x",
            &stripe_track(),
        );
    }

    #[test]
    fn clipping_keeps_the_inside() {
        let square = [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)];
        let wide = [(-5.0, 2.0), (5.0, 2.0), (5.0, 8.0), (-5.0, 8.0)];
        let clipped = clip_convex(&wide, &square);
        let xs: Vec<f32> = clipped.iter().map(|p| p.0).collect();
        assert!(xs.iter().all(|x| (0.0..=5.0).contains(x)));
        assert_eq!(clipped.len(), 4);
        // Outside entirely: nothing left.
        let away = [(20.0, 0.0), (30.0, 0.0), (30.0, 5.0)];
        assert!(clip_convex(&away, &square).len() < 3);
    }

    #[test]
    fn stripes_stay_inside_the_pill() {
        let (w, h, r) = (100.0, 8.0, 4.0);
        let bands = stripes(w, h, r, 16.0, 0.25);
        assert!(!bands.is_empty());
        for band in bands {
            for (x, y) in band {
                assert!((-1e-3..=w + 1e-3).contains(&x) && (-1e-3..=h + 1e-3).contains(&y));
            }
        }
    }
}
