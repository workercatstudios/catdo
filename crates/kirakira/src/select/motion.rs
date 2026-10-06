//! Pop Select's keyframes and timing: the list's squash-pop, its rows' drop, the check's pop and
//! the chevron's flip.

use std::time::Duration;

use crate::motion::{Easing, Keyframes, Pose, Timing, Track, delay_ms, ms};
use crate::overlay::Phase;

/// `kk-pop-select-in`.
pub(super) const OPEN: Duration = ms(300);
/// `kk-pop-select-out`.
pub(super) const CLOSE: Duration = ms(150);
/// `kk-pop-select-fade` and `kk-pop-select-fade-out`, under reduced motion.
const FADE_IN: Duration = ms(150);
const FADE_OUT: Duration = ms(100);
/// `kk-pop-select-flip`, and the transition that turns the chevron back.
pub(super) const FLIP: Duration = ms(360);
pub(super) const BACK: Duration = ms(150);

/// `kk-pop-select-drop`: each row and group label, 40 ms in, then 25 ms per step.
const DROP: Duration = ms(260);
const DROP_DELAY: u64 = 40;
/// `kk-pop-select-check`: the selected row's check, 200 ms in, then 25 ms per step.
const CHECK: Duration = ms(300);
const CHECK_DELAY: u64 = 200;
const STAGGER: u64 = 25;
/// Steps past the tenth share its slot: `min(i + j, 9)` on the web.
pub(super) const LAST_STEP: usize = 9;

// ── The list ────────────────────────────────────────────────────────────────────────────────

/// `opacity` of `kk-pop-select-in`: 0 → 1 by 30 %, ease-in-out.
pub(super) fn in_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.0)
        .at(0.3, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// `scale` of `kk-pop-select-in`: (0.95, 0.6) → (1.02, 1.05) at 55 % → (0.995, 0.98) at 80 % → 1,
/// ease-in-out.
pub(super) fn in_scale() -> Keyframes<Pose> {
    Track::new(Easing::EaseInOut)
        .at(0.0, Pose::new().scale_xy(0.95, 0.6))
        .at(0.55, Pose::new().scale_xy(1.02, 1.05))
        .at(0.8, Pose::new().scale_xy(0.995, 0.98))
        .at(1.0, Pose::new())
        .build()
}

/// `opacity` of `kk-pop-select-out`, on `curve` (Kirakira's `in`): to 0.
pub(super) fn out_opacity(curve: Easing) -> Keyframes<f32> {
    Track::new(curve).at(0.0, 1.0).at(1.0, 0.0).build()
}

/// `scale` of `kk-pop-select-out`, on `curve`: to (0.96, 0.9).
pub(super) fn out_scale(curve: Easing) -> Keyframes<Pose> {
    Track::new(curve)
        .at(0.0, Pose::new())
        .at(1.0, Pose::new().scale_xy(0.96, 0.9))
        .build()
}

/// `kk-pop-select-fade`, the reduced-motion entrance (ease-out).
pub(super) fn fade_in() -> Keyframes<f32> {
    Track::new(Easing::EaseOut)
        .at(0.0, 0.0)
        .at(1.0, 1.0)
        .build()
}

/// `kk-pop-select-fade-out`, the reduced-motion exit (ease-in).
pub(super) fn fade_out() -> Keyframes<f32> {
    Track::new(Easing::EaseIn).at(0.0, 1.0).at(1.0, 0.0).build()
}

/// The list's entrance and exit durations.
pub(super) fn durations(reduced: bool) -> (Duration, Duration) {
    if reduced {
        (FADE_IN, FADE_OUT)
    } else {
        (OPEN, CLOSE)
    }
}

/// How the list's panel is drawn this frame: its scale on each axis, about its transform origin,
/// and its opacity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PanelPose {
    pub sx: f32,
    pub sy: f32,
    pub opacity: f32,
}

impl PanelPose {
    pub const REST: Self = Self {
        sx: 1.0,
        sy: 1.0,
        opacity: 1.0,
    };

    /// Unscaled (it may still be fading).
    pub fn unscaled(&self) -> bool {
        self.sx == 1.0 && self.sy == 1.0
    }

    pub fn alpha(&self) -> f32 {
        self.opacity.clamp(0.0, 1.0)
    }
}

fn progress(duration: Duration, elapsed: Duration) -> f32 {
    Timing::new(duration).sample(elapsed).directed_progress
}

/// The panel's pose in `phase`. `exit` is the exit's curve, Kirakira's `in`.
pub(super) fn panel_pose(phase: Phase, reduced: bool, exit: Easing) -> PanelPose {
    let fade = |opacity: f32| PanelPose {
        opacity,
        ..PanelPose::REST
    };
    let scaled = |scale: Pose, opacity: f32| PanelPose {
        sx: scale.sx,
        sy: scale.sy,
        opacity,
    };
    match phase {
        Phase::Open => PanelPose::REST,
        Phase::Closed => fade(0.0),
        Phase::Opening(elapsed) if reduced => fade(fade_in().sample(progress(FADE_IN, elapsed))),
        Phase::Closing(elapsed) if reduced => fade(fade_out().sample(progress(FADE_OUT, elapsed))),
        Phase::Opening(elapsed) => {
            let t = progress(OPEN, elapsed);
            scaled(in_scale().sample(t), in_opacity().sample(t))
        }
        Phase::Closing(elapsed) => {
            let t = progress(CLOSE, elapsed);
            scaled(
                out_scale(exit.clone()).sample(t),
                out_opacity(exit).sample(t),
            )
        }
    }
}

// ── Rows and the check ──────────────────────────────────────────────────────────────────────

/// A row's (or group label's) stagger step: its group's place in the list plus its own place in
/// the group, as `--kk-pop-select-i` and `-j` add up on the web. In a list without groups each row
/// is its own place.
pub(super) fn step(group: usize, place: usize) -> usize {
    (group + place).min(LAST_STEP)
}

/// `kk-pop-select-drop`'s `translate` in rems: -0.375 → 0.0625 (overshoot, 60 %) → 0, ease-out.
pub(super) fn drop_y() -> Keyframes<f32> {
    Track::new(Easing::EaseOut)
        .at(0.0, -0.375)
        .at(0.6, 0.0625)
        .at(1.0, 0.0)
        .build()
}

/// `kk-pop-select-drop`'s `opacity`: 0 → 1 by 60 %, ease-out.
pub(super) fn drop_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseOut)
        .at(0.0, 0.0)
        .at(0.6, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// `kk-pop-select-check`'s `scale`: 0 → 1.3 → 0.9 → 1, ease-in-out.
pub(super) fn check_track() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.0)
        .at(0.5, 1.3)
        .at(0.75, 0.9)
        .at(1.0, 1.0)
        .build()
}

fn drop_time(step: usize, elapsed: Duration) -> f32 {
    Timing::new(DROP)
        .delay(delay_ms(DROP_DELAY + step as u64 * STAGGER))
        .sample(elapsed)
        .directed_progress
}

fn check_time(step: usize, elapsed: Duration) -> f32 {
    Timing::new(CHECK)
        .delay(delay_ms(CHECK_DELAY + step as u64 * STAGGER))
        .sample(elapsed)
        .directed_progress
}

/// When everything inside the list has landed: the last step's check.
pub(super) fn rows_end() -> Duration {
    ms(CHECK_DELAY + LAST_STEP as u64 * STAGGER) + CHECK
}

/// A row at `step`, `elapsed` after the list opened: its offset in rems and its opacity, or `None`
/// once it has landed. It waits hidden above its place (the `both` fill).
pub(super) fn row_pose(step: usize, elapsed: Duration) -> Option<(f32, f32)> {
    let t = drop_time(step, elapsed);
    (t < 1.0).then(|| (drop_y().sample(t), drop_opacity().sample(t)))
}

/// The check's scale on the row at `step`, or `None` once it rests. It waits at 0.
pub(super) fn check_scale(step: usize, elapsed: Duration) -> Option<f32> {
    let t = check_time(step, elapsed);
    (t < 1.0).then(|| check_track().sample(t))
}

// ── The chevron ─────────────────────────────────────────────────────────────────────────────

/// `kk-pop-select-flip`, in degrees.
pub(super) fn flip_track() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.0)
        .at(0.55, 198.0)
        .at(0.78, 174.0)
        .at(1.0, 180.0)
        .build()
}

/// The chevron's rotation `elapsed` after the list opened.
pub(super) fn flip_angle(elapsed: Duration) -> f32 {
    flip_track().sample(progress(FLIP, elapsed))
}

/// The chevron's rotation `elapsed` into turning back from `from` degrees: `transition: rotate
/// 0.15s ease-out`.
pub(super) fn back_angle(from: f32, elapsed: Duration) -> f32 {
    from * (1.0 - Easing::EaseOut.sample(progress(BACK, elapsed)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parity::{assert_number_track, assert_pose_track};

    const WEB: &str = "pop-select";

    #[test]
    fn the_list_matches_the_web_keyframes() {
        let exit = crate::theme::Curves::default().r#in;
        assert_number_track(WEB, "kk-pop-select-in", "opacity", &in_opacity());
        assert_pose_track(WEB, "kk-pop-select-in", &in_scale(), &[]);
        assert_number_track(
            WEB,
            "kk-pop-select-out",
            "opacity",
            &out_opacity(exit.clone()),
        );
        assert_pose_track(WEB, "kk-pop-select-out", &out_scale(exit), &[]);
        assert_number_track(WEB, "kk-pop-select-fade", "opacity", &fade_in());
        assert_number_track(WEB, "kk-pop-select-fade-out", "opacity", &fade_out());
    }

    #[test]
    fn rows_and_the_check_match_the_web_keyframes() {
        assert_number_track(WEB, "kk-pop-select-drop", "opacity", &drop_opacity());
        // `translate: 0 -0.375rem`: the parser reads the number; the track is in rems too.
        assert_number_track(WEB, "kk-pop-select-drop", "y", &drop_y());
        assert_number_track(WEB, "kk-pop-select-check", "sx", &check_track());
        assert_number_track(WEB, "kk-pop-select-check", "sy", &check_track());
    }

    #[test]
    fn the_chevron_matches_the_web_keyframes() {
        assert_number_track(WEB, "kk-pop-select-flip", "rotate", &flip_track());
    }

    #[test]
    fn the_list_squashes_then_overshoots() {
        let pose = |ms_: u64| panel_pose(Phase::Opening(ms(ms_)), false, Easing::EaseIn);
        let start = pose(0);
        assert_eq!((start.sx, start.sy, start.opacity), (0.95, 0.6, 0.0));
        assert_eq!(pose(90).opacity, 1.0);
        let peak = pose(165);
        assert!((peak.sx - 1.02).abs() < 1e-4 && (peak.sy - 1.05).abs() < 1e-4);
        assert_eq!(
            panel_pose(Phase::Open, false, Easing::EaseIn),
            PanelPose::REST
        );
        let gone = panel_pose(Phase::Closing(CLOSE), false, Easing::EaseIn);
        assert!((gone.sx - 0.96).abs() < 1e-4 && (gone.sy - 0.9).abs() < 1e-4);
        assert!(gone.opacity.abs() < 1e-4);
    }

    #[test]
    fn reduced_motion_only_fades() {
        assert_eq!(durations(true), (ms(150), ms(100)));
        let pose = panel_pose(Phase::Opening(ms(75)), true, Easing::EaseIn);
        assert!(pose.unscaled());
        assert!(pose.opacity > 0.0 && pose.opacity < 1.0);
        assert!(panel_pose(Phase::Closing(ms(50)), true, Easing::EaseIn).unscaled());
    }

    #[test]
    fn steps_add_the_group_and_the_place_up_to_nine() {
        // A flat list: each row is its own place.
        assert_eq!(step(0, 3), 3);
        // The second group's label, then its first row after it.
        assert_eq!(step(1, 0), 1);
        assert_eq!(step(1, 1), 2);
        assert_eq!(step(4, 12), 9);
    }

    #[test]
    fn rows_wait_hidden_then_land_25ms_apart() {
        // Step 2 starts at 90 ms: hidden and 0.375rem up until then.
        assert_eq!(row_pose(2, ms(80)), Some((-0.375, 0.0)));
        let (y, opacity) = row_pose(2, ms(90 + 156)).expect("still landing");
        assert!((y - 0.0625).abs() < 1e-4 && (opacity - 1.0).abs() < 1e-4);
        assert_eq!(row_pose(2, ms(90 + 260)), None);
        assert_eq!(row_pose(0, ms(300)), None);
    }

    #[test]
    fn the_check_pops_after_its_row() {
        assert_eq!(check_scale(1, ms(200)), Some(0.0));
        let peak = check_scale(1, ms(225 + 150)).expect("popping");
        assert!((peak - 1.3).abs() < 1e-4);
        assert_eq!(check_scale(1, ms(225 + 300)), None);
        assert_eq!(rows_end(), ms(200 + 225 + 300));
    }

    #[test]
    fn the_chevron_overshoots_half_a_turn_and_turns_back() {
        assert_eq!(flip_angle(ms(0)), 0.0);
        assert!((flip_angle(ms(198)) - 198.0).abs() < 1e-3);
        assert_eq!(flip_angle(FLIP), 180.0);
        assert_eq!(back_angle(180.0, ms(0)), 180.0);
        assert!(back_angle(180.0, ms(75)) < 90.0);
        assert_eq!(back_angle(180.0, BACK), 0.0);
    }
}
