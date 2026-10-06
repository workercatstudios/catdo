//! Carousel: slides that land with a small overshoot, and arrows that squash.
//!
//! Replaces `gpui_kit::component::carousel`. GPUI Component's carousel moves the strip (snap
//! springs, drag, wheel, keys, looping); this module wraps its parts and adds the motion of
//! Kirakira's Pop Carousel. [`Carousel`], [`CarouselItem`], [`CarouselContent`],
//! [`CarouselPrevious`] and [`CarouselNext`] take the same builders; `CarouselState`, the
//! pagination parts and the rest of the module are GPUI Component's own, re-exported.
//!
//! - A slide out of view waits at 0.92. The moment it starts to come in, it grows past full size
//!   and settles, `0.92 → 1.03 → 0.99 → 1` in 0.5 s (ease-out into the first keyframe, then
//!   ease-in-out), so the overshoot falls as the strip comes to rest. Slides showing when the
//!   carousel first appears don't land.
//! - The arrows squash to `scale: 1.15 0.85` while held (0.08 s, ease-out) and spring back
//!   (0.35 s), and the arrow inside nudges 0.15 em the way it points on hover (0.16 s, ease-out).
//!
//! Under reduced motion slides don't scale and arrows don't squash or nudge.
//!
//! Differences from the web version: a slide is any content, so it scales with
//! [`motion::transform`](crate::motion::transform): evenly, by its rem size. Content sized in
//! rems, relative units or text follows exactly; fixed pixel sizes inside a slide don't scale. The
//! arrows are GPUI Component's outline buttons whose box squashes exactly; the arrow glyph scales
//! evenly. Labels are English, where GPUI Component translates them.
//!
//! Focus works as in GPUI Component: a pointer click on an arrow moves focus to the carousel, so
//! the arrow keys go on working, without drawing its focus ring, which comes back once focus
//! leaves; an arrow pressed from the keyboard keeps focus. GPUI Component's switch for the ring is
//! private, so [`Carousel`] wraps its root and turns the root's ring off through
//! [`FocusableExt::focus_ring`]: the arrows need this module's `Carousel` for the ring to stay
//! hidden. Under GPUI Component's own root they still move focus, and the ring shows.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use gpui_kit::base::{ElementExt as _, StyledExt as _};
use gpui_kit::canvas;
use gpui_kit::component::button::Button;
use gpui_kit::component::carousel::{
    Carousel as UiCarousel, CarouselContent as UiContent, CarouselItem as UiItem,
};
use gpui_kit::component::{
    Disableable as _, FocusableExt, Icon, IconName, Sizable, Size, ThemeStyled as _,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Axis, ElementId, Entity, EntityId, Focusable as _, InteractiveElement as _,
    IntoElement, MouseButton, ParentElement, Pixels, RenderOnce, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Subscription, Window, div, rems,
};

pub use gpui_kit::component::carousel::*;

use crate::button::Pressed;
use crate::motion::{Easing, Keyframes, Pose, Timing, Track, ms, now, split_layout, transform};
use crate::squash::{Squash, squashed};
use crate::state_motion::glide;

/// The scale of a slide waiting out of view.
const WAITING: f32 = 0.92;
const LAND: Duration = ms(500);
const SQUASH: Squash = Squash::new(1.15, 0.85);

/// A slide's landing: `0.92 → 1.03 → 0.99 → 1`.
fn land_track() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, WAITING, Easing::EaseOut)
        .at(0.55, 1.03)
        .at(0.8, 0.99)
        .at(1.0, 1.0)
        .build()
}

thread_local! {
    /// Each carousel's frame (the content's size), recorded by [`CarouselContent`] so the arrows
    /// can sit around it.
    static FRAMES: RefCell<HashMap<EntityId, gpui_kit::Size<Pixels>>> = RefCell::new(HashMap::new());
}

fn frame_size(state: &Entity<CarouselState>) -> Option<gpui_kit::Size<Pixels>> {
    FRAMES.with(|frames| frames.borrow().get(&state.entity_id()).copied())
}

thread_local! {
    /// Carousels focused by a pointer click on an arrow: their ring stays off until focus leaves.
    static QUIET: RefCell<HashSet<EntityId>> = RefCell::new(HashSet::new());
}

fn is_quiet(state: EntityId) -> bool {
    QUIET.with(|quiet| quiet.borrow().contains(&state))
}

fn set_quiet(state: EntityId, quiet: bool) {
    QUIET.with(|set| {
        let mut set = set.borrow_mut();
        if quiet {
            set.insert(state);
        } else {
            set.remove(&state);
        }
    });
}

/// After a pointer click on an arrow, moves keyboard focus to the carousel so the arrow keys keep
/// working, with its ring off. A keyboard press leaves focus on the arrow, and focus already in
/// the carousel stays where it is.
fn focus_after_pointer_click(
    state: &Entity<CarouselState>,
    event: &gpui_kit::ClickEvent,
    window: &mut Window,
    cx: &mut App,
) {
    let focus = state.read(cx).focus_handle(cx);
    if event.is_keyboard() || focus.contains_focused(window, cx) {
        return;
    }
    set_quiet(state.entity_id(), true);
    window.focus(&focus, cx);
}

/// A composable carousel root: GPUI Component's, with its ring kept off while Kirakira's arrows
/// have given it focus.
///
/// Add one [`CarouselContent`] and any optional controls as children. Every part must share the
/// same [`CarouselState`].
#[derive(IntoElement)]
pub struct Carousel {
    state: Entity<CarouselState>,
    inner: UiCarousel,
    focus_ring: bool,
}

impl Carousel {
    /// Creates a Carousel bound to `state`.
    pub fn new(id: impl Into<ElementId>, state: &Entity<CarouselState>) -> Self {
        Self {
            state: state.clone(),
            inner: UiCarousel::new(id, state),
            focus_ring: true,
        }
    }

    /// Sets the name announced for the carousel region.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.inner = self.inner.accessibility_label(label);
        self
    }
}

impl ParentElement for Carousel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.inner.extend(elements);
    }
}

impl Styled for Carousel {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}

impl FocusableExt for Carousel {
    fn focus_ring(mut self, enabled: bool) -> Self {
        self.focus_ring = enabled;
        self
    }

    fn is_focus_ring_enabled(&self) -> bool {
        self.focus_ring
    }
}

/// Brings the ring back once focus leaves the carousel.
struct FocusOut {
    _subscription: Subscription,
}

impl RenderOnce for Carousel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.state.entity_id();
        let focus = self.state.read(cx).focus_handle(cx);
        window.use_keyed_state(("kk-carousel-focus-out", id), cx, |window, cx| FocusOut {
            _subscription: window.on_focus_out(&focus, cx, move |_, _, _| set_quiet(id, false)),
        });
        self.inner.focus_ring(self.focus_ring && !is_quiet(id))
    }
}

/// Whether a slide is in its carousel's window: more than 1 % of it shows.
fn in_view(slide: gpui_kit::Bounds<Pixels>, window: gpui_kit::Bounds<Pixels>) -> bool {
    let area = f32::from(slide.size.width) * f32::from(slide.size.height);
    if area <= 0.0 {
        return false;
    }
    let shown = slide.intersect(&window);
    let shown = f32::from(shown.size.width).max(0.0) * f32::from(shown.size.height).max(0.0);
    shown / area >= 0.01
}

/// The clipped viewport and snap track for Carousel items.
#[derive(IntoElement)]
pub struct CarouselContent {
    state: Entity<CarouselState>,
    inner: UiContent,
}

impl CarouselContent {
    /// Creates content bound to `state`.
    pub fn new(state: &Entity<CarouselState>) -> Self {
        Self {
            state: state.clone(),
            inner: UiContent::new(state),
        }
    }

    /// Sets style overrides for the inner flex track.
    pub fn track_style(mut self, style: StyleRefinement) -> Self {
        self.inner = self.inner.track_style(style);
        self
    }
}

impl ParentElement for CarouselContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.inner.extend(elements);
    }
}

impl Styled for CarouselContent {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}

impl RenderOnce for CarouselContent {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let id = self.state.entity_id();
        div()
            .w_full()
            .child(self.inner)
            .on_prepaint(move |bounds, _, _| {
                FRAMES.with(|frames| frames.borrow_mut().insert(id, bounds.size));
            })
    }
}

#[derive(Default)]
struct Landing {
    /// The slide has reported where it starts.
    seen: bool,
    /// Out of view, waiting at 0.92.
    waiting: bool,
    /// When it came into view after waiting.
    landed_at: Option<Instant>,
}

/// One logical slide in a [`CarouselContent`]. Lands as it comes into view.
#[derive(IntoElement)]
pub struct CarouselItem {
    id: ElementId,
    inner: UiItem,
    children: Vec<AnyElement>,
}

impl CarouselItem {
    /// Creates the item at the zero-based `index` used by `state`.
    pub fn new(id: impl Into<ElementId>, index: usize, state: &Entity<CarouselState>) -> Self {
        let id = id.into();
        Self {
            inner: UiItem::new(id.clone(), index, state),
            id,
            children: Vec::new(),
        }
    }

    /// Replaces the generated "Slide N of M" accessibility label.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.inner = self.inner.accessibility_label(label);
        self
    }
}

impl ParentElement for CarouselItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for CarouselItem {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}

impl RenderOnce for CarouselItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let landing = window.use_keyed_state((self.id.clone(), "kk-landing"), cx, |_, _| {
            Landing::default()
        });
        let now = now();
        let scale = {
            let landing = landing.read(cx);
            match (landing.waiting, landing.landed_at) {
                _ if cx.reduce_motion() => 1.0,
                (true, _) => WAITING,
                (false, Some(at)) if now.saturating_duration_since(at) < LAND => {
                    window.request_animation_frame();
                    let elapsed = now.saturating_duration_since(at);
                    land_track().sample(Timing::new(LAND).sample(elapsed).directed_progress)
                }
                _ => 1.0,
            }
        };

        let content = div().size_full().children(self.children);
        self.inner
            .child(transform(
                (self.id.clone(), "kk-land"),
                Pose::new().scale(scale),
                content,
            ))
            // Watch the slide against the carousel's window (the content clips it). Pinned to the
            // top left: an absolute child without insets would sit below the content.
            .child(
                canvas(
                    move |bounds, window, cx| {
                        let visible = in_view(bounds, window.content_mask().bounds);
                        landing.update(cx, |landing, cx| {
                            if !landing.seen {
                                landing.seen = true;
                                landing.waiting = !visible;
                                if landing.waiting {
                                    cx.notify();
                                }
                            } else if !visible && !landing.waiting {
                                landing.waiting = true;
                                landing.landed_at = None;
                                cx.notify();
                            } else if visible && landing.waiting {
                                landing.waiting = false;
                                landing.landed_at = Some(now);
                                cx.notify();
                            }
                        })
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
    }
}

/// A previous-slide control positioned around the Carousel viewport.
#[derive(IntoElement)]
pub struct CarouselPrevious {
    control: Control,
}

/// A next-slide control positioned around the Carousel viewport.
#[derive(IntoElement)]
pub struct CarouselNext {
    control: Control,
}

struct Control {
    state: Entity<CarouselState>,
    size: Size,
    style: StyleRefinement,
    accessibility_label: Option<SharedString>,
    children: Vec<AnyElement>,
}

impl Control {
    fn new(state: &Entity<CarouselState>) -> Self {
        Self {
            state: state.clone(),
            size: Size::Medium,
            style: StyleRefinement::default(),
            accessibility_label: None,
            children: Vec::new(),
        }
    }
}

macro_rules! control {
    ($name:ident, $next:expr, $doc:literal) => {
        impl $name {
            #[doc = $doc]
            pub fn new(state: &Entity<CarouselState>) -> Self {
                Self {
                    control: Control::new(state),
                }
            }

            /// Replaces the generated accessibility label and tooltip.
            pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
                self.control.accessibility_label = Some(label.into());
                self
            }
        }

        impl Sizable for $name {
            fn with_size(mut self, size: impl Into<Size>) -> Self {
                self.control.size = size.into();
                self
            }
        }

        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.control.style
            }
        }

        impl ParentElement for $name {
            fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
                self.control.children.extend(elements);
            }
        }

        impl RenderOnce for $name {
            fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
                carousel_control(self.control, $next, window, cx)
            }
        }
    };
}

control!(
    CarouselPrevious,
    false,
    "Creates a previous-slide control bound to `state`."
);
control!(
    CarouselNext,
    true,
    "Creates a next-slide control bound to `state`."
);

struct Hovered(bool);

fn carousel_control(
    control: Control,
    next: bool,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let Control {
        state,
        size,
        mut style,
        accessibility_label,
        children,
    } = control;
    let snapshot = state.read(cx);
    let axis = snapshot.axis();
    let disabled = if next {
        !snapshot.has_next()
    } else {
        !snapshot.has_previous()
    };
    let frame = frame_size(&state);
    let (name, default_label, icon) = match (axis, next) {
        (Axis::Horizontal, false) => ("previous", "Previous slide", IconName::ChevronLeft),
        (Axis::Horizontal, true) => ("next", "Next slide", IconName::ChevronRight),
        (Axis::Vertical, false) => ("previous", "Previous slide", IconName::ChevronUp),
        (Axis::Vertical, true) => ("next", "Next slide", IconName::ChevronDown),
    };
    let label: SharedString = accessibility_label.unwrap_or_else(|| default_label.into());
    let has_custom_content = !children.is_empty();
    let id: ElementId = (
        ElementId::from(("kk-carousel-control", state.entity_id())),
        SharedString::from(name),
    )
        .into();

    let pressed = window.use_keyed_state((id.clone(), "kk-pressed"), cx, |_, _| Pressed(false));
    let hovered = window.use_keyed_state((id.clone(), "kk-hovered"), cx, |_, _| Hovered(false));
    let held = !disabled && pressed.read(cx).0;
    let (sx, sy) = SQUASH.scale(&id, held, window, cx);
    let is_hovered = !disabled && hovered.read(cx).0;
    let nudge = glide(
        (id.clone(), "kk-nudge"),
        if is_hovered { 0.15_f32 } else { 0.0 },
        ms(160),
        Easing::EaseOut,
        window,
        cx,
    );
    let nudge = if cx.reduce_motion() { 0.0 } else { nudge };
    let em = rems(0.875).to_pixels(window.rem_size());
    let shift = em * if next { nudge } else { -nudge };
    let cell = match size {
        Size::XSmall => rems(1.25),
        Size::Small => rems(1.5),
        _ => rems(2.0),
    };

    let (down, up, up_out) = (pressed.clone(), pressed.clone(), pressed);
    let outer = split_layout(&mut style);
    let content = if has_custom_content {
        div().flex().children(children)
    } else {
        div().flex().child(Icon::new(icon))
    };
    let button = Button::new(id.clone())
        .outline()
        .with_size(size)
        .accessibility_label(label.clone())
        .tooltip(label)
        .disabled(disabled)
        .rounded_full_style(cx)
        .p_0()
        .size(cell)
        .child(
            div()
                .relative()
                .map(|this| match axis {
                    Axis::Horizontal => this.left(shift),
                    Axis::Vertical => this.top(shift),
                })
                .child(content),
        )
        .when(!disabled, |this| {
            this.on_mouse_down(MouseButton::Left, move |_, _, cx| {
                down.update(cx, |pressed, cx| {
                    pressed.0 = true;
                    cx.notify();
                })
            })
            .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                up.update(cx, |pressed, cx| {
                    pressed.0 = false;
                    cx.notify();
                })
            })
            .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                up_out.update(cx, |pressed, cx| {
                    pressed.0 = false;
                    cx.notify();
                })
            })
            .on_click(move |event, window, cx| {
                state.update(cx, |state, cx| {
                    if next {
                        state.select_next(cx);
                    } else {
                        state.select_previous(cx);
                    }
                });
                focus_after_pointer_click(&state, event, window, cx);
            })
        })
        .refine_style(&style);

    div()
        .absolute()
        .top_0()
        .left_0()
        .when_some(frame, |this, size| this.w(size.width).h(size.height))
        .when(frame.is_none(), |this| this.right_0().bottom_0())
        .child(
            div()
                .id((id.clone(), "kk-hover"))
                .absolute()
                .size(cell)
                .when(axis == Axis::Horizontal && !next, |this| {
                    this.right_full().mr_4().top_0().bottom_0().my_auto()
                })
                .when(axis == Axis::Horizontal && next, |this| {
                    this.left_full().ml_4().top_0().bottom_0().my_auto()
                })
                .when(axis == Axis::Vertical && !next, |this| {
                    this.bottom_full().mb_4().left_0().right_0().mx_auto()
                })
                .when(axis == Axis::Vertical && next, |this| {
                    this.top_full().mt_4().left_0().right_0().mx_auto()
                })
                .refine_style(&outer)
                .on_hover(move |hovering, _, cx| {
                    hovered.update(cx, |hovered, cx| {
                        hovered.0 = *hovering;
                        cx.notify();
                    })
                })
                .child(squashed(id, (sx, sy), button).text((sx * sy).sqrt())),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{AppContext as _, Bounds, point, px, size};

    #[test]
    fn landing_matches_the_web() {
        crate::parity::assert_number_track(
            "pop-carousel",
            "kk-pop-carousel-land",
            "sx",
            &land_track(),
        );
    }

    #[test]
    fn a_slide_counts_as_in_view_past_one_percent() {
        let window = Bounds::new(point(px(0.), px(0.)), size(px(300.), px(300.)));
        let slide = |x: f32| Bounds::new(point(px(x), px(0.)), size(px(300.), px(300.)));
        assert!(in_view(slide(0.), window));
        assert!(in_view(slide(290.), window));
        // Touching the edge, or a hair over it, isn't in view.
        assert!(!in_view(slide(300.), window));
        assert!(!in_view(slide(299.), window));
    }

    struct Host {
        state: Entity<CarouselState>,
    }

    impl gpui_kit::Render for Host {
        fn render(&mut self, _: &mut Window, _: &mut gpui_kit::Context<Self>) -> impl IntoElement {
            let state = &self.state;
            div().size_full().p_16().child(
                Carousel::new("carousel", state)
                    .w(px(200.))
                    .child(CarouselContent::new(state).children((0..3).map(|ix| {
                        CarouselItem::new(("slide", ix), ix, state).child(div().h(px(100.)))
                    })))
                    .child(CarouselPrevious::new(state))
                    .child(
                        CarouselNext::new(state)
                            .child(div().debug_selector(|| "next".into()).size(px(16.))),
                    ),
            )
        }
    }

    /// A pointer click on an arrow hands focus to the carousel with its ring off, as GPUI
    /// Component's arrows do, and the ring comes back once focus leaves.
    #[gpui_kit::test]
    fn a_clicked_arrow_focuses_the_carousel_quietly(cx: &mut gpui_kit::TestAppContext) {
        cx.update(|cx| {
            gpui_kit::component::init(cx);
            crate::init(cx);
        });
        let (host, cx) = cx.add_window_view(|_, cx| Host {
            state: cx.new(|_| CarouselState::new(3)),
        });
        let state = cx.update(|_, cx| host.read(cx).state.clone());
        let id = state.entity_id();
        // Focus events only reach an active window.
        cx.update(|window, _| window.activate_window());
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let next = cx
            .debug_bounds("next")
            .expect("the arrow is drawn")
            .center();
        cx.simulate_click(next, gpui_kit::Modifiers::none());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let focused = |cx: &mut gpui_kit::VisualTestContext| {
            cx.update(|window, cx| state.read(cx).focus_handle(cx).is_focused(window))
        };
        assert!(focused(cx), "the click didn't focus the carousel");
        assert!(is_quiet(id), "the ring isn't held off");
        assert_eq!(cx.update(|_, cx| state.read(cx).selected_index()), Some(1));

        cx.update(|window, cx| window.blur(cx));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert!(!focused(cx));
        assert!(!is_quiet(id), "the ring didn't come back after focus left");
    }
}
