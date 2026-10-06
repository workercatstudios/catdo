//! Hop: throws its child in along an arc, to land with a squash and settle.
//!
//! A thrown arc, one track per axis. Across runs linear, like a thrown ball, and fades in over the
//! first 8 %; up and down runs an ease-out to the peak at 35 % and an ease-in back to the ground,
//! so it hangs at the top. The squash turns on the feet: stretched in the air, squashed flat on
//! landing, then rebounds that halve. It lasts 2.5 times the flight, so the landing falls at 40 %
//! of it.
//!
//! Composition-safe: it runs on a [`Clock`], so inside a [`Timeline`](crate::timeline::Timeline)
//! it plays and seeks on the timeline's clock. Under reduced motion the child simply stands there.
//!
//! Differences from the web version: GPUI can't transform an arbitrary subtree, so the squash
//! reaches children two ways. A child that paints itself, like
//! [`IdleCat`](crate::idle_cat::IdleCat), takes the squash ([`current_squash`]) and applies it
//! exactly, unevenly, from the hop's feet. Any other child is scaled evenly by its rem size (see
//! [`motion::Transform`](crate::motion::Transform)), following the squash's vertical scale as Bounce
//! Text's letters do; rem-sized children squash, pixel sizes don't. Distances take any length, so
//! a hop on a [`Stage`](crate::timeline::Stage) can be given in rems and scale with it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui_kit::base::StyledExt as _;
use gpui_kit::{
    AbsoluteLength, AnyElement, App, Bounds, Element, ElementId, GlobalElementId,
    InspectorElementId, InteractiveElement as _, IntoElement, LayoutId, ParentElement, Pixels,
    RenderOnce, StyleRefinement, Styled, Window, div, px,
};

use crate::motion::{
    Clock, Easing, Keyframes, Pose, Timing, Track, Trigger, child_id, delay_ms, ms, transform,
};

/// The hop's tracks: across, the fade and up over the flight, and the squash, which runs 2.5
/// times as long.
pub struct HopTracks {
    /// Offset left as a fraction of the start offset.
    pub across: Keyframes<f32>,
    pub fade: Keyframes<f32>,
    /// Vertical offset in px.
    pub up: Keyframes<f32>,
    /// `(sx, sy)` from the feet.
    pub squash: Keyframes<Pose>,
}

/// The tracks for a hop that starts `from_y` px below its landing spot and peaks `height` px
/// above it.
pub fn tracks(from_y: f32, height: f32) -> HopTracks {
    let rise = Easing::cubic_bezier(0.33, 1.0, 0.68, 1.0).expect("valid curve");
    let fall = Easing::cubic_bezier(0.32, 0.0, 0.67, 0.0).expect("valid curve");
    HopTracks {
        across: Track::new(Easing::Linear).at(0.0, 1.0).at(1.0, 0.0).build(),
        fade: Track::new(Easing::Linear)
            .at(0.0, 0.0)
            .at(0.08, 1.0)
            .at(1.0, 1.0)
            .build(),
        up: Track::new(Easing::Linear)
            .at_ease(0.0, from_y, rise)
            .at_ease(0.35, -height, fall)
            .at(1.0, 0.0)
            .build(),
        squash: squash_track(),
    }
}

/// `@keyframes kk-hop-squash`: stretched in the air, squashed flat on landing, then rebounds that
/// halve.
pub fn squash_track() -> Keyframes<Pose> {
    let squash = |sx, sy| Pose::new().scale_xy(sx, sy);
    Track::new(Easing::EaseInOut)
        .at(0.0, squash(0.88, 1.16))
        .at(0.14, Pose::new())
        .at(0.34, squash(0.9, 1.14))
        .at(0.4, squash(1.3, 0.74))
        .at(0.54, squash(0.9, 1.12))
        .at(0.68, squash(1.06, 0.95))
        .at(0.82, squash(0.98, 1.02))
        .at(1.0, Pose::new())
        .build()
}

/// Throws its children in along an arc, to land with a squash and settle. For characters and
/// props.
///
/// ```ignore
/// Hop::new("cat").from(rems(-43.75), rems(-2.5)).height(rems(17.5)).delay(200).child(cat)
/// ```
#[derive(IntoElement)]
pub struct Hop {
    id: ElementId,
    from: (AbsoluteLength, AbsoluteLength),
    height: AbsoluteLength,
    delay: u64,
    duration: u64,
    trigger: Trigger,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl Hop {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            from: (px(-480.0).into(), px(0.0).into()),
            height: px(260.0).into(),
            delay: 0,
            duration: 500,
            trigger: Trigger::Mount,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    /// Where the hop starts, from where it lands. Negative x comes from the left. `(-480px, 0)` by
    /// default.
    pub fn from(mut self, x: impl Into<AbsoluteLength>, y: impl Into<AbsoluteLength>) -> Self {
        self.from = (x.into(), y.into());
        self
    }

    /// Height of the arc's peak above the landing spot. 260px by default.
    pub fn height(mut self, height: impl Into<AbsoluteLength>) -> Self {
        self.height = height.into();
        self
    }

    /// Wait before take-off, in ms.
    pub fn delay(mut self, delay_ms: u64) -> Self {
        self.delay = delay_ms;
        self
    }

    /// Time in the air, in ms. The landing squash takes as long again and half more. 500 by
    /// default.
    pub fn duration(mut self, duration_ms: u64) -> Self {
        self.duration = duration_ms;
        self
    }

    /// When it takes off. [`Trigger::Mount`] by default.
    pub fn trigger(mut self, trigger: Trigger) -> Self {
        self.trigger = trigger;
        self
    }
}

impl Styled for Hop {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Hop {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Hop {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let clock = Clock::new(self.id.clone(), self.trigger, window, cx);
        let squash_ms = self.duration * 5 / 2;
        clock.animate(Some(ms(self.delay + squash_ms)), window);
        let elapsed = clock.elapsed();
        let flight = Timing::new(ms(self.duration))
            .delay(delay_ms(self.delay))
            .sample(elapsed)
            .directed_progress;
        let landing = Timing::new(ms(squash_ms))
            .delay(delay_ms(self.delay))
            .sample(elapsed)
            .directed_progress;
        let rem = window.rem_size();
        let from_x = f32::from(self.from.0.to_pixels(rem));
        let from_y = f32::from(self.from.1.to_pixels(rem));
        let height = f32::from(self.height.to_pixels(rem));
        let tracks = tracks(from_y, height);
        let squash = tracks.squash.sample(landing);

        // Children that paint themselves took last frame's squash and apply it exactly; then
        // the subtree isn't rescaled as well.
        let painted = window.use_keyed_state((self.id.clone(), "kk-hop-painted"), cx, |_, _| {
            Rc::new(Cell::new(false))
        });
        let taken = painted.read(cx).clone();
        let self_squashed = taken.replace(false);
        let pose = Pose::new()
            .at(
                from_x * tracks.across.sample(flight),
                tracks.up.sample(flight),
            )
            .opacity(tracks.fade.sample(flight))
            // Even scaling only: follow the vertical squash, which carries the bounce.
            .scale(if self_squashed { 1.0 } else { squash.sy });

        // The transform measures its child at rest; a new rem size (a stage that just measured
        // itself, a resize) needs a new measurement, so it gets a new id.
        let rem_key = (f32::from(rem) * 100.0).round() as usize;
        let children = Publish {
            squash: HopSquash {
                pose: squash,
                taken,
            },
            child: div().children(self.children).into_any_element(),
        };
        clock.observe(
            div()
                .id(self.id.clone())
                .refine_style(&self.style)
                .child(transform(child_id(&self.id, rem_key), pose, children).origin(0.5, 1.0)),
        )
    }
}

/// The squash a [`Hop`] is applying, published while its children are laid out.
///
/// A child that paints itself (like [`IdleCat`](crate::idle_cat::IdleCat)) can [`take`] it and
/// squash exactly from the hop's feet, the bottom centre of its own box; the hop then stops
/// scaling the subtree evenly from the next frame on.
///
/// [`take`]: HopSquash::take
#[derive(Clone)]
pub struct HopSquash {
    pose: Pose,
    taken: Rc<Cell<bool>>,
}

impl HopSquash {
    /// The squash (`sx`, `sy`), to apply around the bottom centre. Taking it tells the hop not to
    /// scale its children as well.
    pub fn take(&self) -> Pose {
        self.taken.set(true);
        self.pose
    }
}

thread_local! {
    static SQUASH: RefCell<Vec<HopSquash>> = const { RefCell::new(Vec::new()) };
}

/// The squash of the innermost [`Hop`] being laid out, if any.
pub fn current_squash() -> Option<HopSquash> {
    SQUASH.with(|squash| squash.borrow().last().cloned())
}

/// Lays out `child` with a squash published, like [`motion::scoped`](crate::motion::scoped)
/// publishes time.
struct Publish {
    squash: HopSquash,
    child: AnyElement,
}

impl IntoElement for Publish {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Publish {
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
    ) -> (LayoutId, ()) {
        struct Pop;
        impl Drop for Pop {
            fn drop(&mut self) {
                SQUASH.with(|squash| squash.borrow_mut().pop());
            }
        }
        SQUASH.with(|squash| squash.borrow_mut().push(self.squash.clone()));
        let _pop = Pop;
        (self.child.request_layout(window, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut (),
        _: &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        self.child.paint(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parity::assert_pose_track;

    // Not checked against the web: `kk-hop-x` and `kk-hop-y` translate by `var(...)` distances,
    // which the parity parser doesn't read. `the_arc_peaks_at_35_percent` checks their values.
    #[test]
    fn squash_matches_the_web() {
        assert_pose_track("hop", "kk-hop-squash", &squash_track(), &[]);
    }

    #[test]
    fn the_arc_peaks_at_35_percent() {
        let t = tracks(-40.0, 260.0);
        assert_eq!(t.up.sample(0.0), -40.0);
        assert_eq!(t.up.sample(0.35), -260.0);
        assert_eq!(t.up.sample(1.0), 0.0);
        // Ease-out on the way up: past halfway a quarter of the way to the peak.
        assert!(t.up.sample(0.0875) < -150.0);
        // Across is linear and the fade is done by 8 %.
        assert!((t.across.sample(0.5) - 0.5).abs() < 1e-5);
        assert_eq!(t.fade.sample(0.08), 1.0);
    }

    #[test]
    fn it_lands_squashed() {
        let t = tracks(0.0, 260.0);
        let landing = t.squash.sample(0.4);
        assert_eq!((landing.sx, landing.sy), (1.3, 0.74));
        assert_eq!(t.squash.sample(1.0), Pose::new());
    }
}
