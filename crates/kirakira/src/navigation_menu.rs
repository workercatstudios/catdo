//! Navigation menu: a panel that pops down out of the menu bar, contents that slide the way the
//! pointer went, and an indicator that springs along under the triggers.
//!
//! GPUI Component has no navigation menu, so this module gives GPUI shadcn/ui's parts:
//! [`NavigationMenu`], [`NavigationMenuList`], [`NavigationMenuItem`], [`NavigationMenuTrigger`],
//! [`NavigationMenuContent`], [`NavigationMenuLink`], [`NavigationMenuIndicator`] and
//! [`navigation_menu_trigger_style`], with Radix's behaviour (open on hover after 200 ms, switch at
//! once while open, close 150 ms after the pointer leaves, click to toggle, Escape to close, a
//! link closes the menu) and the motion of Kirakira's Pop Navigation Menu:
//!
//! - The viewport pops down out of the menu bar from its top edge: `scale` 0.7 → 1.04 → 0.985 → 1
//!   while it drops from 0.5 rem above, opaque by 30 %, in 0.3 s; its contents rise 0.4 em into it
//!   a beat later (0.26 s on the `out` curve, after 60 ms).
//! - Moving to another trigger, the viewport springs to the new content's size (0.35 s, spring
//!   curve) and the new content slides in from 2.5 em the way the pointer went, passing its mark
//!   by a twelfth (0.32 s). The old content goes at once, as in Radix.
//! - The chevron turns past half a turn and settles, 0 → 195° → 175° → 180° in 0.42 s, and turns
//!   back on the `in` curve in 0.2 s from wherever it stands, as the web's transition does.
//! - The indicator springs along under the triggers (0.4 s) and its arrow pops up, `0 → 1.3 →
//!   0.9 → 1` in 0.3 s.
//! - Closing, the panel shrinks to 0.9, lifts 0.25 rem and fades in 0.12 s.
//!
//! With `viewport(false)` each content pops down under its own item instead, and pops away.
//!
//! Under reduced motion panels and contents fade where they rest (0.15 s in, 0.1 s out), and sizes,
//! the chevron and the indicator jump.
//!
//! All of it is timed by [`motion::now`](crate::motion::now), so screenshots can pin any frame. The
//! viewport takes each content's natural size, measured whenever the content shows at rest, so a
//! content that changes size resizes the panel.
//!
//! Differences from the web version: the panel is an owned shape, so it scales exactly; the
//! content inside is any element, so it scales evenly by its rem size
//! ([`motion::transform`](crate::motion::transform)): text and rem sizes follow, fixed pixel sizes
//! don't. The panel's shadow is GPUI's. Radix's viewport is a separate part
//! (`NavigationMenuViewport`); here the menu draws it, switched by [`NavigationMenu::viewport`].
//!
//! Keyboard, as Radix's: the triggers and the links in the list are one row. The arrow keys move
//! along it (both arrows of each axis, stopping at the ends) and Home and End jump to its ends;
//! Enter or Space toggles a trigger; the down arrow on an open trigger moves into its content, on
//! the first link. In a content the arrows, Home and End move between its links, and Escape closes
//! the menu and puts focus back on the trigger, as following a link from the keyboard does.
//! Triggers and links take shadcn's accent when focused and GPUI Component's focus ring when
//! focused from the keyboard; a disabled trigger takes no focus. What differs: Radix tabs from an
//! open trigger into its content and from the content's last link on to the next trigger. Tab here
//! is GPUI Component's root action, which runs before any element hears the key, so Tab follows
//! the window's order, where the content comes after everything else on the page.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::base::{StyledExt as _, h_flex};
use gpui_kit::component::{ActiveTheme as _, ThemeStyled as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Bounds, ClickEvent, Element, ElementId, Entity, FocusHandle, GlobalElementId,
    InspectorElementId, InteractiveElement, IntoElement, KeyDownEvent, Keystroke, LayoutId,
    ParentElement, Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Transformation, Window, canvas, deferred, div, px, radians, size, svg,
};

use crate::icons::CHEVRON_DOWN;
use crate::motion::{Easing, Keyframes, Pose, Timing, Track, delay_ms, ms, now, transform};
use crate::state_motion::glide;
use crate::theme::ActiveKira as _;
use crate::vector::{Layer, Vector};

const OPEN_DELAY: Duration = ms(200);
const CLOSE_DELAY: Duration = ms(150);
const POP_IN: Duration = ms(300);
const POP_OUT: Duration = ms(120);
const RISE: Duration = ms(260);
const SLIDE: Duration = ms(320);
const FLIP: Duration = ms(420);
const ARROW: Duration = ms(300);

fn in_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.0, Easing::EaseOut)
        .at(0.3, 1.0)
        .at(1.0, 1.0)
        .build()
}

fn in_scale() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.7, Easing::EaseOut)
        .at(0.55, 1.04)
        .at(0.8, 0.985)
        .at(1.0, 1.0)
        .build()
}

/// The drop, in rem.
fn in_y() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, -0.5, Easing::EaseOut)
        .at(0.55, 0.0)
        .at(1.0, 0.0)
        .build()
}

fn out_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseIn).at(0.0, 1.0).at(1.0, 0.0).build()
}

fn out_scale() -> Keyframes<f32> {
    Track::new(Easing::EaseIn).at(0.0, 1.0).at(1.0, 0.9).build()
}

/// The lift, in rem.
fn out_y() -> Keyframes<f32> {
    Track::new(Easing::EaseIn)
        .at(0.0, 0.0)
        .at(1.0, -0.25)
        .build()
}

/// The first content rising into the viewport: from 0.4 em below, transparent.
fn rise_y(out: Easing) -> Keyframes<f32> {
    Track::new(out).at(0.0, 0.4).at(1.0, 0.0).build()
}

fn rise_opacity(out: Easing) -> Keyframes<f32> {
    Track::new(out).at(0.0, 0.0).at(1.0, 1.0).build()
}

/// A switched-to content's slide, as a multiple of its shift (2.5 em the way the pointer went).
fn slide_x() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 1.0, Easing::EaseOut)
        .at(0.65, -1.0 / 12.0)
        .at(1.0, 0.0)
        .build()
}

fn slide_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.0, Easing::EaseOut)
        .at(0.35, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// The chevron's turn as it opens, in degrees.
fn flip() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.0)
        .at(0.55, 195.0)
        .at(0.8, 175.0)
        .at(1.0, 180.0)
        .build()
}

fn arrow_scale() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.0)
        .at(0.5, 1.3)
        .at(0.75, 0.9)
        .at(1.0, 1.0)
        .build()
}

/// Reduced motion: fade in 0.15 s ease-out, out 0.1 s ease-in.
fn fade_in() -> Keyframes<f32> {
    Track::new(Easing::EaseOut)
        .at(0.0, 0.0)
        .at(1.0, 1.0)
        .build()
}

fn sample(track: &Keyframes<f32>, duration: Duration, elapsed: Duration) -> f32 {
    track.sample(Timing::new(duration).sample(elapsed).directed_progress)
}

/// The open state of one menu.
#[derive(Default)]
struct NavState {
    open: Option<usize>,
    /// When the panel opened (it pops) and which way the last switch went.
    opened_at: Option<Instant>,
    switched: Option<(Instant, f32)>,
    /// The content that was open, and when the menu closed.
    closing: Option<(usize, Instant)>,
    /// A trigger the pointer rests on, waiting to open.
    pending: Option<(usize, Instant)>,
    /// When the pointer left the triggers and the panel.
    left_at: Option<Instant>,
    /// When each trigger's chevron last flipped open.
    flipped: HashMap<usize, Instant>,
    triggers: HashMap<usize, Bounds<Pixels>>,
    list: Option<Bounds<Pixels>>,
    root: Option<Bounds<Pixels>>,
    /// Each content's natural size, measured whenever it shows at rest.
    contents: HashMap<usize, gpui_kit::Size<Pixels>>,
    /// The rem size and item count the sizes were measured with. A change clears them.
    contents_key: Option<(Pixels, usize)>,
    /// The focus of each trigger and link in the list, by item and place in the item: the trigger
    /// is place 0, the item's links follow.
    focus: HashMap<(usize, usize), FocusHandle>,
    /// The links of each content, in order, as they were last laid out.
    content_links: HashMap<usize, Vec<FocusHandle>>,
}

/// Where an arrow key, Home or End moves focus in a row of items, as Radix's `FocusGroup` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rove {
    Next,
    Previous,
    First,
    Last,
}

impl Rove {
    /// The move a key makes. Both arrows of each axis move, as in Radix.
    fn from_key(keystroke: &Keystroke) -> Option<Self> {
        if keystroke.modifiers.modified() {
            return None;
        }
        match keystroke.key.as_str() {
            "right" | "down" => Some(Self::Next),
            "left" | "up" => Some(Self::Previous),
            "home" => Some(Self::First),
            "end" => Some(Self::Last),
            _ => None,
        }
    }

    /// The item to focus among `len`, from the focused one. Arrows stop at the ends rather than
    /// wrap, as Radix's navigation menu does; from outside the row they enter at its near end.
    fn target(self, current: Option<usize>, len: usize) -> Option<usize> {
        if len == 0 {
            return None;
        }
        match (self, current) {
            (Self::First, _) | (Self::Next, None) => Some(0),
            (Self::Last, _) | (Self::Previous, None) => Some(len - 1),
            (Self::Next, Some(at)) => (at + 1 < len).then_some(at + 1),
            (Self::Previous, Some(at)) => at.checked_sub(1),
        }
    }

    /// Moves focus within `row`. False when the key isn't a move, so it can go on.
    fn apply(event: &KeyDownEvent, row: &[FocusHandle], window: &mut Window, cx: &mut App) -> bool {
        let Some(rove) = Self::from_key(&event.keystroke) else {
            return false;
        };
        let current = row.iter().position(|handle| handle.is_focused(window));
        if let Some(target) = rove.target(current, row.len()) {
            window.focus(&row[target], cx);
        }
        true
    }
}

impl NavState {
    fn open_item(&mut self, ix: usize, now: Instant) {
        match self.open {
            Some(current) if current == ix => {}
            Some(current) => {
                self.switched = Some((now, if ix > current { 1.0 } else { -1.0 }));
                self.closing = Some((current, now));
                self.open = Some(ix);
                self.flipped.insert(ix, now);
            }
            None => {
                self.open = Some(ix);
                self.opened_at = Some(now);
                self.switched = None;
                self.closing = None;
                self.flipped.insert(ix, now);
            }
        }
        self.pending = None;
        self.left_at = None;
    }

    fn close(&mut self, now: Instant) {
        if let Some(open) = self.open.take() {
            self.closing = Some((open, now));
        }
        self.pending = None;
        self.left_at = None;
    }

    /// Opens or closes on the timers. True while one is still waiting.
    fn tick(&mut self, now: Instant) -> bool {
        if let Some((ix, at)) = self.pending
            && now.saturating_duration_since(at) >= OPEN_DELAY
        {
            self.open_item(ix, now);
        }
        if let Some(at) = self.left_at
            && now.saturating_duration_since(at) >= CLOSE_DELAY
        {
            self.close(now);
        }
        self.pending.is_some() || self.left_at.is_some()
    }

    /// Drops the measured content sizes unless they were measured under `key`.
    fn forget_stale_sizes(&mut self, key: (Pixels, usize)) {
        if self.contents_key != Some(key) {
            self.contents.clear();
            self.contents_key = Some(key);
        }
    }

    /// Records a content's natural size; true if it changed.
    fn record_size(&mut self, ix: usize, size: gpui_kit::Size<Pixels>) -> bool {
        self.contents.insert(ix, size) != Some(size)
    }

    fn enter_trigger(&mut self, ix: usize, now: Instant) {
        self.left_at = None;
        if self.open.is_some() {
            self.open_item(ix, now);
        } else if self.pending.map(|(pending, _)| pending) != Some(ix) {
            self.pending = Some((ix, now));
        }
    }

    fn leave(&mut self, now: Instant) {
        self.pending = None;
        if self.open.is_some() {
            self.left_at = Some(now);
        }
    }

    fn trigger_focus(&self, ix: usize) -> Option<FocusHandle> {
        self.focus.get(&(ix, 0)).cloned()
    }
}

thread_local! {
    /// The menu and item whose content is being laid out, so its links can close it and join its
    /// keyboard order.
    static MENU: RefCell<Vec<(Entity<NavState>, usize)>> = const { RefCell::new(Vec::new()) };
}

/// Lays out the content of item `ix` with `menu` published to the links inside it.
struct InMenu {
    menu: Entity<NavState>,
    ix: usize,
    child: AnyElement,
}

impl IntoElement for InMenu {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for InMenu {
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
                MENU.with(|menu| menu.borrow_mut().pop());
            }
        }
        // The links register again as they lay out.
        let ix = self.ix;
        self.menu.update(cx, |state, _| {
            state.content_links.insert(ix, Vec::new());
        });
        MENU.with(|menu| menu.borrow_mut().push((self.menu.clone(), ix)));
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

/// Calls `record` with an element's bounds at prepaint. Pinned to the top left, so the bounds are
/// the element's own.
fn measure(
    record: impl FnOnce(Bounds<Pixels>, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    canvas(
        move |bounds, window, cx| record(bounds, window, cx),
        |_, _, _, _| {},
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

/// A key on trigger `ix`: Escape closes, the down arrow on an open trigger enters its content, and
/// the arrows, Home and End move along the list.
fn trigger_key(
    event: &KeyDownEvent,
    ix: usize,
    state: &Entity<NavState>,
    row: &[FocusHandle],
    window: &mut Window,
    cx: &mut App,
) {
    let plain = !event.keystroke.modifiers.modified();
    match event.keystroke.key.as_str() {
        "escape" => {
            state.update(cx, |state, cx| {
                state.close(crate::motion::now());
                cx.notify();
            });
            cx.stop_propagation();
        }
        "down" if plain && state.read(cx).open == Some(ix) => {
            let first = state
                .read(cx)
                .content_links
                .get(&ix)
                .and_then(|links| links.first().cloned());
            if let Some(first) = first {
                window.focus(&first, cx);
            }
            cx.stop_propagation();
        }
        _ => {
            if Rove::apply(event, row, window, cx) {
                cx.stop_propagation();
            }
        }
    }
}

/// A key inside the open content of item `ix`: Escape closes and puts focus back on the trigger;
/// the arrows, Home and End move between the content's links.
fn content_key(
    event: &KeyDownEvent,
    ix: usize,
    state: &Entity<NavState>,
    window: &mut Window,
    cx: &mut App,
) {
    if event.keystroke.key == "escape" {
        let trigger = state.update(cx, |state, cx| {
            state.close(crate::motion::now());
            cx.notify();
            state.trigger_focus(ix)
        });
        if let Some(trigger) = trigger {
            window.focus(&trigger, cx);
        }
        cx.stop_propagation();
        return;
    }
    let links = state
        .read(cx)
        .content_links
        .get(&ix)
        .cloned()
        .unwrap_or_default();
    if Rove::apply(event, &links, window, cx) {
        cx.stop_propagation();
    }
}

/// The trigger look: shadcn's `navigationMenuTriggerStyle()` with Kirakira's round corners. Use
/// it on a [`NavigationMenuLink`] that sits in the list beside the triggers; the link brings its
/// own hover.
pub fn navigation_menu_trigger_style<E: Styled>(element: E, cx: &App) -> E {
    element
        .flex()
        .flex_row()
        .h_9()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(cx.theme().background)
        .px_4()
        .py_2()
        .text_sm()
        .font_weight(gpui_kit::FontWeight::MEDIUM)
}

/// The root of a navigation menu: a list of items and the viewport their contents open in.
#[derive(IntoElement)]
pub struct NavigationMenu {
    id: ElementId,
    style: StyleRefinement,
    viewport: bool,
    default_open: Option<usize>,
    lists: Vec<NavigationMenuList>,
}

impl NavigationMenu {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            viewport: true,
            default_open: None,
            lists: Vec::new(),
        }
    }

    /// Open contents in one shared viewport under the menu (the default), or each under its
    /// own item.
    pub fn viewport(mut self, viewport: bool) -> Self {
        self.viewport = viewport;
        self
    }

    /// Open the item at `index` (counting every item, with a trigger or not) when the menu first
    /// renders, like Radix's `defaultValue`. It shows at rest, without popping.
    pub fn default_open(mut self, index: usize) -> Self {
        self.default_open = Some(index);
        self
    }

    pub fn child(mut self, list: NavigationMenuList) -> Self {
        self.lists.push(list);
        self
    }
}

impl Styled for NavigationMenu {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// The row of items.
#[derive(Default)]
pub struct NavigationMenuList {
    style: StyleRefinement,
    items: Vec<NavigationMenuItem>,
    indicator: Option<NavigationMenuIndicator>,
}

/// What a [`NavigationMenuList`] holds.
pub enum NavigationMenuListChild {
    Item(Box<NavigationMenuItem>),
    Indicator(Box<NavigationMenuIndicator>),
}

impl From<NavigationMenuItem> for NavigationMenuListChild {
    fn from(item: NavigationMenuItem) -> Self {
        Self::Item(Box::new(item))
    }
}

impl From<NavigationMenuIndicator> for NavigationMenuListChild {
    fn from(indicator: NavigationMenuIndicator) -> Self {
        Self::Indicator(Box::new(indicator))
    }
}

impl NavigationMenuList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn child(mut self, child: impl Into<NavigationMenuListChild>) -> Self {
        match child.into() {
            NavigationMenuListChild::Item(item) => self.items.push(*item),
            NavigationMenuListChild::Indicator(indicator) => self.indicator = Some(*indicator),
        }
        self
    }

    pub fn children(
        mut self,
        children: impl IntoIterator<Item = impl Into<NavigationMenuListChild>>,
    ) -> Self {
        for child in children {
            self = self.child(child);
        }
        self
    }
}

impl Styled for NavigationMenuList {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// One entry of the list: a trigger with its content, or a link.
#[derive(Default)]
pub struct NavigationMenuItem {
    style: StyleRefinement,
    trigger: Option<NavigationMenuTrigger>,
    content: Option<NavigationMenuContent>,
    children: Vec<ItemPart>,
}

/// An item's other children, in order. Links join the list's keyboard order.
enum ItemPart {
    Element(AnyElement),
    Link(Box<NavigationMenuLink>),
}

/// What a [`NavigationMenuItem`] holds.
pub enum NavigationMenuItemChild {
    Trigger(NavigationMenuTrigger),
    Content(NavigationMenuContent),
    /// A link in the list, beside the triggers: the arrow keys reach it too.
    Link(NavigationMenuLink),
    Element(AnyElement),
}

impl From<NavigationMenuTrigger> for NavigationMenuItemChild {
    fn from(trigger: NavigationMenuTrigger) -> Self {
        Self::Trigger(trigger)
    }
}

impl From<NavigationMenuContent> for NavigationMenuItemChild {
    fn from(content: NavigationMenuContent) -> Self {
        Self::Content(content)
    }
}

impl From<NavigationMenuLink> for NavigationMenuItemChild {
    fn from(link: NavigationMenuLink) -> Self {
        Self::Link(link)
    }
}

impl From<AnyElement> for NavigationMenuItemChild {
    fn from(element: AnyElement) -> Self {
        Self::Element(element)
    }
}

impl NavigationMenuItem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn child(mut self, child: impl Into<NavigationMenuItemChild>) -> Self {
        match child.into() {
            NavigationMenuItemChild::Trigger(trigger) => self.trigger = Some(trigger),
            NavigationMenuItemChild::Content(content) => self.content = Some(content),
            NavigationMenuItemChild::Link(link) => {
                self.children.push(ItemPart::Link(Box::new(link)))
            }
            NavigationMenuItemChild::Element(element) => {
                self.children.push(ItemPart::Element(element))
            }
        }
        self
    }
}

impl Styled for NavigationMenuItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// The button that opens an item's content, with a chevron that turns.
#[derive(Default)]
pub struct NavigationMenuTrigger {
    style: StyleRefinement,
    children: Vec<AnyElement>,
    disabled: bool,
}

impl NavigationMenuTrigger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl ParentElement for NavigationMenuTrigger {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for NavigationMenuTrigger {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// What an item shows in the viewport when it's open.
#[derive(Default)]
pub struct NavigationMenuContent {
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl NavigationMenuContent {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ParentElement for NavigationMenuContent {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for NavigationMenuContent {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// The arrow under the open trigger.
#[derive(Default)]
pub struct NavigationMenuIndicator {
    style: StyleRefinement,
}

impl NavigationMenuIndicator {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Styled for NavigationMenuIndicator {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// A link in a content, or in the list styled with [`navigation_menu_trigger_style`]. Clicking it
/// closes the menu.
#[derive(IntoElement)]
pub struct NavigationMenuLink {
    id: ElementId,
    style: StyleRefinement,
    children: Vec<AnyElement>,
    active: bool,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    /// Set by the menu for a link in the list: its focus, and the list's keyboard order.
    focus: Option<FocusHandle>,
    row: Option<Rc<[FocusHandle]>>,
}

impl NavigationMenuLink {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            children: Vec::new(),
            active: false,
            on_click: None,
            focus: None,
            row: None,
        }
    }

    /// The link to the current page.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for NavigationMenuLink {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for NavigationMenuLink {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for NavigationMenuLink {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let (accent, accent_foreground) = (theme.accent, theme.accent_foreground);
        let menu = MENU.with(|menu| menu.borrow().last().cloned());
        let focus = match self.focus {
            Some(focus) => focus,
            None => window
                .use_keyed_state((self.id.clone(), "kk-focus"), cx, |_, cx| {
                    cx.focus_handle().tab_stop(true)
                })
                .read(cx)
                .clone(),
        };
        // A link in a content joins that content's keyboard order.
        if let Some((menu, ix)) = &menu {
            menu.update(cx, |state, _| {
                state
                    .content_links
                    .entry(*ix)
                    .or_default()
                    .push(focus.clone())
            });
        }
        let focus_visible = focus.is_focused(window) && window.last_input_was_keyboard();
        let row = self.row;
        let on_click = self.on_click;
        div()
            .id(self.id)
            .track_focus(&focus)
            .role(Role::Link)
            .flex()
            .flex_col()
            .gap_1()
            .rounded_lg()
            .p_2()
            .text_sm()
            .cursor_pointer()
            .when(self.active, |this| {
                this.bg(accent.opacity(0.5)).text_color(accent_foreground)
            })
            .hover(move |this| this.bg(accent).text_color(accent_foreground))
            .focus(move |this| this.bg(accent).text_color(accent_foreground))
            .refine_style(&self.style)
            .when(focus_visible, |this| this.focus_ring_style(window, cx))
            .when_some(row, |this, row| {
                this.on_key_down(move |event: &KeyDownEvent, window, cx| {
                    if Rove::apply(event, &row, window, cx) {
                        cx.stop_propagation();
                    }
                })
            })
            .on_click(move |event, window, cx| {
                if let Some(on_click) = on_click.as_ref() {
                    on_click(event, window, cx);
                }
                if let Some((menu, _)) = menu.as_ref() {
                    let trigger = menu.update(cx, |state, cx| {
                        let trigger = state.open.and_then(|ix| state.trigger_focus(ix));
                        state.close(crate::motion::now());
                        cx.notify();
                        trigger
                    });
                    // From the keyboard, focus goes back to the trigger rather than out with
                    // the content.
                    if event.is_keyboard()
                        && let Some(trigger) = trigger
                    {
                        window.focus(&trigger, cx);
                    }
                }
            })
            .children(self.children)
    }
}

/// One content's panel this frame.
struct Panel {
    ix: usize,
    content: NavigationMenuContent,
    /// The panel's pose: scale about its top centre, drop or lift, opacity.
    pose: Pose,
    /// The content's own pose inside it: slide or rise.
    inner: Pose,
}

impl RenderOnce for NavigationMenu {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let default_open = self.default_open;
        let state = window.use_keyed_state((id.clone(), "kk-nav"), cx, |_, _| NavState {
            open: default_open,
            ..NavState::default()
        });
        let now = now();
        let reduced = cx.reduce_motion();
        if state.update(cx, |state, _| state.tick(now)) {
            window.request_animation_frame();
        }
        let theme = cx.theme().clone();
        let curves = cx.curves();
        let rem = window.rem_size();
        // Sizes measured at another rem size, or for another list of items, are stale.
        let contents_key = (
            rem,
            self.lists
                .iter()
                .map(|list| list.items.len())
                .sum::<usize>(),
        );
        state.update(cx, |state, _| state.forget_stale_sizes(contents_key));
        let use_viewport = self.viewport;

        let (
            open,
            opened_at,
            switched,
            closing,
            flipped,
            triggers,
            list_bounds,
            root_bounds,
            sizes,
        ) = {
            let s = state.read(cx);
            (
                s.open,
                s.opened_at,
                s.switched,
                s.closing,
                s.flipped.clone(),
                s.triggers.clone(),
                s.list,
                s.root,
                s.contents.clone(),
            )
        };
        let mut animating = false;
        let since = |at: Instant| now.saturating_duration_since(at);

        // The list's keyboard order, Radix's `FocusGroup`: every enabled trigger and every link in
        // the list, left to right.
        let order: Rc<[FocusHandle]> = state.update(cx, |state, cx| {
            let mut row = Vec::new();
            let mut focus = |key: (usize, usize)| {
                state
                    .focus
                    .entry(key)
                    .or_insert_with(|| cx.focus_handle().tab_stop(true))
                    .clone()
            };
            let items = self.lists.iter().flat_map(|list| &list.items);
            for (ix, item) in items.enumerate() {
                if item
                    .trigger
                    .as_ref()
                    .is_some_and(|trigger| !trigger.disabled)
                {
                    row.push(focus((ix, 0)));
                }
                let links = item
                    .children
                    .iter()
                    .filter(|part| matches!(part, ItemPart::Link(_)));
                for (place, _) in links.enumerate() {
                    row.push(focus((ix, place + 1)));
                }
            }
            row.into()
        });
        let focus_of =
            |ix: usize, place: usize, cx: &App| state.read(cx).focus.get(&(ix, place)).cloned();

        let mut lists = Vec::new();
        let mut panels = Vec::new();
        let mut item_ix = 0usize;
        for list in self.lists {
            let mut cells = Vec::new();
            let row = h_flex()
                .flex_1()
                .items_center()
                .justify_center()
                .gap_1()
                .refine_style(&list.style);
            for item in list.items {
                let ix = item_ix;
                item_ix += 1;
                let mut cell = div().relative().refine_style(&item.style);
                if let Some(trigger) = item.trigger {
                    let is_open = open == Some(ix);
                    // Open, the chevron flips, and the glide follows it at once; closed, the glide
                    // turns it home from wherever it stood, as the web's `transition` does.
                    let target = match flipped.get(&ix) {
                        Some(at) if is_open && !reduced && since(*at) < FLIP => {
                            animating = true;
                            sample(&flip(), FLIP, since(*at))
                        }
                        _ if is_open => 180.0_f32,
                        _ => 0.0,
                    };
                    let flip_angle = glide(
                        (id.clone(), SharedString::from(format!("kk-chevron-{ix}"))),
                        target,
                        if is_open { Duration::ZERO } else { ms(200) },
                        curves.r#in.clone(),
                        window,
                        cx,
                    );
                    let (enter, keys, click, record) =
                        (state.clone(), state.clone(), state.clone(), state.clone());
                    let disabled = trigger.disabled;
                    let (accent, accent_foreground) = (theme.accent, theme.accent_foreground);
                    // A disabled trigger can't take focus, as a disabled button can't.
                    let focus = focus_of(ix, 0, cx).filter(|_| !disabled);
                    let focus_visible =
                        focus.as_ref().is_some_and(|focus| focus.is_focused(window))
                            && window.last_input_was_keyboard();
                    let order = order.clone();
                    cell = cell.child(
                        div()
                            .id((id.clone(), SharedString::from(format!("trigger-{ix}"))))
                            .role(Role::Button)
                            .aria_expanded(is_open)
                            .when_some(focus, |this, focus| this.track_focus(&focus))
                            .relative()
                            .flex()
                            .h_9()
                            .w_auto()
                            .items_center()
                            .justify_center()
                            .rounded_full()
                            .bg(theme.background)
                            .px_4()
                            .py_2()
                            .text_sm()
                            .font_weight(gpui_kit::FontWeight::MEDIUM)
                            .when(is_open, |this| {
                                this.bg(accent.opacity(0.5)).text_color(accent_foreground)
                            })
                            .when(!disabled, |this| {
                                this.cursor_pointer()
                                    .hover(move |this| {
                                        this.bg(accent).text_color(accent_foreground)
                                    })
                                    .on_hover(move |hovering, _, cx| {
                                        enter.update(cx, |state, cx| {
                                            if *hovering {
                                                state.enter_trigger(ix, crate::motion::now());
                                            } else {
                                                state.leave(crate::motion::now());
                                            }
                                            cx.notify();
                                        })
                                    })
                                    .on_click(move |_, _, cx| {
                                        click.update(cx, |state, cx| {
                                            if state.open == Some(ix) {
                                                state.close(crate::motion::now());
                                            } else {
                                                state.open_item(ix, crate::motion::now());
                                            }
                                            cx.notify();
                                        })
                                    })
                                    .on_key_down(move |event: &KeyDownEvent, window, cx| {
                                        trigger_key(event, ix, &keys, &order, window, cx)
                                    })
                            })
                            .when(disabled, |this| this.opacity(0.5))
                            .focus(move |this| this.bg(accent).text_color(accent_foreground))
                            .refine_style(&trigger.style)
                            .when(focus_visible, |this| this.focus_ring_style(window, cx))
                            .children(trigger.children)
                            .child(
                                svg()
                                    .data(CHEVRON_DOWN.as_bytes())
                                    .size_3()
                                    .ml_1()
                                    .relative()
                                    .top(px(1.))
                                    .flex_none()
                                    .text_color(if is_open {
                                        accent_foreground
                                    } else {
                                        theme.foreground
                                    })
                                    .with_transformation(Transformation::rotate(radians(
                                        flip_angle.to_radians(),
                                    ))),
                            )
                            .child(measure(move |bounds, _, cx| {
                                record.update(cx, |state, _| {
                                    state.triggers.insert(ix, bounds);
                                })
                            })),
                    );
                }
                let mut place = 0;
                for part in item.children {
                    cell = cell.child(match part {
                        ItemPart::Element(element) => element,
                        ItemPart::Link(mut link) => {
                            place += 1;
                            link.focus = focus_of(ix, place, cx);
                            link.row = Some(order.clone());
                            link.into_any_element()
                        }
                    });
                }
                if let Some(content) = item.content {
                    let is_open = open == Some(ix);
                    let is_closing = closing.is_some_and(|(closing, at)| {
                        closing == ix
                            && since(at) < POP_OUT
                            && open.is_none_or(|open| !use_viewport && open != ix)
                    });
                    if is_open || is_closing {
                        let (pose, inner) = if is_open {
                            let opened = opened_at.map(since).unwrap_or(Duration::MAX);
                            let switch = switched
                                .filter(|_| use_viewport)
                                .map(|(at, dir)| (since(at), dir));
                            if reduced {
                                let fade = opened.min(switch.map_or(Duration::MAX, |s| s.0));
                                let a = sample(&fade_in(), ms(150), fade);
                                animating |= fade < ms(150);
                                (Pose::new().opacity(a), Pose::new())
                            } else {
                                animating |= opened < POP_IN.max(RISE + ms(60));
                                let pose = Pose::new()
                                    .scale(sample(&in_scale(), POP_IN, opened))
                                    .y(sample(&in_y(), POP_IN, opened) * f32::from(rem))
                                    .opacity(sample(&in_opacity(), POP_IN, opened));
                                let em = f32::from(rem);
                                let inner = match switch {
                                    Some((elapsed, dir)) => {
                                        animating |= elapsed < SLIDE;
                                        Pose::new()
                                            .x(sample(&slide_x(), SLIDE, elapsed) * dir * 2.5 * em)
                                            .opacity(sample(&slide_opacity(), SLIDE, elapsed))
                                    }
                                    None if use_viewport => {
                                        let timing = Timing::new(RISE).delay(delay_ms(60));
                                        let p = timing.sample(opened).directed_progress;
                                        Pose::new()
                                            .y(rise_y(curves.out.clone()).sample(p) * em)
                                            .opacity(rise_opacity(curves.out.clone()).sample(p))
                                    }
                                    None => Pose::new(),
                                };
                                (pose, inner)
                            }
                        } else {
                            let elapsed = closing.map_or(POP_OUT, |(_, at)| since(at));
                            animating = true;
                            if reduced {
                                let a = 1.0 - sample(&fade_in(), ms(100), elapsed);
                                (Pose::new().opacity(a), Pose::new())
                            } else {
                                (
                                    Pose::new()
                                        .scale(sample(&out_scale(), POP_OUT, elapsed))
                                        .y(sample(&out_y(), POP_OUT, elapsed) * f32::from(rem))
                                        .opacity(sample(&out_opacity(), POP_OUT, elapsed)),
                                    Pose::new(),
                                )
                            }
                        };
                        panels.push(Panel {
                            ix,
                            content,
                            pose,
                            inner,
                        });
                    } else if !sizes.contains_key(&ix) {
                        // Measure every content up front, unseen, so a switch knows the size it
                        // springs to.
                        panels.push(Panel {
                            ix,
                            content,
                            pose: Pose::new(),
                            inner: Pose::new(),
                        });
                    }
                }
                cells.push(cell.into_any_element());
            }
            // The indicator, under the open trigger.
            let indicator = list.indicator.map(|indicator| {
                let shown = open.and_then(|ix| triggers.get(&ix).copied());
                let list_left = list_bounds.map_or(px(0.), |b| b.left());
                let (x, w) =
                    shown.map_or((px(0.), px(0.)), |b| (b.left() - list_left, b.size.width));
                let travel = if reduced { Duration::ZERO } else { ms(400) };
                let x = glide(
                    (id.clone(), "kk-indicator-x"),
                    x,
                    travel,
                    curves.spring.clone(),
                    window,
                    cx,
                );
                let w = glide(
                    (id.clone(), "kk-indicator-w"),
                    w,
                    travel,
                    curves.spring.clone(),
                    window,
                    cx,
                );
                let alpha = glide(
                    (id.clone(), "kk-indicator-alpha"),
                    if shown.is_some() { 1.0_f32 } else { 0.0 },
                    if shown.is_some() { ms(150) } else { ms(120) },
                    if shown.is_some() {
                        Easing::EaseOut
                    } else {
                        Easing::EaseIn
                    },
                    window,
                    cx,
                );
                let arrow = match opened_at {
                    Some(at) if !reduced && open.is_some() && since(at) < ARROW => {
                        animating = true;
                        sample(&arrow_scale(), ARROW, since(at))
                    }
                    _ => 1.0,
                };
                let diagonal = px(8.) * std::f32::consts::SQRT_2;
                div()
                    .absolute()
                    .top_full()
                    .left(x)
                    .w(w)
                    .h_1p5()
                    .overflow_hidden()
                    .opacity(alpha)
                    .refine_style(&indicator.style)
                    .child(
                        div()
                            .absolute()
                            .top(px(5.6) - diagonal / 2.)
                            .left_0()
                            .right_0()
                            .flex()
                            .justify_center()
                            .child(
                                Vector::new(size(diagonal, diagonal))
                                    .layer(Layer::svg(diamond(diagonal), theme.border))
                                    .pose(Pose::new().scale(arrow)),
                            ),
                    )
            });
            let record = state.clone();
            lists.push(
                div()
                    .relative()
                    .child(row.children(cells))
                    .children(indicator)
                    .child(measure(move |bounds, _, cx| {
                        record.update(cx, |state, _| state.list = Some(bounds))
                    }))
                    .into_any_element(),
            );
        }
        if animating {
            window.request_animation_frame();
        }

        // The viewport, or each content under its item.
        let root_left = root_bounds.map_or(px(0.), |b| b.left());
        let viewport_size = open.and_then(|ix| sizes.get(&ix).copied());
        let resize = if reduced { Duration::ZERO } else { ms(350) };
        let viewport_w = viewport_size.map(|size| {
            glide(
                (id.clone(), "kk-viewport-w"),
                size.width,
                resize,
                curves.spring.clone(),
                window,
                cx,
            )
        });
        let viewport_h = viewport_size.map(|size| {
            glide(
                (id.clone(), "kk-viewport-h"),
                size.height,
                resize,
                curves.spring.clone(),
                window,
                cx,
            )
        });
        let panels: Vec<AnyElement> = panels
            .into_iter()
            .map(|panel| {
                let natural = sizes.get(&panel.ix).copied();
                let (w, h) = match (use_viewport, natural) {
                    (true, Some(_)) if open == Some(panel.ix) => (viewport_w, viewport_h),
                    (_, Some(natural)) => (Some(natural.width), Some(natural.height)),
                    _ => (None, None),
                };
                let x = if use_viewport {
                    px(0.)
                } else {
                    triggers
                        .get(&panel.ix)
                        .map_or(px(0.), |b| b.left() - root_left)
                };
                let record = state.clone();
                let ix = panel.ix;
                // Measure whenever the content is at rest, so one that changes size resizes the
                // panel; a scaled content would report its scaled size.
                let at_rest = (panel.pose.sx - 1.0).abs() < 1e-4;
                let content = div()
                    .relative()
                    .flex_none()
                    .p_2()
                    .pr_2p5()
                    .refine_style(&panel.content.style)
                    .children(panel.content.children)
                    .when(at_rest, |this| {
                        this.child(measure(move |bounds, window, cx| {
                            if record.update(cx, |state, _| state.record_size(ix, bounds.size)) {
                                window.request_animation_frame();
                            }
                        }))
                    });
                // A row that doesn't stretch the content to the panel: it lays out at its natural
                // size wherever the panel is, so that's what it measures.
                let content = div().flex().items_start().child(content);
                let menu = state.clone();
                let hover = state.clone();
                let keys = state.clone();
                let panel_box = match (w, h) {
                    (Some(w), Some(h)) => {
                        let s = panel.pose.sx;
                        let (sw, sh) = (w * s, h * s);
                        let shift = (w - sw) / 2.;
                        div()
                            .absolute()
                            .top_full()
                            .left(x)
                            .mt(px(6.) + px(panel.pose.y))
                            .w(w)
                            .h(h)
                            .opacity(panel.pose.alpha())
                            .child(
                                div()
                                    .id((id.clone(), SharedString::from(format!("panel-{ix}"))))
                                    .absolute()
                                    .left(shift)
                                    .top_0()
                                    .w(sw)
                                    .h(sh)
                                    .rounded_xl()
                                    .border_1()
                                    .border_color(theme.border)
                                    .bg(theme.popover)
                                    .text_color(theme.popover_foreground)
                                    .shadow_md()
                                    .overflow_hidden()
                                    .on_hover(move |hovering, _, cx| {
                                        hover.update(cx, |state, cx| {
                                            if *hovering {
                                                state.left_at = None;
                                            } else {
                                                state.leave(crate::motion::now());
                                            }
                                            cx.notify();
                                        })
                                    })
                                    .on_key_down(move |event: &KeyDownEvent, window, cx| {
                                        content_key(event, ix, &keys, window, cx)
                                    })
                                    .child(
                                        div().absolute().left(-shift).top(px(-1.)).w(w).child(
                                            transform(
                                                (
                                                    id.clone(),
                                                    SharedString::from(format!("content-{ix}")),
                                                ),
                                                Pose {
                                                    sx: s,
                                                    sy: s,
                                                    ..panel.inner
                                                },
                                                InMenu {
                                                    menu,
                                                    ix,
                                                    child: content.into_any_element(),
                                                },
                                            )
                                            .origin(0.5, 0.0),
                                        ),
                                    ),
                            )
                            .into_any_element()
                    }
                    // Not measured yet: lay it out unseen to measure it.
                    _ => {
                        window.request_animation_frame();
                        div()
                            .absolute()
                            .top_full()
                            .left(x)
                            .invisible()
                            .child(InMenu {
                                menu,
                                ix,
                                child: content.into_any_element(),
                            })
                            .into_any_element()
                    }
                };
                deferred(panel_box).with_priority(1).into_any_element()
            })
            .collect();
        let record = state.clone();
        div()
            .id(id)
            .role(Role::Navigation)
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .refine_style(&self.style)
            .children(lists)
            .children(panels)
            .child(measure(move |bounds, _, cx| {
                record.update(cx, |state, _| state.root = Some(bounds))
            }))
    }
}

/// An 8 px square turned 45°, its top corner rounded, filling a box `diagonal` wide.
fn diamond(diagonal: Pixels) -> String {
    let d = f32::from(diagonal);
    let h = d / 2.;
    let r = 1.41;
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {d} {d}" width="{d}" height="{d}"><path d="M{a} {r} Q{h} 0 {b} {r} L{d} {h} L{h} {d} L0 {h} Z" fill="black"/></svg>"#,
        a = h - r,
        b = h + r,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parity::assert_number_track;

    const C: &str = "pop-navigation-menu";

    #[test]
    fn keyframes_match_the_web() {
        assert_number_track(C, "kk-pop-navigation-menu-flip", "rotate", &flip());
        assert_number_track(C, "kk-pop-navigation-menu-in", "opacity", &in_opacity());
        assert_number_track(C, "kk-pop-navigation-menu-in", "sx", &in_scale());
        // The web drops in rem; the track is in rem too.
        assert_number_track(C, "kk-pop-navigation-menu-in", "y", &in_y());
        assert_number_track(C, "kk-pop-navigation-menu-out", "opacity", &out_opacity());
        assert_number_track(C, "kk-pop-navigation-menu-out", "sx", &out_scale());
        assert_number_track(C, "kk-pop-navigation-menu-out", "y", &out_y());
        assert_number_track(
            C,
            "kk-pop-navigation-menu-rise",
            "y",
            &rise_y(Easing::Linear),
        );
        assert_number_track(
            C,
            "kk-pop-navigation-menu-rise",
            "opacity",
            &rise_opacity(Easing::Linear),
        );
        assert_number_track(C, "kk-pop-navigation-menu-arrow", "sx", &arrow_scale());
        assert_number_track(C, "kk-pop-navigation-menu-fade-in", "opacity", &fade_in());
    }

    #[test]
    fn the_slide_passes_its_mark_by_a_twelfth() {
        // kk-pop-navigation-menu-slide-in translates by `var(--kk-pop-navigation-menu-shift)`,
        // which the parity parser can't read: check the multipliers by hand.
        assert_eq!(slide_x().sample(0.0), 1.0);
        assert!((slide_x().sample(0.65) + 1.0 / 12.0).abs() < 1e-6);
        assert_eq!(slide_opacity().sample(0.35), 1.0);
    }

    fn at(ms_: u64) -> Instant {
        // A fixed base so the arithmetic is exact.
        thread_local!(static BASE: Instant = Instant::now());
        BASE.with(|base| *base + ms(ms_))
    }

    #[test]
    fn hover_opens_after_a_delay_and_switches_at_once() {
        let mut state = NavState::default();
        state.enter_trigger(0, at(0));
        assert!(state.tick(at(150)));
        assert_eq!(state.open, None);
        state.tick(at(200));
        assert_eq!(state.open, Some(0));
        // While open, another trigger takes over at once, sliding from the end.
        state.enter_trigger(1, at(300));
        assert_eq!(state.open, Some(1));
        assert_eq!(state.switched.map(|(_, dir)| dir), Some(1.0));
    }

    #[test]
    fn content_sizes_follow_the_content_and_the_rem_size() {
        let mut state = NavState::default();
        state.forget_stale_sizes((px(16.), 3));
        assert!(state.record_size(0, size(px(400.), px(200.))));
        assert!(!state.record_size(0, size(px(400.), px(200.))));
        // Content that grows is measured again.
        assert!(state.record_size(0, size(px(400.), px(260.))));
        state.forget_stale_sizes((px(16.), 3));
        assert_eq!(state.contents.len(), 1);
        // Another rem size or another list of items starts over.
        state.forget_stale_sizes((px(20.), 3));
        assert!(state.contents.is_empty());
        state.record_size(0, size(px(500.), px(325.)));
        state.forget_stale_sizes((px(20.), 4));
        assert!(state.contents.is_empty());
    }

    #[test]
    fn keys_rove_like_radix() {
        let key = |text: &str| Rove::from_key(&Keystroke::parse(text).unwrap());
        // Both arrows of each axis move, Home and End jump; modified keys are left alone.
        assert_eq!(key("right"), Some(Rove::Next));
        assert_eq!(key("down"), Some(Rove::Next));
        assert_eq!(key("left"), Some(Rove::Previous));
        assert_eq!(key("up"), Some(Rove::Previous));
        assert_eq!(key("home"), Some(Rove::First));
        assert_eq!(key("end"), Some(Rove::Last));
        assert_eq!(key("ctrl-right"), None);
        assert_eq!(key("shift-home"), None);
        assert_eq!(key("enter"), None);
        assert_eq!(key("tab"), None);
    }

    #[test]
    fn arrows_stop_at_the_ends() {
        assert_eq!(Rove::Next.target(Some(0), 3), Some(1));
        assert_eq!(Rove::Next.target(Some(2), 3), None);
        assert_eq!(Rove::Previous.target(Some(1), 3), Some(0));
        assert_eq!(Rove::Previous.target(Some(0), 3), None);
        assert_eq!(Rove::First.target(Some(2), 3), Some(0));
        assert_eq!(Rove::Last.target(Some(0), 3), Some(2));
        // From outside the row, the arrows enter at its near end.
        assert_eq!(Rove::Next.target(None, 3), Some(0));
        assert_eq!(Rove::Previous.target(None, 3), Some(2));
        assert_eq!(Rove::Next.target(None, 0), None);
    }

    struct Host {
        clicks: Rc<std::cell::Cell<usize>>,
    }

    fn tagged(tag: &'static str) -> gpui_kit::Div {
        div().debug_selector(move || tag.into()).child(tag)
    }

    impl gpui_kit::Render for Host {
        fn render(&mut self, _: &mut Window, _: &mut gpui_kit::Context<Self>) -> impl IntoElement {
            let clicks = self.clicks.clone();
            let item = |name: &'static str, links: [&'static str; 2]| {
                NavigationMenuItem::new()
                    .child(NavigationMenuTrigger::new().child(tagged(name)))
                    .child(NavigationMenuContent::new().children(
                        links.map(|link| NavigationMenuLink::new(link).child(tagged(link))),
                    ))
            };
            div().size_full().p_4().child(
                NavigationMenu::new("nav").child(
                    NavigationMenuList::new()
                        .child(
                            NavigationMenuItem::new()
                                .child(NavigationMenuTrigger::new().child(tagged("A")))
                                .child(
                                    NavigationMenuContent::new()
                                        .child(
                                            NavigationMenuLink::new("a1")
                                                .on_click(move |_, _, _| {
                                                    clicks.set(clicks.get() + 1)
                                                })
                                                .child(tagged("a1")),
                                        )
                                        .child(NavigationMenuLink::new("a2").child(tagged("a2"))),
                                ),
                        )
                        .child(item("B", ["b1", "b2"]))
                        .child(
                            NavigationMenuItem::new().child(
                                NavigationMenuLink::new("docs")
                                    .child(tagged("docs"))
                                    .on_click(|_, _, _| {}),
                            ),
                        ),
                ),
            )
        }
    }

    /// The arrows walk the triggers and the list's link, the down arrow enters the open content,
    /// and Escape closes it with focus back on its trigger.
    #[gpui_kit::test]
    fn the_keyboard_walks_the_menu(cx: &mut gpui_kit::TestAppContext) {
        cx.update(|cx| {
            gpui_kit::component::init(cx);
            crate::init(cx);
        });
        let start = Instant::now();
        let clicks = Rc::new(std::cell::Cell::new(0));
        let host_clicks = clicks.clone();
        let (_, cx) = cx.add_window_view(|_, _| Host {
            clicks: host_clicks,
        });
        let mut time = 0;
        let mut settle = |cx: &mut gpui_kit::VisualTestContext| {
            // Long enough for every motion to finish; the contents measure themselves on the way.
            for _ in 0..4 {
                time += 1000;
                crate::motion::freeze_time(Some(start + ms(time)));
                cx.update(|window, cx| window.draw(cx).clear(cx));
            }
        };
        let press = |cx: &mut gpui_kit::VisualTestContext, key: &str| {
            cx.simulate_keystrokes(key);
            cx.simulate_event(gpui_kit::KeyUpEvent {
                keystroke: Keystroke::parse(key).unwrap(),
            });
        };
        let shows = |cx: &mut gpui_kit::VisualTestContext, tag: &'static str| {
            cx.debug_bounds(tag).is_some()
        };
        settle(cx);
        // Tab reaches the first trigger; the right arrow moves to the second, Enter opens it.
        cx.update(|window, cx| window.focus_next(cx));
        press(cx, "right");
        press(cx, "enter");
        settle(cx);
        assert!(shows(cx, "b1"), "B didn't open from the keyboard");
        assert!(!shows(cx, "a1"));
        // Escape closes it and leaves focus on B: Enter opens it again.
        press(cx, "escape");
        settle(cx);
        assert!(!shows(cx, "b1"));
        press(cx, "enter");
        settle(cx);
        assert!(shows(cx, "b1"), "focus didn't stay on B");
        // Home goes back to A while B stays open; the down arrow does nothing on a closed
        // trigger but move along, so open A first.
        press(cx, "escape");
        press(cx, "home");
        press(cx, "enter");
        settle(cx);
        assert!(shows(cx, "a1"), "Home didn't reach A");
        // Down enters A's content, on its first link; Enter follows it and closes the menu.
        press(cx, "down");
        press(cx, "enter");
        settle(cx);
        assert_eq!(
            clicks.get(),
            1,
            "the down arrow didn't focus the first link"
        );
        assert!(!shows(cx, "a1"));
        // Focus came back to A: open it, walk to the second link and back, and escape.
        press(cx, "enter");
        settle(cx);
        press(cx, "down");
        press(cx, "end");
        press(cx, "up");
        press(cx, "enter");
        settle(cx);
        assert_eq!(
            clicks.get(),
            2,
            "the arrows didn't walk the content's links"
        );
        // End reaches the list's link past the last trigger, and the arrows stop at the ends.
        press(cx, "end");
        press(cx, "right");
        press(cx, "left");
        press(cx, "enter");
        settle(cx);
        assert!(shows(cx, "b1"), "Left from the link didn't reach B");
        crate::motion::freeze_time(None);
    }

    #[test]
    fn leaving_closes_after_a_grace_period() {
        let mut state = NavState::default();
        state.open_item(0, at(0));
        state.leave(at(100));
        state.tick(at(200));
        assert_eq!(state.open, Some(0));
        state.tick(at(250));
        assert_eq!(state.open, None);
        assert_eq!(state.closing.map(|(ix, _)| ix), Some(0));
    }
}
