//! Tooltip: a speech bubble that pops out at the pointer, and fades when it goes.
//!
//! Replaces `gpui_kit::component::tooltip`: the same `Tooltip` view and builder, for GPUI's
//! `.tooltip(|window, cx| Tooltip::new("Save").build(window, cx))`.
//!
//! The bubble pops: `scale` runs 0.4 → 1.1 → 0.96 → 1 over 0.26 s from its corner at the
//! pointer, while it slides the last 0.375rem away from the pointer (by 50 %); opacity is done by
//! 30 %. When the pointer leaves, it fades out in 0.1 s where it rests (ease-in). Under reduced
//! motion it fades in over 0.12 s where it rests; the exit is the same fade.
//!
//! # How it's built
//!
//! The same view as GPUI Component's (gpui-base's `Tooltip` with the popover colours, ring,
//! shadow and an optional key binding), wrapped in [`motion::transform`](crate::motion::transform).
//! GPUI builds a tooltip's view once each time it shows and keeps it while it is up, so the view
//! remembers when it first rendered and pops from then. GPUI places tooltips below and right of
//! the pointer and flips them above when they don't fit; the slide follows the side the bubble
//! was painted on.
//!
//! GPUI drops a tooltip's view the frame the pointer leaves its trigger, and the trigger is the
//! app's own element, so nothing of the tooltip is left to play an exit. The window's tooltip
//! layer does it instead: each view records what it shows (its text or element builder, key
//! binding and style) and where GPUI painted it, and
//! [`Root::render_notification_layer`](crate::Root::render_notification_layer), which a GPUI
//! Component app already draws in its main view, paints the bubble once more from that record,
//! above everything and without input, while it fades. So that no frame goes blank between GPUI
//! dropping the view and the layer noticing, the layer paints a bubble that has finished its pop
//! itself and the view draws its bubble hidden underneath, to keep GPUI's placement.
//!
//! # Differences from the web version
//!
//! - The exit needs Kirakira's notification layer in the window (see [`crate::root`]). Without it
//!   a tooltip vanishes the moment GPUI removes it, as GPUI Component's does.
//! - A tooltip that closes before its 0.26 s pop has finished is missing for one frame before its
//!   fade starts: the view still draws it then, and GPUI drops it without a last frame.
//! - The scale is even and comes from the rem size: the text and padding scale, the ring and
//!   shadow don't.
//! - Controls that take a tooltip string (`Button::tooltip`) show it through GPUI Component's
//!   own tooltip overlay, which keeps GPUI Component's motion. Build a Kirakira tooltip with
//!   `.tooltip(|window, cx| Tooltip::new(..).build(window, cx))` to get the pop.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::base::{ElementExt as _, StyledExt as _, Tooltip as BaseTooltip};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::text::Text;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    Action, AnyElement, AnyView, App, AppContext as _, Bounds, BoxShadow, Context, Div, ElementId,
    IntoElement, ParentElement as _, Pixels, Render, SharedString, StyleRefinement, Styled,
    WeakEntity, Window, div, hsla, px,
};

use crate::motion::{self, Pose};
use crate::overlay::{self, Phase, Presence, Side, Surface};

/// Tooltips paint over everything else, popups included.
const PRIORITY: usize = 10_000;

#[derive(Clone)]
enum TooltipContext {
    Text(Text),
    Element(Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>),
}

/// Everything a bubble is drawn from, so the tooltip layer can draw it after GPUI drops the view.
#[derive(Clone)]
struct Bubble {
    content: TooltipContext,
    key_binding: Option<Kbd>,
    style: StyleRefinement,
}

impl Bubble {
    /// GPUI Component's tooltip bubble, drawn at `alpha`.
    ///
    /// GPUI fades each primitive on its own, so a fading bubble would show its shadow through it
    /// as a grey slab; the shadow's ink follows the cube of the opacity instead, as the other
    /// overlays' does ([`overlay::shadow`]).
    fn render(&self, alpha: f32, window: &mut Window, cx: &mut App) -> impl IntoElement + use<> {
        let theme = cx.theme();
        let ink = hsla(0., 0., 0., 0.1 * alpha.clamp(0., 1.).powi(3));
        BaseTooltip::new("tooltip-popup")
            .h_flex()
            .font_family(theme.font_family.clone())
            .m_3()
            .bg(theme.tokens.popover)
            .text_color(theme.popover_foreground)
            .border_1()
            .border_color(theme.border)
            .map(|this| {
                if alpha >= 1.0 {
                    this.shadow_md()
                } else {
                    // `shadow_md`, fading.
                    this.shadow(vec![
                        BoxShadow::new(px(0.), px(4.), ink)
                            .blur_radius(px(6.))
                            .spread_radius(px(-1.)),
                        BoxShadow::new(px(0.), px(2.), ink)
                            .blur_radius(px(4.))
                            .spread_radius(px(-2.)),
                    ])
                }
            })
            .rounded(theme.radius)
            .justify_between()
            .py_0p5()
            .px_2()
            .text_sm()
            .gap_3()
            .refine_style(&self.style)
            .map(|this| {
                this.child(div().map(|this| match &self.content {
                    TooltipContext::Text(text) => this.child(text.clone()),
                    TooltipContext::Element(builder) => this.child(builder(window, cx)),
                }))
            })
            .when_some(self.key_binding.clone(), |this, kbd| {
                this.child(
                    div()
                        .text_xs()
                        .flex_shrink_0()
                        .text_color(cx.theme().muted_foreground)
                        .child(kbd.appearance(false)),
                )
            })
    }
}

/// The bubble's pose at `now`: popping in since `shown`, and fading out since `left` once the
/// view is gone. The origin is its corner at the pointer: top left, or bottom left when GPUI
/// flipped it above.
fn pose(
    shown: Instant,
    left: Option<Instant>,
    now: Instant,
    side: Side,
    reduced: bool,
    window: &Window,
) -> (Phase, Pose, (f32, f32)) {
    let mut presence = Presence::default();
    presence.set(true, shown);
    if let Some(left) = left {
        presence.set(false, left);
    }
    let motion = Surface::tooltip();
    let phase = motion.phase(&presence, now, reduced);
    let pose = motion.pose(phase, side, reduced, window.rem_size());
    let origin = if side == Side::Top {
        (0.0, 1.0)
    } else {
        (0.0, 0.0)
    };
    (phase, pose, origin)
}

/// The bubble wrapped in its motion in `outer`, as the view and the layer both paint it. (Give
/// `outer` its `on_prepaint` before the bubble goes in: the canvas it adds sits where it is put
/// in the flow, and only first does it report the box's own origin.)
fn posed(
    outer: Div,
    id: impl Into<ElementId>,
    bubble: impl IntoElement,
    pose: Pose,
    origin: (f32, f32),
) -> Div {
    outer.child(motion::transform(id, pose, bubble).origin(origin.0, origin.1))
}

/// A tooltip the window's layer knows about: shown now, or fading after GPUI dropped it.
pub(crate) struct Shown {
    view: WeakEntity<Tooltip>,
    bubble: Bubble,
    shown: Instant,
    /// When the view last rendered: it is gone from the next frame on.
    seen: Instant,
    /// Where GPUI painted it, and on which side of the pointer.
    bounds: Option<Bounds<Pixels>>,
    side: Side,
    /// The layer painted it this frame, so the view draws itself hidden.
    painted: bool,
}

/// The tooltips of one window, in [`crate::root`]'s layers.
#[derive(Default)]
pub(crate) struct TooltipLayer {
    shown: Vec<Shown>,
}

impl TooltipLayer {
    /// The record of `view`, made on its first render.
    fn track(
        &mut self,
        view: &WeakEntity<Tooltip>,
        bubble: Bubble,
        shown: Instant,
        now: Instant,
        exit: Duration,
    ) -> &mut Shown {
        // Without the layer in the window nobody paints the gone ones; forget them.
        self.shown.retain(|entry| {
            entry.view.upgrade().is_some() || now.saturating_duration_since(entry.seen) < exit
        });
        let ix = match self.shown.iter().position(|entry| &entry.view == view) {
            Some(ix) => ix,
            None => {
                self.shown.push(Shown {
                    view: view.clone(),
                    bubble: bubble.clone(),
                    shown,
                    seen: now,
                    bounds: None,
                    side: Side::Bottom,
                    painted: false,
                });
                self.shown.len() - 1
            }
        };
        let entry = &mut self.shown[ix];
        entry.bubble = bubble;
        entry.seen = now;
        entry
    }
}

/// Paints the window's tooltips that GPUI has dropped while they fade, and the ones at rest, so
/// none of them skips a frame when it goes. Drawn by [`crate::Root::render_notification_layer`].
pub(crate) fn render_layer(window: &mut Window, cx: &mut App) -> Vec<AnyElement> {
    let layers = crate::root::layers(window, cx);
    let now = motion::now();
    let reduced = cx.reduce_motion();
    let exit = Surface::tooltip().durations(reduced).1;
    let mut paint = Vec::new();
    let mut fading = false;
    layers.update(cx, |layers, _| {
        let shown = &mut layers.tooltips.shown;
        shown.retain(|entry| {
            entry.view.upgrade().is_some() || now.saturating_duration_since(entry.seen) < exit
        });
        for entry in shown.iter_mut() {
            let alive = entry.view.upgrade().is_some();
            let left = (!alive).then_some(entry.seen);
            let (phase, pose, origin) = pose(entry.shown, left, now, entry.side, reduced, window);
            // A bubble still popping in is the view's to paint: the layer would mount its
            // transform mid-scale, which hides it for a frame while it measures.
            entry.painted = alive && phase == Phase::Open;
            if let Some(bounds) = entry.bounds
                && (entry.painted || !alive)
            {
                let id = entry.view.entity_id();
                paint.push((entry.bubble.clone(), bounds, pose, origin, id));
                fading |= !alive;
            }
        }
    });
    if fading {
        window.request_animation_frame();
    }
    paint
        .into_iter()
        .map(|(bubble, bounds, pose, origin, id)| {
            let bubble = bubble.render(pose.alpha(), window, cx);
            let id = ElementId::NamedInteger("kk-tooltip-layer".into(), id.as_u64());
            let outer = div();
            #[cfg(test)]
            let outer =
                gpui_kit::InteractiveElement::debug_selector(outer, || "kk-tooltip-layer".into());
            overlay::ghost_at(bounds, PRIORITY, posed(outer, id, bubble, pose, origin))
        })
        .collect()
}

/// A Tooltip element that can display text or custom content, with optional key binding
/// information.
pub struct Tooltip {
    style: StyleRefinement,
    content: TooltipContext,
    key_binding: Option<Kbd>,
    action: Option<(Box<dyn Action>, Option<SharedString>)>,
    /// When the bubble first rendered: GPUI keeps a tooltip's view while it is shown.
    shown: Option<Instant>,
    /// The side of the pointer the bubble was painted on.
    side: Rc<Cell<Side>>,
}

impl Tooltip {
    fn with_content(content: TooltipContext) -> Self {
        Self {
            style: StyleRefinement::default(),
            content,
            key_binding: None,
            action: None,
            shown: None,
            side: Rc::new(Cell::new(Side::Bottom)),
        }
    }

    /// Create a Tooltip with a text content.
    pub fn new(text: impl Into<Text>) -> Self {
        Self::with_content(TooltipContext::Text(text.into()))
    }

    /// Create a Tooltip with a custom element.
    pub fn element<E, F>(builder: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        Self::with_content(TooltipContext::Element(Rc::new(move |window, cx| {
            builder(window, cx).into_any_element()
        })))
    }

    /// Set Action to display key binding information for the tooltip if it exists.
    pub fn action(mut self, action: &dyn Action, context: Option<&str>) -> Self {
        self.action = Some((action.boxed_clone(), context.map(SharedString::new)));
        self
    }

    /// Set KeyBinding information for the tooltip.
    pub fn key_binding(mut self, key_binding: Option<Kbd>) -> Self {
        self.key_binding = key_binding;
        self
    }

    /// Build the tooltip and return it as an `AnyView`.
    pub fn build(self, _: &mut Window, cx: &mut App) -> AnyView {
        cx.new(|_| self).into()
    }
}

impl FluentBuilder for Tooltip {}

impl Styled for Tooltip {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Render for Tooltip {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let key_binding = if let Some(key_binding) = &self.key_binding {
            Some(key_binding.clone())
        } else if let Some((action, context)) = &self.action {
            Kbd::binding_for_action(action.as_ref(), context.as_deref(), window)
        } else {
            None
        };
        let bubble = Bubble {
            content: self.content.clone(),
            key_binding,
            style: self.style.clone(),
        };

        let now = motion::now();
        let shown = *self.shown.get_or_insert(now);
        let reduced = cx.reduce_motion();
        let side = self.side.get();
        let (phase, pose, origin) = pose(shown, None, now, side, reduced, window);
        if phase.running() {
            window.request_animation_frame();
        }

        // Tell the window's tooltip layer what is shown, and whether it painted it this frame.
        let layers = crate::root::layers(window, cx);
        let view = cx.entity().downgrade();
        let painted = layers.update(cx, |layers, _| {
            let exit = Surface::tooltip().durations(reduced).1;
            let entry = layers
                .tooltips
                .track(&view, bubble.clone(), shown, now, exit);
            std::mem::take(&mut entry.painted)
        });

        let side_cell = self.side.clone();
        let settled = matches!(phase, Phase::Open);
        let outer = div().on_prepaint(move |bounds, window, cx| {
            // Above or below the pointer? GPUI flips a tooltip above when it runs out of room.
            if !settled {
                let pointer = window.mouse_position();
                side_cell.set(if bounds.center().y < pointer.y {
                    Side::Top
                } else {
                    Side::Bottom
                });
            }
            let side = side_cell.get();
            layers.update(cx, |layers, _| {
                if let Some(entry) = layers
                    .tooltips
                    .shown
                    .iter_mut()
                    .find(|entry| entry.view == view)
                {
                    entry.bounds = Some(bounds);
                    entry.side = side;
                }
            });
        });
        let bubble = bubble.render(pose.alpha(), window, cx);
        posed(outer, "kk-tooltip", bubble, pose, origin).when(painted, |this| this.invisible())
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{InteractiveElement as _, Modifiers, StatefulInteractiveElement as _, point};

    use super::*;
    use crate::motion::ms;

    /// A trigger with a Kirakira tooltip, and the notification layer that paints its fade.
    struct Host;

    impl Render for Host {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(
                    div()
                        .id("trigger")
                        .size(px(60.))
                        .tooltip(|window, cx| Tooltip::new("Save").build(window, cx)),
                )
                .children(crate::Root::render_notification_layer(window, cx))
        }
    }

    /// When GPUI drops a tooltip, the layer paints it where it was for its 0.1 s fade.
    #[gpui_kit::test]
    fn a_dropped_tooltip_fades_in_the_layer(cx: &mut gpui_kit::TestAppContext) {
        cx.update(|cx| {
            gpui_kit::component::init(cx);
            crate::init(cx);
        });
        let start = Instant::now();
        motion::freeze_time(Some(start));
        let (_, cx) = cx.add_window_view(|_, _| Host);
        let draw = |cx: &mut gpui_kit::VisualTestContext| {
            cx.update(|window, cx| window.draw(cx).clear(cx));
        };
        let shown = |cx: &mut gpui_kit::VisualTestContext| {
            cx.update(|window, cx| {
                let layers = crate::root::layers(window, cx);
                let layers = layers.read(cx);
                layers
                    .tooltips
                    .shown
                    .iter()
                    .map(|entry| (entry.view.upgrade().is_some(), entry.bounds.is_some()))
                    .collect::<Vec<_>>()
            })
        };
        draw(cx);
        cx.simulate_mouse_move(point(px(30.), px(30.)), None, Modifiers::none());
        cx.executor().advance_clock(ms(600));
        cx.run_until_parked();
        draw(cx);
        draw(cx);
        assert_eq!(shown(cx), [(true, true)], "the tooltip shows and is placed");

        // At rest the layer paints it, and the view keeps its place hidden.
        motion::freeze_time(Some(start + ms(1000)));
        draw(cx);
        assert!(cx.debug_bounds("kk-tooltip-layer").is_some());

        cx.simulate_mouse_move(point(px(300.), px(300.)), None, Modifiers::none());
        draw(cx);
        cx.run_until_parked();
        draw(cx);
        assert_eq!(
            shown(cx),
            [(false, true)],
            "GPUI dropped the view, the layer kept it"
        );
        assert!(
            cx.debug_bounds("kk-tooltip-layer").is_some(),
            "the layer paints the fade"
        );

        motion::freeze_time(Some(start + ms(1000 + 100)));
        draw(cx);
        assert!(shown(cx).is_empty(), "the fade is over after 0.1 s");
        assert!(cx.debug_bounds("kk-tooltip-layer").is_none());
        motion::freeze_time(None);
    }
}
