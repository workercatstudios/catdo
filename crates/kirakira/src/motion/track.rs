use gpui_kit::base::Interpolate;

use super::{Easing, Keyframe, Keyframes};

/// A CSS `@keyframes` rule.
///
/// CSS applies the animation's timing function to each segment between keyframes, not to the
/// whole run, and a keyframe's own `animation-timing-function` overrides it for the segment that
/// starts there. `Track` does the same: [`Track::new`] takes the animation's timing function, and
/// [`Track::at_ease`] overrides one segment. Pair the result with a linear
/// [`Timing`](super::Timing).
///
/// ```
/// use kirakira::motion::{Easing, Pose, Track};
///
/// // @keyframes kk-pop-button { 0% { scale: .95 } 45% { scale: 1.05 } 75% { scale: .98 } 100% { scale: 1 } }
/// let pop = Track::new(Easing::EaseInOut)
///     .at(0.0, Pose::new().scale(0.95))
///     .at(0.45, Pose::new().scale(1.05))
///     .at(0.75, Pose::new().scale(0.98))
///     .at(1.0, Pose::new())
///     .build();
/// assert_eq!(pop.sample(1.0), Pose::new());
/// ```
pub struct Track<T> {
    ease: Easing,
    frames: Vec<Keyframe<T>>,
}

impl<T: Interpolate> Track<T> {
    /// A rule whose segments ease with `ease`, the `animation-timing-function`.
    pub fn new(ease: Easing) -> Self {
        Self {
            ease,
            frames: Vec::new(),
        }
    }

    /// A keyframe at `offset` (`0.0..=1.0`).
    pub fn at(mut self, offset: f32, value: T) -> Self {
        let ease = self.ease.clone();
        self.frames.push(Keyframe::new(offset, value).ease(ease));
        self
    }

    /// A keyframe whose outgoing segment eases with `ease`.
    pub fn at_ease(mut self, offset: f32, value: T, ease: Easing) -> Self {
        self.frames.push(Keyframe::new(offset, value).ease(ease));
        self
    }

    /// The same value at several offsets, like `40%, 100% { scale: 0 }`.
    pub fn at_each(mut self, offsets: &[f32], value: T) -> Self {
        for &offset in offsets {
            self = self.at(offset, value.clone());
        }
        self
    }

    /// Builds the keyframes.
    ///
    /// # Panics
    ///
    /// When the offsets don't start at 0, end at 1 and increase: tracks are written as constants,
    /// so that is a programming error.
    pub fn build(self) -> Keyframes<T> {
        Keyframes::try_new(self.frames).expect("keyframe offsets run from 0 to 1 in order")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_segment_eases_on_its_own() {
        let track = Track::new(Easing::EaseIn)
            .at(0.0, 0.0_f32)
            .at(0.5, 1.0)
            .at(1.0, 2.0)
            .build();
        // Halfway through the first segment an ease-in curve is still below the midpoint.
        assert!(track.sample(0.25) < 0.5);
        // Keyframe values land exactly at their offsets.
        assert_eq!(track.sample(0.5), 1.0);
        assert!(track.sample(0.75) < 1.5);
    }

    #[test]
    fn a_keyframe_overrides_its_segment() {
        let track = Track::new(Easing::EaseIn)
            .at_ease(0.0, 0.0_f32, Easing::Linear)
            .at(1.0, 1.0)
            .build();
        assert!((track.sample(0.25) - 0.25).abs() < 1e-6);
    }
}
