//! Popover: a panel that pops out of its trigger.
//!
//! Replaces `gpui_kit::component::popover`, with the same builder and the same `PopoverState`.
//!
//! The panel grows out of the trigger: `scale` runs 0.7 → 1.04 → 0.985 → 1 over 0.3 s from the
//! trigger-facing edge, while it slides the last 0.5rem away from the trigger (by 55 %); opacity
//! is done by 30 %. Closing shrinks it to 0.9 and fades it out in 0.12 s, sliding half way back.
//! Under reduced motion it fades in over 0.15 s and out over 0.1 s where it rests.
//!
//! # How it's built
//!
//! GPUI Component's `Popover` is a thin skin over gpui-base's `Popover`, and so is this one: the
//! same surface (`popover_style`, `p_3`, the anchor offset), wrapped in
//! [`motion::transform`](crate::motion::transform). The slide hangs on the side the panel landed
//! on, measured from where the trigger and panel were painted (the anchor decides until then),
//! like Base UI's `data-side`.
//!
//! gpui-base stops rendering a popover's content the frame it closes. To play the exit, the
//! panel is built once more from the same content builder and painted where it was for 0.12 s,
//! on top and without input.
//!
//! # Differences from the web version
//!
//! - The scale is even and comes from the rem size, so what is sized in rems (GPUI's spacing and
//!   text) scales and pixel sizes (`w(px(..))`, borders, the shadow) don't.
//! - GPUI fades each primitive rather than the composited panel, so a half-faded panel would
//!   show its own shadow through it. The shadow fades in with the cube of the panel's opacity
//!   (as GPUI Component's dropdowns do), so it only shows once the panel covers it.

use std::rc::Rc;

use gpui_kit::base::{Popover as BasePopover, StyledExt as _};
use gpui_kit::component::{Selectable, ThemeStyled as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    Anchor, AnyElement, App, Context, Div, ElementId, FocusHandle, InteractiveElement as _,
    IntoElement, MouseButton, ParentElement, RenderOnce, Stateful, StyleRefinement, Styled, Window,
    div,
};

pub use gpui_kit::base::PopoverState;

use crate::motion;
use crate::overlay::{self, Anchored, Phase, Side, Surface, anchor_alignment};

type ContentBuilder =
    Rc<dyn Fn(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> AnyElement + 'static>;

/// A popover element that can be triggered by a button or any other element.
#[derive(IntoElement)]
pub struct Popover {
    id: ElementId,
    style: StyleRefinement,
    anchor: Anchor,
    default_open: bool,
    open: Option<bool>,
    tracked_focus_handle: Option<FocusHandle>,
    trigger: Option<Box<dyn FnOnce(bool, &Window, &App) -> AnyElement + 'static>>,
    content: Option<ContentBuilder>,
    children: Vec<AnyElement>,
    trigger_style: Option<StyleRefinement>,
    mouse_button: MouseButton,
    appearance: bool,
    overlay_closable: bool,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl Popover {
    /// Create a new Popover with `view` mode.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            anchor: Anchor::TopLeft,
            trigger: None,
            trigger_style: None,
            content: None,
            tracked_focus_handle: None,
            children: vec![],
            mouse_button: MouseButton::Left,
            appearance: true,
            overlay_closable: true,
            default_open: false,
            open: None,
            on_open_change: None,
        }
    }

    /// Set the anchor corner of the popover, default is [`Anchor::TopLeft`].
    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    /// Set the mouse button to trigger the popover, default is `MouseButton::Left`.
    pub fn mouse_button(mut self, mouse_button: MouseButton) -> Self {
        self.mouse_button = mouse_button;
        self
    }

    /// Set the trigger element of the popover.
    pub fn trigger<T>(mut self, trigger: T) -> Self
    where
        T: Selectable + IntoElement + 'static,
    {
        self.trigger = Some(Box::new(|is_open, _, _| {
            let selected = trigger.is_selected();
            trigger.selected(selected || is_open).into_any_element()
        }));
        self
    }

    /// Set the default open state of the popover, default is `false`.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }

    /// Force set the open state of the popover. Use with `on_open_change`.
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    /// Add a callback to be called when the open state changes, with the new state.
    pub fn on_open_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_open_change = Some(Rc::new(callback));
        self
    }

    /// Set the style for the trigger element.
    pub fn trigger_style(mut self, style: StyleRefinement) -> Self {
        self.trigger_style = Some(style);
        self
    }

    /// Set whether clicking outside the popover will dismiss it, default is `true`.
    pub fn overlay_closable(mut self, closable: bool) -> Self {
        self.overlay_closable = closable;
        self
    }

    /// Set the content builder for content of the Popover. It runs on every render.
    pub fn content<F, E>(mut self, content: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> E + 'static,
    {
        self.content = Some(Rc::new(move |state, window, cx| {
            content(state, window, cx).into_any_element()
        }));
        self
    }

    /// Set whether the popover has its surface (background, ring, shadow, padding), default is
    /// `true`. The motion plays either way.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Bind the focus handle to receive focus when the popover is opened.
    pub fn track_focus(mut self, handle: &FocusHandle) -> Self {
        self.tracked_focus_handle = Some(handle.clone());
        self
    }

    /// GPUI Component's popover surface.
    fn render_popover_content(anchor: Anchor, appearance: bool, cx: &App) -> Stateful<Div> {
        gpui_kit::base::v_flex()
            .id("content")
            .occlude()
            .tab_group()
            .when(appearance, |this| this.popover_style(cx).p_3())
            .map(|this| match anchor {
                Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => this.bottom_1(),
                _ => this.top_1(),
            })
    }
}

impl ParentElement for Popover {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Popover {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Popover {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let anchor = self.anchor;
        let appearance = self.appearance;
        let style = self.style;
        let content = self.content;
        let kk = window.use_keyed_state((self.id.clone(), "kk-popover"), cx, |_, _| {
            Anchored::<PopoverState>::default()
        });

        // gpui-base decides the open state; read it the way it will.
        let open = self.open.unwrap_or_else(|| {
            kk.read(cx)
                .state
                .as_ref()
                .map_or(self.default_open, |state| state.read(cx).is_open())
        });
        let now = motion::now();
        let reduced = cx.reduce_motion();
        if !open {
            kk.update(cx, |kk, _| kk.presence.set(false, now));
        }
        let phase = Surface::pop().phase(&kk.read(cx).presence, now, reduced);
        let side = kk.read(cx).side(Side::of_anchor(anchor));
        let origin = side.origin(anchor_alignment(anchor));

        // The exit: the surface once more, where it was.
        let (children, ghost) = match (phase, kk.read(cx).state.clone(), kk.read(cx).surface) {
            (Phase::Closing(_), Some(state), Some(bounds)) => {
                window.request_animation_frame();
                let pose = Surface::pop().pose(phase, side, reduced, window.rem_size());
                let children = self.children;
                let content = content.clone();
                let surface = state.update(cx, |state, cx| {
                    Self::render_popover_content(anchor, appearance, cx)
                        .when_some(content, |this, content| {
                            this.child(content(state, window, cx))
                        })
                        .children(children)
                        .refine_style(&style)
                        .when(appearance, |this| {
                            this.when_some(overlay::pose_shadow(pose, cx), |this, shadow| {
                                this.shadow(shadow)
                            })
                        })
                });
                let ghost = overlay::ghost(
                    bounds,
                    motion::transform("kk-popover-ghost", pose, surface).origin(origin.0, origin.1),
                );
                (Vec::new(), Some(ghost))
            }
            _ => (self.children, None),
        };

        let surface_kk = kk.clone();
        let trigger_kk = kk.clone();
        BasePopover::new(self.id)
            .anchor(anchor)
            .mouse_button(self.mouse_button)
            .default_open(self.default_open)
            .overlay_closable(self.overlay_closable)
            .content(move |state, window, cx| {
                // gpui-base renders this only while open, so this is where an open begins.
                let (pose, origin) =
                    overlay::open_pose(&surface_kk, Surface::pop(), anchor, window, cx);
                let panel = Self::render_popover_content(anchor, appearance, cx)
                    .when_some(content, |this, content| {
                        this.child(content(state, window, cx))
                    })
                    .children(children)
                    .refine_style(&style)
                    .when(appearance, |this| {
                        this.when_some(overlay::pose_shadow(pose, cx), |this, shadow| {
                            this.shadow(shadow)
                        })
                    });
                overlay::surface(&surface_kk, "kk-popover-surface", pose, origin, panel)
            })
            .when_some(self.trigger, |this, trigger| {
                this.trigger_with(move |open, window, cx| {
                    overlay::record_trigger(&trigger_kk, div())
                        .child(trigger(open, window, cx))
                        .children(ghost)
                        .into_any_element()
                })
            })
            .when_some(self.open, |this, open| this.open(open))
            .when_some(self.tracked_focus_handle, |this, handle| {
                this.track_focus(&handle)
            })
            .when_some(self.on_open_change, |this, callback| {
                this.on_open_change(move |open, window, cx| callback(open, window, cx))
            })
            .into_any_element()
    }
}
