//! Scroll: a scrollbar thumb that pops out of nothing when it appears.
//!
//! Replaces `gpui_kit::component::scroll`. [`ScrollableElement`] (`overflow_y_scrollbar()` and
//! friends), [`Scrollable`] and [`Scrollbar`] take the same builders as GPUI Component's; the
//! rest of the module (`ScrollbarAxis`, `ScrollbarMode`, `ScrollbarHandle`, the style types...)
//! is re-exported. gpui-base's scrollbar still does the work (hit testing, dragging, track
//! clicks, wheel and visibility); Kirakira paints the thumb over it with the motion of Pop Scroll
//! Area:
//!
//! - The thumb rests as a pill narrower than the track: GPUI Component's 6 px of the 8 px it
//!   takes when hovered or dragged.
//! - When the scrollbar appears (on scroll, on hover, or at mount when it always shows) the thumb
//!   pops out of nothing across the track: `0 → 135 % → 90 %` of its resting width, then rest,
//!   in 0.3 s (ease-out into the first keyframe, then ease-in-out).
//! - Hovering or dragging springs it out to the full 8 px (0.3 s, spring curve) and darkens it
//!   (0.15 s).
//! - Hiding is a quick fade (0.2 s, ease-in).
//!
//! Under reduced motion the scrollbar still appears, fades away, widens and darkens, without the
//! pop or the spring.
//!
//! Differences from the web version: GPUI's thumb is right-anchored and grows leftwards; this one
//! scales about the track's centre line, like the web's `scale`. When to show follows GPUI
//! Component's scrollbar modes (`Scrolling`, `Hover`, `Always`) rather than Radix's: in `Hover`
//! mode the bar shows while the pointer is over the track, not the whole area. Thumb widths come
//! from GPUI Component's defaults; widths set with [`Scrollbar::styles`] aren't readable from
//! gpui-base, so they don't reach the painted thumb (track styles still apply).

use std::panic::Location;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::base::{InteractiveElementExt as _, Scrollbar as BaseScrollbar, StyledExt as _};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Axis, Bounds, Div, Element, ElementId, Hsla, InteractiveElement,
    Interactivity, IntoElement, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Overflow,
    ParentElement, Pixels, PointRefinement, RenderOnce, ScrollHandle, Size, Stateful,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, canvas, div, fill, point, px,
    size,
};

pub use gpui_kit::component::scroll::*;

use crate::motion::{Easing, Keyframes, Pulse, Timing, Track, ms, now};
use crate::state_motion::glide;
use crate::theme::ActiveKira as _;

/// GPUI Component's thumb width at rest, and hovered or dragged.
const REST: Pixels = px(6.);
const FULL: Pixels = px(8.);
const INSET: Pixels = px(4.);
const MIN_LENGTH: Pixels = px(48.);
/// How long GPUI Component keeps a scroll-revealed scrollbar.
const IDLE: Duration = Duration::from_secs(2);
const POP: Duration = ms(300);
const FADE: Duration = ms(200);

/// The thumb's pop as a fraction of its resting width: `0 → 1.35 → 0.9 → 1`.
fn pop_track() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.0, Easing::EaseOut)
        .at(0.55, 1.35)
        .at(0.8, 0.9)
        .at(1.0, 1.0)
        .build()
}

/// The hide: `to { opacity: 0 }`, ease-in.
fn fade_track() -> Keyframes<f32> {
    Track::new(Easing::EaseIn).at(0.0, 1.0).at(1.0, 0.0).build()
}

/// Where the thumb sits along a track: its start (from the track's origin) and length, like
/// gpui-base's geometry. `None` when the content fits.
fn thumb_span(
    container: Pixels,
    content: Pixels,
    margin_end: Pixels,
    offset: Pixels,
) -> Option<(Pixels, Pixels)> {
    if content <= container {
        return None;
    }
    let track = (container - margin_end).max(px(0.));
    let logical = (container / content * container).max(MIN_LENGTH).min(track);
    let inset = INSET.clamp(px(0.), logical / 2.);
    let travel = track - logical;
    let extent = content - container;
    let start = inset + (-offset / extent).clamp(0., 1.) * travel;
    Some((start, logical - inset * 2.))
}

/// The motion state of one scrollbar.
struct Motion {
    last_offset: Option<gpui_kit::Point<Pixels>>,
    last_scroll: Option<Instant>,
    hovered: bool,
    dragging: bool,
    visible: bool,
    hidden_at: Option<Instant>,
    /// A timer is waiting for the scroll-revealed hold to run out.
    idle_timer: bool,
}

/// What the painter needs this frame.
#[derive(Clone, Copy)]
struct Paint {
    width: Pixels,
    opacity: f32,
    color: Hsla,
    radius: Pixels,
    visible: bool,
}

/// Scrollbar control for scroll-area or a uniform-list, with Kirakira's thumb.
#[derive(IntoElement)]
pub struct Scrollbar {
    id: ElementId,
    base: BaseScrollbar,
    handle: Rc<dyn ScrollbarHandle>,
    axis: ScrollbarAxis,
    mode: Option<ScrollbarMode>,
    scroll_size: Option<Size<Pixels>>,
    viewport_bounds: Option<Bounds<Pixels>>,
    use_layout_bounds: bool,
}

impl Scrollbar {
    /// Create a new scrollbar.
    ///
    /// This will have both vertical and horizontal scrollbars.
    #[track_caller]
    pub fn new<H: ScrollbarHandle + Clone>(scroll_handle: &H) -> Self {
        let id = ElementId::CodeLocation(*Location::caller());
        Self {
            base: BaseScrollbar::new(scroll_handle).id(id.clone()),
            id,
            handle: Rc::new(scroll_handle.clone()),
            axis: ScrollbarAxis::Both,
            mode: None,
            scroll_size: None,
            viewport_bounds: None,
            use_layout_bounds: false,
        }
    }

    /// Create with horizontal scrollbar.
    #[track_caller]
    pub fn horizontal<H: ScrollbarHandle + Clone>(scroll_handle: &H) -> Self {
        Self::new(scroll_handle).axis(ScrollbarAxis::Horizontal)
    }

    /// Create with vertical scrollbar.
    #[track_caller]
    pub fn vertical<H: ScrollbarHandle + Clone>(scroll_handle: &H) -> Self {
        Self::new(scroll_handle).axis(ScrollbarAxis::Vertical)
    }

    /// Set a specific element id, default is the [`Location::caller`].
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self.base = self.base.id(self.id.clone());
        self
    }

    /// Set the scrollbar show mode [`ScrollbarMode`].
    pub fn mode(mut self, mode: ScrollbarMode) -> Self {
        self.mode = Some(mode);
        self.base = self.base.mode(mode);
        self
    }

    /// Set a special scroll size of the content area, default is None.
    pub fn scroll_size(mut self, scroll_size: Size<Pixels>) -> Self {
        self.scroll_size = Some(scroll_size);
        self.base = self.base.scroll_size(scroll_size);
        self
    }

    /// Override the viewport bounds that this scrollbar overlays.
    pub fn viewport_bounds(mut self, bounds: Bounds<Pixels>) -> Self {
        self.viewport_bounds = Some(bounds);
        self.base = self.base.viewport_bounds(bounds);
        self
    }

    /// Use the scrollbar element's layout bounds as its viewport.
    pub fn viewport_from_layout(mut self) -> Self {
        self.use_layout_bounds = true;
        self.base = self.base.viewport_from_layout();
        self
    }

    /// Set scrollbar axis.
    pub fn axis(mut self, axis: impl Into<ScrollbarAxis>) -> Self {
        self.axis = axis.into();
        self.base = self.base.axis(self.axis);
        self
    }

    /// Track styles. The thumb is Kirakira's: its styles here don't reach it.
    pub fn styles(mut self, build: impl FnOnce(ScrollbarStyles) -> ScrollbarStyles) -> Self {
        self.base = self.base.styles(build);
        self
    }

    /// Set maximum frames per second for scrolling by drag. Default is 120 FPS.
    #[doc(hidden)]
    pub fn max_fps(mut self, max_fps: usize) -> Self {
        self.base = self.base.max_fps(max_fps);
        self
    }

    /// The width of the scrollbar track.
    #[doc(hidden)]
    pub const fn width() -> Pixels {
        BaseScrollbar::width()
    }

    fn axes(&self) -> Vec<Axis> {
        match self.axis {
            ScrollbarAxis::Vertical => vec![Axis::Vertical],
            ScrollbarAxis::Horizontal => vec![Axis::Horizontal],
            ScrollbarAxis::Both => vec![Axis::Horizontal, Axis::Vertical],
        }
    }
}

/// The thumb's box this frame, for an axis of a viewport.
fn thumb_bounds(
    axis: Axis,
    viewport: Bounds<Pixels>,
    content: Size<Pixels>,
    offset: gpui_kit::Point<Pixels>,
    margin_end: Pixels,
    width: Pixels,
) -> Option<(Bounds<Pixels>, Bounds<Pixels>)> {
    let track = Scrollbar::width();
    if axis == Axis::Vertical {
        let (start, length) =
            thumb_span(viewport.size.height, content.height, margin_end, offset.y)?;
        let centre = viewport.right() - track / 2.;
        let hit = Bounds::new(
            point(viewport.right() - track, viewport.top() + start),
            size(track, length),
        );
        let thumb = Bounds::new(
            point(centre - width / 2., viewport.top() + start),
            size(width, length),
        );
        Some((thumb, hit))
    } else {
        let (start, length) = thumb_span(viewport.size.width, content.width, margin_end, offset.x)?;
        let centre = viewport.bottom() - track / 2.;
        let hit = Bounds::new(
            point(viewport.left() + start, viewport.bottom() - track),
            size(length, track),
        );
        let thumb = Bounds::new(
            point(viewport.left() + start, centre - width / 2.),
            size(length, width),
        );
        Some((thumb, hit))
    }
}

fn track_bounds(axis: Axis, viewport: Bounds<Pixels>) -> Bounds<Pixels> {
    let track = Scrollbar::width();
    if axis == Axis::Vertical {
        Bounds::new(
            point(viewport.right() - track, viewport.top()),
            size(track, viewport.size.height),
        )
    } else {
        Bounds::new(
            point(viewport.left(), viewport.bottom() - track),
            size(viewport.size.width, track),
        )
    }
}

impl RenderOnce for Scrollbar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let motion = window.use_keyed_state((id.clone(), "kk-motion"), cx, |_, _| Motion {
            last_offset: None,
            last_scroll: None,
            hovered: false,
            dragging: false,
            visible: false,
            hidden_at: None,
            idle_timer: false,
        });
        let pulse = Pulse::new((id.clone(), "kk-pop"), window, cx);
        let theme = cx.theme();
        let mode = self.mode.unwrap_or(theme.scrollbar_mode);
        let now = now();
        let offset = self.handle.offset();

        // When to show, as GPUI Component's modes decide it.
        let (became_visible, hovered, dragging, visible, hidden_at, wait_for_idle) =
            motion.update(cx, |motion, _| {
                if motion.last_offset.is_some_and(|last| last != offset) {
                    motion.last_scroll = Some(now);
                }
                motion.last_offset = Some(offset);
                let scrolled = motion
                    .last_scroll
                    .is_some_and(|at| now.saturating_duration_since(at) < IDLE);
                let visible = match mode {
                    ScrollbarMode::Always => true,
                    ScrollbarMode::Hover => motion.hovered || motion.dragging || scrolled,
                    ScrollbarMode::Scrolling => {
                        motion.dragging || scrolled || (motion.hovered && motion.visible)
                    }
                };
                let became_visible = visible && !motion.visible;
                if motion.visible && !visible {
                    motion.hidden_at = Some(now);
                }
                motion.visible = visible;
                // One timer per hold, not one per scroll: it re-arms itself while scrolling
                // keeps pushing the hold out.
                let wait_for_idle = scrolled && mode != ScrollbarMode::Always && !motion.idle_timer;
                motion.idle_timer |= wait_for_idle;
                (
                    became_visible,
                    motion.hovered,
                    motion.dragging,
                    visible,
                    motion.hidden_at,
                    wait_for_idle,
                )
            });
        if wait_for_idle {
            // Render again once the hold runs out, so the bar can hide. Nothing on screen changes
            // until then, so no frames are asked for meanwhile.
            let motion = motion.clone();
            window
                .spawn(cx, async move |cx| {
                    loop {
                        let wait = cx.update(|_, cx| {
                            motion.read(cx).last_scroll.map(|at| {
                                IDLE.saturating_sub(
                                    crate::motion::now().saturating_duration_since(at),
                                )
                            })
                        });
                        match wait {
                            Ok(Some(wait)) if !wait.is_zero() => {
                                cx.background_executor().timer(wait).await
                            }
                            Ok(_) => break,
                            Err(_) => return,
                        }
                    }
                    cx.update(|_, cx| {
                        motion.update(cx, |motion, cx| {
                            motion.idle_timer = false;
                            cx.notify();
                        })
                    })
                    .ok();
                })
                .detach();
        }
        if became_visible {
            pulse.fire(cx);
        }
        let pulse = Pulse::new((id.clone(), "kk-pop"), window, cx);
        let active = hovered || dragging;
        let theme = cx.theme();
        let (rest_color, active_color) = (theme.scrollbar_thumb, theme.scrollbar_thumb_hover);
        let radius = theme.radius;
        let rest = glide(
            (id.clone(), "kk-width"),
            if active { FULL } else { REST },
            ms(300),
            cx.curves().spring,
            window,
            cx,
        );
        let color = glide(
            (id.clone(), "kk-color"),
            if active { active_color } else { rest_color },
            ms(150),
            Easing::EaseOut,
            window,
            cx,
        );
        let width = match pulse.running(POP) {
            Some(elapsed) => {
                pulse.animate(POP, window);
                rest * pop_track().sample(Timing::new(POP).sample(elapsed).directed_progress)
            }
            None => rest,
        };
        let opacity = if visible {
            1.0
        } else {
            match hidden_at.map(|at| now.saturating_duration_since(at)) {
                Some(elapsed) if elapsed < FADE => {
                    window.request_animation_frame();
                    fade_track().sample(Timing::new(FADE).sample(elapsed).directed_progress)
                }
                _ => 0.0,
            }
        };
        let paint = Paint {
            width,
            opacity,
            color,
            radius,
            visible,
        };

        let handle = self.handle.clone();
        let axes = self.axes();
        let scroll_size = self.scroll_size;
        let explicit = self.viewport_bounds;
        let use_layout = self.use_layout_bounds;
        let base = self.base.styles(|styles| {
            styles
                .thumb(|thumb| thumb.bg(gpui_kit::transparent_black()))
                .thumb_hover(|thumb| thumb.bg(gpui_kit::transparent_black()))
                .thumb_active(|thumb| thumb.bg(gpui_kit::transparent_black()))
        });

        div().absolute().inset_0().child(base).child(
            canvas(
                move |bounds, _, _| {
                    let viewport = explicit.unwrap_or(if use_layout {
                        bounds
                    } else {
                        handle.viewport_bounds()
                    });
                    let content = scroll_size.unwrap_or(handle.content_size());
                    let offset = handle.offset();
                    let both = axes.len() == 2;
                    axes.iter()
                        .filter_map(|axis| {
                            let margin_end = if both
                                && *axis == Axis::Horizontal
                                && content.height > viewport.size.height
                            {
                                Scrollbar::width()
                            } else {
                                px(0.)
                            };
                            thumb_bounds(*axis, viewport, content, offset, margin_end, paint.width)
                                .map(|(thumb, hit)| {
                                    (*axis, track_bounds(*axis, viewport), thumb, hit)
                                })
                        })
                        .collect::<Vec<_>>()
                },
                move |_, thumbs, window, _| {
                    for (_, track, thumb, hit) in thumbs {
                        if paint.opacity > 0.0 && thumb.size.width > px(0.) {
                            let radius = if paint.radius > px(0.) {
                                (thumb.size.width.min(thumb.size.height)) / 2.
                            } else {
                                px(0.)
                            };
                            window.paint_layer(track, |window| {
                                window.paint_quad(
                                    fill(thumb, paint.color.opacity(paint.opacity))
                                        .corner_radii(radius),
                                );
                            });
                        }
                        // Watch the pointer before gpui-base's scrollbar handles it.
                        let hover = motion.clone();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, _, cx| {
                            if !phase.capture() {
                                return;
                            }
                            let over = track.contains(&event.position);
                            hover.update(cx, |motion, cx| {
                                if motion.hovered != over {
                                    motion.hovered = over;
                                    cx.notify();
                                }
                            });
                        });
                        let down = motion.clone();
                        let visible = paint.visible;
                        window.on_mouse_event(move |event: &MouseDownEvent, phase, _, cx| {
                            if phase.capture() && visible && hit.contains(&event.position) {
                                down.update(cx, |motion, cx| {
                                    motion.dragging = true;
                                    cx.notify();
                                });
                            }
                        });
                        let up = motion.clone();
                        window.on_mouse_event(move |_: &MouseUpEvent, phase, _, cx| {
                            if phase.capture() {
                                up.update(cx, |motion, cx| {
                                    if motion.dragging {
                                        motion.dragging = false;
                                        cx.notify();
                                    }
                                });
                            }
                        });
                    }
                },
            )
            .absolute()
            .inset_0()
            .size_full(),
        )
    }
}

/// A trait for elements that can be made scrollable with Kirakira scrollbars.
///
/// The wrapped element is the scroll area itself, rather than being inserted as
/// a child of a new scroll area.
pub trait ScrollableElement: InteractiveElement + Styled + ParentElement + Element {
    /// Adds a scrollbar to the element.
    #[track_caller]
    fn scrollbar<H: ScrollbarHandle + Clone>(
        self,
        scroll_handle: &H,
        axis: impl Into<ScrollbarAxis>,
    ) -> Self {
        self.child(
            Scrollbar::new(scroll_handle)
                .id(ElementId::CodeLocation(*Location::caller()))
                .axis(axis),
        )
    }

    /// Adds a vertical scrollbar to the element.
    #[track_caller]
    fn vertical_scrollbar<H: ScrollbarHandle + Clone>(self, scroll_handle: &H) -> Self {
        self.scrollbar(scroll_handle, ScrollbarAxis::Vertical)
    }

    /// Adds a horizontal scrollbar to the element.
    #[track_caller]
    fn horizontal_scrollbar<H: ScrollbarHandle + Clone>(self, scroll_handle: &H) -> Self {
        self.scrollbar(scroll_handle, ScrollbarAxis::Horizontal)
    }

    /// Almost equivalent to [`StatefulInteractiveElement::overflow_scroll`], but adds scrollbars.
    #[track_caller]
    fn overflow_scrollbar(self) -> Scrollable<Self> {
        Scrollable::new(self, ScrollbarAxis::Both)
    }

    /// Almost equivalent to [`StatefulInteractiveElement::overflow_x_scroll`], but adds a
    /// horizontal scrollbar.
    #[track_caller]
    fn overflow_x_scrollbar(self) -> Scrollable<Self> {
        Scrollable::new(self, ScrollbarAxis::Horizontal)
    }

    /// Almost equivalent to [`StatefulInteractiveElement::overflow_y_scroll`], but adds a
    /// vertical scrollbar.
    #[track_caller]
    fn overflow_y_scrollbar(self) -> Scrollable<Self> {
        Scrollable::new(self, ScrollbarAxis::Vertical)
    }
}

impl ScrollableElement for Div {}
impl<E> ScrollableElement for Stateful<E>
where
    E: ParentElement + Styled + Element,
    Self: InteractiveElement,
{
}

/// A scrollable element wrapper that renders the original element as the scroll area and
/// overlays Kirakira scrollbars.
#[derive(IntoElement)]
pub struct Scrollable<E: InteractiveElement + Styled + ParentElement + Element + 'static> {
    id: ElementId,
    element: E,
    axis: ScrollbarAxis,
}

impl<E> Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element,
{
    #[track_caller]
    fn new(element: E, axis: impl Into<ScrollbarAxis>) -> Self {
        Self {
            id: ElementId::CodeLocation(*Location::caller()),
            element,
            axis: axis.into(),
        }
    }

    /// Set a specific element id, default is the [`std::panic::Location::caller`].
    ///
    /// Only needed when one call site creates several scrollables, which would
    /// otherwise share a single scroll position.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }
}

impl<E> Styled for Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element,
{
    fn style(&mut self) -> &mut StyleRefinement {
        self.element.style()
    }
}

impl<E> ParentElement for Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element,
{
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.element.extend(elements)
    }
}

impl<E> InteractiveElement for Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element,
{
    fn interactivity(&mut self) -> &mut Interactivity {
        self.element.interactivity()
    }

    fn track_focus(mut self, focus: &gpui_kit::FocusHandle) -> Self {
        self.element = self.element.track_focus(focus);
        self
    }
}

/// The outer layout styles of the scroll area, as GPUI Component copies them.
fn root_style_from<E: Styled>(element: &mut E, axis: ScrollbarAxis) -> StyleRefinement {
    let style = element.style();
    StyleRefinement {
        size: style.size.clone(),
        min_size: style.min_size.clone(),
        max_size: style.max_size.clone(),
        flex_grow: style.flex_grow,
        flex_shrink: style.flex_shrink,
        flex_basis: style.flex_basis,
        align_self: style.align_self,
        overflow: PointRefinement {
            x: axis.has_horizontal().then_some(Overflow::Hidden),
            y: axis.has_vertical().then_some(Overflow::Hidden),
        },
        ..Default::default()
    }
}

impl<E> RenderOnce for Scrollable<E>
where
    E: InteractiveElement + Styled + ParentElement + Element + 'static,
{
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let scroll_handle = window
            .use_keyed_state(self.id.clone(), cx, |_, _| ScrollHandle::default())
            .read(cx)
            .clone();
        let root_style = root_style_from(&mut self.element, self.axis);
        let content = self
            .element
            .id((self.id.clone(), "content"))
            .flex_none()
            .map(|this| match self.axis {
                ScrollbarAxis::Vertical => this.h_auto().min_h_full(),
                ScrollbarAxis::Horizontal => this.w_auto().min_w_full(),
                ScrollbarAxis::Both => this.size_auto().min_size_full(),
            });
        let scroll_area = div()
            .id((self.id.clone(), "area"))
            .size_full()
            .flex()
            .track_scroll(&scroll_handle)
            .map(|this| match self.axis {
                ScrollbarAxis::Vertical => this.flex_col().overflow_y_scroll(),
                ScrollbarAxis::Horizontal => this.flex_row().overflow_x_scroll(),
                ScrollbarAxis::Both => this.overflow_scroll(),
            })
            .lock_scroll_axis()
            .child(content);

        div()
            .id(self.id.clone())
            .size_full()
            .refine_style(&root_style)
            .relative()
            .child(scroll_area)
            .when(!window.is_inspector_picking(cx), |this| {
                this.child(
                    Scrollbar::new(&scroll_handle)
                        .id((self.id.clone(), "scrollbar"))
                        .axis(self.axis)
                        .viewport_from_layout(),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_matches_the_web() {
        crate::parity::assert_number_track(
            "pop-scroll-area",
            "kk-pop-scroll-area-fade",
            "opacity",
            &fade_track(),
        );
    }

    #[test]
    fn the_thumb_pops_out_of_nothing() {
        // kk-pop-scroll-area-pop-y scales by `calc(var(--kk-pop-scroll-area-rest) * 1.35)`, which
        // the parity parser can't read: check the multipliers by hand.
        let pop = pop_track();
        assert_eq!(pop.sample(0.0), 0.0);
        assert!((pop.sample(0.55) - 1.35).abs() < 1e-6);
        assert!((pop.sample(0.8) - 0.9).abs() < 1e-6);
        assert_eq!(pop.sample(1.0), 1.0);
    }

    #[test]
    fn the_thumb_follows_the_offset_like_gpui_base() {
        // 100 px of a 400 px document: a 48 px minimum thumb, inset 4 px.
        let (start, length) = thumb_span(px(100.), px(400.), px(0.), px(0.)).unwrap();
        assert_eq!((start, length), (px(4.), px(40.)));
        // Scrolled to the end, the thumb sits at the end of the track.
        let (start, length) = thumb_span(px(100.), px(400.), px(0.), px(-300.)).unwrap();
        assert_eq!(start + length + px(4.), px(100.));
        assert!(thumb_span(px(100.), px(80.), px(0.), px(0.)).is_none());
    }

    #[test]
    fn the_thumb_scales_about_the_track_centre() {
        let viewport = Bounds::new(point(px(0.), px(0.)), size(px(200.), px(100.)));
        let content = size(px(200.), px(400.));
        let (thumb, hit) = thumb_bounds(
            Axis::Vertical,
            viewport,
            content,
            point(px(0.), px(0.)),
            px(0.),
            px(6.),
        )
        .unwrap();
        assert_eq!(thumb.origin.x, px(189.));
        assert_eq!(thumb.size.width, px(6.));
        assert_eq!(hit.origin.x, px(184.));
    }
}
