//! `ContextMenu`: a popup menu opened with a right click, at the pointer.

use std::{cell::RefCell, rc::Rc};

use gpui_kit::{
    Anchor, AnyElement, App, Context, DismissEvent, Element, ElementId, Entity, FocusHandle,
    Focusable as _, GlobalElementId, Hitbox, HitboxBehavior, InspectorElementId,
    InteractiveElement, IntoElement, MouseButton, MouseDownEvent, ParentElement, Pixels, Point,
    StyleRefinement, Styled, Subscription, Window, anchored, deferred, div, px,
};

use super::PopupMenu;
use crate::motion;
use crate::overlay::{self, Side, anchor_alignment};

/// A extension trait for adding a context menu to an element.
pub trait ContextMenuExt: InteractiveElement + ParentElement + Styled {
    /// Add a context menu to the element.
    ///
    /// This will changed the element to be `relative` positioned, and add a child `ContextMenu`
    /// element. Because the `ContextMenu` element is positioned `absolute`, it will not affect
    /// the layout of the parent element.
    #[track_caller]
    fn context_menu(
        mut self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> ContextMenu<Self>
    where
        Self: Sized,
    {
        // The ID must be stable across renders, otherwise the open menu is lost on every render.
        let caller = std::panic::Location::caller();
        let id = self
            .interactivity()
            .element_id
            .clone()
            .map(|id| ElementId::Name(format!("context-menu-{:?}", id).into()))
            .unwrap_or_else(|| ElementId::CodeLocation(*caller));
        ContextMenu::new(id, self).menu(f)
    }
}

impl<E: InteractiveElement + ParentElement + Styled> ContextMenuExt for E {}

type MenuBuilder = Rc<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>;

/// A context menu that can be shown on right-click.
pub struct ContextMenu<E: ParentElement + Styled + Sized> {
    id: ElementId,
    element: Option<E>,
    menu: Option<MenuBuilder>,
    // This is not in use, just for style refinement forwarding.
    _ignore_style: StyleRefinement,
    anchor: Anchor,
}

impl<E: ParentElement + Styled> ContextMenu<E> {
    /// Create a new context menu with the given ID.
    pub fn new(id: impl Into<ElementId>, element: E) -> Self {
        Self {
            id: id.into(),
            element: Some(element),
            menu: None,
            anchor: Anchor::TopLeft,
            _ignore_style: StyleRefinement::default(),
        }
    }

    /// Build the context menu using the given builder function.
    #[must_use]
    fn menu<F>(mut self, builder: F) -> Self
    where
        F: Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    {
        self.menu = Some(Rc::new(builder));
        self
    }

    fn with_element_state<R>(
        &mut self,
        id: &GlobalElementId,
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(&mut Self, &mut ContextMenuState, &mut Window, &mut App) -> R,
    ) -> R {
        window.with_optional_element_state::<ContextMenuState, _>(
            Some(id),
            |element_state, window| {
                let mut element_state = element_state.flatten().unwrap_or_default();
                let result = f(self, &mut element_state, window, cx);
                (result, Some(element_state))
            },
        )
    }
}

impl<E: ParentElement + Styled> ParentElement for ContextMenu<E> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        if let Some(element) = &mut self.element {
            element.extend(elements);
        }
    }
}

impl<E: ParentElement + Styled> Styled for ContextMenu<E> {
    fn style(&mut self) -> &mut StyleRefinement {
        if let Some(element) = &mut self.element {
            element.style()
        } else {
            &mut self._ignore_style
        }
    }
}

impl<E: ParentElement + Styled + IntoElement + 'static> IntoElement for ContextMenu<E> {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

struct ContextMenuSharedState {
    menu_view: Option<Entity<PopupMenu>>,
    /// A dismissed menu playing its exit.
    closing: Option<Entity<PopupMenu>>,
    open: bool,
    position: Point<Pixels>,
    /// Registered on this element's dispatch node every frame and never focused, so the menu can
    /// resolve its shortcut hints against the trigger's key contexts on the frame it opens.
    trigger_focus_handle: Option<FocusHandle>,
    _subscription: Option<Subscription>,
}

pub struct ContextMenuState {
    element: Option<AnyElement>,
    shared_state: Rc<RefCell<ContextMenuSharedState>>,
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self {
            element: None,
            shared_state: Rc::new(RefCell::new(ContextMenuSharedState {
                menu_view: None,
                closing: None,
                open: false,
                position: Default::default(),
                trigger_focus_handle: None,
                _subscription: None,
            })),
        }
    }
}

impl<E: ParentElement + Styled + IntoElement + 'static> Element for ContextMenu<E> {
    type RequestLayoutState = ContextMenuState;
    type PrepaintState = Hitbox;

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
    ) -> (gpui_kit::LayoutId, Self::RequestLayoutState) {
        let anchor = self.anchor;

        self.with_element_state(
            id.expect("ContextMenu has an id"),
            window,
            cx,
            |this, state: &mut ContextMenuState, window, cx| {
                let (position, open) = {
                    let shared_state = state.shared_state.borrow();
                    (shared_state.position, shared_state.open)
                };
                state
                    .shared_state
                    .borrow_mut()
                    .trigger_focus_handle
                    .get_or_insert_with(|| cx.focus_handle());
                let menu_view = state.shared_state.borrow().menu_view.clone();
                let mut menu_elements: Vec<AnyElement> = Vec::new();

                // A dismissed menu keeps being painted where it was while it plays its exit.
                let closing = state.shared_state.borrow().closing.clone();
                if let Some(menu) = closing {
                    let done = menu.read(cx).exit_done(motion::now(), cx.reduce_motion());
                    match menu.read(cx).surface_bounds() {
                        Some(bounds) if !done => {
                            menu_elements.push(overlay::ghost(bounds, menu));
                        }
                        _ => state.shared_state.borrow_mut().closing = None,
                    }
                }

                if open && let Some(menu) = menu_view.filter(|menu| !menu.read(cx).is_empty()) {
                    // The menu opens below and right of the pointer.
                    menu.update(cx, |menu, _| {
                        menu.place(Side::of_anchor(anchor), anchor_alignment(anchor))
                    });
                    // Focus the menu, so that can be handle the action.
                    if !menu.focus_handle(cx).contains_focused(window, cx) {
                        menu.focus_handle(cx).focus(window, cx);
                    }
                    menu_elements.push(
                        deferred(
                            anchored().child(
                                div()
                                    .w(window.bounds().size.width)
                                    .h(window.bounds().size.height)
                                    .on_scroll_wheel(|_, _, cx| {
                                        cx.stop_propagation();
                                    })
                                    .child(
                                        anchored()
                                            .position(position)
                                            .snap_to_window_with_margin(px(8.))
                                            .anchor(anchor)
                                            .child(menu),
                                    ),
                            ),
                        )
                        .with_priority(gpui_kit::base::POPUP_PRIORITY)
                        .into_any(),
                    );
                }

                let mut element = this
                    .element
                    .take()
                    .expect("Element should exists.")
                    .children(menu_elements)
                    .into_any_element();

                let layout_id = element.request_layout(window, cx);

                (
                    layout_id,
                    ContextMenuState {
                        element: Some(element),
                        shared_state: state.shared_state.clone(),
                    },
                )
            },
        )
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: gpui_kit::Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if let Some(trigger_focus) = request_layout
            .shared_state
            .borrow()
            .trigger_focus_handle
            .as_ref()
        {
            window.set_focus_handle(trigger_focus, cx);
        }
        if let Some(element) = &mut request_layout.element {
            element.prepaint(window, cx);
        }
        window.insert_hitbox(bounds, HitboxBehavior::Normal)
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: gpui_kit::Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(element) = &mut request_layout.element {
            element.paint(window, cx);
        }

        let builder = self.menu.clone();

        self.with_element_state(
            id.expect("ContextMenu has an id"),
            window,
            cx,
            |_view, state: &mut ContextMenuState, window, _| {
                let shared_state = state.shared_state.clone();
                let hitbox = hitbox.clone();
                // A right click builds the menu and shows it at the pointer.
                window.on_mouse_event(move |event: &MouseDownEvent, phase, window, cx| {
                    if phase.bubble()
                        && event.button == MouseButton::Right
                        && hitbox.is_hovered(window)
                    {
                        // Capture the focused element to restore focus to on dismiss. If focus is
                        // still on the previous menu, keep its captured focus.
                        let previous_focus_handle = window.focused(cx).and_then(|focused| {
                            let shared_state = shared_state.borrow();
                            match shared_state.menu_view.as_ref() {
                                Some(menu) if menu.read(cx).focus_handle == focused => {
                                    menu.read(cx).previous_focus_handle.clone()
                                }
                                _ => Some(focused),
                            }
                        });

                        {
                            let mut shared_state = shared_state.borrow_mut();
                            shared_state.menu_view = None;
                            shared_state._subscription = None;
                            shared_state.position = event.position;
                            shared_state.open = true;
                        }

                        // Build the menu in the next frame, avoiding race conditions.
                        window.defer(cx, {
                            let shared_state = shared_state.clone();
                            let builder = builder.clone();
                            move |window, cx| {
                                let menu = PopupMenu::build(window, cx, move |menu, window, cx| {
                                    let Some(build) = &builder else {
                                        return menu;
                                    };
                                    build(menu, window, cx)
                                });
                                let trigger_focus_handle =
                                    shared_state.borrow().trigger_focus_handle.clone();
                                menu.update(cx, |menu, cx| {
                                    menu.set_trigger_focus(trigger_focus_handle, cx);
                                    menu.set_previous_focus(previous_focus_handle, cx);
                                });

                                // Dismissing closes it, and it plays its exit where it was. The
                                // state owns this subscription, so it holds the state weakly.
                                let subscription = window.subscribe(&menu, cx, {
                                    let shared_state = Rc::downgrade(&shared_state);
                                    move |menu, _: &DismissEvent, window, cx| {
                                        menu.update(cx, |menu, _| menu.close_motion(motion::now()));
                                        let Some(shared_state) = shared_state.upgrade() else {
                                            return;
                                        };
                                        let mut shared_state = shared_state.borrow_mut();
                                        shared_state.open = false;
                                        shared_state.closing = Some(menu);
                                        window.refresh();
                                    }
                                });

                                let mut state = shared_state.borrow_mut();
                                state.menu_view = Some(menu.clone());
                                state._subscription = Some(subscription);
                                window.refresh();
                            }
                        });
                    }
                });
            },
        );
    }
}
