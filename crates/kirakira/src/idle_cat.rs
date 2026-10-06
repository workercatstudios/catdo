//! Idle Cat: a painted cat that breathes, blinks, twitches its ears and wags its tail.
//!
//! The rig: every part owns one motion, so nested parts never fight over a transform. Body-level
//! parts scale from the feet; the ears turn about their own base, the eyes blink about their own
//! middle. `Happy` hops on the spot with an anticipation squash and a landing squash, its shadow
//! shrinking while it's up; `Sleepy` breathes slower and deeper, with closed eyes and two drifting
//! z's.
//!
//! Idle Cat is one of Kirakira's characters: it shares their [`Mood`], their 140 × 120 drawing box
//! and ground line, and their breath, blink, hop and z's, so a cast stands on one floor and moves
//! together. Its own motions are the ears and the tail.
//!
//! Composition-safe: every loop runs on a [`Clock`](crate::motion::Clock), so inside a
//! [`Timeline`](crate::timeline::Timeline) it follows the timeline's clock. Under reduced motion the
//! cat holds still.
//!
//! Differences from the web version: the same shapes and keyframes, painted as paths in a `canvas`
//! instead of an SVG, so every part can turn and squash. The z's are painted text; their transform
//! origin is estimated from the glyph's advance and a typical x-height, where SVG measures the
//! glyph. The size takes any length and defaults to 10rem (160px at the default rem size), so the
//! cat scales on a [`Stage`](crate::timeline::Stage). Inside a [`Hop`](crate::hop::Hop) it takes
//! the hop's squash and paints it exactly, from the bottom centre of its box, as the web cat
//! squashes inside the hop's squash layer.

use std::time::Duration;

use gpui_kit::{
    AbsoluteLength, App, Bounds, ElementId, Hsla, IntoElement, Pixels, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, rems, rgb,
};

pub use crate::character::Mood;
use crate::character::{self, Frame, INK, Rig, closed_eye, fill, progress};
use crate::motion::{Easing, Keyframes, Pose, Track};
use crate::shapes::{Affine, Shape};
use crate::theme::ActiveKira as _;

/// Idle Cat's mood: the [`Mood`] every Kirakira character shares.
pub type IdleCatMood = Mood;

/// The middle of the cat's feet: body-level parts squash and breathe from here, on the ground line.
const ANCHOR: f32 = 64.0;
const TAIL: &str = "M94 101C116 103 129 92 127 77C125.5 66 113 64.5 112 73";
/// Each ear: outer path, inner path and the turning point (50 % 76 % of its box).
const EARS: [(&str, &str, (f32, f32)); 2] = [
    (
        "M30 60L31 19L60 40Z",
        "M35 47L35.5 28L50 39Z",
        (45.0, 50.16),
    ),
    (
        "M98 60L97 19L68 40Z",
        "M93 47L92.5 28L78 39Z",
        (83.0, 50.16),
    ),
];
const HEAD: &str =
    "M20 86C20 52 40 34 64 34C88 34 108 52 108 86C108 104 94 112 64 112C34 112 20 104 20 86Z";
const STRIPES: &str = "M58 41V47M64 39V47.5M70 41V47";
const NOSE: &str = "M61.6 75.2h4.8l-2.4 2.6z";
const MOUTH: &str = "M64 77.6V79.6M58.5 79.6Q61.25 83.4 64 79.6Q66.75 83.4 69.5 79.6";
const WHISKERS: &str = "M32 76L15 73.5M32 80.5L16 83M96 76L113 73.5M96 80.5L112 83";
/// The z's: x, baseline y and font size in the 140 × 120 view.
const ZS: [(f32, f32, f32); 2] = [(102.0, 38.0, 13.0), (113.0, 25.0, 10.0)];

/// Where every part of the cat is at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
struct CatPose {
    /// The parts every character shares.
    rig: Rig,
    /// The tail's turn in degrees.
    tail: f32,
    /// Each ear's turn in degrees.
    ears: [f32; 2],
}

impl CatPose {
    /// The cat at rest: what reduced motion shows.
    const STILL: Self = Self {
        rig: Rig::STILL,
        tail: 0.0,
        ears: [0.0, 0.0],
    };
}

/// Idle most of the cycle, then two quick flicks outward. `side` is -1 or 1.
fn twitch(side: f32) -> Keyframes<f32> {
    let mut track = Track::new(Easing::EaseInOut);
    for (offset, value) in [
        (0.0, 0.0),
        (0.56, 0.0),
        (0.585, 16.0),
        (0.61, 0.0),
        (0.635, 9.0),
        (0.66, 0.0),
        (1.0, 0.0),
    ] {
        track = track.at(offset, value * side);
    }
    track.build()
}

fn wag() -> Keyframes<f32> {
    let mut track = Track::new(Easing::EaseInOut);
    for (offset, value) in [
        (0.0, 0.0),
        (0.12, -14.0),
        (0.26, 6.0),
        (0.4, -10.0),
        (0.52, 3.0),
        (0.62, 0.0),
        (1.0, 0.0),
    ] {
        track = track.at(offset, value);
    }
    track.build()
}

/// The cat at `elapsed` for `mood`.
fn pose(mood: Mood, elapsed: Duration) -> CatPose {
    let (ear_cycle, tail_cycle) = match mood {
        Mood::Idle => (4.4, 3.2),
        Mood::Happy => (4.4, 1.2),
        Mood::Sleepy => (7.0, 6.4),
    };
    CatPose {
        rig: Rig::at(mood, elapsed),
        ears: [
            twitch(-1.0).sample(progress(ear_cycle, 0.0, elapsed)),
            // 2.2 s apart, half the idle cycle, so the ears twitch one at a time.
            twitch(1.0).sample(progress(ear_cycle, 2.2, elapsed)),
        ],
        tail: wag().sample(progress(tail_cycle, 0.0, elapsed)),
    }
}

/// An idling painted cat.
///
/// ```ignore
/// IdleCat::new("mochi").mood(Mood::Happy).size(px(220.)).label("An orange cat, happy")
/// ```
#[derive(IntoElement)]
pub struct IdleCat {
    id: ElementId,
    color: Option<Hsla>,
    size: AbsoluteLength,
    mood: Mood,
    label: SharedString,
    style: StyleRefinement,
}

impl IdleCat {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            color: None,
            size: rems(10.0).into(),
            mood: Mood::Idle,
            label: "Cat".into(),
            style: StyleRefinement::default(),
        }
    }

    /// Fur colour. Kirakira orange by default.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Width; the height is 6/7 of it. 10rem by default.
    pub fn size(mut self, size: impl Into<AbsoluteLength>) -> Self {
        self.size = size.into();
        self
    }

    pub fn mood(mut self, mood: Mood) -> Self {
        self.mood = mood;
        self
    }

    /// Accessible name. "Cat" by default.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
}

impl Styled for IdleCat {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

struct Colors {
    fur: Hsla,
    ink: Hsla,
    pink: Hsla,
}

fn paint_cat(
    bounds: Bounds<Pixels>,
    mood: Mood,
    pose: &CatPose,
    outer: Pose,
    colors: &Colors,
    window: &mut Window,
    cx: &mut App,
) {
    let rig = &pose.rig;
    let frame = Frame::new(bounds, outer, rig, ANCHOR);
    let white: Hsla = rgb(0xffffff).into();
    let Colors { fur, ink, pink } = *colors;

    frame.paint_shadow(rig, 40.0, ink, window);

    let tail = Affine::rotate(pose.tail)
        .around(96.0, 101.0)
        .then(frame.body);
    fill(window, fur, |s| {
        s.stroke_path(TAIL, &tail, 10.0);
    });

    let breathe = frame.breathe;
    for (i, (outer, inner, origin)) in EARS.iter().enumerate() {
        let ear = Affine::rotate(pose.ears[i])
            .around(origin.0, origin.1)
            .then(breathe);
        fill(window, fur, |s| {
            s.path(outer, &ear).stroke_path(outer, &ear, 8.0);
        });
        fill(window, pink.opacity(0.5), |s| {
            s.path(inner, &ear).stroke_path(inner, &ear, 3.0);
        });
    }
    fill(window, fur, |s| {
        s.path(HEAD, &breathe);
    });
    fill(window, white.opacity(0.3), |s| {
        s.ellipse(64.0, 101.0, 20.0, 9.0, &breathe);
    });
    fill(window, ink.opacity(0.14), |s| {
        s.stroke_path(STRIPES, &breathe, 3.2);
    });

    if mood == Mood::Idle {
        let eyes = Affine::scale(1.0, rig.blink)
            .around(64.0, 72.0)
            .then(breathe);
        fill(window, ink, |s| {
            s.ellipse(48.0, 72.0, 4.6, 5.6, &eyes)
                .ellipse(80.0, 72.0, 4.6, 5.6, &eyes);
        });
        fill(window, white, |s| {
            s.ellipse(49.6, 70.0, 1.6, 1.6, &eyes)
                .ellipse(81.6, 70.0, 1.6, 1.6, &eyes);
        });
    } else {
        fill(window, ink, |s| {
            for x in [48.0, 80.0] {
                if let Some(d) = closed_eye(mood, x, 72.0) {
                    s.stroke_path(&d, &breathe, 3.0);
                }
            }
        });
    }

    fill(window, pink.opacity(0.45), |s| {
        s.ellipse(38.0, 82.0, 6.5, 3.8, &breathe)
            .ellipse(90.0, 82.0, 6.5, 3.8, &breathe);
    });
    fill(window, pink, |s| {
        s.path(NOSE, &breathe).stroke_path(NOSE, &breathe, 1.6);
    });
    fill(window, ink, |s| {
        s.stroke_path(MOUTH, &breathe, 2.0);
    });
    fill(window, ink.opacity(0.4), |s| {
        s.stroke_path(WHISKERS, &breathe, 1.6);
    });
    for x in [50.0, 78.0] {
        fill(window, rgb(0xfffaf3).into(), |s| {
            s.ellipse(x, 109.0, 8.5, 5.0, &breathe);
        });
        fill(window, ink.opacity(0.12), |s| {
            let outline = Shape::ellipse_points(x, 109.0, 8.5, 5.0, &breathe);
            s.stroke_polyline(&outline, true, 1.4 * breathe.scale_factor());
        });
    }

    if mood == Mood::Sleepy {
        frame.paint_zs(rig, ZS, window, cx);
    }
}

impl RenderOnce for IdleCat {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let pose = character::time(self.id.clone(), window, cx)
            .map_or(CatPose::STILL, |elapsed| pose(self.mood, elapsed));
        // Inside a Hop, squash with it, exactly.
        let outer = character::outer_squash();
        let palette = cx.kira();
        let colors = Colors {
            fur: self.color.unwrap_or(palette.orange),
            ink: rgb(INK).into(),
            pink: palette.pink,
        };
        let mood = self.mood;
        character::root(
            self.id,
            self.label,
            self.size,
            &self.style,
            window,
            move |bounds, window, cx| paint_cat(bounds, mood, &pose, outer, &colors, window, cx),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parity::assert_number_track;

    // Not checked against the web: `kk-idle-cat-twitch` turns by `calc(var(--kk-ear) * ...)`,
    // which the parity parser doesn't read. `ears_twitch_half_a_cycle_apart` checks its values.
    #[test]
    fn keyframes_match_the_web() {
        character::assert_shared_keyframes("idle-cat");
        assert_number_track("idle-cat", "kk-idle-cat-wag", "rotate", &wag());
    }

    fn secs(s: f32) -> Duration {
        Duration::from_secs_f32(s)
    }

    #[test]
    fn ears_twitch_half_a_cycle_apart() {
        // The left ear flicks out (-16°) at 58.5 % of 4.4 s; the right one 2.2 s earlier.
        let left = pose(Mood::Idle, secs(4.4 * 0.585));
        assert!((left.ears[0] + 16.0).abs() < 1e-3);
        let right = pose(Mood::Idle, secs(4.4 * 0.585 - 2.2));
        assert!((right.ears[1] - 16.0).abs() < 1e-3);
    }

    #[test]
    fn the_wag_speeds_up_to_the_hop() {
        let wag = pose(Mood::Happy, secs(1.2 * 0.12));
        assert!((wag.tail + 14.0).abs() < 1e-3);
    }
}
