//! Pop Command's motion, as pure functions of [`motion::now`](crate::motion::now).

use std::time::{Duration, Instant};

use crate::motion::{Easing, Keyframes, Pose, Track, cubic_bezier, ms};

const HIGHLIGHT_MOVE: Duration = ms(320);
const HIGHLIGHT_FADE: Duration = ms(150);
const HIGHLIGHT_SCALE: Duration = ms(200);
pub(crate) const LIST_RESIZE: Duration = ms(200);
pub(crate) const EMPTY_POP: Duration = ms(300);
pub(crate) const EMPTY_FADE: Duration = ms(150);

fn progress(elapsed: Duration, duration: Duration) -> f32 {
    (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}

/// Kirakira's spring, `cubic-bezier(0.34, 1.56, 0.64, 1)`, overshoot and all.
pub(crate) fn spring() -> Easing {
    cubic_bezier(0.34, 1.56, 0.64, 1.0)
}

/// Kirakira's out curve, `cubic-bezier(0.05, 0.3, 0.1, 1)`.
pub(crate) fn out() -> Easing {
    cubic_bezier(0.05, 0.3, 0.1, 1.0)
}

/// A CSS transition on one number: a new target eases from wherever the value is now.
pub(crate) struct Glide {
    from: f32,
    to: f32,
    start: Option<Instant>,
    duration: Duration,
    ease: Easing,
}

impl Glide {
    pub fn new(duration: Duration, ease: Easing) -> Self {
        Self {
            from: 0.0,
            to: 0.0,
            start: None,
            duration,
            ease,
        }
    }

    pub fn value(&self, now: Instant) -> f32 {
        match self.start {
            None => self.to,
            Some(start) => {
                let p = progress(now.saturating_duration_since(start), self.duration);
                self.from + (self.to - self.from) * self.ease.sample(p)
            }
        }
    }

    /// Lands on `to` at once.
    pub fn jump(&mut self, to: f32) {
        self.from = to;
        self.to = to;
        self.start = None;
    }

    /// Heads for `to` from where the value is at `now`; `jump` lands at once.
    pub fn target(&mut self, to: f32, now: Instant, jump: bool) {
        if jump {
            self.jump(to);
        } else if (to - self.to).abs() > f32::EPSILON {
            self.from = self.value(now);
            self.to = to;
            self.start = Some(now);
        }
    }

    pub fn target_value(&self) -> f32 {
        self.to
    }

    pub fn running(&self, now: Instant) -> bool {
        self.start
            .is_some_and(|start| now.saturating_duration_since(start) < self.duration)
    }
}

/// The highlight plate behind the selected row.
pub(crate) struct Highlight {
    top: Glide,
    height: Glide,
    shown: bool,
    toggled: Option<Instant>,
}

/// Where the highlight is drawn this frame, in the list's content coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct HighlightFrame {
    pub top: f32,
    pub height: f32,
    pub opacity: f32,
    pub scale: f32,
    pub running: bool,
}

impl Default for Highlight {
    fn default() -> Self {
        Self {
            top: Glide::new(HIGHLIGHT_MOVE, spring()),
            height: Glide::new(HIGHLIGHT_MOVE, spring()),
            shown: false,
            toggled: None,
        }
    }
}

impl Highlight {
    /// Points the highlight at the selected row's `(top, height)`, or hides it.
    ///
    /// Appearing, it lands in place and fades in; moving, it springs there (or jumps under
    /// reduced motion); with nothing selected it fades where it is.
    pub fn update(&mut self, target: Option<(f32, f32)>, now: Instant, reduced: bool) {
        match target {
            Some((top, height)) if !self.shown => {
                self.top.jump(top);
                self.height.jump(height);
                self.shown = true;
                self.toggled = Some(now);
            }
            Some((top, height)) => {
                self.top.target(top, now, reduced);
                self.height.target(height, now, reduced);
            }
            None if self.shown => {
                self.shown = false;
                self.toggled = Some(now);
            }
            None => {}
        }
    }

    pub fn frame(&self, now: Instant, reduced: bool) -> HighlightFrame {
        let since = self
            .toggled
            .map(|toggled| now.saturating_duration_since(toggled))
            .unwrap_or(Duration::MAX);
        let fade = Easing::EaseOut.sample(progress(since, HIGHLIGHT_FADE));
        let grow = spring().sample(progress(since, HIGHLIGHT_SCALE));
        let (opacity, scale) = if self.shown {
            (fade, 0.9 + 0.1 * grow)
        } else {
            (1.0 - fade, 1.0 - 0.1 * grow)
        };
        HighlightFrame {
            top: self.top.value(now),
            height: self.height.value(now),
            opacity,
            scale: if reduced { 1.0 } else { scale },
            running: self.top.running(now)
                || self.height.running(now)
                || since < HIGHLIGHT_SCALE.max(HIGHLIGHT_FADE),
        }
    }
}

/// `kk-pop-command-empty`'s scale: 0.85 → (1.05, 1.08) at 55 % → 0.98 at 80 % → 1, ease-in-out.
pub(crate) fn empty_scale() -> Keyframes<Pose> {
    Track::new(Easing::EaseInOut)
        .at(0.0, Pose::new().scale(0.85))
        .at(0.55, Pose::new().scale_xy(1.05, 1.08))
        .at(0.8, Pose::new().scale(0.98))
        .at(1.0, Pose::new())
        .build()
}

/// `kk-pop-command-empty`'s opacity: 0 → 1 by 40 %.
pub(crate) fn empty_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.0_f32)
        .at(0.4, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// The empty message `elapsed` after it appeared, and whether it still moves.
pub(crate) fn empty_pop(elapsed: Duration, reduced: bool) -> (Pose, bool) {
    if reduced {
        let o = Easing::EaseOut.sample(progress(elapsed, EMPTY_FADE));
        return (Pose::new().opacity(o), elapsed < EMPTY_FADE);
    }
    let p = progress(elapsed, EMPTY_POP);
    (
        empty_scale().sample(p).opacity(empty_opacity().sample(p)),
        elapsed < EMPTY_POP,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn glide_eases_from_where_it_is() {
        let start = Instant::now();
        let mut glide = Glide::new(ms(200), Easing::Linear);
        glide.target(10.0, start, false);
        glide.jump(10.0);
        glide.target(20.0, start, false);
        assert!(close(glide.value(start + ms(100)), 15.0));
        // Retargeted halfway, it sets out from 15.
        glide.target(0.0, start + ms(100), false);
        assert!(close(glide.value(start + ms(100)), 15.0));
        assert!(close(glide.value(start + ms(300)), 0.0));
        assert!(!glide.running(start + ms(300)));
    }

    #[test]
    fn highlight_appears_in_place_then_springs() {
        let start = Instant::now();
        let mut highlight = Highlight::default();
        highlight.update(Some((4.0, 32.0)), start, false);
        let frame = highlight.frame(start, false);
        assert_eq!((frame.top, frame.height, frame.opacity), (4.0, 32.0, 0.0));
        assert!(close(frame.scale, 0.9));
        highlight.update(Some((36.0, 32.0)), start + ms(500), false);
        let mid = highlight.frame(start + ms(500 + 160), false);
        assert!(mid.top > 20.0 && mid.top < 40.0);
        // The spring overshoots its target on the way.
        let peak = (0..32)
            .map(|i| highlight.frame(start + ms(500 + i * 10), false).top)
            .fold(f32::MIN, f32::max);
        assert!(peak > 36.0);
        assert!(close(highlight.frame(start + ms(900), false).top, 36.0));
        highlight.update(None, start + ms(1000), false);
        assert!(close(highlight.frame(start + ms(1150), false).opacity, 0.0));
    }

    #[test]
    fn reduced_highlight_jumps() {
        let start = Instant::now();
        let mut highlight = Highlight::default();
        highlight.update(Some((4.0, 32.0)), start, true);
        highlight.update(Some((36.0, 32.0)), start + ms(500), true);
        assert_eq!(highlight.frame(start + ms(500), true).top, 36.0);
    }

    #[test]
    fn empty_matches_the_web_keyframes() {
        crate::parity::assert_pose_track(
            "pop-command",
            "kk-pop-command-empty",
            &empty_scale(),
            &[],
        );
        crate::parity::assert_number_track(
            "pop-command",
            "kk-pop-command-empty",
            "opacity",
            &empty_opacity(),
        );
    }
}
