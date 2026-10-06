//! `DropdownMenu`: a popup menu opened from a button.

use std::rc::Rc;

use gpui_kit::base::{Popover as BasePopover, PopoverState};
use gpui_kit::component::Selectable;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    Anchor, AnyElement, App, Bounds, Context, DismissEvent, Element, ElementId, Entity,
    FocusHandle, Focusable as _, GlobalElementId, InspectorElementId, InteractiveElement as _,
    IntoElement, LayoutId, ParentElement as _, Pixels, RenderOnce, SharedString, Styled as _,
    Window, div,
};

use super::PopupMenu;
use crate::motion;
use crate::overlay::{self, Anchored, Side, anchor_alignment};

/// A dropdown menu trait for buttons and other interactive elements.
///
/// Implemented for Kirakira's [`Button`](crate::button::Button) and GPUI Component's.
pub trait DropdownMenu:
    gpui_kit::Styled + Selectable + gpui_kit::InteractiveElement + IntoElement + 'static
{
    /// Create a dropdown menu with the given items, anchored to the TopLeft corner
    fn dropdown_menu(
        self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> DropdownMenuPopover<Self> {
        self.dropdown_menu_with_anchor(Anchor::TopLeft, f)
    }

    /// Create a dropdown menu with the given items, anchored to the given corner
    fn dropdown_menu_with_anchor(
        mut self,
        anchor: impl Into<Anchor>,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> DropdownMenuPopover<Self> {
        let id = self.interactivity().element_id.clone();
        DropdownMenuPopover::new(id.unwrap_or(0.into()), anchor, self, f)
    }
}

impl DropdownMenu for crate::button::Button {}
impl DropdownMenu for gpui_kit::component::button::Button {}

type MenuBuilder = Rc<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>;

#[derive(IntoElement)]
pub struct DropdownMenuPopover<T: Selectable + IntoElement + 'static> {
    id: ElementId,
    anchor: Anchor,
    trigger: T,
    builder: MenuBuilder,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl<T> DropdownMenuPopover<T>
where
    T: Selectable + IntoElement + 'static,
{
    fn new(
        id: ElementId,
        anchor: impl Into<Anchor>,
        trigger: T,
        builder: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        Self {
            id: SharedString::from(format!("dropdown-menu:{:?}", id)).into(),
            anchor: anchor.into(),
            trigger,
            builder: Rc::new(builder),
            on_open_change: None,
        }
    }

    /// Set the anchor corner for the dropdown menu popover.
    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    /// Add a callback to be called when the menu opens or closes, with the new state.
    pub fn on_open_change(
        mut self,
        callback: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(callback));
        self
    }
}

#[derive(Default)]
struct DropdownMenuState {
    /// The open menu. Built once per open and reused while it stays open.
    menu: Option<Entity<PopupMenu>>,
    /// The menu playing its exit.
    closing: Option<Entity<PopupMenu>>,
    /// Whether the popover was open last frame.
    was_open: bool,
}

impl<T> RenderOnce for DropdownMenuPopover<T>
where
    T: Selectable + IntoElement + 'static,
{
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        TriggerFocus::new(self.id.clone(), move |trigger_focus, window, cx| {
            self.render_popover(trigger_focus, window, cx)
        })
    }
}

impl<T> DropdownMenuPopover<T>
where
    T: Selectable + IntoElement + 'static,
{
    fn render_popover(
        self,
        trigger_focus: FocusHandle,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let builder = self.builder.clone();
        let anchor = self.anchor;
        let menu_state =
            window.use_keyed_state(self.id.clone(), cx, |_, _| DropdownMenuState::default());
        let kk = window.use_keyed_state((self.id.clone(), "kk-dropdown"), cx, |_, _| {
            Anchored::<PopoverState>::default()
        });
        let now = motion::now();
        let reduced = cx.reduce_motion();

        // Closed without the menu dismissing itself (the trigger clicked again): close the menu
        // too, and build a fresh one next time so its entrance plays.
        let open = kk
            .read(cx)
            .state
            .as_ref()
            .is_some_and(|state| state.read(cx).is_open());
        menu_state.update(cx, |state, cx| {
            if state.was_open
                && !open
                && let Some(menu) = state.menu.take()
            {
                menu.update(cx, |menu, _| menu.close_motion(now));
                state.closing = Some(menu);
            }
            state.was_open = open;
            if let Some(closing) = state.closing.as_ref()
                && closing.read(cx).exit_done(now, reduced)
            {
                state.closing = None;
            }
        });
        let ghost = menu_state.read(cx).closing.clone().and_then(|menu| {
            let bounds = menu.read(cx).surface_bounds()?;
            Some(overlay::ghost(bounds, menu))
        });

        let surface_kk = kk.clone();
        let trigger_kk = kk.clone();
        let trigger = self.trigger;
        BasePopover::new(SharedString::from(format!("popover:{}", self.id)))
            .overlay_closable(false)
            .anchor(anchor)
            .trigger_with(move |open, _, _| {
                // Measure the trigger, and paint the closing menu from it.
                let selected = trigger.is_selected();
                overlay::record_trigger(&trigger_kk, div())
                    .child(trigger.selected(selected || open))
                    .children(ghost)
                    .into_any_element()
            })
            .when_some(self.on_open_change, |this, callback| {
                this.on_open_change(move |open, window, cx| callback(open, window, cx))
            })
            .content(move |_, window, cx| {
                let popover_state = cx.entity();
                surface_kk.update(cx, |kk, _| kk.state = Some(popover_state.clone()));
                // The menu is built once per open and kept in state, because this runs on every
                // render. It is dropped when dismissed, so the builder runs again next time.
                let menu = match menu_state.read(cx).menu.clone() {
                    Some(menu) => menu,
                    None => {
                        let builder = builder.clone();
                        let menu = PopupMenu::build(window, cx, move |menu, window, cx| {
                            builder(menu, window, cx)
                        });
                        menu.update(cx, |menu, cx| {
                            menu.set_trigger_focus(Some(trigger_focus.clone()), cx)
                        });
                        menu_state.update(cx, |state, _| state.menu = Some(menu.clone()));
                        menu.focus_handle(cx).focus(window, cx);

                        // Close the popover when the menu dismisses itself, and let it play its
                        // exit. The subscription lives as long as the menu, which `menu_state`
                        // holds, so it holds the states weakly: once the dropdown stops rendering,
                        // its keyed states drop and take the menu with them.
                        let popover_state = popover_state.downgrade();
                        window
                            .subscribe(&menu, cx, {
                                let menu_state = menu_state.downgrade();
                                move |menu, _: &DismissEvent, window, cx| {
                                    if let Some(popover_state) = popover_state.upgrade() {
                                        popover_state
                                            .update(cx, |state, cx| state.dismiss(window, cx));
                                    }
                                    menu.update(cx, |menu, _| menu.close_motion(motion::now()));
                                    if let Some(menu_state) = menu_state.upgrade() {
                                        menu_state.update(cx, |state, _| {
                                            state.menu = None;
                                            state.was_open = false;
                                            state.closing = Some(menu);
                                        });
                                    }
                                }
                            })
                            .detach();
                        menu
                    }
                };

                // Open from the side it landed on, measured once painted.
                let measured = surface_kk
                    .read(cx)
                    .trigger
                    .zip(menu.read(cx).surface_bounds());
                let side = measured
                    .and_then(|(trigger, surface)| Side::measure(trigger, surface))
                    .unwrap_or(Side::of_anchor(anchor));
                menu.update(cx, |menu, _| menu.place(side, anchor_alignment(anchor)));

                gpui_kit::base::v_flex()
                    .id("content")
                    .occlude()
                    .tab_group()
                    .map(|this| match anchor {
                        Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => {
                            this.bottom_1()
                        }
                        _ => this.top_1(),
                    })
                    .child(menu)
            })
            .into_any_element()
    }
}

type TriggerFocusBuild = Box<dyn FnOnce(FocusHandle, &mut Window, &mut App) -> AnyElement>;

/// Registers a focus handle on the trigger's dispatch node without ever focusing it, so the menu
/// opened from the trigger can resolve its shortcut hints against the trigger's key contexts on
/// the frame it opens. (GPUI Component's, which is private to it.)
struct TriggerFocus {
    id: ElementId,
    build: Option<TriggerFocusBuild>,
}

#[derive(Default)]
struct TriggerFocusState {
    focus_handle: Option<FocusHandle>,
}

struct TriggerFocusFrame {
    focus_handle: FocusHandle,
    child: AnyElement,
}

impl TriggerFocus {
    fn new(
        id: ElementId,
        build: impl FnOnce(FocusHandle, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        Self {
            id,
            build: Some(Box::new(build)),
        }
    }
}

impl IntoElement for TriggerFocus {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TriggerFocus {
    type RequestLayoutState = TriggerFocusFrame;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let focus_handle =
            window.with_optional_element_state::<TriggerFocusState, _>(id, |state, _| {
                let mut state = state.flatten().unwrap_or_default();
                let focus_handle = state
                    .focus_handle
                    .get_or_insert_with(|| cx.focus_handle())
                    .clone();
                (focus_handle, Some(state))
            });
        let build = self.build.take().expect("TriggerFocus is laid out once");
        let mut child = build(focus_handle.clone(), window, cx);
        let layout_id = child.request_layout(window, cx);
        (
            layout_id,
            TriggerFocusFrame {
                focus_handle,
                child,
            },
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        frame: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.set_focus_handle(&frame.focus_handle, cx);
        frame.child.prepaint(window, cx);
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        frame: &mut Self::RequestLayoutState,
        _: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        frame.child.paint(window, cx);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use gpui_kit::WeakEntity;

    use super::*;
    use crate::button::Button;

    type Built = Rc<RefCell<Option<WeakEntity<PopupMenu>>>>;

    struct Host {
        shown: bool,
        built: Built,
    }

    impl gpui_kit::Render for Host {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let built = self.built.clone();
            div().size_full().when(self.shown, |this| {
                this.child(
                    Button::new("more")
                        .label("More")
                        .debug_selector(|| "trigger".into())
                        .dropdown_menu(move |menu, _, cx| {
                            *built.borrow_mut() = Some(cx.entity().downgrade());
                            menu.label("Item")
                        }),
                )
            })
        }
    }

    /// A menu that dismissed itself and is playing its exit goes with the dropdown when the
    /// dropdown stops rendering.
    #[gpui_kit::test]
    fn a_dismissed_menu_is_dropped_with_its_dropdown(cx: &mut gpui_kit::TestAppContext) {
        cx.update(|cx| {
            gpui_kit::component::init(cx);
            crate::init(cx);
        });
        let built = Built::default();
        let (host, cx) = cx.add_window_view({
            let built = built.clone();
            move |_, _| Host { shown: true, built }
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let trigger = cx.debug_bounds("trigger").expect("the trigger is drawn");
        cx.simulate_click(trigger.center(), gpui_kit::Modifiers::none());
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let menu = built.borrow().clone().expect("the menu opened");
        menu.update(cx, |_, cx| cx.emit(DismissEvent)).unwrap();
        cx.update(|window, cx| window.draw(cx).clear(cx));

        host.update(cx, |host, cx| {
            host.shown = false;
            cx.notify();
        });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.run_until_parked();
        assert!(menu.upgrade().is_none(), "the dismissed menu leaked");
    }
}
