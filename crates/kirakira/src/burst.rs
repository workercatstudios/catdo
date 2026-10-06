//! Burst: a ring, streaks and confetti that fire out from behind their children.
//!
//! - The ring is a disc that grows while a hole opens a beat behind it and eats it from inside.
//! - Streaks are two crosses of four: thick, long ones first, thin, short ones a beat later, turned
//!   45°. Each is a rounded bar that slides out through a rounded window at the end of its spoke.
//! - Ten confetti dots fly out along 36° spokes, scaling in on the way and out at the end.
//!
//! `BurstTrigger::Mount` fires once when it mounts and runs on a [`Clock`], so it is
//! composition-safe: inside a [`Timeline`](crate::timeline::Timeline) it plays on the timeline's
//! clock. Clicks and [`Burst::fire`] restart it on real time with a [`Pulse`]. Under reduced motion
//! there is no burst.
//!
//! Differences from the web version: the shapes are painted in a `canvas` behind the children
//! rather than built from masked and clipped spans, which gives the same outlines. The ring's hole
//! has a hard edge where the web mask feathers it over 1px.

use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Bounds, ElementId, Hsla, InteractiveElement as _, IntoElement, ParentElement,
    Pixels, RenderOnce, StatefulInteractiveElement as _, StyleRefinement, Styled, Window, canvas,
    div,
};

use crate::bounce_text::font_size;
use crate::motion::{Clock, Easing, Keyframes, Pulse, Timing, Track, Trigger, delay_ms, ms};
use crate::shapes::Shape;
use crate::theme::ActiveKira as _;

/// One of the burst's effects.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BurstEffect {
    Ring,
    Streaks,
    Confetti,
}

/// When the burst fires on its own.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BurstTrigger {
    /// On every click inside.
    #[default]
    Click,
    /// Once, when it mounts.
    Mount,
}

/// How long one burst runs: the latest confetti dot finishes scaling out at 0.85 s.
pub const BURST_MS: u64 = 850;

/// A streak: its angle in degrees (0° points up, clockwise), weight, reach and delay in seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Streak {
    pub angle: f32,
    pub weight: f32,
    pub reach: f32,
    pub delay: f32,
}

/// Two crosses of four: thick and long first, thin and short a beat later, turned 45°.
pub fn streaks() -> [Streak; 8] {
    std::array::from_fn(|i| {
        let late = i >= 4;
        Streak {
            angle: (i % 4) as f32 * 90.0 + if late { 45.0 } else { 0.0 },
            weight: if late { 1.5 } else { 2.5 },
            reach: if late { 0.85 } else { 1.0 },
            delay: if late { 0.1 } else { 0.0 },
        }
    })
}

/// A confetti dot: where it lands as a fraction of the radius, its weight and delay in seconds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dot {
    pub x: f32,
    pub y: f32,
    pub weight: f32,
    pub delay: f32,
}

/// Ten dots on 36° spokes. Index rules vary size, reach and delay so the ring of dots breaks up.
pub fn confetti() -> [Dot; 10] {
    std::array::from_fn(|i| {
        let angle = (i as f32 * 36.0 - 90.0).to_radians();
        let (reach, weight, delay) = if i % 3 == 2 {
            (0.8, 3.0, 0.1)
        } else if i % 2 == 1 {
            (0.92, 4.0, 0.05)
        } else {
            (1.05, 6.0, 0.0)
        };
        Dot {
            x: angle.cos() * reach,
            y: angle.sin() * reach,
            weight,
            delay,
        }
    })
}

/// `@keyframes kk-burst-grow` and `kk-burst-in`: scale from nothing to full size.
pub fn grow(ease: Easing) -> Keyframes<f32> {
    Track::new(ease).at(0.0, 0.0).at(1.0, 1.0).build()
}

/// `@keyframes kk-burst-out`: scale from full size to nothing.
pub fn shrink(ease: Easing) -> Keyframes<f32> {
    Track::new(ease).at(0.0, 1.0).at(1.0, 0.0).build()
}

/// `@keyframes kk-burst-slide`: the bar's offset as a fraction of the window, from 101 % below it
/// (towards the centre) to 101 % past it.
pub fn slide(ease: Easing) -> Keyframes<f32> {
    Track::new(ease).at(0.0, 1.01).at(1.0, -1.01).build()
}

/// The visible part of a streak whose bar sits `shift` window-heights below its window (the
/// [`slide`] value): the stretch of its spoke, as distances from the centre, where the bar and
/// the window overlap. Both are stadiums `width` wide; the window covers the outer 42 % of a
/// spoke `length` long. `None` while they don't touch.
pub fn streak_span(length: f32, width: f32, shift: f32) -> Option<(f32, f32, f32)> {
    let window = 0.42 * length;
    let r = (width / 2.0).min(window / 2.0);
    let shift = window * shift;
    let (w0, w1) = (length - window + r, length - r);
    let (b0, b1) = (w0 - shift, w1 - shift);
    let (start, end) = (w0.max(b0), w1.min(b1));
    if end >= start {
        Some((start, end, r))
    } else if start - end < 2.0 * r {
        // The rounded ends only graze: a shrinking lens, drawn as a dot.
        let mid = (start + end) / 2.0;
        Some((mid, mid, r - (start - end) / 2.0))
    } else {
        None
    }
}

/// Fires a ring, streaks and confetti from behind its children.
///
/// ```ignore
/// Burst::new("stamp").trigger(BurstTrigger::Mount).size(1.7).child(stamp)
/// ```
#[derive(IntoElement)]
pub struct Burst {
    id: ElementId,
    effects: Vec<BurstEffect>,
    trigger: BurstTrigger,
    fire: Option<usize>,
    colors: Option<Vec<Hsla>>,
    size: f32,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl Burst {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            effects: vec![
                BurstEffect::Ring,
                BurstEffect::Streaks,
                BurstEffect::Confetti,
            ],
            trigger: BurstTrigger::Click,
            fire: None,
            colors: None,
            size: 1.0,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    /// Which effects to fire. All three by default.
    pub fn effects(mut self, effects: impl IntoIterator<Item = BurstEffect>) -> Self {
        self.effects = effects.into_iter().collect();
        self
    }

    /// When it fires on its own. [`BurstTrigger::Click`] by default.
    pub fn trigger(mut self, trigger: BurstTrigger) -> Self {
        self.trigger = trigger;
        self
    }

    /// Controlled firing: each change of this number fires once. Turns off click firing.
    pub fn fire(mut self, fire: usize) -> Self {
        self.fire = Some(fire);
        self
    }

    /// Ring, streak and confetti colours, cycled in order. Kirakira pink, yellow, sky and lime by
    /// default.
    pub fn colors(mut self, colors: impl IntoIterator<Item = impl Into<Hsla>>) -> Self {
        self.colors = Some(colors.into_iter().map(Into::into).collect());
        self
    }

    /// Scales the burst radius, which is 3em at 1.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }
}

impl Styled for Burst {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Burst {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

#[derive(Default)]
struct LastFire(Option<usize>);

/// Linear progress through an animation of `duration_s` after `delay_s`, held at both ends.
fn progress(elapsed: std::time::Duration, delay_s: f32, duration_s: f32) -> f32 {
    Timing::new(std::time::Duration::from_secs_f32(duration_s))
        .delay(delay_ms((delay_s * 1000.0).round() as u64))
        .sample(elapsed)
        .directed_progress
}

struct Frame {
    elapsed: std::time::Duration,
    radius: f32,
    size: f32,
    effects: Vec<BurstEffect>,
    colors: Vec<Hsla>,
    out: Easing,
    snap: Easing,
    r#in: Easing,
}

impl Frame {
    fn color(&self, i: usize) -> Hsla {
        self.colors[i % self.colors.len()]
    }

    fn paint(&self, bounds: Bounds<Pixels>, window: &mut Window) {
        let center = bounds.center();
        let (cx, cy) = (f32::from(center.x), f32::from(center.y));
        let t = self.elapsed;
        let r = self.radius;

        if self.effects.contains(&BurstEffect::Ring) {
            let grow = grow(self.out.clone()).sample(progress(t, 0.0, 0.45));
            let hole = self.out.sample(progress(t, 0.07, 0.45));
            let outer = r * grow;
            let inner = outer * hole;
            if outer > 0.0 && hole < 1.0 {
                let mut ring = Shape::new();
                ring.polygon(Shape::circle_points(cx, cy, outer));
                if inner > 0.0 {
                    ring.hole(Shape::circle_points(cx, cy, inner));
                }
                ring.paint(window, self.color(0));
            }
        }

        if self.effects.contains(&BurstEffect::Streaks) {
            for (color, set) in [(1, 0..4), (2, 4..8)] {
                let mut shape = Shape::new();
                for streak in &streaks()[set] {
                    let width = self.size * streak.weight + 0.5;
                    let length = r * streak.reach;
                    let shift = slide(self.snap.clone()).sample(progress(t, streak.delay, 0.5));
                    let Some((start, end, half)) = streak_span(length, width, shift) else {
                        continue;
                    };
                    let a = streak.angle.to_radians();
                    let (dx, dy) = (a.sin(), -a.cos());
                    let points = [
                        (cx + dx * start, cy + dy * start),
                        (cx + dx * end, cy + dy * end),
                    ];
                    shape.stroke_polyline(&points, false, half * 2.0);
                }
                shape.paint(window, self.color(color));
            }
        }

        if self.effects.contains(&BurstEffect::Confetti) {
            for (i, dot) in confetti().iter().enumerate() {
                let fly = self.out.sample(progress(t, dot.delay, 0.7));
                let grow = grow(Easing::EaseOut).sample(progress(t, dot.delay, 0.3));
                // The scale-out only applies once it starts (fill-mode: forwards), then wins.
                let shrink_delay = dot.delay + 0.45;
                let scale = if t.as_secs_f32() >= shrink_delay {
                    shrink(self.r#in.clone()).sample(progress(t, shrink_delay, 0.3))
                } else {
                    grow
                };
                let d = self.size * dot.weight + 1.0;
                let radius = d / 2.0 * scale;
                if radius > 0.01 {
                    let mut shape = Shape::new();
                    shape.circle(cx + dot.x * r * fly, cy + dot.y * r * fly, radius);
                    shape.paint(window, self.color(i));
                }
            }
        }
    }
}

impl RenderOnce for Burst {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let clock = Clock::new(self.id.clone(), Trigger::Mount, window, cx);
        let pulse = Pulse::new((self.id.clone(), "kk-burst"), window, cx);

        // A change of `fire` fires once; its first value doesn't.
        let last = window.use_keyed_state((self.id.clone(), "kk-burst-fire"), cx, |_, _| {
            LastFire(self.fire)
        });
        if let Some(fire) = self.fire
            && last.read(cx).0 != Some(fire)
        {
            last.update(cx, |last, _| last.0 = Some(fire));
            pulse.fire(cx);
        }
        let pulse = Pulse::new((self.id.clone(), "kk-burst"), window, cx);

        let end = ms(BURST_MS);
        let elapsed = match pulse.elapsed() {
            Some(elapsed) => {
                pulse.animate(end, window);
                Some(elapsed)
            }
            None if self.trigger == BurstTrigger::Mount && !clock.reduced() => {
                clock.animate(Some(end), window);
                Some(clock.elapsed())
            }
            None => None,
        }
        .filter(|elapsed| *elapsed < end);

        let palette = cx.kira();
        let colors = self
            .colors
            .filter(|colors| !colors.is_empty())
            .unwrap_or_else(|| vec![palette.pink, palette.yellow, palette.sky, palette.lime]);
        let curves = cx.curves();
        let size = self.size;
        let radius = size * 3.0 * f32::from(font_size(&self.style, window));
        let frame = elapsed.map(|elapsed| Frame {
            elapsed,
            radius,
            size,
            effects: self.effects,
            colors,
            out: curves.out,
            snap: curves.snap,
            r#in: curves.r#in,
        });

        let clickable = self.trigger == BurstTrigger::Click && self.fire.is_none();
        div()
            .id(self.id)
            .relative()
            .flex()
            .flex_none()
            .refine_style(&self.style)
            .when(clickable, |this| {
                this.on_click(move |_, _, cx| pulse.fire(cx))
            })
            .when_some(frame, |this, frame| {
                // Behind the children, like the web layer's z-index: -1.
                this.child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, _, window, _| frame.paint(bounds, window),
                    )
                    .absolute()
                    .size_full(),
                )
            })
            .children(self.children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parity::assert_number_track;

    #[test]
    fn streaks_make_two_crosses() {
        let angles: Vec<f32> = streaks().iter().map(|s| s.angle).collect();
        assert_eq!(angles, [0.0, 90.0, 180.0, 270.0, 45.0, 135.0, 225.0, 315.0]);
        assert_eq!(streaks()[5].weight, 1.5);
        assert_eq!(streaks()[5].reach, 0.85);
    }

    #[test]
    fn confetti_follows_the_index_rules() {
        let dots = confetti();
        // Dot 0 points straight up at full reach.
        assert!((dots[0].x).abs() < 1e-4 && (dots[0].y + 1.05).abs() < 1e-4);
        assert_eq!((dots[1].weight, dots[1].delay), (4.0, 0.05));
        assert_eq!((dots[2].weight, dots[2].delay), (3.0, 0.1));
        assert_eq!((dots[5].weight, dots[5].delay), (3.0, 0.1));
        assert!((dots[2].x.hypot(dots[2].y) - 0.8).abs() < 1e-4);
    }

    // Not checked against the web: `kk-burst-eat` animates a custom property (the mask's hole,
    // ported as the ring's inner radius) and `kk-burst-fly` translates by `calc(var(...))`.
    #[test]
    fn grow_matches_the_web() {
        assert_number_track("burst", "kk-burst-grow", "sx", &grow(Easing::Linear));
        assert_number_track("burst", "kk-burst-in", "sx", &grow(Easing::Linear));
    }

    #[test]
    fn shrink_matches_the_web() {
        assert_number_track("burst", "kk-burst-out", "sx", &shrink(Easing::Linear));
    }

    #[test]
    fn slide_matches_the_web() {
        assert_number_track("burst", "kk-burst-slide", "yp", &slide(Easing::Linear));
    }

    #[test]
    fn a_streak_slides_through_its_window() {
        // Hidden below the window at the start and past it at the end.
        assert_eq!(streak_span(100.0, 5.0, 1.01), None);
        assert_eq!(streak_span(100.0, 5.0, -1.01), None);
        // Halfway, the bar fills the window: the outer 42 % of the spoke.
        let (start, end, r) = streak_span(100.0, 5.0, 0.0).unwrap();
        assert!((start - r - 58.0).abs() < 1e-3 && (end + r - 100.0).abs() < 1e-3);
    }
}
