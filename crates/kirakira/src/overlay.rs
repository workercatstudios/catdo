//! The motion shared by Kirakira's floating surfaces: popover, hover card, tooltip and menus.
//!
//! On the web each `pop-*` overlay hangs its keyframes on Base UI's `data-side`: the surface pops
//! out of its trigger, sliding the last stretch away from the side it opens on, and shrinks back
//! into it on close. Here a [`Presence`] remembers when a surface opened or closed (on
//! [`motion::now`](crate::motion::now), so screenshots can pin it), a [`Surface`] turns that into
//! a [`Pose`] for [`motion::transform`](crate::motion::transform), and [`ghost`] keeps a closed
//! surface painted where it was while it plays its exit.

use std::time::{Duration, Instant};

use gpui_kit::base::{ElementExt, POPUP_PRIORITY, Positioner};
use gpui_kit::{
    Anchor, AnyElement, App, Bounds, BoxShadow, ContentMask, Context, Div, Element, ElementId,
    Entity, GlobalElementId, InspectorElementId, IntoElement, LayoutId, ParentElement as _, Pixels,
    Window, deferred, div, hsla, point, px,
};

use crate::motion::{Easing, Keyframes, Pose, Timing, Track, ms, transform};

/// The side of its trigger a surface opens on: Base UI's `data-side`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Side {
    Top,
    #[default]
    Bottom,
    Left,
    Right,
}

impl Side {
    /// The side a corner-anchored surface lands on. GPUI Component's popovers and menus put the
    /// `anchor` corner of the surface on the trigger and never flip, so the anchor decides.
    pub fn of_anchor(anchor: Anchor) -> Self {
        match anchor {
            Anchor::TopLeft | Anchor::TopCenter | Anchor::TopRight => Side::Bottom,
            Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => Side::Top,
            Anchor::LeftCenter => Side::Right,
            Anchor::RightCenter => Side::Left,
        }
    }

    /// The side `surface` sits on relative to `trigger`, measured from where both were painted.
    /// `None` when they overlap on both axes, so the caller keeps its own guess.
    pub fn measure(trigger: Bounds<Pixels>, surface: Bounds<Pixels>) -> Option<Self> {
        // A pixel of slack for rounding.
        let slack = px(1.);
        if surface.top() >= trigger.bottom() - slack {
            Some(Side::Bottom)
        } else if surface.bottom() <= trigger.top() + slack {
            Some(Side::Top)
        } else if surface.left() >= trigger.right() - slack {
            Some(Side::Right)
        } else if surface.right() <= trigger.left() + slack {
            Some(Side::Left)
        } else {
            None
        }
    }

    /// Where the surface starts relative to where it rests, as a unit vector: towards the
    /// trigger. `data-side="bottom"` starts above (`translate: 0 -0.5rem`), and so on.
    pub fn toward_trigger(self) -> (f32, f32) {
        match self {
            Side::Top => (0.0, 1.0),
            Side::Bottom => (0.0, -1.0),
            Side::Left => (1.0, 0.0),
            Side::Right => (-1.0, 0.0),
        }
    }

    /// The transform origin on the trigger-facing edge, as Base UI sets `--transform-origin`: along that edge at the alignment point (`along`, 0 for
    /// start, 0.5 for centre, 1 for end).
    pub fn origin(self, along: f32) -> (f32, f32) {
        match self {
            Side::Bottom => (along, 0.0),
            Side::Top => (along, 1.0),
            Side::Right => (0.0, along),
            Side::Left => (1.0, along),
        }
    }
}

/// How far along the trigger-facing edge an anchor aligns the surface: 0 start, 0.5 centre, 1 end.
pub fn anchor_alignment(anchor: Anchor) -> f32 {
    match anchor {
        Anchor::TopLeft | Anchor::BottomLeft => 0.0,
        Anchor::TopCenter | Anchor::BottomCenter | Anchor::LeftCenter | Anchor::RightCenter => 0.5,
        Anchor::TopRight | Anchor::BottomRight => 1.0,
    }
}

/// Where a surface is in its life.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    /// Playing its entrance, this long in.
    Opening(Duration),
    /// At rest.
    Open,
    /// Playing its exit, this long in. Still painted, no longer interactive.
    Closing(Duration),
    /// Gone.
    Closed,
}

impl Phase {
    /// Whether frames are still needed.
    pub fn running(self) -> bool {
        matches!(self, Phase::Opening(_) | Phase::Closing(_))
    }
}

/// When a surface last opened or closed.
#[derive(Clone, Copy, Debug, Default)]
pub struct Presence {
    open: bool,
    since: Option<Instant>,
}

impl Presence {
    /// Records the state at `now`. A change restarts the clock; the same state keeps it.
    pub fn set(&mut self, open: bool, now: Instant) {
        if self.open != open {
            self.open = open;
            self.since = Some(now);
        }
    }

    /// Opens afresh at `now`, even if it was open: a menu shown again.
    pub fn restart(&mut self, now: Instant) {
        self.open = true;
        self.since = Some(now);
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Whether it has ever opened.
    pub fn started(&self) -> bool {
        self.since.is_some()
    }

    /// The phase at `now`, with entrances of `enter` and exits of `exit`.
    pub fn phase(&self, now: Instant, enter: Duration, exit: Duration) -> Phase {
        let elapsed = self.since.map(|since| now.saturating_duration_since(since));
        match (self.open, elapsed) {
            (true, Some(elapsed)) if elapsed < enter => Phase::Opening(elapsed),
            (true, _) => Phase::Open,
            (false, Some(elapsed)) if elapsed < exit => Phase::Closing(elapsed),
            (false, _) => Phase::Closed,
        }
    }
}

/// One `@keyframes` rule as tracks, sampled with a linear timing over `duration`: opacity, an
/// even scale, how much of the slide distance the surface is away from rest towards its trigger
/// (1 the whole distance, 0 at rest), and a rotation in degrees (only Hover Card's swings).
pub struct Clip {
    duration: Duration,
    opacity: Keyframes<f32>,
    scale: Keyframes<f32>,
    slide: Keyframes<f32>,
    rotate: Keyframes<f32>,
}

impl Clip {
    fn pose(&self, elapsed: Duration, toward: (f32, f32)) -> Pose {
        let t = Timing::new(self.duration).sample(elapsed).directed_progress;
        let slide = self.slide.sample(t);
        Pose::new()
            .at(toward.0 * slide, toward.1 * slide)
            .scale(self.scale.sample(t))
            .rotate(self.rotate.sample(t))
            .opacity(self.opacity.sample(t))
    }
}

/// A property that holds still through a rule.
fn constant(value: f32) -> Keyframes<f32> {
    Track::new(Easing::Linear)
        .at(0.0, value)
        .at(1.0, value)
        .build()
}

// ── Pop Popover and the menus: `kk-pop-popover-in`, `kk-pop-dropdown-menu-in`, ... ──────────

/// `opacity` of `kk-pop-*-in`: 0 → 1 by 30 % (ease-out).
pub fn pop_in_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.0, Easing::EaseOut)
        .at(0.3, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// `scale` of `kk-pop-*-in`: 0.7 → 1.04 (55 %, ease-out) → 0.985 (80 %) → 1.
pub fn pop_in_scale() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.7, Easing::EaseOut)
        .at(0.55, 1.04)
        .at(0.8, 0.985)
        .at(1.0, 1.0)
        .build()
}

/// `translate` of `kk-pop-*-in`: the whole 0.5rem towards the trigger, at rest by 55 %.
pub fn pop_in_slide() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 1.0, Easing::EaseOut)
        .at(0.55, 0.0)
        .at(1.0, 0.0)
        .build()
}

/// `opacity` of `kk-pop-*-out` (0.12 s ease-in): to 0.
pub fn pop_out_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseIn).at(0.0, 1.0).at(1.0, 0.0).build()
}

/// `scale` of `kk-pop-*-out`: to 0.9.
pub fn pop_out_scale() -> Keyframes<f32> {
    Track::new(Easing::EaseIn).at(0.0, 1.0).at(1.0, 0.9).build()
}

/// `translate` of `kk-pop-*-out`: half the slide back towards the trigger.
pub fn pop_out_slide() -> Keyframes<f32> {
    Track::new(Easing::EaseIn).at(0.0, 0.0).at(1.0, 0.5).build()
}

/// `kk-pop-*-fade-in`, the reduced-motion entrance (ease-out): from opacity 0.
pub fn fade_in() -> Keyframes<f32> {
    Track::new(Easing::EaseOut)
        .at(0.0, 0.0)
        .at(1.0, 1.0)
        .build()
}

/// `kk-pop-*-fade-out`, the reduced-motion exit (ease-in): to opacity 0. Also Pop Tooltip's
/// `kk-pop-tooltip-out`.
pub fn fade_out() -> Keyframes<f32> {
    Track::new(Easing::EaseIn).at(0.0, 1.0).at(1.0, 0.0).build()
}

// ── Pop Hover Card: `kk-pop-hover-card-in` ──────────────────────────────────────────────────

/// `opacity` of `kk-pop-hover-card-in`: 0 → 1 by 25 % (ease-out).
pub fn hover_card_in_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.0, Easing::EaseOut)
        .at(0.25, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// `scale` of `kk-pop-hover-card-in`: 0.85 → 1 by 40 % (ease-out).
pub fn hover_card_in_scale() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.85, Easing::EaseOut)
        .at(0.4, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// `translate` of `kk-pop-hover-card-in`: at rest by 40 %.
pub fn hover_card_in_slide() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 1.0, Easing::EaseOut)
        .at(0.4, 0.0)
        .at(1.0, 0.0)
        .build()
}

/// `rotate` of `kk-pop-hover-card-in`, in degrees, with `--kk-pop-hover-card-swing` at its 6deg:
/// 6 → −1.92 (40 %, ease-out) → 0.96 (70 %) → 0. The settle swing, a first rebound of a third,
/// then halving.
pub fn hover_card_in_swing() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 6.0, Easing::EaseOut)
        .at(0.4, -1.92)
        .at(0.7, 0.96)
        .at(1.0, 0.0)
        .build()
}

/// How far the centre of a box of `size` moves when `pose` turns it by `pose.rotate` about
/// `origin` (fractions of the box) at its scale: what a rotation does to where a box is, without
/// the tilt. Add it to the pose's translation to sway a box that can't be turned.
pub fn sway(pose: &Pose, origin: (f32, f32), size: gpui_kit::Size<Pixels>) -> (f32, f32) {
    let arm_x = (0.5 - origin.0) * f32::from(size.width) * pose.sx;
    let arm_y = (0.5 - origin.1) * f32::from(size.height) * pose.sy;
    let (sin, cos) = pose.rotate.to_radians().sin_cos();
    (
        cos * arm_x - sin * arm_y - arm_x,
        sin * arm_x + cos * arm_y - arm_y,
    )
}

// ── Pop Tooltip: `kk-pop-tooltip-in` ────────────────────────────────────────────────────────

/// `scale` of `kk-pop-tooltip-in`: 0.4 → 1.1 (50 %, ease-out) → 0.96 (75 %) → 1. Its opacity is
/// [`pop_in_opacity`]'s.
pub fn tooltip_in_scale() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.4, Easing::EaseOut)
        .at(0.5, 1.1)
        .at(0.75, 0.96)
        .at(1.0, 1.0)
        .build()
}

/// `translate` of `kk-pop-tooltip-in`: the whole 0.375rem, at rest by 50 %.
pub fn tooltip_in_slide() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 1.0, Easing::EaseOut)
        .at(0.5, 0.0)
        .at(1.0, 0.0)
        .build()
}

/// The motion of one kind of surface.
pub struct Surface {
    enter: Clip,
    exit: Clip,
    /// Reduced motion: a fade in where it rests, and a fade out.
    fade_in: Clip,
    fade_out: Clip,
    /// The slide, in rems.
    distance: f32,
}

impl Surface {
    fn fade(duration: Duration, opacity: Keyframes<f32>) -> Clip {
        Clip {
            duration,
            opacity,
            scale: constant(1.0),
            slide: constant(0.0),
            rotate: constant(0.0),
        }
    }

    /// Pop Popover and the menus: `scale` 0.7 → 1.04 → 0.985 → 1 over 0.3 s, sliding the last
    /// 0.5rem away from the trigger by 55 %; opacity done by 30 %. Out: 0.12 s ease-in to
    /// opacity 0, scale 0.9, half the slide back. Reduced: 0.15 s fade in, 0.1 s fade out.
    pub fn pop() -> Self {
        Self {
            enter: Clip {
                duration: ms(300),
                opacity: pop_in_opacity(),
                scale: pop_in_scale(),
                slide: pop_in_slide(),
                rotate: constant(0.0),
            },
            exit: Clip {
                duration: ms(120),
                opacity: pop_out_opacity(),
                scale: pop_out_scale(),
                slide: pop_out_slide(),
                rotate: constant(0.0),
            },
            fade_in: Self::fade(ms(150), fade_in()),
            fade_out: Self::fade(ms(100), fade_out()),
            distance: 0.5,
        }
    }

    /// Pop Hover Card: grows from 0.85 and drops the last 0.5rem by 40 % of 0.5 s while it swings
    /// 6° → −1.92° → 0.96° → 0 about its trigger; opacity done by 25 %. The swing is in the pose's
    /// `rotate`, which [`motion::transform`](crate::motion::transform) can't apply to a subtree;
    /// Hover Card draws it (see [`crate::hover_card`]). Out and reduced as [`Surface::pop`].
    pub fn hover_card() -> Self {
        Self {
            enter: Clip {
                duration: ms(500),
                opacity: hover_card_in_opacity(),
                scale: hover_card_in_scale(),
                slide: hover_card_in_slide(),
                rotate: hover_card_in_swing(),
            },
            ..Self::pop()
        }
    }

    /// Pop Tooltip: `scale` 0.4 → 1.1 → 0.96 → 1 over 0.26 s, sliding the last 0.375rem by 50 %;
    /// opacity done by 30 %. Out: a 0.1 s fade. Reduced motion fades in over 0.12 s.
    pub fn tooltip() -> Self {
        Self {
            enter: Clip {
                duration: ms(260),
                opacity: pop_in_opacity(),
                scale: tooltip_in_scale(),
                slide: tooltip_in_slide(),
                rotate: constant(0.0),
            },
            exit: Self::fade(ms(100), fade_out()),
            fade_in: Self::fade(ms(120), fade_in()),
            fade_out: Self::fade(ms(100), fade_out()),
            distance: 0.375,
        }
    }

    /// Entrance and exit durations.
    pub fn durations(&self, reduced: bool) -> (Duration, Duration) {
        if reduced {
            (self.fade_in.duration, self.fade_out.duration)
        } else {
            (self.enter.duration, self.exit.duration)
        }
    }

    /// The phase of `presence` at `now`.
    pub fn phase(&self, presence: &Presence, now: Instant, reduced: bool) -> Phase {
        let (enter, exit) = self.durations(reduced);
        presence.phase(now, enter, exit)
    }

    /// The pose for `phase` on `side`, with `rem` pixels to the rem.
    pub fn pose(&self, phase: Phase, side: Side, reduced: bool, rem: Pixels) -> Pose {
        let (dx, dy) = side.toward_trigger();
        let distance = self.distance * f32::from(rem);
        let toward = (dx * distance, dy * distance);
        match phase {
            Phase::Open => Pose::new(),
            Phase::Closed => Pose::new().opacity(0.0),
            Phase::Opening(elapsed) if reduced => self.fade_in.pose(elapsed, toward),
            Phase::Closing(elapsed) if reduced => self.fade_out.pose(elapsed, toward),
            Phase::Opening(elapsed) => self.enter.pose(elapsed, toward),
            Phase::Closing(elapsed) => self.exit.pose(elapsed, toward),
        }
    }
}

/// What a wrapped popup (Popover, Hover Card) keeps between frames: its presence, the gpui-base
/// state it was last open with, and where its trigger and surface were painted.
pub struct Anchored<S: 'static> {
    pub presence: Presence,
    /// The primitive's own state, captured from its content builder, so a closing surface can be
    /// built once more for its exit.
    pub state: Option<Entity<S>>,
    pub trigger: Option<Bounds<Pixels>>,
    pub surface: Option<Bounds<Pixels>>,
}

impl<S: 'static> Default for Anchored<S> {
    fn default() -> Self {
        Self {
            presence: Presence::default(),
            state: None,
            trigger: None,
            surface: None,
        }
    }
}

impl<S: 'static> Anchored<S> {
    /// The side the surface opened on: measured once both have been painted, else `fallback`.
    pub fn side(&self, fallback: Side) -> Side {
        self.trigger
            .zip(self.surface)
            .and_then(|(trigger, surface)| Side::measure(trigger, surface))
            .unwrap_or(fallback)
    }
}

/// The pose of a wrapped popup this frame, from inside its content builder: gpui-base calls that
/// only while the popup is open, so this is where an open begins. Captures the primitive's state
/// for the exit, and asks for frames while the entrance plays. Returns the pose and the
/// transform origin.
pub fn open_pose<S: 'static>(
    anchored: &Entity<Anchored<S>>,
    motion: Surface,
    anchor: Anchor,
    window: &mut Window,
    cx: &mut Context<S>,
) -> (Pose, (f32, f32)) {
    let now = crate::motion::now();
    let reduced = cx.reduce_motion();
    let state = cx.entity();
    anchored.update(cx, |anchored, _| {
        anchored.state = Some(state);
        anchored.presence.set(true, now);
    });
    let anchored = anchored.read(cx);
    let phase = motion.phase(&anchored.presence, now, reduced);
    if phase.running() {
        window.request_animation_frame();
    }
    let side = anchored.side(Side::of_anchor(anchor));
    (
        motion.pose(phase, side, reduced, window.rem_size()),
        side.origin(anchor_alignment(anchor)),
    )
}

/// Records where `element` is painted as the trigger of `anchored`.
pub fn record_trigger<S: 'static, E: ElementExt>(anchored: &Entity<Anchored<S>>, element: E) -> E {
    let anchored = anchored.clone();
    element.on_prepaint(move |bounds, _, cx| {
        anchored.update(cx, |anchored, _| anchored.trigger = Some(bounds))
    })
}

/// Wraps a popup surface: records where it rests and applies `pose`, scaling about `origin`.
pub fn surface<S: 'static>(
    anchored: &Entity<Anchored<S>>,
    id: impl Into<ElementId>,
    pose: Pose,
    origin: (f32, f32),
    child: impl IntoElement,
) -> Div {
    let anchored = anchored.clone();
    div()
        .on_prepaint(move |bounds, _, cx| {
            anchored.update(cx, |anchored, _| anchored.surface = Some(bounds))
        })
        .child(transform(id, pose, child).origin(origin.0, origin.1))
}

/// GPUI Component's popup shadow (`popover_style`'s hairline ring and two soft layers) at
/// `strength` of its ink.
///
/// GPUI fades each primitive on its own rather than the composited surface, so a fading panel
/// shows its own shadow through it as a grey slab. Ramping the shadow by the cube of the
/// opacity, as GPUI Component's dropdowns do, keeps it out of sight until the panel covers it.
pub fn shadow(strength: f32, cx: &gpui_kit::App) -> Vec<BoxShadow> {
    use gpui_kit::component::ActiveTheme as _;
    let strength = strength.clamp(0.0, 1.0);
    let ring = cx.theme().foreground.alpha(0.1 * strength);
    let ink = hsla(0.0, 0.0, 0.0, 0.1 * strength);
    vec![
        BoxShadow::new(px(0.), px(0.), ring)
            .blur_radius(px(0.))
            .spread_radius(px(1.)),
        BoxShadow::new(px(0.), px(4.), ink)
            .blur_radius(px(3.))
            .spread_radius(px(-1.)),
        BoxShadow::new(px(0.), px(2.), ink)
            .blur_radius(px(2.))
            .spread_radius(px(-2.)),
    ]
}

/// The shadow for a surface drawn at `pose`: [`shadow`] by the cube of its opacity.
pub fn pose_shadow(pose: Pose, cx: &gpui_kit::App) -> Option<Vec<BoxShadow>> {
    let alpha = pose.alpha();
    (alpha < 1.0).then(|| shadow(alpha * alpha * alpha, cx))
}

/// Paints a closed surface at `bounds` while it plays its exit, above everything like the popup
/// it was. Ghosts take no input: the pointer goes through to whatever is underneath.
pub fn ghost(bounds: Bounds<Pixels>, child: impl IntoElement) -> AnyElement {
    ghost_at(bounds, POPUP_PRIORITY, child)
}

/// [`ghost`] at paint priority `priority`: a submenu over its parent menu, a tooltip over
/// everything.
pub fn ghost_at(bounds: Bounds<Pixels>, priority: usize, child: impl IntoElement) -> AnyElement {
    deferred(
        Positioner::corner(Anchor::TopLeft, point(bounds.origin.x, bounds.origin.y))
            .margin(px(0.))
            .child(Inert::new(child)),
    )
    .with_priority(priority)
    .into_any_element()
}

/// Paints `child` as it is but takes no input: the pointer goes through it to whatever is
/// underneath, and none of its listeners, hover styles or cursors fire.
///
/// GPUI hit-tests each hitbox against the content mask it was inserted under, in prepaint, and
/// draws against the mask current at paint. So the child is prepainted under an empty mask, which
/// no point is inside, and painted under the real one. (A `deferred` inside the child keeps the
/// empty mask and isn't drawn.)
pub struct Inert {
    child: AnyElement,
}

impl Inert {
    pub fn new(child: impl IntoElement) -> Self {
        Self {
            child: child.into_any_element(),
        }
    }
}

impl IntoElement for Inert {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Inert {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let nowhere = ContentMask {
            bounds: Bounds::default(),
        };
        window.with_content_mask(Some(nowhere), |window| self.child.prepaint(window, cx));
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

/// Keyframe parity for the `pop-*` overlays (tests only).
///
/// Their entrance and exit rules set `translate` through custom properties
/// (`var(--kk-pop-popover-x)`, `calc(var(..) / 2)`) and the hover card's `rotate` through
/// `var(--kk-pop-hover-card-swing, 6deg)`, which `crate::parity` can't read. These helpers put in
/// the fallback of a `var(..)` that has one and work out a `calc(..)` that is a plain product
/// (`calc(6deg * -0.32)`), drop the declarations still using either, and check the rest with the
/// same parser; the slide is checked where the CSS writes it out (`translate: 0 0`).
#[cfg(test)]
pub(crate) mod web {
    use crate::motion::Keyframes;
    use crate::parity::{self, CssFrame};

    /// `css` with each `var(--name, value)` replaced by its fallback `value` (when it has no
    /// parentheses), then each `calc(a * b)` of two numbers by their product, in `a`'s unit.
    pub(super) fn resolve(css: &str) -> String {
        let mut out = String::with_capacity(css.len());
        let mut rest = css;
        while let Some(start) = rest.find("var(") {
            let Some(close) = rest[start..].find(')').map(|close| start + close) else {
                break;
            };
            let inner = &rest[start + 4..close];
            out.push_str(&rest[..start]);
            match inner.split_once(',') {
                Some((_, fallback)) if !inner.contains('(') => out.push_str(fallback.trim()),
                _ => out.push_str(&rest[start..=close]),
            }
            rest = &rest[close + 1..];
        }
        out.push_str(rest);

        let css = out;
        let mut out = String::with_capacity(css.len());
        let mut rest = css.as_str();
        while let Some(start) = rest.find("calc(") {
            let Some(close) = rest[start..].find(')').map(|close| start + close) else {
                break;
            };
            let inner = &rest[start + 5..close];
            out.push_str(&rest[..start]);
            match product(inner) {
                Some(value) => out.push_str(&value),
                None => out.push_str(&rest[start..=close]),
            }
            rest = &rest[close + 1..];
        }
        out.push_str(rest);
        out
    }

    /// `a * b` for two numbers, at most one with a unit: the product in that unit.
    fn product(inner: &str) -> Option<String> {
        if inner.contains('(') {
            return None;
        }
        let (a, b) = inner.split_once('*')?;
        let split = |value: &str| {
            let value = value.trim();
            let end = value
                .find(|c: char| c.is_ascii_alphabetic() || c == '%')
                .unwrap_or(value.len());
            Some((value[..end].parse::<f32>().ok()?, value[end..].to_string()))
        };
        let ((a, unit_a), (b, unit_b)) = (split(a)?, split(b)?);
        if !unit_a.is_empty() && !unit_b.is_empty() {
            return None;
        }
        Some(format!("{}{unit_a}{unit_b}", a * b))
    }

    /// `css` without the declarations that use `var(...)` or `calc(...)`.
    fn strip_custom(css: &str) -> String {
        css.split_inclusive([';', '{', '}'])
            .map(|piece| {
                if !(piece.contains("var(") || piece.contains("calc(")) {
                    piece
                } else if piece.ends_with(';') {
                    ""
                } else {
                    &piece[piece.len() - 1..]
                }
            })
            .collect()
    }

    /// The frames of `@keyframes name` in `component`'s web source, or `None` outside the
    /// monorepo.
    pub fn frames(component: &str, name: &str) -> Option<Vec<CssFrame>> {
        let Some(css) = parity::source(component) else {
            eprintln!("parity: {component}.tsx not found; skipping {name}");
            return None;
        };
        Some(parity::keyframes(&strip_custom(&resolve(&css)), name))
    }

    /// Checks `track` against `property` (`"opacity"`, `"sx"` or `"rotate"`) at every keyframe
    /// that sets it.
    pub fn assert_track(component: &str, name: &str, property: &str, track: &Keyframes<f32>) {
        let Some(frames) = frames(component, name) else {
            return;
        };
        let mut checked = 0;
        for frame in &frames {
            let expected = match property {
                "opacity" => frame.opacity,
                "sx" => frame.sx,
                "rotate" => frame.rotate,
                other => panic!("unknown property {other}"),
            };
            if let Some(expected) = expected {
                let actual = track.sample(frame.offset);
                assert!(
                    (expected - actual).abs() < 1e-3,
                    "{name} at {}%: {property} is {actual}, the web has {expected}",
                    frame.offset * 100.0
                );
                checked += 1;
            }
        }
        assert!(checked > 0, "{name} never sets {property}");
    }

    /// Checks a slide track: 0 wherever the CSS writes `translate: 0 0`, and the whole distance at
    /// the start (where the CSS sets it through `var(...)`).
    pub fn assert_slide(component: &str, name: &str, track: &Keyframes<f32>) {
        let Some(frames) = frames(component, name) else {
            return;
        };
        assert_eq!(track.sample(0.0), 1.0, "{name} starts the whole slide away");
        let rests: Vec<f32> = frames
            .iter()
            .filter(|frame| frame.x == Some(0.0) && frame.y == Some(0.0))
            .map(|frame| frame.offset)
            .collect();
        assert!(!rests.is_empty(), "{name} never writes translate: 0 0");
        for offset in rests {
            assert!(
                track.sample(offset).abs() < 1e-3,
                "{name} at {}%: still sliding, the web is at rest",
                offset * 100.0
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parity::assert_number_track;
    use gpui_kit::size;

    /// The popover and the three menus share one entrance, exit and fades.
    #[test]
    fn pop_matches_the_web_keyframes() {
        for (component, prefix) in [
            ("pop-popover", "kk-pop-popover"),
            ("pop-dropdown-menu", "kk-pop-dropdown-menu"),
            ("pop-context-menu", "kk-pop-context-menu"),
            ("pop-menubar", "kk-pop-menubar"),
        ] {
            let name = |suffix: &str| format!("{prefix}-{suffix}");
            web::assert_track(component, &name("in"), "opacity", &pop_in_opacity());
            web::assert_track(component, &name("in"), "sx", &pop_in_scale());
            web::assert_slide(component, &name("in"), &pop_in_slide());
            web::assert_track(component, &name("out"), "opacity", &pop_out_opacity());
            web::assert_track(component, &name("out"), "sx", &pop_out_scale());
            assert_number_track(component, &name("fade-in"), "opacity", &fade_in());
            assert_number_track(component, &name("fade-out"), "opacity", &fade_out());
        }
        // The exit's `translate` is `calc(var(..) / 2)`: half the slide.
        assert_eq!(pop_out_slide().sample(1.0), 0.5);
    }

    #[test]
    fn hover_card_matches_the_web_keyframes() {
        let component = "pop-hover-card";
        web::assert_track(
            component,
            "kk-pop-hover-card-in",
            "opacity",
            &hover_card_in_opacity(),
        );
        web::assert_track(
            component,
            "kk-pop-hover-card-in",
            "sx",
            &hover_card_in_scale(),
        );
        web::assert_slide(component, "kk-pop-hover-card-in", &hover_card_in_slide());
        // `rotate: var(--kk-pop-hover-card-swing, 6deg)` and its `calc(..)` multiples.
        web::assert_track(
            component,
            "kk-pop-hover-card-in",
            "rotate",
            &hover_card_in_swing(),
        );
        web::assert_track(
            component,
            "kk-pop-hover-card-out",
            "opacity",
            &pop_out_opacity(),
        );
        web::assert_track(component, "kk-pop-hover-card-out", "sx", &pop_out_scale());
        assert_number_track(
            component,
            "kk-pop-hover-card-fade-in",
            "opacity",
            &fade_in(),
        );
        assert_number_track(
            component,
            "kk-pop-hover-card-fade-out",
            "opacity",
            &fade_out(),
        );
    }

    #[test]
    fn tooltip_matches_the_web_keyframes() {
        let component = "pop-tooltip";
        web::assert_track(component, "kk-pop-tooltip-in", "opacity", &pop_in_opacity());
        web::assert_track(component, "kk-pop-tooltip-in", "sx", &tooltip_in_scale());
        web::assert_slide(component, "kk-pop-tooltip-in", &tooltip_in_slide());
        assert_number_track(component, "kk-pop-tooltip-out", "opacity", &fade_out());
        assert_number_track(component, "kk-pop-tooltip-fade-in", "opacity", &fade_in());
    }

    #[test]
    fn parity_resolves_fallbacks_and_products() {
        let css = "a { rotate: calc(var(--swing, 6deg) * -0.32); translate: calc(var(--x) / 2) 0; \
                   b: var(--ease, cubic-bezier(0, 0, 1, 1)); }";
        let resolved = web::resolve(css);
        assert!(resolved.contains("rotate: -1.92deg;"), "{resolved}");
        assert!(resolved.contains("calc(var(--x) / 2)"), "{resolved}");
        assert!(
            resolved.contains("var(--ease, cubic-bezier(0, 0, 1, 1))"),
            "{resolved}"
        );
    }

    #[test]
    fn the_hover_card_swings_about_its_pin() {
        let swing = hover_card_in_swing();
        assert_eq!(swing.sample(0.0), 6.0);
        assert!((swing.sample(0.4) + 1.92).abs() < 1e-5);
        assert!((swing.sample(0.7) - 0.96).abs() < 1e-5);
        assert_eq!(swing.sample(1.0), 0.0);
        let card = Surface::hover_card();
        let start = card.pose(Phase::Opening(Duration::ZERO), Side::Bottom, false, px(16.));
        assert_eq!(start.rotate, 6.0);
        // No swing under reduced motion, and none on the way out.
        let reduced = card.pose(Phase::Opening(at(100)), Side::Bottom, true, px(16.));
        assert_eq!(reduced.rotate, 0.0);
        let out = card.pose(Phase::Closing(at(60)), Side::Bottom, false, px(16.));
        assert_eq!(out.rotate, 0.0);
    }

    #[test]
    fn sway_moves_the_centre_as_the_turn_would() {
        let size = size(px(200.), px(100.));
        // Hung from its top centre and turned 90°: the centre swings from below the pin to its
        // left (positive degrees turn clockwise on screen, so the bottom goes left).
        let (dx, dy) = sway(&Pose::new().rotate(90.0), (0.5, 0.0), size);
        assert!(
            (dx + 50.0).abs() < 1e-3 && (dy + 50.0).abs() < 1e-3,
            "{dx} {dy}"
        );
        // Turning about the centre doesn't move it; neither does no turn.
        assert_eq!(
            sway(&Pose::new().rotate(30.0), (0.5, 0.5), size),
            (0.0, 0.0)
        );
        assert_eq!(sway(&Pose::new(), (0.5, 0.0), size), (0.0, 0.0));
    }

    fn at(ms_: u64) -> Duration {
        ms(ms_)
    }

    #[test]
    fn presence_runs_through_its_phases() {
        let t0 = Instant::now();
        let mut presence = Presence::default();
        assert_eq!(presence.phase(t0, at(300), at(120)), Phase::Closed);
        presence.set(true, t0);
        assert_eq!(
            presence.phase(t0 + at(100), at(300), at(120)),
            Phase::Opening(at(100))
        );
        assert_eq!(presence.phase(t0 + at(300), at(300), at(120)), Phase::Open);
        // The same state doesn't restart the clock.
        presence.set(true, t0 + at(400));
        assert_eq!(presence.phase(t0 + at(400), at(300), at(120)), Phase::Open);
        presence.set(false, t0 + at(500));
        assert_eq!(
            presence.phase(t0 + at(560), at(300), at(120)),
            Phase::Closing(at(60))
        );
        assert_eq!(
            presence.phase(t0 + at(620), at(300), at(120)),
            Phase::Closed
        );
        presence.restart(t0 + at(700));
        assert_eq!(
            presence.phase(t0 + at(700), at(300), at(120)),
            Phase::Opening(Duration::ZERO)
        );
    }

    #[test]
    fn pop_hits_its_keyframes() {
        let pop = Surface::pop();
        let rem = px(16.);
        let start = pop.pose(Phase::Opening(Duration::ZERO), Side::Bottom, false, rem);
        assert_eq!(start.opacity, 0.0);
        assert!((start.sx - 0.7).abs() < 1e-5);
        // Opens below the trigger, so it starts half a rem higher.
        assert!((start.y + 8.0).abs() < 1e-4);
        let overshoot = pop.pose(Phase::Opening(at(165)), Side::Bottom, false, rem);
        assert!((overshoot.sx - 1.04).abs() < 1e-4);
        assert!(overshoot.y.abs() < 1e-4);
        assert_eq!(overshoot.opacity, 1.0);
        let settle = pop.pose(Phase::Opening(at(240)), Side::Top, false, rem);
        assert!((settle.sx - 0.985).abs() < 1e-4);
        assert_eq!(pop.pose(Phase::Open, Side::Top, false, rem), Pose::new());
    }

    #[test]
    fn exits_shrink_back_towards_the_trigger() {
        let pop = Surface::pop();
        let end = pop.pose(Phase::Closing(at(120)), Side::Right, false, px(16.));
        assert!(end.opacity.abs() < 1e-5);
        assert!((end.sx - 0.9).abs() < 1e-5);
        // Opened to the right, so it slides back left by a quarter rem.
        assert!((end.x + 4.0).abs() < 1e-4);
        let tooltip = Surface::tooltip();
        let fading = tooltip.pose(Phase::Closing(at(50)), Side::Top, false, px(16.));
        assert_eq!((fading.sx, fading.x, fading.y), (1.0, 0.0, 0.0));
        assert!(fading.opacity > 0.0 && fading.opacity < 1.0);
    }

    #[test]
    fn reduced_motion_only_fades() {
        let pop = Surface::pop();
        assert_eq!(pop.durations(true), (ms(150), ms(100)));
        let pose = pop.pose(Phase::Opening(at(75)), Side::Bottom, true, px(16.));
        assert_eq!((pose.sx, pose.x, pose.y), (1.0, 0.0, 0.0));
        assert!(pose.opacity > 0.0 && pose.opacity < 1.0);
    }

    #[test]
    fn sides_are_measured_from_painted_bounds() {
        let trigger = Bounds::new(point(px(100.), px(100.)), size(px(80.), px(30.)));
        let below = Bounds::new(point(px(100.), px(134.)), size(px(200.), px(100.)));
        let above = Bounds::new(point(px(100.), px(0.)), size(px(200.), px(96.)));
        let right = Bounds::new(point(px(190.), px(100.)), size(px(100.), px(40.)));
        assert_eq!(Side::measure(trigger, below), Some(Side::Bottom));
        assert_eq!(Side::measure(trigger, above), Some(Side::Top));
        assert_eq!(Side::measure(trigger, right), Some(Side::Right));
        assert_eq!(Side::measure(trigger, trigger), None);
        assert_eq!(Side::of_anchor(Anchor::BottomRight), Side::Top);
        assert_eq!(Side::Bottom.origin(0.5), (0.5, 0.0));
        assert_eq!(Side::Left.origin(0.0), (1.0, 0.0));
    }

    /// A click target with a button over its corner, the button `Inert` or not.
    struct Stack {
        inert: bool,
        under: usize,
        over: usize,
    }

    impl gpui_kit::Render for Stack {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            use gpui_kit::{InteractiveElement as _, StatefulInteractiveElement as _, Styled as _};
            let over = div()
                .id("over")
                .size(px(100.))
                .occlude()
                .on_click(cx.listener(|this, _, _, _| this.over += 1));
            div()
                .id("under")
                .size(px(400.))
                .on_click(cx.listener(|this, _, _, _| this.under += 1))
                .child(if self.inert {
                    Inert::new(over).into_any_element()
                } else {
                    over.into_any_element()
                })
        }
    }

    #[gpui_kit::test]
    fn inert_content_lets_the_pointer_through(cx: &mut gpui_kit::TestAppContext) {
        for inert in [false, true] {
            let (view, cx) = cx.add_window_view(|_, _| Stack {
                inert,
                under: 0,
                over: 0,
            });
            cx.run_until_parked();
            cx.simulate_click(point(px(50.), px(50.)), gpui_kit::Modifiers::none());
            let (under, over) = view.read_with(cx, |stack, _| (stack.under, stack.over));
            if inert {
                assert_eq!((under, over), (1, 0), "the inert button took the click");
            } else {
                assert_eq!(
                    (under, over),
                    (0, 1),
                    "the live button should take the click"
                );
            }
        }
    }
}
