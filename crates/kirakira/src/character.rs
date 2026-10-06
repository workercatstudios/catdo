//! What every Kirakira character shares: the mood, the drawing box, the motion of the shared parts
//! and the painting around them. [`IdleCat`](crate::idle_cat::IdleCat) and
//! [`Ferris`](crate::ferris::Ferris) are built on it; a new character copies `ferris.rs` and
//! redraws it.
//!
//! The contract, the same as the web characters' (`apps/kirakira/AGENTS.md`, "Characters"):
//!
//! - Every character is drawn in a 140 × 120 view box and stands on the ground line, y = 112. Its
//!   body-level parts squash and breathe from the anchor `(anchor_x, 112)`: the middle of its feet,
//!   70 for a symmetric character (Ferris), 64 for Idle Cat, whose tail takes the right side. The
//!   shadow is an ellipse centred under the anchor at y = 113, 4.5 tall.
//! - The shared parts nest `shadow`, `hop` > `squash` > `breathe`, and every part the character adds
//!   sits inside `breathe` (or `squash`, for one that shouldn't breathe, like Idle Cat's tail).
//! - The shared motion has the same timings in every character, so a cast moves together: breathe
//!   1.1 s (sleepy: 2.6 s and deeper), blink cycle 5 s (idle), hop 1.2 s (happy), z's 2.8 s
//!   (sleepy). [`Rig::at`] samples all of it; a character adds its one signature motion.
//! - Ink on the body is cocoa ([`INK`]) in both themes; marks drawn on the page, off the body (the
//!   z's), take the theme's ink, `cx.kira().ink`. Closed eyes are [`closed_eye`] arcs, 3 wide.
//! - Dark parts that meet the page get a rim behind them ([`paint_rim`]), white at 0.3, so they
//!   keep their silhouette on a dark page.

use std::time::Duration;

use gpui_kit::base::StyledExt as _;
use gpui_kit::{
    AbsoluteLength, App, Bounds, Div, ElementId, FontWeight, Hsla, InteractiveElement as _,
    ParentElement as _, Pixels, Role, SharedString, Stateful, StatefulInteractiveElement as _,
    StyleRefinement, Styled as _, TextAlign, TextRun, Window, canvas, div, point, px, rgb,
};

use crate::motion::{
    Clock, Easing, IterationCount, Keyframes, Pose, SignedDuration, Timing, Track, Trigger,
};
use crate::shapes::{Affine, Shape};
use crate::theme::ActiveKira as _;

/// A character's mood. Every Kirakira character takes the same three, with the same meaning.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Mood {
    /// Breathes, blinks and plays its own fidget now and then.
    #[default]
    Idle,
    /// Hops on the spot with an anticipation squash and a landing squash, its shadow shrinking
    /// while it's up, with happy ^ ^ eyes.
    Happy,
    /// Breathes slowly and deeply with closed eyes and two drifting z's.
    Sleepy,
}

/// Outline ink: cocoa in both themes, like the web characters' `--kk-<name>-ink`.
pub(crate) const INK: u32 = 0x4b3832;
/// The view box every character is drawn in.
pub(crate) const VIEW_WIDTH: f32 = 140.0;
pub(crate) const VIEW_HEIGHT: f32 = 120.0;
/// The ground line: feet stand on it.
pub(crate) const GROUND: f32 = 112.0;

/// The shared loops, in seconds.
pub(crate) const BREATHE: f32 = 1.1;
pub(crate) const SLEEPY_BREATHE: f32 = 2.6;
pub(crate) const BLINK: f32 = 5.0;
pub(crate) const HOP: f32 = 1.2;
pub(crate) const Z: f32 = 2.8;

/// Where the shared parts of a character are at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Rig {
    /// Scale and opacity of the shadow, about its middle.
    pub shadow: Pose,
    /// The hop's lift in view units (negative is up).
    pub hop: f32,
    /// The body's squash from the anchor.
    pub squash: Pose,
    /// The breath, from the anchor.
    pub breathe: Pose,
    /// The eyes' vertical scale, about each eye's middle.
    pub blink: f32,
    /// Each z's move, scale and opacity.
    pub zs: [Pose; 2],
}

impl Rig {
    /// At rest: what reduced motion shows.
    pub const STILL: Self = Self {
        shadow: Pose::IDENTITY,
        hop: 0.0,
        squash: Pose::IDENTITY,
        breathe: Pose::IDENTITY,
        blink: 1.0,
        zs: [Pose::IDENTITY.opacity(0.6), Pose::IDENTITY.opacity(0.6)],
    };

    /// The shared parts at `elapsed` for `mood`.
    pub fn at(mood: Mood, elapsed: Duration) -> Self {
        let mut rig = Self::STILL;
        match mood {
            Mood::Idle | Mood::Happy => {
                rig.breathe = breathe().sample(progress(BREATHE, 0.0, elapsed));
            }
            Mood::Sleepy => {
                rig.breathe = breathe_deep().sample(progress(SLEEPY_BREATHE, 0.0, elapsed));
            }
        }
        match mood {
            Mood::Idle => rig.blink = blink().sample(progress(BLINK, 0.0, elapsed)),
            Mood::Happy => {
                let p = progress(HOP, 0.0, elapsed);
                rig.hop = hop().sample(p);
                rig.squash = squash().sample(p);
                rig.shadow = shadow().sample(p);
            }
            Mood::Sleepy => {
                let (drift, fade) = z_drift();
                for (i, z) in rig.zs.iter_mut().enumerate() {
                    let p = progress(Z, i as f32 * Z / 2.0, elapsed);
                    *z = drift.sample(p).opacity(fade.sample(p));
                }
            }
        }
        rig
    }
}

/// An infinite loop of `seconds`, started `offset` seconds ago: a CSS `animation-delay` of
/// `-offset`.
pub(crate) fn looping(seconds: f32, offset: f32) -> Timing {
    Timing::new(Duration::from_secs_f32(seconds))
        .delay(SignedDuration::negative(Duration::from_secs_f32(offset)))
        .iterations(IterationCount::Infinite)
}

/// How far through its current lap a [`looping`] animation is at `elapsed`, `0..1`.
pub(crate) fn progress(seconds: f32, offset: f32, elapsed: Duration) -> f32 {
    looping(seconds, offset).sample(elapsed).directed_progress
}

/// `kk-<name>-breathe`: a 4 % swell, a little less wide than tall.
pub(crate) fn breathe() -> Keyframes<Pose> {
    breath(0.04)
}

/// `kk-<name>-breathe-deep`: the sleepy breath, 6 %.
pub(crate) fn breathe_deep() -> Keyframes<Pose> {
    breath(0.06)
}

fn breath(depth: f32) -> Keyframes<Pose> {
    Track::new(Easing::EaseInOut)
        .at(0.0, Pose::new())
        .at(0.5, Pose::new().scale_xy(1.0 + depth * 0.6, 1.0 + depth))
        .at(1.0, Pose::new())
        .build()
}

/// `kk-<name>-blink`: one blink, then a double blink, each about 150 ms.
pub(crate) fn blink() -> Keyframes<f32> {
    let mut track = Track::new(Easing::EaseInOut);
    for (offset, value) in [
        (0.0, 1.0),
        (0.3, 1.0),
        (0.315, 0.1),
        (0.33, 1.0),
        (0.86, 1.0),
        (0.875, 0.1),
        (0.89, 1.0),
        (0.92, 1.0),
        (0.935, 0.1),
        (0.95, 1.0),
        (1.0, 1.0),
    ] {
        track = track.at(offset, value);
    }
    track.build()
}

/// `kk-<name>-hop`: rise on an ease-out and fall on an ease-in, like a thrown ball.
pub(crate) fn hop() -> Keyframes<f32> {
    let rise = Easing::cubic_bezier(0.33, 1.0, 0.68, 1.0).expect("valid curve");
    let fall = Easing::cubic_bezier(0.32, 0.0, 0.67, 0.0).expect("valid curve");
    Track::new(Easing::Ease)
        .at_ease(0.0, 0.0, rise.clone())
        .at_ease(0.22, 0.0, rise)
        .at_ease(0.44, -20.0, fall)
        .at(0.66, 0.0)
        .at(1.0, 0.0)
        .build()
}

/// `kk-<name>-squash`: anticipation, launch stretch, fall stretch, landing squash, settle.
pub(crate) fn squash() -> Keyframes<Pose> {
    let mut track = Track::new(Easing::EaseInOut);
    for (offset, sx, sy) in [
        (0.0, 1.0, 1.0),
        (0.14, 1.16, 0.84),
        (0.26, 0.88, 1.14),
        (0.44, 0.98, 1.02),
        (0.62, 0.92, 1.08),
        (0.7, 1.18, 0.8),
        (0.8, 0.97, 1.03),
        (0.9, 1.0, 1.0),
        (1.0, 1.0, 1.0),
    ] {
        track = track.at(offset, Pose::new().scale_xy(sx, sy));
    }
    track.build()
}

/// `kk-<name>-shadow`: shrinks and fades while the character is up, spreads as it lands.
pub(crate) fn shadow() -> Keyframes<Pose> {
    let mut track = Track::new(Easing::EaseInOut);
    for (offset, pose) in [
        (0.0, Pose::new()),
        (0.22, Pose::new()),
        (0.44, Pose::new().scale(0.7).opacity(0.5)),
        (0.66, Pose::new()),
        (0.7, Pose::new().scale_xy(1.12, 1.0)),
        (1.0, Pose::new()),
    ] {
        track = track.at(offset, pose);
    }
    track.build()
}

/// `kk-<name>-z`: the move and scale, and the fade.
pub(crate) fn z_drift() -> (Keyframes<Pose>, Keyframes<f32>) {
    (
        Track::new(Easing::EaseOut)
            .at(0.0, Pose::new().at(-2.0, 6.0).scale(0.6))
            .at(1.0, Pose::new().at(6.0, -10.0))
            .build(),
        Track::new(Easing::EaseOut)
            .at(0.0, 0.0)
            .at(0.3, 0.6)
            .at(1.0, 0.0)
            .build(),
    )
}

/// A closed eye centred on `(x, y)` as SVG path data: an arch like ^ when happy, a dipped lid when
/// sleepy, 10 wide. Stroke it 3 wide. `None` when the eyes are open.
pub(crate) fn closed_eye(mood: Mood, x: f32, y: f32) -> Option<String> {
    let (bend, y) = match mood {
        Mood::Idle => return None,
        Mood::Happy => (-7.5, y + 2.0),
        Mood::Sleepy => (5.5, y - 1.0),
    };
    Some(format!(
        "M{} {y} Q{x} {} {} {y}",
        x - 5.0,
        y + bend,
        x + 5.0
    ))
}

/// The transforms a character paints with, from the view box to the window.
pub(crate) struct Frame {
    /// View units to window pixels, with a [`Hop`](crate::hop::Hop)'s squash.
    pub view: Affine,
    /// The hop and the squash: for parts that don't breathe.
    pub body: Affine,
    /// The breath inside the body: for everything else.
    pub breathe: Affine,
    /// Window pixels per view unit.
    pub scale: f32,
    /// The squash from outside, about the bottom centre of the box.
    pub outer: Pose,
    anchor: f32,
}

impl Frame {
    /// The frame for a character anchored at `(anchor_x, GROUND)`, painted into `bounds`.
    pub fn new(bounds: Bounds<Pixels>, outer: Pose, rig: &Rig, anchor_x: f32) -> Self {
        let scale = f32::from(bounds.size.width) / VIEW_WIDTH;
        let view = Affine::scale(outer.sx, outer.sy)
            .around(VIEW_WIDTH / 2.0, VIEW_HEIGHT)
            .then(Affine::scale(scale, scale))
            .then(Affine::translate(
                f32::from(bounds.origin.x),
                f32::from(bounds.origin.y),
            ));
        let body = Affine::scale(rig.squash.sx, rig.squash.sy)
            .around(anchor_x, GROUND)
            .then(Affine::translate(0.0, rig.hop))
            .then(view);
        let breathe = Affine::scale(rig.breathe.sx, rig.breathe.sy)
            .around(anchor_x, GROUND)
            .then(body);
        Self {
            view,
            body,
            breathe,
            scale,
            outer,
            anchor: anchor_x,
        }
    }

    /// The shadow under the anchor, `rx` wide each way, in ink at 12 %.
    pub fn paint_shadow(&self, rig: &Rig, rx: f32, ink: Hsla, window: &mut Window) {
        let (x, y) = (self.anchor, GROUND + 1.0);
        let shadow = Affine::scale(rig.shadow.sx, rig.shadow.sy)
            .around(x, y)
            .then(self.view);
        fill(window, ink.opacity(0.12 * rig.shadow.alpha()), |s| {
            s.ellipse(x, y, rx, 4.5, &shadow);
        });
    }

    /// The sleepy z's, each at `(x, baseline y, font size)` in the view, extra bold. They're
    /// drawn on the page, not the body, so they take the theme's ink (`cx.kira().ink`, the web's
    /// `--kk-ink`), which turns light in dark mode.
    pub fn paint_zs(&self, rig: &Rig, zs: [(f32, f32, f32); 2], window: &mut Window, cx: &mut App) {
        let ink = cx.kira().ink;
        let mut font = window.text_style().font();
        font.weight = FontWeight::EXTRA_BOLD;
        let outer = self.outer.uniform_scale();
        for (&(x, y, size), z) in zs.iter().zip(&rig.zs) {
            let font_size = px(size * z.sx * self.scale * outer);
            let run = TextRun {
                len: 1,
                font: font.clone(),
                color: ink.opacity(z.alpha()),
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let line = window
                .text_system()
                .shape_line("z".into(), font_size, &[run], None);
            // Scale about the glyph's middle, estimated from its advance and x-height.
            let advance = f32::from(line.width) / self.scale / outer.max(1e-3) / z.sx.max(1e-3);
            let (mx, my) = (x + advance / 2.0, y - size * 0.27);
            let baseline = Affine::scale(z.sx, z.sy)
                .around(mx, my)
                .then(Affine::translate(z.x, z.y))
                .then(self.view)
                .apply(x, y);
            let line_height = font_size;
            let above = (line_height - line.ascent - line.descent) / 2.0 + line.ascent;
            let origin = point(px(baseline.0), px(baseline.1) - above);
            line.paint(origin, line_height, TextAlign::Left, None, window, cx)
                .ok();
        }
    }
}

/// Paints what `build` adds to a fresh [`Shape`] in one colour. Fills and strokes added together
/// join, so a translucent part blends once, like an SVG group with `opacity`.
pub(crate) fn fill(window: &mut Window, color: Hsla, build: impl FnOnce(&mut Shape)) {
    let mut shape = Shape::new();
    build(&mut shape);
    shape.paint(window, color);
}

/// How much wider than a dark part its rim is stroked: 1.5 view units show past each edge.
pub(crate) const RIM: f32 = 3.0;
/// The rim's opacity, white.
pub(crate) const RIM_OPACITY: f32 = 0.3;

/// Paints the rim behind a character's dark parts (a near-black body, black hair, tyres), so they
/// keep their silhouette on a dark page. Call it before painting the parts, with the dark parts'
/// silhouettes in their own transforms: [`rim_fill`] for a filled path, [`rim_ellipse`] for an
/// ellipse, `stroke_path(d, t, width + RIM)` for a stroked one. It's white at [`RIM_OPACITY`],
/// so it vanishes into a light page (the light theme looks unchanged) and catches the edge like a
/// rim light on a dark one; the parts painted over it hide all but the outer edge. The web draws
/// the same strokes in one `<g opacity="0.3">`; here they join into one path, so overlaps don't
/// add up either.
pub(crate) fn paint_rim(window: &mut Window, build: impl FnOnce(&mut Shape)) {
    fill(
        window,
        Hsla::from(rgb(0xffffff)).opacity(RIM_OPACITY),
        build,
    );
}

/// A filled path's rim: the path filled and stroked [`RIM`] wide.
pub(crate) fn rim_fill(shape: &mut Shape, d: &str, transform: &Affine) {
    shape.path(d, transform).stroke_path(d, transform, RIM);
}

/// A filled ellipse's rim: the ellipse filled and its outline stroked [`RIM`] wide.
pub(crate) fn rim_ellipse(shape: &mut Shape, cx: f32, cy: f32, rx: f32, ry: f32, t: &Affine) {
    let outline = Shape::ellipse_points(cx, cy, rx, ry, t);
    shape
        .ellipse(cx, cy, rx, ry, t)
        .stroke_polyline(&outline, true, RIM * t.scale_factor());
}

/// The character's time on its [`Clock`], or `None` under reduced motion, when it holds still.
/// Keeps frames coming while it runs.
pub(crate) fn time(id: ElementId, window: &mut Window, cx: &mut App) -> Option<Duration> {
    let clock = Clock::new(id, Trigger::Mount, window, cx);
    if clock.reduced() {
        return None;
    }
    clock.animate(None, window);
    Some(clock.elapsed())
}

/// The squash of a [`Hop`](crate::hop::Hop) the character is inside, taken so the hop doesn't
/// scale it as well. Apply it about the bottom centre of the box ([`Frame::new`] does).
pub(crate) fn outer_squash() -> Pose {
    crate::hop::current_squash().map_or(Pose::IDENTITY, |squash| squash.take())
}

/// The character's element: an image `size` wide (and 6/7 of that tall) named `label`, whose
/// canvas runs `paint`.
pub(crate) fn root(
    id: ElementId,
    label: SharedString,
    size: AbsoluteLength,
    style: &StyleRefinement,
    window: &Window,
    paint: impl FnOnce(Bounds<Pixels>, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let width = size.to_pixels(window.rem_size());
    div()
        .id(id)
        .role(Role::Image)
        .aria_label(label)
        .flex_none()
        .w(width)
        .h(width * (VIEW_HEIGHT / VIEW_WIDTH))
        .refine_style(style)
        .child(
            canvas(
                |_, _, _| {},
                move |bounds, _, window, cx| paint(bounds, window, cx),
            )
            .size_full(),
        )
}

/// Checks the shared keyframes of a web character, `kk-<component>-breathe`, `-breathe-deep`,
/// `-blink`, `-hop`, `-squash`, `-shadow` and `-z`, against the tracks above.
#[cfg(test)]
pub(crate) fn assert_shared_keyframes(component: &str) {
    use crate::parity::{assert_number_track, assert_pose_opacity, assert_pose_track};
    let name = |part: &str| format!("kk-{component}-{part}");
    assert_pose_track(component, &name("breathe"), &breathe(), &[]);
    assert_pose_track(component, &name("breathe-deep"), &breathe_deep(), &[]);
    assert_number_track(component, &name("blink"), "sy", &blink());
    assert_number_track(component, &name("hop"), "y", &hop());
    assert_pose_track(component, &name("squash"), &squash(), &[]);
    assert_pose_track(component, &name("shadow"), &shadow(), &[]);
    assert_pose_opacity(component, &name("shadow"), &shadow());
    let (drift, fade) = z_drift();
    assert_pose_track(component, &name("z"), &drift, &[]);
    assert_number_track(component, &name("z"), "opacity", &fade);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secs(s: f32) -> Duration {
        Duration::from_secs_f32(s)
    }

    #[test]
    fn idle_blinks_at_31_5_percent() {
        let rig = Rig::at(Mood::Idle, secs(BLINK * 0.315));
        assert!((rig.blink - 0.1).abs() < 1e-4);
        assert_eq!(Rig::at(Mood::Idle, secs(1.0)).blink, 1.0);
        // Only idle eyes are open to blink.
        assert_eq!(Rig::at(Mood::Happy, secs(BLINK * 0.315)).blink, 1.0);
    }

    #[test]
    fn happy_hops_and_squashes() {
        let up = Rig::at(Mood::Happy, secs(HOP * 0.44));
        assert!((up.hop + 20.0).abs() < 1e-3);
        assert!((up.shadow.sx - 0.7).abs() < 1e-4 && (up.shadow.opacity - 0.5).abs() < 1e-4);
        let landed = Rig::at(Mood::Happy, secs(HOP * 0.7));
        assert!((landed.squash.sx - 1.18).abs() < 1e-4 && (landed.squash.sy - 0.8).abs() < 1e-4);
        assert_eq!(Rig::at(Mood::Idle, secs(HOP * 0.44)).hop, 0.0);
    }

    #[test]
    fn sleepy_breathes_deeper_and_the_zs_drift() {
        let breath = Rig::at(Mood::Sleepy, secs(SLEEPY_BREATHE / 2.0));
        assert!((breath.breathe.sy - 1.06).abs() < 1e-4);
        let awake = Rig::at(Mood::Idle, secs(BREATHE / 2.0));
        assert!((awake.breathe.sx - 1.024).abs() < 1e-4 && (awake.breathe.sy - 1.04).abs() < 1e-4);
        let z = Rig::at(Mood::Sleepy, secs(Z * 0.3));
        assert!((z.zs[0].opacity - 0.6).abs() < 1e-4);
        // The second z is half a cycle on.
        let z = Rig::at(Mood::Sleepy, secs(0.0));
        assert!(z.zs[1].opacity > 0.0 && z.zs[0].opacity == 0.0);
    }

    #[test]
    fn closed_eyes_arch_when_happy_and_dip_when_sleepy() {
        assert_eq!(closed_eye(Mood::Idle, 48.0, 72.0), None);
        assert_eq!(
            closed_eye(Mood::Happy, 48.0, 72.0).as_deref(),
            Some("M43 74 Q48 66.5 53 74")
        );
        assert_eq!(
            closed_eye(Mood::Sleepy, 48.0, 72.0).as_deref(),
            Some("M43 71 Q48 76.5 53 71")
        );
    }
}
