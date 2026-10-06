//! `PopupMenu`: GPUI Component's popup menu, rebuilt so its rows can move.

use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::base::actions::{Cancel, Confirm, SelectDown, SelectLeft, SelectRight, SelectUp};
use gpui_kit::base::{ElementExt as _, h_flex, v_flex};
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Side, Sizable as _, Size, ThemeStyled as _,
};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    Action, Anchor, AnyElement, App, AppContext as _, Bounds, ClickEvent, Context, DismissEvent,
    Edges, Entity, EventEmitter, FocusHandle, Focusable, Half as _, InteractiveElement as _,
    IntoElement, MouseDownEvent, OwnedMenuItem, ParentElement as _, Pixels, Point, Render, Role,
    ScrollHandle, SharedString, StatefulInteractiveElement as _, Styled as _, Subscription,
    Transformation, WeakEntity, Window, anchored, deferred, div, px, rems, size,
};

use super::menu_item::MenuItemElement;
use crate::motion::{self, Easing, Keyframes, Timing, Track, delay_ms, ms};
use crate::overlay::{self, Phase, Presence, Surface};
use crate::theme::ActiveKira as _;

const CONTEXT: &str = "PopupMenu";

/// How long each row takes to drop into place.
const ROW: Duration = ms(240);
/// The first row starts 50 ms after the menu opens, each next one 35 ms later.
const ROW_DELAY: u64 = 50;
const ROW_STAGGER: u64 = 35;
/// Rows past the twelfth share its slot, as `:nth-child(n+12)` does on the web.
const LAST_SLOT: usize = 11;
/// A row's `text_sm`, in rems: what its `em` is.
const ROW_TEXT_REM: f32 = 0.875;
/// The check mark's pop.
const TICK: Duration = ms(300);

/// When row `row` starts to drop, after the menu opens.
pub(crate) fn row_delay(row: usize) -> Duration {
    ms(ROW_DELAY + row.min(LAST_SLOT) as u64 * ROW_STAGGER)
}

/// How far through its drop row `row` is, `elapsed` after the menu opened, before easing: 0
/// while it waits (the `backwards` fill), 1 once it has landed.
pub(crate) fn row_time(row: usize, elapsed: Duration) -> f32 {
    Timing::new(ROW)
        .delay(delay_ms(row_delay(row).as_millis() as u64))
        .sample(elapsed)
        .directed_progress
}

/// `opacity` of `@keyframes kk-pop-dropdown-menu-row { from { opacity: 0; translate: 0 -0.4em } }`
/// on `curve` (Kirakira's `out`).
pub(crate) fn row_opacity(curve: Easing) -> Keyframes<f32> {
    Track::new(curve).at(0.0, 0.0).at(1.0, 1.0).build()
}

/// The row's vertical `translate`, in ems of its text.
pub(crate) fn row_drop(curve: Easing) -> Keyframes<f32> {
    Track::new(curve).at(0.0, -0.4).at(1.0, 0.0).build()
}

/// When the last of `rows` rows lands.
pub(crate) fn rows_end(rows: usize) -> Duration {
    row_delay(rows.saturating_sub(1)) + ROW
}

/// `@keyframes kk-pop-dropdown-menu-tick`: 0 → 1.3 → 0.9 → 1, ease-in-out.
pub(crate) fn tick() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.0)
        .at(0.45, 1.3)
        .at(0.75, 0.9)
        .at(1.0, 1.0)
        .build()
}

/// An menu item in a popup menu.
pub enum PopupMenuItem {
    /// A menu separator item.
    Separator,
    /// A non-interactive label item.
    Label(SharedString),
    /// A standard menu item.
    Item {
        icon: Option<Icon>,
        label: SharedString,
        disabled: bool,
        checked: bool,
        is_link: bool,
        action: Option<Box<dyn Action>>,
        // For link item
        handler: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    },
    /// A menu item with custom element render.
    ElementItem {
        icon: Option<Icon>,
        disabled: bool,
        checked: bool,
        action: Option<Box<dyn Action>>,
        render: Box<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>,
        handler: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    },
    /// A submenu item that opens another popup menu.
    Submenu {
        icon: Option<Icon>,
        label: SharedString,
        disabled: bool,
        menu: Entity<PopupMenu>,
    },
}

impl FluentBuilder for PopupMenuItem {}

impl PopupMenuItem {
    /// Create a new menu item with the given label.
    #[inline]
    pub fn new(label: impl Into<SharedString>) -> Self {
        PopupMenuItem::Item {
            icon: None,
            label: label.into(),
            disabled: false,
            checked: false,
            action: None,
            is_link: false,
            handler: None,
        }
    }

    /// Create a new menu item with custom element render.
    #[inline]
    pub fn element<F, E>(builder: F) -> Self
    where
        F: Fn(&mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        PopupMenuItem::ElementItem {
            icon: None,
            disabled: false,
            checked: false,
            action: None,
            render: Box::new(move |window, cx| builder(window, cx).into_any_element()),
            handler: None,
        }
    }

    /// Create a new submenu item that opens another popup menu.
    #[inline]
    pub fn submenu(label: impl Into<SharedString>, menu: Entity<PopupMenu>) -> Self {
        PopupMenuItem::Submenu {
            icon: None,
            label: label.into(),
            disabled: false,
            menu,
        }
    }

    /// Create a separator menu item.
    #[inline]
    pub fn separator() -> Self {
        PopupMenuItem::Separator
    }

    /// Creates a label menu item.
    #[inline]
    pub fn label(label: impl Into<SharedString>) -> Self {
        PopupMenuItem::Label(label.into())
    }

    /// Set the icon for the menu item. Only works for `Item`, `ElementItem` and `Submenu`.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        match &mut self {
            PopupMenuItem::Item { icon: i, .. }
            | PopupMenuItem::ElementItem { icon: i, .. }
            | PopupMenuItem::Submenu { icon: i, .. } => *i = Some(icon.into()),
            _ => {}
        }
        self
    }

    /// Set the action for the menu item. Only works for `Item` and `ElementItem`.
    pub fn action(mut self, action: Box<dyn Action>) -> Self {
        match &mut self {
            PopupMenuItem::Item { action: a, .. }
            | PopupMenuItem::ElementItem { action: a, .. } => *a = Some(action),
            _ => {}
        }
        self
    }

    /// Set the disabled state for the menu item. Only works for `Item`, `ElementItem` and
    /// `Submenu`.
    pub fn disabled(mut self, disabled: bool) -> Self {
        match &mut self {
            PopupMenuItem::Item { disabled: d, .. }
            | PopupMenuItem::ElementItem { disabled: d, .. }
            | PopupMenuItem::Submenu { disabled: d, .. } => *d = disabled,
            _ => {}
        }
        self
    }

    /// Set checked state for the menu item.
    ///
    /// NOTE: If `check_side` is [`Side::Left`], the icon will replace with a check icon.
    pub fn checked(mut self, checked: bool) -> Self {
        match &mut self {
            PopupMenuItem::Item { checked: c, .. }
            | PopupMenuItem::ElementItem { checked: c, .. } => *c = checked,
            _ => {}
        }
        self
    }

    /// Add a click handler for the menu item. Only works for `Item` and `ElementItem`.
    pub fn on_click<F>(mut self, handler: F) -> Self
    where
        F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        match &mut self {
            PopupMenuItem::Item { handler: h, .. }
            | PopupMenuItem::ElementItem { handler: h, .. } => *h = Some(Rc::new(handler)),
            _ => {}
        }
        self
    }

    /// Create a link menu item.
    #[inline]
    pub fn link(label: impl Into<SharedString>, href: impl Into<String>) -> Self {
        let href = href.into();
        PopupMenuItem::Item {
            icon: None,
            label: label.into(),
            disabled: false,
            checked: false,
            action: None,
            is_link: true,
            handler: Some(Rc::new(move |_, _, cx| cx.open_url(&href))),
        }
    }

    #[inline]
    fn is_clickable(&self) -> bool {
        matches!(
            self,
            PopupMenuItem::Item {
                disabled: false,
                ..
            } | PopupMenuItem::ElementItem {
                disabled: false,
                ..
            } | PopupMenuItem::Submenu {
                disabled: false,
                ..
            }
        )
    }

    #[inline]
    fn is_separator(&self) -> bool {
        matches!(self, PopupMenuItem::Separator)
    }

    fn has_left_icon(&self, check_side: Side) -> bool {
        match self {
            PopupMenuItem::Item { icon, checked, .. }
            | PopupMenuItem::ElementItem { icon, checked, .. } => {
                icon.is_some() || (check_side.is_left() && *checked)
            }
            PopupMenuItem::Submenu { icon, .. } => icon.is_some(),
            _ => false,
        }
    }

    #[inline]
    fn is_checked(&self) -> bool {
        match self {
            PopupMenuItem::Item { checked, .. } | PopupMenuItem::ElementItem { checked, .. } => {
                *checked
            }
            _ => false,
        }
    }

    fn a11y_label(&self) -> Option<SharedString> {
        match self {
            PopupMenuItem::Item { label, .. }
            | PopupMenuItem::Label(label)
            | PopupMenuItem::Submenu { label, .. } => Some(label.clone()),
            PopupMenuItem::Separator | PopupMenuItem::ElementItem { .. } => None,
        }
    }
}

/// Where a menu is in its open and close, and where it opens from.
#[derive(Default)]
pub(crate) struct MenuMotion {
    presence: Presence,
    /// When it last opened: the rows and ticks count from here.
    opened: Option<Instant>,
    /// The side of its trigger it opens on, and how it aligns along that edge.
    side: overlay::Side,
    along: f32,
    /// Where it rests, for ghosts.
    surface: Option<Bounds<Pixels>>,
    /// The submenu shown last frame, so a newly shown one pops and a hidden one plays its exit.
    shown_submenu: Option<usize>,
    shown_menu: Option<Entity<PopupMenu>>,
    /// Submenus playing their exit, painted where they were until it ends.
    closing_submenus: Vec<Entity<PopupMenu>>,
}

pub struct PopupMenu {
    pub(crate) focus_handle: FocusHandle,
    pub(crate) menu_items: Vec<PopupMenuItem>,
    /// The focus handle of Entity to handle actions.
    pub(crate) action_context: Option<FocusHandle>,
    /// The focus to restore on dismiss, without changing where actions are dispatched.
    pub(crate) previous_focus_handle: Option<FocusHandle>,
    /// A focus handle on the trigger's dispatch path, so shortcut hints resolve on the frame the
    /// menu opens.
    pub(crate) trigger_focus_handle: Option<FocusHandle>,
    selected_index: Option<usize>,
    min_width: Option<Pixels>,
    max_width: Option<Pixels>,
    max_height: Option<Pixels>,
    bounds: Bounds<Pixels>,
    size: Size,
    check_side: Side,

    /// The parent menu of this menu, if this is a submenu
    parent_menu: Option<WeakEntity<Self>>,
    scrollable: bool,
    external_link_icon: bool,
    scroll_handle: ScrollHandle,
    // This will update on render
    submenu_anchor: (Anchor, Pixels),
    /// Paint priority for this menu layer; each nested submenu draws one above its parent.
    priority: usize,
    motion: MenuMotion,

    _subscriptions: Vec<Subscription>,
}

impl PopupMenu {
    pub(crate) fn new(cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            action_context: None,
            previous_focus_handle: None,
            trigger_focus_handle: None,
            parent_menu: None,
            menu_items: Vec::new(),
            selected_index: None,
            min_width: None,
            max_width: None,
            max_height: None,
            check_side: Side::Left,
            bounds: Bounds::default(),
            scrollable: false,
            scroll_handle: ScrollHandle::default(),
            external_link_icon: true,
            size: Size::default(),
            submenu_anchor: (Anchor::TopLeft, Pixels::ZERO),
            priority: gpui_kit::base::POPUP_PRIORITY,
            motion: MenuMotion::default(),
            _subscriptions: vec![],
        }
    }

    pub fn build(
        window: &mut Window,
        cx: &mut App,
        f: impl FnOnce(Self, &mut Window, &mut Context<PopupMenu>) -> Self,
    ) -> Entity<Self> {
        cx.new(|cx| f(Self::new(cx), window, cx))
    }

    /// Set the focus handle of Entity to handle actions.
    ///
    /// When the menu is dismissed or before an action is triggered, the focus will be returned to
    /// this handle. Then the action will be dispatched to this handle.
    pub fn action_context(mut self, handle: FocusHandle) -> Self {
        self.action_context = Some(handle);
        self
    }

    pub(crate) fn set_action_context(
        &mut self,
        action_context: Option<FocusHandle>,
        cx: &mut Context<Self>,
    ) {
        self.action_context = action_context.clone();
        for item in &self.menu_items {
            if let PopupMenuItem::Submenu { menu, .. } = item {
                menu.update(cx, |menu, cx| {
                    menu.set_action_context(action_context.clone(), cx)
                });
            }
        }
    }

    pub(crate) fn set_previous_focus(
        &mut self,
        handle: Option<FocusHandle>,
        cx: &mut Context<Self>,
    ) {
        self.previous_focus_handle = handle.clone();
        for item in &self.menu_items {
            if let PopupMenuItem::Submenu { menu, .. } = item {
                menu.update(cx, |menu, cx| menu.set_previous_focus(handle.clone(), cx));
            }
        }
    }

    pub(crate) fn set_trigger_focus(
        &mut self,
        handle: Option<FocusHandle>,
        cx: &mut Context<Self>,
    ) {
        self.trigger_focus_handle = handle.clone();
        for item in &self.menu_items {
            if let PopupMenuItem::Submenu { menu, .. } = item {
                menu.update(cx, |menu, cx| menu.set_trigger_focus(handle.clone(), cx));
            }
        }
    }

    /// Set min width of the popup menu, default is 120px
    pub fn min_w(mut self, width: impl Into<Pixels>) -> Self {
        self.min_width = Some(width.into());
        self
    }

    /// Set max width of the popup menu, default is 500px
    pub fn max_w(mut self, width: impl Into<Pixels>) -> Self {
        self.max_width = Some(width.into());
        self
    }

    /// Set max height of the popup menu, default is half of the window height
    pub fn max_h(mut self, height: impl Into<Pixels>) -> Self {
        self.max_height = Some(height.into());
        self
    }

    /// Set the menu to be scrollable to show vertical scrollbar.
    pub fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = scrollable;
        self
    }

    /// Set the side to show check icon, default is `Side::Left`.
    pub fn check_side(mut self, side: Side) -> Self {
        self.check_side = side;
        self
    }

    /// Set the menu to show external link icon, default is true.
    pub fn external_link_icon(mut self, visible: bool) -> Self {
        self.external_link_icon = visible;
        self
    }

    /// Add Menu Item
    pub fn menu(self, label: impl Into<SharedString>, action: Box<dyn Action>) -> Self {
        self.menu_with_disabled(label, action, false)
    }

    /// Add Menu Item with enable state
    pub fn menu_with_enable(
        mut self,
        label: impl Into<SharedString>,
        action: Box<dyn Action>,
        enable: bool,
    ) -> Self {
        self.add_menu_item(label, None, action, !enable, false);
        self
    }

    /// Add Menu Item with disabled state
    pub fn menu_with_disabled(
        mut self,
        label: impl Into<SharedString>,
        action: Box<dyn Action>,
        disabled: bool,
    ) -> Self {
        self.add_menu_item(label, None, action, disabled, false);
        self
    }

    /// Add label
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.menu_items.push(PopupMenuItem::label(label.into()));
        self
    }

    /// Add Menu to open link
    pub fn link(self, label: impl Into<SharedString>, href: impl Into<String>) -> Self {
        self.link_with_disabled(label, href, false)
    }

    /// Add Menu to open link with disabled state
    pub fn link_with_disabled(
        mut self,
        label: impl Into<SharedString>,
        href: impl Into<String>,
        disabled: bool,
    ) -> Self {
        self.menu_items
            .push(PopupMenuItem::link(label, href).disabled(disabled));
        self
    }

    /// Add Menu to open link
    pub fn link_with_icon(
        mut self,
        label: impl Into<SharedString>,
        icon: impl Into<Icon>,
        href: impl Into<String>,
    ) -> Self {
        self.menu_items
            .push(PopupMenuItem::link(label, href).icon(icon));
        self
    }

    /// Add Menu Item with Icon.
    pub fn menu_with_icon(
        self,
        label: impl Into<SharedString>,
        icon: impl Into<Icon>,
        action: Box<dyn Action>,
    ) -> Self {
        self.menu_with_icon_and_disabled(label, icon, action, false)
    }

    /// Add Menu Item with Icon and disabled state
    pub fn menu_with_icon_and_disabled(
        mut self,
        label: impl Into<SharedString>,
        icon: impl Into<Icon>,
        action: Box<dyn Action>,
        disabled: bool,
    ) -> Self {
        self.add_menu_item(label, Some(icon.into()), action, disabled, false);
        self
    }

    /// Add Menu Item with check icon
    pub fn menu_with_check(
        self,
        label: impl Into<SharedString>,
        checked: bool,
        action: Box<dyn Action>,
    ) -> Self {
        self.menu_with_check_and_disabled(label, checked, action, false)
    }

    /// Add Menu Item with check icon and disabled state
    pub fn menu_with_check_and_disabled(
        mut self,
        label: impl Into<SharedString>,
        checked: bool,
        action: Box<dyn Action>,
        disabled: bool,
    ) -> Self {
        self.add_menu_item(label, None, action, disabled, checked);
        self
    }

    /// Add Menu Item with custom element render.
    pub fn menu_element<F, E>(self, action: Box<dyn Action>, builder: F) -> Self
    where
        F: Fn(&mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.menu_element_with_check(false, action, builder)
    }

    /// Add Menu Item with custom element render with disabled state.
    pub fn menu_element_with_disabled<F, E>(
        mut self,
        action: Box<dyn Action>,
        disabled: bool,
        builder: F,
    ) -> Self
    where
        F: Fn(&mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.menu_items.push(
            PopupMenuItem::element(builder)
                .action(action)
                .disabled(disabled),
        );
        self
    }

    /// Add Menu Item with custom element render with icon.
    pub fn menu_element_with_icon<F, E>(
        mut self,
        icon: impl Into<Icon>,
        action: Box<dyn Action>,
        builder: F,
    ) -> Self
    where
        F: Fn(&mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.menu_items
            .push(PopupMenuItem::element(builder).action(action).icon(icon));
        self
    }

    /// Add Menu Item with custom element render with check state
    pub fn menu_element_with_check<F, E>(
        mut self,
        checked: bool,
        action: Box<dyn Action>,
        builder: F,
    ) -> Self
    where
        F: Fn(&mut Window, &mut App) -> E + 'static,
        E: IntoElement,
    {
        self.menu_items.push(
            PopupMenuItem::element(builder)
                .action(action)
                .checked(checked),
        );
        self
    }

    /// Add a separator Menu Item
    pub fn separator(mut self) -> Self {
        if self.menu_items.is_empty() {
            return self;
        }
        if let Some(PopupMenuItem::Separator) = self.menu_items.last() {
            return self;
        }
        self.menu_items.push(PopupMenuItem::separator());
        self
    }

    /// Add a Submenu
    pub fn submenu(
        self,
        label: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        self.submenu_with_icon(None, label, window, cx, f)
    }

    /// Add a Submenu item with icon
    pub fn submenu_with_icon(
        mut self,
        icon: Option<Icon>,
        label: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> Self {
        let submenu = PopupMenu::build(window, cx, f);
        let parent_menu = cx.entity().downgrade();
        let parent_priority = self.priority;
        submenu.update(cx, |view, _| {
            view.parent_menu = Some(parent_menu);
            view.priority = parent_priority + 1;
        });
        self.menu_items.push(
            PopupMenuItem::submenu(label, submenu).when_some(icon, |this, icon| this.icon(icon)),
        );
        self
    }

    /// Add menu item.
    pub fn item(mut self, item: impl Into<PopupMenuItem>) -> Self {
        self.menu_items.push(item.into());
        self
    }

    /// Replace all menu items by re-running a builder on this menu, keeping its identity (focus,
    /// parent menu, layer priority).
    pub fn rebuild(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        f: impl FnOnce(Self, &mut Window, &mut Context<Self>) -> Self,
    ) {
        let mut menu = std::mem::replace(self, Self::new(cx));
        menu.menu_items.clear();
        menu.selected_index = None;
        *self = f(menu, window, cx);
        cx.notify();
    }

    fn add_menu_item(
        &mut self,
        label: impl Into<SharedString>,
        icon: Option<Icon>,
        action: Box<dyn Action>,
        disabled: bool,
        checked: bool,
    ) -> &mut Self {
        self.menu_items.push(
            PopupMenuItem::new(label)
                .when_some(icon, |item, icon| item.icon(icon))
                .disabled(disabled)
                .checked(checked)
                .action(action),
        );
        self
    }

    pub(crate) fn with_menu_items<I>(
        mut self,
        items: impl IntoIterator<Item = I>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self
    where
        I: Into<OwnedMenuItem>,
    {
        for item in items {
            match item.into() {
                OwnedMenuItem::Action {
                    name,
                    action,
                    checked,
                    disabled,
                    ..
                } => {
                    self = self.menu_with_check_and_disabled(
                        name,
                        checked,
                        action.boxed_clone(),
                        disabled,
                    )
                }
                OwnedMenuItem::Separator => self = self.separator(),
                OwnedMenuItem::Submenu(submenu) => {
                    self = self.submenu(submenu.name, window, cx, move |menu, window, cx| {
                        menu.with_menu_items(submenu.items.clone(), window, cx)
                    })
                }
                OwnedMenuItem::SystemMenu(_) => {}
            }
        }
        if self.menu_items.len() > 20 {
            self.scrollable = true;
        }
        self
    }

    pub(crate) fn active_submenu(&self) -> Option<Entity<PopupMenu>> {
        let item = self.menu_items.get(self.selected_index?)?;
        match item {
            PopupMenuItem::Submenu { menu, .. } => Some(menu.clone()),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.menu_items.is_empty()
    }

    // ── Motion, for the containers ─────────────────────────────────────────────────────────

    /// Plays the entrance again from `now`: a menu shown once more.
    pub(crate) fn open_motion(&mut self, now: Instant) {
        self.motion.presence.restart(now);
        self.motion.opened = Some(now);
    }

    /// Starts the exit. The menu stops taking input; its container keeps painting it with
    /// [`overlay::ghost`] until [`Self::exit_done`].
    pub(crate) fn close_motion(&mut self, now: Instant) {
        self.motion.presence.set(false, now);
    }

    /// Whether the exit has finished, so the container can stop painting the menu.
    pub(crate) fn exit_done(&self, now: Instant, reduced: bool) -> bool {
        Surface::pop().phase(&self.motion.presence, now, reduced) == Phase::Closed
    }

    /// The side of its trigger the menu opens on, and its alignment along that edge (0 start,
    /// 1 end).
    pub(crate) fn place(&mut self, side: overlay::Side, along: f32) {
        self.motion.side = side;
        self.motion.along = along;
    }

    /// Where the menu rests, once painted.
    pub(crate) fn surface_bounds(&self) -> Option<Bounds<Pixels>> {
        self.motion.surface
    }

    fn closing(&self) -> bool {
        self.motion.presence.started() && !self.motion.presence.is_open()
    }

    // ── Interaction ─────────────────────────────────────────────────────────────────────────

    fn clickable_menu_items(&self) -> impl Iterator<Item = (usize, &PopupMenuItem)> {
        self.menu_items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.is_clickable())
    }

    fn on_click(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        window.prevent_default();
        self.selected_index = Some(ix);
        self.confirm(&Confirm { secondary: false }, window, cx);
    }

    fn confirm(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.selected_index else {
            return;
        };
        match self.menu_items.get(index) {
            Some(PopupMenuItem::Item {
                handler, action, ..
            })
            | Some(PopupMenuItem::ElementItem {
                handler, action, ..
            }) => {
                if let Some(handler) = handler {
                    handler(&ClickEvent::default(), window, cx);
                } else if let Some(action) = action.as_ref() {
                    self.dispatch_confirm_action(action.as_ref(), window, cx);
                }
                self.dismiss(&Cancel, window, cx)
            }
            _ => {}
        }
    }

    fn dispatch_confirm_action(
        &self,
        action: &dyn Action,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(context) = self.action_context.as_ref() {
            context.focus(window, cx);
        }
        window.dispatch_action(action.boxed_clone(), cx);
    }

    fn set_selected_index(&mut self, ix: usize, cx: &mut Context<Self>) {
        if self.selected_index != Some(ix) {
            self.selected_index = Some(ix);
            self.scroll_handle.scroll_to_item(ix);
            cx.notify();
        }
    }

    fn select_up(&mut self, _: &SelectUp, _: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        let ix = self.selected_index.unwrap_or(0);
        if let Some((prev_ix, _)) = self
            .menu_items
            .iter()
            .enumerate()
            .rev()
            .find(|(i, item)| *i < ix && item.is_clickable())
        {
            self.set_selected_index(prev_ix, cx);
            return;
        }
        let last_clickable_ix = self.clickable_menu_items().last().map(|(ix, _)| ix);
        self.set_selected_index(last_clickable_ix.unwrap_or(0), cx);
    }

    fn select_down(&mut self, _: &SelectDown, _: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        let Some(ix) = self.selected_index else {
            self.set_selected_index(0, cx);
            return;
        };
        if let Some((next_ix, _)) = self
            .menu_items
            .iter()
            .enumerate()
            .find(|(i, item)| *i > ix && item.is_clickable())
        {
            self.set_selected_index(next_ix, cx);
            return;
        }
        self.set_selected_index(0, cx);
    }

    fn select_left(&mut self, _: &SelectLeft, window: &mut Window, cx: &mut Context<Self>) {
        let handled = if matches!(self.submenu_anchor.0, Anchor::TopLeft | Anchor::BottomLeft) {
            self.unselect_submenu(window, cx)
        } else {
            self.select_submenu(window, cx)
        };
        if self.parent_side(cx).is_left() {
            self.focus_parent_menu(window, cx);
        }
        if handled {
            return;
        }
        // For parent AppMenuBar to handle.
        if self.parent_menu.is_none() {
            cx.propagate();
        }
    }

    fn select_right(&mut self, _: &SelectRight, window: &mut Window, cx: &mut Context<Self>) {
        let handled = if matches!(self.submenu_anchor.0, Anchor::TopLeft | Anchor::BottomLeft) {
            self.select_submenu(window, cx)
        } else {
            self.unselect_submenu(window, cx)
        };
        if self.parent_side(cx).is_right() {
            self.focus_parent_menu(window, cx);
        }
        if handled {
            return;
        }
        // For parent AppMenuBar to handle.
        if self.parent_menu.is_none() {
            cx.propagate();
        }
    }

    fn select_submenu(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if let Some(active_submenu) = self.active_submenu() {
            // Focus the submenu, so that can be handle the action.
            active_submenu.update(cx, |view, cx| {
                view.set_selected_index(0, cx);
                view.focus_handle.focus(window, cx);
            });
            cx.notify();
            return true;
        }
        false
    }

    fn unselect_submenu(&mut self, _: &mut Window, cx: &mut Context<Self>) -> bool {
        if let Some(active_submenu) = self.active_submenu() {
            active_submenu.update(cx, |view, cx| {
                view.selected_index = None;
                cx.notify();
            });
            return true;
        }
        false
    }

    fn focus_parent_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(parent) = self
            .parent_menu
            .as_ref()
            .and_then(|parent| parent.upgrade())
        else {
            return;
        };
        self.selected_index = None;
        parent.update(cx, |view, cx| {
            view.focus_handle.focus(window, cx);
            cx.notify();
        });
    }

    fn parent_side(&self, cx: &App) -> Side {
        let Some(parent) = self
            .parent_menu
            .as_ref()
            .and_then(|parent| parent.upgrade())
        else {
            return Side::Left;
        };
        match parent.read(cx).submenu_anchor.0 {
            Anchor::TopRight | Anchor::BottomRight => Side::Right,
            _ => Side::Left,
        }
    }

    /// Dismiss the menu and the entire parent chain.
    fn dismiss(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        self.selected_index = None;
        cx.emit(DismissEvent);

        // Focus back to the previous focused handle, unless the item's click handler has already
        // moved focus elsewhere.
        let focus_moved_away =
            window.focused(cx).is_some() && !self.focus_handle.contains_focused(window, cx);
        if !focus_moved_away
            && let Some(handle) = self
                .previous_focus_handle
                .as_ref()
                .or(self.action_context.as_ref())
        {
            window.focus(handle, cx);
        }

        let Some(parent_menu) = self.parent_menu.clone() else {
            return;
        };
        _ = parent_menu.update(cx, |view, cx| view.dismiss(&Cancel, window, cx));
    }

    fn handle_dismiss(
        &mut self,
        position: &Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Do not dismiss, if click inside the parent menu
        if let Some(parent) = self
            .parent_menu
            .as_ref()
            .and_then(|parent| parent.upgrade())
            && parent.read(cx).bounds.contains(position)
        {
            return;
        }
        // An active submenu handles the click itself.
        if self.active_submenu().is_some() {
            return;
        }
        self.dismiss(&Cancel, window, cx);
    }

    fn on_mouse_down_out(
        &mut self,
        e: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.handle_dismiss(&e.position, window, cx);
    }

    fn render_key_binding(&self, action: Option<Box<dyn Action>>, window: &Window) -> Option<Kbd> {
        let action = action?;
        [
            self.action_context.as_ref(),
            self.trigger_focus_handle.as_ref(),
            self.previous_focus_handle.as_ref(),
            Some(&self.focus_handle),
        ]
        .into_iter()
        .flatten()
        .find_map(|handle| Kbd::binding_for_action_in(action.as_ref(), handle, window))
        .or_else(|| Kbd::global_binding_for_action(action.as_ref(), window))
        .map(|this| {
            this.p_0()
                .flex_nowrap()
                .border_0()
                .bg(gpui_kit::transparent_white())
        })
    }

    /// The icon column. A check mark pops in with `tick` (its scale) while the menu opens.
    fn render_icon(
        has_icon: bool,
        checked: bool,
        icon: Option<Icon>,
        tick: Option<f32>,
    ) -> Option<Icon> {
        if !has_icon {
            return None;
        }
        let icon = if let Some(icon) = icon {
            icon
        } else if checked {
            Self::check_icon(tick)
        } else {
            Icon::empty()
        };
        Some(icon.xsmall())
    }

    fn check_icon(tick: Option<f32>) -> Icon {
        Icon::new(IconName::Check).when_some(tick, |icon, scale| {
            let scale = scale.max(1e-3);
            icon.transform(Transformation::scale(size(scale, scale)))
        })
    }

    #[inline]
    fn max_width(&self) -> Pixels {
        self.max_width.unwrap_or(px(500.))
    }

    /// Calculate the anchor corner and left offset for child submenu
    fn update_submenu_menu_anchor(&mut self, window: &Window) {
        let bounds = self.bounds;
        let max_width = self.max_width();
        let (anchor, left) = if max_width + bounds.origin.x > window.bounds().size.width {
            (Anchor::TopRight, -px(16.))
        } else {
            (Anchor::TopLeft, bounds.size.width - px(8.))
        };
        let is_bottom_pos = bounds.origin.y + bounds.size.height > window.bounds().size.height;
        self.submenu_anchor = if is_bottom_pos {
            (anchor.other_side_along(gpui_kit::Axis::Vertical), left)
        } else {
            (anchor, left)
        };
    }

    /// The side a submenu opens on, and its alignment, from where this menu places it.
    fn submenu_placement(&self) -> (overlay::Side, f32) {
        match self.submenu_anchor.0 {
            Anchor::TopRight => (overlay::Side::Left, 0.0),
            Anchor::BottomLeft => (overlay::Side::Right, 1.0),
            Anchor::BottomRight => (overlay::Side::Left, 1.0),
            _ => (overlay::Side::Right, 0.0),
        }
    }

    fn render_item(
        &self,
        ix: usize,
        item: &PopupMenuItem,
        options: RenderOptions,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> MenuItemElement {
        let has_left_icon = options.has_left_icon;
        let interactive = options.interactive;
        let is_left_check = options.check_side.is_left() && item.is_checked();
        let right_check_icon = if options.check_side.is_right() && item.is_checked() {
            Some(Self::check_icon(options.tick).xsmall())
        } else {
            None
        };

        let selected = interactive && self.selected_index == Some(ix);
        const EDGE_PADDING: Pixels = px(4.);

        let is_submenu = matches!(item, PopupMenuItem::Submenu { .. });
        let group_name = format!("{}:item-{}", cx.entity().entity_id(), ix);

        // GPUI Component's 26px and 20px rows and 8px padding, in rems so the pop scales them.
        let (item_height, radius) = match self.size {
            Size::Small => (rems(1.25), options.radius.half()),
            _ => (rems(1.625), options.radius),
        };

        let this = MenuItemElement::new(ix, &group_name)
            .relative()
            .text_sm()
            .py_0()
            .px(rems(0.5))
            .rounded(radius)
            .items_center()
            .selected(selected)
            .when(interactive, |this| {
                this.on_hover(cx.listener(move |this, hovered, _, cx| {
                    if *hovered {
                        this.selected_index = Some(ix);
                    } else if !is_submenu && this.selected_index == Some(ix) {
                        this.selected_index = None;
                    }
                    cx.notify();
                }))
            })
            .when_some(item.a11y_label(), |this, label| this.aria_label(label));

        match item {
            PopupMenuItem::Separator => this
                .h_auto()
                .p_0()
                .my_0p5()
                .mx_neg_1()
                .border_b(px(2.))
                .border_color(cx.theme().border)
                .disabled(true),
            PopupMenuItem::Label(label) => this.disabled(true).cursor_default().child(
                h_flex()
                    .cursor_default()
                    .items_center()
                    .gap_x_1()
                    .children(Self::render_icon(has_left_icon, false, None, None))
                    .child(div().flex_1().child(label.clone())),
            ),
            PopupMenuItem::ElementItem {
                render,
                icon,
                disabled,
                ..
            } => this
                .when(!disabled && interactive, |this| {
                    this.on_click(
                        cx.listener(move |this, _, window, cx| this.on_click(ix, window, cx)),
                    )
                })
                .disabled(*disabled)
                .child(
                    h_flex()
                        .flex_1()
                        .min_h(item_height)
                        .items_center()
                        .gap_x_1()
                        .children(Self::render_icon(
                            has_left_icon,
                            is_left_check,
                            icon.clone(),
                            options.tick,
                        ))
                        .child((render)(window, cx))
                        .children(right_check_icon.map(|icon| icon.ml_3())),
                ),
            PopupMenuItem::Item {
                icon,
                label,
                action,
                disabled,
                is_link,
                ..
            } => {
                let show_link_icon = *is_link && self.external_link_icon;
                let action = action.as_ref().map(|action| action.boxed_clone());
                let key = self.render_key_binding(action, window);

                this.when(!disabled && interactive, |this| {
                    this.on_click(
                        cx.listener(move |this, _, window, cx| this.on_click(ix, window, cx)),
                    )
                })
                .disabled(*disabled)
                .h(item_height)
                .gap_x_1()
                .children(Self::render_icon(
                    has_left_icon,
                    is_left_check,
                    icon.clone(),
                    options.tick,
                ))
                .child(
                    h_flex()
                        .w_full()
                        .gap_3()
                        .items_center()
                        .justify_between()
                        .when(!show_link_icon, |this| this.child(label.clone()))
                        .children(right_check_icon)
                        .when(show_link_icon, |this| {
                            this.child(
                                h_flex()
                                    .w_full()
                                    .justify_between()
                                    .gap_1p5()
                                    .child(label.clone())
                                    .child(
                                        Icon::new(IconName::ExternalLink)
                                            .xsmall()
                                            .text_color(cx.theme().muted_foreground),
                                    ),
                            )
                        })
                        .children(key),
                )
            }
            PopupMenuItem::Submenu {
                icon,
                label,
                menu,
                disabled,
            } => this
                .selected(selected)
                .disabled(*disabled)
                .items_start()
                .child(
                    h_flex()
                        .min_h(item_height)
                        .size_full()
                        .items_center()
                        .gap_x_1()
                        .children(Self::render_icon(has_left_icon, false, icon.clone(), None))
                        .child(
                            h_flex()
                                .flex_1()
                                .gap_2()
                                .items_center()
                                .justify_between()
                                .child(label.clone())
                                .child(
                                    Icon::new(IconName::ChevronRight)
                                        .xsmall()
                                        .text_color(cx.theme().muted_foreground),
                                ),
                        ),
                )
                .when(selected, |this| {
                    this.child({
                        let (anchor, left) = self.submenu_anchor;
                        let is_bottom_pos =
                            matches!(anchor, Anchor::BottomLeft | Anchor::BottomRight);
                        deferred(
                            anchored()
                                .anchor(anchor)
                                .child(
                                    div()
                                        .id("submenu")
                                        .occlude()
                                        .when(is_bottom_pos, |this| this.bottom_0())
                                        .when(!is_bottom_pos, |this| this.top_neg_1())
                                        .left(left)
                                        .child(menu.clone()),
                                )
                                .snap_to_window_with_margin(Edges::all(EDGE_PADDING)),
                        )
                        .with_priority(self.priority + 1)
                    })
                }),
        }
    }
}

impl FluentBuilder for PopupMenu {}
impl EventEmitter<DismissEvent> for PopupMenu {}
impl Focusable for PopupMenu {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[derive(Clone, Copy)]
struct RenderOptions {
    has_left_icon: bool,
    check_side: Side,
    radius: Pixels,
    /// The check marks' scale while they pop in; `None` once they rest.
    tick: Option<f32>,
    /// False while the menu plays its exit.
    interactive: bool,
}

impl Render for PopupMenu {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = motion::now();
        let reduced = cx.reduce_motion();
        let surface_motion = Surface::pop();
        if !self.motion.presence.started() {
            self.open_motion(now);
        }
        let phase = surface_motion.phase(&self.motion.presence, now, reduced);
        if phase == Phase::Closed {
            return div().into_any_element();
        }
        let interactive = !self.closing();

        self.update_submenu_menu_anchor(window);

        // Submenus attached through `item()` have no parent wired at construction time.
        let parent = cx.entity().downgrade();
        let parent_priority = self.priority;
        for item in &self.menu_items {
            if let PopupMenuItem::Submenu { menu, .. } = item
                && menu.read(cx).parent_menu.is_none()
            {
                menu.update(cx, |menu, _| {
                    menu.parent_menu = Some(parent.clone());
                    menu.priority = parent_priority + 1;
                });
            }
        }

        // A submenu shown for the first time since it was last hidden pops from its side; the one
        // it replaces, or the open one when this menu closes, shrinks back into its side.
        let shown_submenu = if interactive {
            self.selected_index
                .filter(|_| self.active_submenu().is_some())
        } else {
            None
        };
        if shown_submenu != self.motion.shown_submenu {
            if let Some(menu) = self.motion.shown_menu.take() {
                menu.update(cx, |menu, _| menu.close_motion(now));
                self.motion.closing_submenus.push(menu);
            }
            if let Some(menu) = self.active_submenu().filter(|_| shown_submenu.is_some()) {
                self.motion
                    .closing_submenus
                    .retain(|closing| closing != &menu);
                let (side, along) = self.submenu_placement();
                menu.update(cx, |menu, _| {
                    menu.place(side, along);
                    menu.open_motion(now);
                });
                self.motion.shown_menu = Some(menu);
            }
            self.motion.shown_submenu = shown_submenu;
        }
        self.motion
            .closing_submenus
            .retain(|menu| !menu.read(cx).exit_done(now, reduced));
        let closing_submenus: Vec<AnyElement> = self
            .motion
            .closing_submenus
            .iter()
            .filter_map(|menu| {
                let bounds = menu.read(cx).surface_bounds()?;
                Some(overlay::ghost_at(bounds, self.priority + 1, menu.clone()))
            })
            .collect();

        let view = cx.entity().clone();
        let items_count = self.menu_items.len();
        let max_height = self.max_height.unwrap_or_else(|| {
            let window_half_height = window.window_bounds().get_bounds().size.height * 0.5;
            window_half_height.min(px(450.))
        });
        let has_left_icon = self
            .menu_items
            .iter()
            .any(|item| item.has_left_icon(self.check_side));

        // The rows and check marks count from when the menu opened.
        let since_open = self.motion.opened.map_or(Duration::MAX, |opened| {
            now.saturating_duration_since(opened)
        });
        let visible: Vec<usize> = (0..items_count)
            .filter(|ix| !(*ix + 1 == items_count && self.menu_items[*ix].is_separator()))
            .collect();
        let cascading = !reduced && since_open < rows_end(visible.len());
        let ticking = !reduced && since_open < TICK;
        if phase.running() || cascading || ticking || !closing_submenus.is_empty() {
            window.request_animation_frame();
        }
        let tick =
            ticking.then(|| tick().sample(Timing::new(TICK).sample(since_open).directed_progress));

        let max_width = self.max_width();
        let options = RenderOptions {
            has_left_icon,
            check_side: self.check_side,
            radius: cx.theme().radius.min(px(8.)),
            tick,
            interactive,
        };
        let curve = cx.curves().out.clone();
        let (row_opacity, row_drop) = (row_opacity(curve.clone()), row_drop(curve));
        let em = ROW_TEXT_REM * f32::from(window.rem_size());

        let rows: Vec<AnyElement> = visible
            .iter()
            .enumerate()
            .map(|(row, &ix)| {
                let item = self.render_item(ix, &self.menu_items[ix], options, window, cx);
                if !cascading {
                    return item.into_any_element();
                }
                let t = row_time(row, since_open);
                item.top(px(row_drop.sample(t) * em))
                    .opacity(row_opacity.sample(t))
                    .into_any_element()
            })
            .collect();

        let side = self.motion.side;
        let origin = side.origin(self.motion.along);
        let pose = surface_motion.pose(phase, side, reduced, window.rem_size());
        let surface = v_flex()
            .id("popup-menu")
            .role(Role::Menu)
            .when(interactive, |this| {
                this.key_context(CONTEXT)
                    .track_focus(&self.focus_handle)
                    .on_action(cx.listener(Self::select_up))
                    .on_action(cx.listener(Self::select_down))
                    .on_action(cx.listener(Self::select_left))
                    .on_action(cx.listener(Self::select_right))
                    .on_action(cx.listener(Self::confirm))
                    .on_action(cx.listener(Self::dismiss))
                    .on_mouse_down_out(cx.listener(Self::on_mouse_down_out))
                    .occlude()
            })
            .popover_style(cx)
            .when_some(overlay::pose_shadow(pose, cx), |this, shadow| {
                this.shadow(shadow)
            })
            .text_color(cx.theme().popover_foreground)
            .relative()
            .child({
                let view = view.clone();
                v_flex()
                    .id("items")
                    .p_1()
                    .gap_y_0p5()
                    .min_w(rems(8.))
                    .when_some(self.min_width, |this, min_width| this.min_w(min_width))
                    .max_w(max_width)
                    .when(self.scrollable, |this| {
                        this.max_h(max_height)
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll_handle)
                    })
                    .children(rows)
                    .on_prepaint(move |bounds, _, cx| view.update(cx, |r, _| r.bounds = bounds))
            })
            .when(self.scrollable, |this| {
                this.vertical_scrollbar(&self.scroll_handle)
            });

        div()
            .on_prepaint(move |bounds, _, cx| {
                view.update(cx, |menu, _| menu.motion.surface = Some(bounds))
            })
            .child(motion::transform("kk-popup-menu", pose, surface).origin(origin.0, origin.1))
            .children(closing_submenus)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_cascade_35ms_apart_from_50ms() {
        assert_eq!(row_delay(0), ms(50));
        assert_eq!(row_delay(3), ms(155));
        // Past the twelfth row they share its slot.
        assert_eq!(row_delay(11), ms(435));
        assert_eq!(row_delay(30), ms(435));
        assert_eq!(rows_end(4), ms(155 + 240));
    }

    #[test]
    fn a_row_waits_hidden_then_lands() {
        assert_eq!(row_time(2, ms(100)), 0.0);
        assert!((row_time(2, ms(240)) - 0.5).abs() < 1e-4);
        assert_eq!(row_time(2, ms(400)), 1.0);
    }

    /// Rows and check marks are the same in all three `pop-*` menus.
    #[test]
    fn rows_and_ticks_match_the_web_keyframes() {
        use crate::parity::assert_number_track;
        let curve = crate::theme::Curves::default().out;
        for (component, prefix) in [
            ("pop-dropdown-menu", "kk-pop-dropdown-menu"),
            ("pop-context-menu", "kk-pop-context-menu"),
            ("pop-menubar", "kk-pop-menubar"),
        ] {
            let row = format!("{prefix}-row");
            assert_number_track(component, &row, "opacity", &row_opacity(curve.clone()));
            // `translate: 0 -0.4em`: the parser reads the number; the track is in ems too.
            assert_number_track(component, &row, "y", &row_drop(curve.clone()));
            assert_number_track(component, &format!("{prefix}-tick"), "sx", &tick());
        }
    }

    /// A submenu the pointer leaves shrinks away, painted without input, until its exit is done.
    #[gpui_kit::test]
    fn a_hidden_submenu_plays_its_exit(cx: &mut gpui_kit::TestAppContext) {
        cx.update(|cx| {
            gpui_kit::component::init(cx);
            crate::init(cx);
        });
        let start = Instant::now();
        motion::freeze_time(Some(start));
        let (menu, cx) = cx.add_window_view(|window, cx| {
            PopupMenu::new(cx)
                .submenu("Status", window, cx, |menu, _, _| menu.label("Away"))
                .label("Log out")
        });
        let select = |ix: usize, cx: &mut gpui_kit::VisualTestContext| {
            menu.update(cx, |menu, cx| {
                menu.selected_index = Some(ix);
                cx.notify();
            });
            cx.update(|window, cx| window.draw(cx).clear(cx));
        };
        select(0, cx);
        let submenu = menu.read_with(cx, |menu, _| menu.active_submenu().expect("a submenu"));
        motion::freeze_time(Some(start + ms(400)));
        select(1, cx);
        assert!(submenu.read_with(cx, |submenu, _| submenu.closing()));
        let closing = |cx: &mut gpui_kit::VisualTestContext| {
            menu.read_with(cx, |menu, _| menu.motion.closing_submenus.len())
        };
        assert_eq!(closing(cx), 1, "the submenu plays its exit");

        motion::freeze_time(Some(start + ms(400 + 120)));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        assert_eq!(closing(cx), 0, "the exit is done after 0.12 s");
        motion::freeze_time(None);
    }

    #[test]
    fn the_tick_overshoots() {
        let tick = tick();
        assert_eq!(tick.sample(0.0), 0.0);
        assert!((tick.sample(0.45) - 1.3).abs() < 1e-5);
        assert!((tick.sample(0.75) - 0.9).abs() < 1e-5);
        assert_eq!(tick.sample(1.0), 1.0);
    }
}
