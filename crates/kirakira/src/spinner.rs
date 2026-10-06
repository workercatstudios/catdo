//! Spinner: GPUI Component's spinner, plus Kirakira's pop loaders.
//!
//! Replaces `gpui_kit::component::spinner`. `Spinner::new()` is the same turning loader icon with
//! the same builder (`icon`, `color`, `ease`, `Sizable`); [`Spinner::variant`] picks one of the
//! Pop Loader's playful loops instead:
//!
//! - [`SpinnerVariant::Dots`]: three dots hop in turn, squashing as they land and stretching as
//!   they leave.
//! - [`SpinnerVariant::Wave`]: the letters of [`Spinner::label`] crouch and jump one after another.
//! - [`SpinnerVariant::Roll`]: a rounded tile tips over its corner along a track, pops in at the
//!   start and out at the end.
//! - [`SpinnerVariant::Ripple`]: rings open out of a beating core.
//! - [`SpinnerVariant::Bars`]: five bars spike at unrelated tempos, so they never line up.
//!
//! The loaders are sized in ems of the spinner size: GPUI Component's icon sizes, in rems (1 rem
//! for `Medium`), so they follow the rem size as icons do; `with_size(px(48.))` is the web demo's
//! `text-5xl`. They are painted in the theme's primary unless [`Spinner::color`] says otherwise,
//! or [`Spinner::inherit_color`] paints them in the text colour around them, as the web's
//! `text-current`. The dots, tile, rings and bars are painted shapes, and a moving letter of the
//! wave is a [vector glyph](crate::vector), so everything squashes, tips and scales exactly as on
//! the web.
//!
//! Any spinner goes in a Kirakira [`Button`](crate::button::Button)'s icon slot, and a loading
//! Kirakira button turns this spinner rather than GPUI Component's: see [`crate::button`].
//!
//! The loops run on Kirakira's time: inside a [`Timeline`](crate::timeline::Timeline) they follow
//! its clock, elsewhere every spinner shares one phase. Under reduced motion they stand still in
//! their resting pose, and nothing asks for frames.
//!
//! Every spinner is a status, as the web's `role="status"`, named by [`Spinner::label`]; the
//! wave's letters carry no name of their own. Its element id defaults to the place in the source
//! that built it; set [`Spinner::id`] when one place builds several under one parent.

use std::panic::Location;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable, Size};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, ElementId, FontWeight, Hsla, InteractiveElement as _, IntoElement, ParentElement as _,
    Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement as _, Styled as _,
    TextStyleRefinement, Transformation, Window, canvas, div, ease_in_out, px, radians, relative,
};
use unicode_segmentation::UnicodeSegmentation as _;

use crate::motion::{
    Easing, IterationCount, Keyframes, MotionPhase, Pose, SignedDuration, Timing, Track, ms,
    scope_time,
};
use crate::painted::{self, Affine};
use crate::theme::ActiveKira as _;
use crate::vector::{TextSetting, Vector};

/// Which loader a [`Spinner`] draws.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SpinnerVariant {
    /// GPUI Component's turning icon.
    #[default]
    Icon,
    /// Three dots hopping in turn.
    Dots,
    /// The label's letters jumping in a wave.
    Wave,
    /// A tile tipping over along a track.
    Roll,
    /// Rings opening out of a beating core.
    Ripple,
    /// Five bars spiking out of step.
    Bars,
}

/// A cycling loading spinner.
#[derive(IntoElement)]
pub struct Spinner {
    size: Size,
    icon: Icon,
    speed: Duration,
    easing: Box<dyn Fn(f32) -> f32>,
    color: Option<Hsla>,
    inherit_color: bool,
    pub(crate) variant: SpinnerVariant,
    label: SharedString,
    id: ElementId,
}

impl Default for Spinner {
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}

impl Spinner {
    /// Create a new loading spinner.
    #[track_caller]
    pub fn new() -> Self {
        Self {
            size: Size::Medium,
            speed: Duration::from_secs_f64(0.8),
            easing: Box::new(ease_in_out),
            icon: Icon::new(IconName::Loader),
            color: None,
            inherit_color: false,
            variant: SpinnerVariant::Icon,
            label: "Loading".into(),
            id: (ElementId::from(Location::caller()), "kk-spinner").into(),
        }
    }

    /// The element id of the spinner's root, which carries its accessible role and name. By
    /// default, the source location that built it.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Set specified icon for the spinner. Default is [`IconName::Loader`].
    ///
    /// Only the [`SpinnerVariant::Icon`] spinner draws it.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = icon.into();
        self
    }

    /// Set the colour. The icon inherits the text colour by default; the loaders use the theme's
    /// primary, like the web's `text-primary`.
    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    /// Paint the loaders in the text colour around them instead of the theme's primary, like the
    /// web's `text-current`: a loader in a button takes the button's text colour. A colour set with
    /// [`Spinner::color`] still wins. The icon spinner inherits the text colour anyway.
    pub fn inherit_color(mut self) -> Self {
        self.inherit_color = true;
        self
    }

    /// Set the easing function of the icon's turn.
    pub fn ease(mut self, easing: impl Fn(f32) -> f32 + 'static) -> Self {
        self.easing = Box::new(easing);
        self
    }

    /// Draw one of Kirakira's loaders instead of the turning icon.
    pub fn variant(mut self, variant: SpinnerVariant) -> Self {
        self.variant = variant;
        self
    }

    /// What screen readers announce, and the text the [`SpinnerVariant::Wave`] loader animates.
    /// "Loading" by default.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = label.into();
        self
    }
}

impl Sizable for Spinner {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

/// The em the loaders are drawn in: the icon size GPUI Component gives each [`Size`], in rems, so
/// a loader follows the rem size as an icon does (a popping button scales its rem size).
fn em(size: Size, rem: Pixels) -> f32 {
    let rems = match size {
        Size::Size(size) => return f32::from(size),
        Size::XSmall => 0.75,
        Size::Small => 0.875,
        Size::Medium => 1.0,
        Size::Large => 1.5,
    };
    rems * f32::from(rem)
}

/// What a loader is painted in: a colour, or the text colour where it is painted.
#[derive(Clone, Copy)]
enum Ink {
    Fixed(Hsla),
    Inherit,
}

impl Ink {
    fn color(self, window: &Window) -> Hsla {
        match self {
            Self::Fixed(color) => color,
            Self::Inherit => window.text_style().color,
        }
    }
}

/// When the shared spinner phase began.
fn epoch() -> Instant {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    *EPOCH.get_or_init(crate::motion::now)
}

/// How long the loops have run, asking for the next frame, or `None` under reduced motion.
///
/// `Spinner::new()` takes no id, so there is no keyed start time: a timeline's clock drives it
/// when there is one, and every other spinner shares one phase.
fn loop_time(window: &mut Window, cx: &App) -> Option<Duration> {
    if cx.reduce_motion() {
        return None;
    }
    match scope_time() {
        Some(scope) => {
            if scope.free {
                window.request_animation_frame();
            }
            Some(scope.time)
        }
        None => {
            window.request_animation_frame();
            Some(crate::motion::now().saturating_duration_since(epoch()))
        }
    }
}

/// How far through its current turn of `period` a loop that has run `time` is, `0..1`. Counted in
/// whole nanoseconds, so it stays exact however long the process has been up.
fn cycle_progress(time: Duration, period: Duration) -> f32 {
    // At least a millisecond, as a turn can't be quicker than that.
    let period = period.as_nanos().max(1_000_000);
    ((time.as_nanos() % period) as f64 / period as f64) as f32
}

fn bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> Easing {
    Easing::cubic_bezier(x1, y1, x2, y2).expect("valid curve")
}

/// A squash and stretch keyed at its own offsets: one track for each axis, from
/// `(offset, sx, sy, easing of the segment that starts here)`.
fn squash(frames: &[(f32, f32, f32, Option<Easing>)]) -> (Keyframes<f32>, Keyframes<f32>) {
    let axis = |pick: fn(&(f32, f32, f32, Option<Easing>)) -> f32| {
        frames
            .iter()
            .fold(Track::new(Easing::EaseInOut), |track, frame| {
                match &frame.3 {
                    Some(ease) => track.at_ease(frame.0, pick(frame), ease.clone()),
                    None => track.at(frame.0, pick(frame)),
                }
            })
            .build()
    };
    (axis(|frame| frame.1), axis(|frame| frame.2))
}

/// The tracks of a hop: rise in ems, and the horizontal and vertical scale.
struct Hop {
    rise: Keyframes<f32>,
    sx: Keyframes<f32>,
    sy: Keyframes<f32>,
}

/// `kk-pop-loader-hop`: rise and fall in ems, and the squash and stretch, keyed apart.
fn hop() -> Hop {
    let leave = bezier(0.2, 0.6, 0.4, 1.0);
    let fall = bezier(0.6, 0.0, 0.8, 0.4);
    let (sx, sy) = squash(&[
        (0.0, 1.0, 1.0, None),
        (0.12, 1.3, 0.7, Some(leave.clone())),
        (0.22, 0.8, 1.3, None),
        (0.32, 1.0, 1.0, Some(fall.clone())),
        (0.43, 0.85, 1.2, None),
        (0.46, 1.3, 0.7, None),
        (0.54, 0.92, 1.08, None),
        (0.62, 1.0, 1.0, None),
        (1.0, 1.0, 1.0, None),
    ]);
    Hop {
        rise: Track::new(Easing::EaseInOut)
            .at(0.0, 0.0)
            .at_ease(0.12, 0.0, leave)
            .at_ease(0.32, -0.75, fall)
            .at(0.46, 0.0)
            .at(0.62, 0.0)
            .at(1.0, 0.0)
            .build(),
        sx,
        sy,
    }
}

/// `kk-pop-loader-wave`: a letter's rise in ems and its squash and stretch.
fn wave() -> Hop {
    let (sx, sy) = squash(&[
        (0.0, 1.0, 1.0, None),
        (0.14, 1.3, 0.72, None),
        (0.3, 0.82, 1.3, None),
        (0.46, 1.18, 0.8, None),
        (0.56, 0.96, 1.04, None),
        (0.64, 1.0, 1.0, None),
        (1.0, 1.0, 1.0, None),
    ]);
    Hop {
        rise: Track::new(Easing::EaseInOut)
            .at(0.0, 0.0)
            .at(0.14, 0.0)
            .at(0.3, -0.3)
            .at(0.46, 0.0)
            .at(0.64, 0.0)
            .at(1.0, 0.0)
            .build(),
        sx,
        sy,
    }
}

/// `kk-pop-loader-pop` and `kk-pop-loader-tip`: the tile's pop at the ends of the track, and its
/// roll in degrees, which snaps back to 0 as the cell jumps.
fn roll() -> (Keyframes<f32>, Keyframes<f32>) {
    let tip = bezier(0.55, 0.0, 0.9, 0.6);
    let step_end = Easing::steps(1, gpui_kit::base::StepPosition::JumpEnd).expect("one step");
    (
        Track::new(Easing::EaseInOut)
            .at(0.0, 0.0)
            .at(0.07, 1.15)
            .at(0.12, 1.0)
            .at(0.82, 1.0)
            .at(0.87, 1.12)
            .at(0.94, 0.0)
            .at(1.0, 0.0)
            .build(),
        Track::new(Easing::EaseInOut)
            .at_ease(0.0, 0.0, tip.clone())
            .at_ease(0.14, 0.0, tip.clone())
            .at_ease(0.31, 90.0, step_end.clone())
            .at(0.311, 0.0)
            .at(0.37, 8.0)
            .at_ease(0.43, 0.0, tip.clone())
            .at_ease(0.47, 0.0, tip)
            .at_ease(0.64, 90.0, step_end)
            .at(0.641, 0.0)
            .at(0.7, 8.0)
            .at(0.76, 0.0)
            .at(1.0, 0.0)
            .build(),
    )
}

/// `kk-pop-loader-step`: which cell the tile stands in, in ems, on `step-end`.
fn roll_cell() -> Keyframes<f32> {
    let step_end = Easing::steps(1, gpui_kit::base::StepPosition::JumpEnd).expect("one step");
    Track::new(step_end)
        .at(0.0, 0.0)
        .at(0.311, 1.0)
        .at(0.641, 2.0)
        .at(1.0, 2.0)
        .build()
}

/// `kk-pop-loader-ring`: scale and opacity, on Kirakira's out curve.
fn ripple(out: Easing) -> (Keyframes<f32>, Keyframes<f32>) {
    (
        Track::new(out.clone())
            .at(0.0, 0.3)
            .at(0.8, 1.0)
            .at(1.0, 1.0)
            .build(),
        Track::new(out)
            .at(0.0, 1.0)
            .at(0.25, 1.0)
            .at(0.8, 0.0)
            .at(1.0, 0.0)
            .build(),
    )
}

/// `kk-pop-loader-beat`: the core's double beat.
fn beat() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 1.0)
        .at(0.06, 1.25)
        .at(0.16, 0.9)
        .at(0.26, 1.0)
        .at(0.39, 1.18)
        .at(0.49, 0.94)
        .at(0.58, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// `kk-pop-loader-spike`: a bar's height as a fraction of the loader's.
fn spike() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.3)
        .at(0.14, 1.0)
        .at(0.28, 0.45)
        .at(0.38, 0.7)
        .at(0.55, 0.3)
        .at(1.0, 0.3)
        .build()
}

/// Each bar's period and start offset in ms, and the height it rests at without motion. The
/// periods share no common beat, so the bars never line up.
const BARS: [(u64, u64, f32); 5] = [
    (1100, 0, 0.5),
    (1700, 900, 0.8),
    (1300, 400, 0.6),
    (1900, 1300, 1.0),
    (1500, 700, 0.7),
];

fn looping(period: u64) -> Timing {
    Timing::new(ms(period)).iterations(IterationCount::Infinite)
}

/// Progress through a loop of `period` ms that starts `delay` ms in, or `None` before it starts.
fn progress(time: Duration, period: u64, delay: SignedDuration) -> Option<f32> {
    let sample = looping(period).delay(delay).sample(time);
    (sample.phase != MotionPhase::Before).then_some(sample.directed_progress)
}

impl RenderOnce for Spinner {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let time = loop_time(window, cx);
        let e = em(self.size, window.rem_size());
        let ink = match self.color {
            Some(color) => Ink::Fixed(color),
            None if self.inherit_color => Ink::Inherit,
            None => Ink::Fixed(cx.theme().primary),
        };
        let label = self.label.clone();

        let art = match self.variant {
            SpinnerVariant::Icon => {
                // GPUI Component's turn: one eased revolution per `speed`.
                let turn = time.map_or(0.0, |time| (self.easing)(cycle_progress(time, self.speed)));
                div()
                    .child(
                        self.icon
                            .with_size(self.size)
                            .when_some(self.color, |this, color| this.text_color(color))
                            .transform(Transformation::rotate(radians(
                                turn * std::f32::consts::TAU,
                            ))),
                    )
                    .into_any_element()
            }
            SpinnerVariant::Dots => dots(time, e, ink).into_any_element(),
            SpinnerVariant::Wave => wave_text(time, e, ink, &label, window).into_any_element(),
            SpinnerVariant::Roll => roll_tile(time, e, ink).into_any_element(),
            SpinnerVariant::Ripple => {
                ripple_rings(time, e, ink, cx.curves().out).into_any_element()
            }
            SpinnerVariant::Bars => bars(time, e, ink).into_any_element(),
        };

        div()
            .id(self.id)
            .role(Role::Status)
            .aria_label(label)
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .child(art)
            .into_element()
    }
}

/// Three dots that hop in turn, 0.15 s apart, squashing from their feet.
fn dots(time: Option<Duration>, e: f32, ink: Ink) -> impl IntoElement {
    let hop = hop();
    let d = 0.42 * e;
    let gap = 0.22 * e;
    let floor = 1.2 * e;
    let poses: Vec<(f32, Pose)> = (0..3)
        .map(|i| {
            match time.and_then(|time| progress(time, 1200, crate::motion::delay_ms(i * 150))) {
                Some(p) => (
                    hop.rise.sample(p) * e,
                    Pose::new().scale_xy(hop.sx.sample(p), hop.sy.sample(p)),
                ),
                None => (0.0, Pose::new()),
            }
        })
        .collect();
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
            let color = ink.color(window);
            for (i, (rise, pose)) in poses.iter().enumerate() {
                let x = ox + i as f32 * (d + gap);
                let bottom = oy + floor;
                // Squash about the dot's foot, then lift it.
                let map = Affine::scale_about(x + d / 2.0, bottom, pose.sx, pose.sy)
                    .then(Affine::translate(0.0, *rise));
                painted::paint(
                    window,
                    painted::ellipse(x + d / 2.0, bottom - d / 2.0, d / 2.0, d / 2.0, map),
                    color,
                );
            }
        },
    )
    .w(px(3.0 * d + 2.0 * gap))
    .h(px(1.3 * e))
    .flex_none()
}

/// The label's letters, jumping one after another 0.06 s apart. A moving letter is a vector
/// glyph, so it squashes and stretches as on the web; at rest it is GPUI's own text.
fn wave_text(
    time: Option<Duration>,
    e: f32,
    ink: Ink,
    label: &SharedString,
    window: &mut Window,
) -> impl IntoElement {
    let wave = wave();
    // A moving letter is drawn now, in the colour of the text it is laid out in; resting letters
    // are text and inherit it themselves.
    let color = ink.color(window);
    let text = TextStyleRefinement {
        font_size: Some(px(e).into()),
        font_weight: Some(FontWeight::BLACK),
        // The vector box is 1.5em, centred on the 1em line, so descenders aren't clipped and the
        // baseline stays put.
        line_height: Some(relative(1.5)),
        color: Some(color),
        ..Default::default()
    };
    let setting = TextSetting::new(&text, window);
    let letters = label
        .graphemes(true)
        .enumerate()
        .map(|(i, glyph)| {
            let pose = match time
                .and_then(|time| progress(time, 1500, crate::motion::delay_ms(i as u64 * 60)))
            {
                Some(p) => Pose::new()
                    .y(wave.rise.sample(p) * e)
                    .scale_xy(wave.sx.sample(p), wave.sy.sample(p)),
                None => Pose::new(),
            };
            let moving = !pose.is_identity();
            let line_box = setting.line_box(glyph, window);
            let vector = moving.then(|| {
                div().absolute().top(px(-0.25 * e)).left_0().child(
                    Vector::new(line_box.size)
                        .layer(setting.layer(glyph, line_box))
                        .pose(pose)
                        .origin(0.5, 1.25 / 1.5),
                )
            });
            // `tracking-wide`: a fortieth of an em after each letter.
            div()
                .relative()
                .flex_none()
                .mr(px(0.025 * e))
                .child(
                    div()
                        .when(moving, |this| this.invisible())
                        .child(SharedString::from(glyph.to_string())),
                )
                .children(vector)
        })
        .collect::<Vec<_>>();
    div()
        .flex()
        .flex_none()
        .items_end()
        .pt(px(0.3 * e))
        .text_size(px(e))
        .line_height(relative(1.0))
        .font_weight(FontWeight::BLACK)
        .whitespace_nowrap()
        .when(matches!(ink, Ink::Fixed(_)), |this| this.text_color(color))
        .children(letters)
}

/// A rounded tile that tips over its corner from cell to cell along a faint track.
fn roll_tile(time: Option<Duration>, e: f32, ink: Ink) -> impl IntoElement {
    let (pop, tip) = roll();
    // Without motion the tile rests in the middle cell, as the web's base `translate: 1em`.
    let (cell, scale, angle) =
        match time.and_then(|time| progress(time, 1800, SignedDuration::ZERO)) {
            Some(p) => (roll_cell().sample(p), pop.sample(p), tip.sample(p)),
            None => (1.0, 1.0, 0.0),
        };
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
            let color = ink.color(window);
            let line = 0.12 * e;
            painted::paint(
                window,
                painted::rounded_rect(
                    ox,
                    oy + 1.6 * e - line,
                    3.0 * e,
                    line,
                    line,
                    Affine::IDENTITY,
                ),
                color.opacity(0.25),
            );
            // In the cell's own box: tip about the bottom-right corner, pop about the bottom
            // centre, then place the cell on the track.
            let map = Affine::rotate_about(e, e, angle)
                .then(Affine::scale_about(e / 2.0, e, scale, scale))
                .then(Affine::translate(ox + cell * e, oy + 1.6 * e - line - e));
            painted::paint(
                window,
                painted::rounded_rect(0.0, 0.0, e, e, 0.28 * e, map),
                color,
            );
        },
    )
    .w(px(3.0 * e))
    .h(px(1.6 * e))
    .flex_none()
}

/// Two rings opening out of a core that beats twice a cycle.
fn ripple_rings(time: Option<Duration>, e: f32, ink: Ink, out: Easing) -> impl IntoElement {
    let (grow, fade) = ripple(out);
    let heart = beat();
    let rings: Vec<(f32, f32)> = (0..2)
        .map(|i| {
            match time.and_then(|time| progress(time, 1500, crate::motion::delay_ms(i * 500))) {
                Some(p) => (grow.sample(p), fade.sample(p).clamp(0.0, 1.0)),
                // Before its first cycle, and without motion, a ring rests at its base size.
                None => (0.7 + i as f32 * 0.3, 0.6 - i as f32 * 0.3),
            }
        })
        .collect();
    let core = time
        .and_then(|time| progress(time, 1500, SignedDuration::ZERO))
        .map_or(1.0, |p| heart.sample(p));
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
            let color = ink.color(window);
            let (cx, cy) = (ox + 0.8 * e, oy + 0.8 * e);
            for (scale, alpha) in &rings {
                if *alpha > 0.0 && *scale > 0.0 {
                    painted::paint(
                        window,
                        painted::ring(cx, cy, 0.8 * e * scale, 0.12 * e * scale),
                        color.opacity(*alpha),
                    );
                }
            }
            let r = 0.8 * e * 0.32 * core;
            painted::paint(
                window,
                painted::ellipse(cx, cy, r, r, Affine::IDENTITY),
                color,
            );
        },
    )
    .size(px(1.6 * e))
    .flex_none()
}

/// Five bars spiking from their middles, each on its own tempo.
fn bars(time: Option<Duration>, e: f32, ink: Ink) -> impl IntoElement {
    let height = spike();
    let scales: Vec<f32> = BARS
        .iter()
        .map(|&(period, offset, rest)| {
            time.and_then(|time| {
                progress(
                    time,
                    period,
                    SignedDuration::negative(Duration::from_millis(offset)),
                )
            })
            .map_or(rest, |p| height.sample(p))
        })
        .collect();
    let w = 0.2 * e;
    let gap = 0.14 * e;
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let (ox, oy) = (f32::from(bounds.origin.x), f32::from(bounds.origin.y));
            let color = ink.color(window);
            let h = 1.2 * e;
            for (i, scale) in scales.iter().enumerate() {
                let x = ox + i as f32 * (w + gap);
                // The whole pill scales, rounded ends included, as CSS `scale` does.
                let map = Affine::scale_about(x + w / 2.0, oy + h / 2.0, 1.0, *scale);
                painted::paint(
                    window,
                    painted::rounded_rect(x, oy, w, h, w / 2.0, map),
                    color,
                );
            }
        },
    )
    .w(px(5.0 * w + 4.0 * gap))
    .h(px(1.2 * e))
    .flex_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tile_steps_a_cell_as_it_lands() {
        let cell = roll_cell();
        assert_eq!(cell.sample(0.0), 0.0);
        assert_eq!(cell.sample(0.31), 0.0);
        assert_eq!(cell.sample(0.32), 1.0);
        assert_eq!(cell.sample(0.65), 2.0);
    }

    #[test]
    fn the_tile_snaps_upright_as_it_jumps() {
        let (_, tip) = roll();
        // It holds the full 90° to the end of the roll, then stands upright in the next cell.
        assert!((tip.sample(0.3105) - 90.0).abs() < 1e-3);
        assert!(tip.sample(0.3112).abs() < 0.1);
        assert_eq!(tip.sample(0.37), 8.0);
    }

    #[test]
    fn dots_rest_between_hops() {
        let hop = hop();
        assert_eq!(hop.rise.sample(0.32), -0.75);
        assert_eq!(hop.rise.sample(0.8), 0.0);
        assert_eq!((hop.sx.sample(0.12), hop.sy.sample(0.12)), (1.3, 0.7));
        assert_eq!((hop.sx.sample(0.8), hop.sy.sample(0.8)), (1.0, 1.0));
    }

    #[test]
    fn bars_start_mid_cycle() {
        // A negative delay starts the loop part-way through, as `animation-delay: -0.9s` does.
        let p = progress(Duration::ZERO, 1700, SignedDuration::negative(ms(900))).unwrap();
        assert!((p - 900.0 / 1700.0).abs() < 1e-4);
        // A positive delay holds the loop back.
        assert_eq!(progress(ms(100), 1500, crate::motion::delay_ms(500)), None);
    }

    #[test]
    fn the_turn_stays_exact_after_days() {
        let speed = ms(800);
        assert_eq!(cycle_progress(Duration::ZERO, speed), 0.0);
        assert_eq!(cycle_progress(ms(200), speed), 0.25);
        assert_eq!(cycle_progress(ms(1000), speed), 0.25);
        // Thirty days and a quarter turn in: f32 seconds would be off by a large part of a turn.
        let days = Duration::from_secs(30 * 24 * 60 * 60);
        assert_eq!(cycle_progress(days + ms(200), speed), 0.25);
        assert_eq!(cycle_progress(days + ms(601), speed), 601.0 / 800.0);
    }

    #[test]
    fn spinners_built_apart_get_their_own_ids() {
        let a = Spinner::new();
        let b = Spinner::new();
        assert_ne!(a.id, b.id);
        assert_eq!(Spinner::new().id("x").id, ElementId::from("x"));
    }

    #[test]
    fn sizes_follow_the_icon_sizes() {
        assert_eq!(em(Size::Medium, px(16.0)), 16.0);
        assert_eq!(em(Size::Small, px(16.0)), 14.0);
        assert_eq!(em(Size::Large, px(16.0)), 24.0);
        // In rems, so a scaled rem (a popping button) scales them; pixel sizes stay put.
        assert_eq!(em(Size::Medium, px(20.0)), 20.0);
        assert_eq!(em(Size::Size(px(48.0)), px(20.0)), 48.0);
    }

    #[test]
    fn keyframes_match_the_web() {
        use crate::parity::assert_number_track;
        // Lengths are in ems; the reader takes `-0.75em` as -0.75.
        let hop = hop();
        assert_number_track("pop-loader", "kk-pop-loader-hop", "y", &hop.rise);
        assert_number_track("pop-loader", "kk-pop-loader-hop", "sx", &hop.sx);
        assert_number_track("pop-loader", "kk-pop-loader-hop", "sy", &hop.sy);
        let wave = wave();
        assert_number_track("pop-loader", "kk-pop-loader-wave", "y", &wave.rise);
        assert_number_track("pop-loader", "kk-pop-loader-wave", "sx", &wave.sx);
        assert_number_track("pop-loader", "kk-pop-loader-wave", "sy", &wave.sy);
        assert_number_track("pop-loader", "kk-pop-loader-step", "x", &roll_cell());
        let (pop, tip) = roll();
        assert_number_track("pop-loader", "kk-pop-loader-pop", "sx", &pop);
        assert_number_track("pop-loader", "kk-pop-loader-tip", "rotate", &tip);
        let (grow, fade) = ripple(Easing::EaseOut);
        assert_number_track("pop-loader", "kk-pop-loader-ring", "sx", &grow);
        assert_number_track("pop-loader", "kk-pop-loader-ring", "opacity", &fade);
        assert_number_track("pop-loader", "kk-pop-loader-beat", "sx", &beat());
        assert_number_track("pop-loader", "kk-pop-loader-spike", "sy", &spike());
    }
}
