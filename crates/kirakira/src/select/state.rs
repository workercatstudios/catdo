//! `SelectState` and `Select`, ported from GPUI Component's `select.rs` with the motion added.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use gpui_kit::base::ElementExt as _;
use gpui_kit::base::actions::Cancel;
use gpui_kit::base::{
    Align, DeferredPopover, GlobalState, POPUP_PRIORITY, Positioner, Select as BaseSelect,
    TestSupportExt as _,
};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::list::{List, ListState};
use gpui_kit::component::searchable_list::{
    SearchableListChange, SearchableListDelegate, SearchableListItem,
};
use gpui_kit::component::{
    ActiveTheme as _, Colorize as _, Disableable as _, FocusableExt, Icon, IconName, IndexPath,
    Placement, Sizable, Size, StyleSized as _, StyledExt as _, ThemeStyled as _, h_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AbsoluteLength, AnyElement, App, AppContext as _, Bounds, ClickEvent, Context, DefiniteLength,
    DismissEvent, ElementId, Entity, EventEmitter, FocusHandle, Focusable, Hsla,
    InteractiveElement as _, IntoElement, Length, ParentElement as _, Pixels, Render, RenderOnce,
    SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled, Subscription,
    Transformation, Window, deferred, div, px, radians, rems,
};

use super::adapter::Adapter;
use super::motion::{BACK, FLIP, back_angle, durations, flip_angle, panel_pose};
use super::panel::panel;
use crate::overlay::{Inert, Phase, Presence, Side};
use crate::theme::ActiveKira as _;

/// GPUI Component's `Select.placeholder` in English: its translations are private to it.
const PLACEHOLDER: &str = "Please select";

/// Events emitted by [`SelectState`].
pub enum SelectEvent<D: SearchableListDelegate + 'static>
where
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    Confirm(Option<<D::Item as SearchableListItem>::Value>),
}

// MARK: SelectOptions (builder only — applied to the state during render)

struct SelectOptions {
    style: StyleRefinement,
    size: Size,
    icon: Option<Icon>,
    cleanable: bool,
    placeholder: Option<SharedString>,
    accessibility_label: Option<SharedString>,
    title_prefix: Option<SharedString>,
    search_placeholder: Option<SharedString>,
    menu_width: Length,
    menu_max_h: Length,
    disabled: bool,
    appearance: bool,
    focus_ring_enabled: bool,
}

impl Default for SelectOptions {
    fn default() -> Self {
        Self {
            style: StyleRefinement::default(),
            size: Size::default(),
            icon: None,
            cleanable: false,
            placeholder: None,
            accessibility_label: None,
            title_prefix: None,
            menu_width: Length::Auto,
            menu_max_h: rems(20.).into(),
            disabled: false,
            appearance: true,
            focus_ring_enabled: true,
            search_placeholder: None,
        }
    }
}

/// What the motion keeps between frames.
#[derive(Default)]
struct Motion {
    /// When the list last opened or closed, on Kirakira's clock.
    presence: Presence,
    /// The panel's size at rest, measured on the first frame of each open: the plate scales from
    /// it.
    natural: Option<gpui_kit::Size<Pixels>>,
    /// Where the panel was last painted, to tell which side of the trigger it landed on.
    surface: Option<Bounds<Pixels>>,
    /// The chevron's angle when the list closed: it turns back from there.
    back_from: f32,
    /// When the list last opened, while it's open; its rows read it.
    opened: Rc<Cell<Option<Instant>>>,
}

// MARK: SelectState

/// State of the [`Select`] component.
pub struct SelectState<D: SearchableListDelegate + 'static>
where
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    focus_handle: FocusHandle,
    list: Entity<ListState<Adapter<D>>>,
    /// The list's own focus handle and its search field's: focus in either keeps it open.
    list_focus: FocusHandle,
    search_focus: FocusHandle,
    selection: Vec<(IndexPath, D::Item)>,
    open: bool,
    /// Held while the popup is open, so a select dropped without closing takes its registration
    /// with it.
    deferred_context: Option<DeferredPopover>,
    bounds: Bounds<Pixels>,

    // Options, from the `Select` element.
    size: Size,
    style: StyleRefinement,
    cleanable: bool,
    placeholder: Option<SharedString>,
    search_placeholder: Option<SharedString>,
    menu_width: Length,
    menu_max_h: Length,
    disabled: bool,
    appearance: bool,
    empty: Option<Box<dyn Fn(&mut Window, &App) -> AnyElement + 'static>>,

    searchable: bool,
    list_searchable: Option<bool>,
    icon: Option<Icon>,
    title_prefix: Option<SharedString>,
    focus_ring_enabled: bool,

    motion: Motion,
    _subscriptions: Vec<Subscription>,
}

/// A select bound to a [`SelectState`].
#[derive(IntoElement)]
pub struct Select<D: SearchableListDelegate + 'static>
where
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    id: ElementId,
    state: Entity<SelectState<D>>,
    options: SelectOptions,
    empty: Option<Box<dyn Fn(&mut Window, &App) -> AnyElement + 'static>>,
}

/// Clears the list's search query, if a search has run, so it shows every item again.
fn clear_list_query<D: SearchableListDelegate + 'static>(
    list: &mut ListState<Adapter<D>>,
    window: &mut Window,
    cx: &mut Context<ListState<Adapter<D>>>,
) {
    if list.delegate().searched {
        list.set_query("", window, cx);
        list.delegate_mut().searched = false;
    }
}

/// Sets the list's cursor and scrolls to it.
fn move_cursor<D: SearchableListDelegate + 'static>(
    list: &mut ListState<Adapter<D>>,
    ix: Option<IndexPath>,
    window: &mut Window,
    cx: &mut Context<ListState<Adapter<D>>>,
) {
    list.set_selected_index(ix, window, cx);
    list.scroll_to_selected_item(window, cx);
}

impl<D> SelectState<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    /// Create a new Select state.
    pub fn new(
        delegate: D,
        selected_index: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let weak = cx.entity().downgrade();
        let weak_confirm = weak.clone();
        let weak_cancel = weak.clone();
        let weak_empty = weak;

        let selected_indices = selected_index.into_iter().collect::<Vec<_>>();
        let motion = Motion::default();

        let adapter = Adapter::new(
            delegate,
            // on_confirm — commit the selection
            move |selected_index, _secondary, window, cx| {
                cx.defer_in(window, {
                    let weak_confirm = weak_confirm.clone();
                    move |list_state, window, cx| {
                        let mut selection = weak_confirm
                            .upgrade()
                            .map(|e| e.read(cx).selection.clone())
                            .unwrap_or_default();

                        let mut changes: Vec<SearchableListChange> = selection
                            .iter()
                            .map(|(ix, _)| SearchableListChange::Deselect { index: *ix })
                            .collect();
                        if let Some(ix) = selected_index {
                            changes.push(SearchableListChange::Select { index: ix });
                        }

                        // Called directly: an entity-handle update would re-enter the list's
                        // lock that `defer_in` holds for this callback.
                        list_state
                            .delegate_mut()
                            .delegate
                            .on_will_change(&mut selection, &changes);

                        let confirmed = weak_confirm.update(cx, |this, cx| {
                            this.selection = selection;
                            let final_value =
                                this.selection.first().map(|(_, i)| i.value().clone());
                            cx.emit(SelectEvent::Confirm(final_value.clone()));
                            cx.notify();
                            this.set_open(false, cx);
                            this.focus(window, cx);
                            (this.selection.clone(), final_value)
                        });

                        // The committed index pointed into the filtered view, so resolve the value
                        // in the restored full list; otherwise the cursor and the mark would
                        // disagree on the next open.
                        if let Ok((mut new_selection, final_value)) = confirmed {
                            clear_list_query(list_state, window, cx);

                            if let Some(ix) = final_value
                                .as_ref()
                                .and_then(|value| list_state.delegate().delegate.position(value))
                            {
                                list_state.set_selected_index(Some(ix), window, cx);
                                if let Some((slot, _)) = new_selection.first_mut() {
                                    *slot = ix;
                                }
                                _ = weak_confirm.update(cx, |this, cx| {
                                    if let Some((slot, _)) = this.selection.first_mut() {
                                        *slot = ix;
                                    }
                                    cx.notify();
                                });
                            }

                            list_state
                                .delegate_mut()
                                .update_selection_snapshot(new_selection.clone());
                            list_state
                                .delegate_mut()
                                .delegate
                                .on_confirm(&new_selection);
                        }
                    }
                });
            },
            // on_cancel — clear the query, restore the cursor to the committed row, close
            move |_, window, cx| {
                cx.defer_in(window, {
                    let weak_cancel = weak_cancel.clone();
                    move |list_state, window, cx| {
                        let committed_ix = weak_cancel
                            .upgrade()
                            .and_then(|e| e.read(cx).selection.first().map(|(ix, _)| *ix));

                        clear_list_query(list_state, window, cx);
                        list_state.set_selected_index(committed_ix, window, cx);

                        _ = weak_cancel.update(cx, |this, cx| {
                            this.set_open(false, cx);
                            this.focus(window, cx);
                        });
                    }
                });
            },
            // on_render_empty
            move |window, cx| {
                if let Some(empty) = weak_empty
                    .upgrade()
                    .and_then(|e| e.read(cx).empty.as_ref().map(|f| f(window, cx)))
                {
                    empty
                } else {
                    h_flex()
                        .justify_center()
                        .py_6()
                        .text_color(cx.theme().muted_foreground.opacity(0.6))
                        .child(Icon::new(IconName::Inbox).size(px(28.)))
                        .into_any_element()
                }
            },
            motion.opened.clone(),
        );

        // The list's focus handle depends on whether it's searchable; take both.
        let list = cx.new(|cx| ListState::new(adapter, window, cx).searchable(true));
        let search_focus = list.read(cx).focus_handle(cx);
        list.update(cx, |list, cx| list.set_searchable(false, cx));
        let list_focus = list.read(cx).focus_handle(cx);

        let selection = {
            let delegate = &list.read(cx).delegate().delegate;
            selected_indices
                .iter()
                .copied()
                .filter_map(|ix| delegate.item(ix).map(|i| (ix, i.clone())))
                .collect::<Vec<_>>()
        };

        if let Some(cursor) = selected_indices.first().copied() {
            list.update(cx, |list, cx| {
                list.set_selected_index(Some(cursor), window, cx)
            });
        }

        // Prime the snapshot so the very first render sees the right check.
        let snapshot = selection.clone();
        list.update(cx, |list, _| {
            list.delegate_mut().update_selection_snapshot(snapshot)
        });

        let focus_handle = cx.focus_handle();
        let _subscriptions = vec![
            cx.on_blur(&list_focus, window, Self::on_blur),
            cx.on_blur(&search_focus, window, Self::on_blur),
            cx.on_blur(&focus_handle, window, Self::on_blur),
        ];

        Self {
            focus_handle,
            list,
            list_focus,
            search_focus,
            selection,
            open: false,
            deferred_context: None,
            bounds: Bounds::default(),
            size: Size::default(),
            style: StyleRefinement::default(),
            cleanable: false,
            placeholder: None,
            search_placeholder: None,
            menu_width: Length::Auto,
            menu_max_h: rems(20.).into(),
            disabled: false,
            appearance: true,
            empty: None,
            searchable: false,
            list_searchable: Some(false),
            icon: None,
            title_prefix: None,
            focus_ring_enabled: true,
            motion,
            _subscriptions,
        }
    }

    /// Sets whether the dropdown menu is searchable, default is `false`.
    ///
    /// When `true`, a search input appears at the top of the dropdown menu.
    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }

    /// Set the selected index for the select.
    pub fn set_selected_index(
        &mut self,
        selected_index: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.list.update(cx, |list, cx| {
            move_cursor(list, selected_index, window, cx);
        });

        let item = selected_index
            .and_then(|ix| self.list.read(cx).delegate().delegate.item(ix))
            .cloned();

        self.selection = match (selected_index, item) {
            (Some(ix), Some(item)) => vec![(ix, item)],
            _ => vec![],
        };
        self.sync_snapshot(cx);
    }

    /// Set selected value for the select.
    ///
    /// Looks up the position from the delegate and sets the selected index accordingly.
    /// Passes `None` when the value is not found.
    ///
    /// The delegate looks the value up in its matched items, so an active search query is
    /// cleared first to get an index into the full item list.
    pub fn set_selected_value(
        &mut self,
        selected_value: &<D::Item as SearchableListItem>::Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.clear_query(window, cx);

        let selected_index = self
            .list
            .read(cx)
            .delegate()
            .delegate
            .position(selected_value);

        self.set_selected_index(selected_index, window, cx);
    }

    /// Replace the delegate (item data) for the select state.
    pub fn set_items(&mut self, items: D, _: &mut Window, cx: &mut Context<Self>)
    where
        D: SearchableListDelegate + 'static,
    {
        self.list.update(cx, |list, _| {
            list.delegate_mut().delegate = items;
        });
    }

    /// Get the current selected index.
    pub fn selected_index(&self, cx: &App) -> Option<IndexPath> {
        self.list.read(cx).selected_index()
    }

    /// Get the current selected value.
    pub fn selected_value(&self) -> Option<&<D::Item as SearchableListItem>::Value> {
        self.selection.first().map(|(_, i)| i.value())
    }

    /// Focus the select trigger input.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.focus_handle.focus(window, cx);
    }

    /// Push the selection into the adapter's snapshot so the next render sees the right check.
    fn sync_snapshot(&self, cx: &mut Context<Self>) {
        let snapshot = self.selection.clone();
        self.list.update(cx, |list, _| {
            list.delegate_mut().update_selection_snapshot(snapshot);
        });
    }

    fn clear_query(&mut self, window: &mut Window, cx: &mut App) {
        self.list
            .update(cx, |list, cx| clear_list_query(list, window, cx));
    }

    fn on_blur(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.list_focus.is_focused(window)
            || self.search_focus.is_focused(window)
            || self.focus_handle.is_focused(window)
        {
            return;
        }

        self.clear_query_and_restore_cursor(window, cx);
        self.set_open(false, cx);
        cx.notify();
    }

    fn toggle_menu(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();

        self.set_open(!self.open, cx);

        if self.open {
            self.list.focus_handle(cx).focus(window, cx);
        } else {
            self.clear_query_and_restore_cursor(window, cx);
        }

        cx.notify();
    }

    fn escape(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        if !self.open {
            cx.propagate();
            return;
        }

        cx.stop_propagation();
        self.clear_query_and_restore_cursor(window, cx);
        self.set_open(false, cx);
        self.focus(window, cx);
        cx.notify();
    }

    /// Drop the search query and move the cursor back to the committed selection, so the next
    /// open shows every item. Call on every close that doesn't go through confirm or cancel.
    fn clear_query_and_restore_cursor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.clear_query(window, cx);

        let committed_ix = self.selection.first().map(|(ix, _)| *ix);
        self.list.update(cx, |list, cx| {
            if list.selected_index() != committed_ix {
                list.set_selected_index(committed_ix, window, cx);
            }
        });
    }

    fn set_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if open != self.open {
            let now = crate::motion::now();
            if open {
                // Measure the panel afresh: its rows may have changed since it last rested.
                self.motion.natural = None;
            } else {
                self.motion.back_from = self.chevron_angle(now, cx.reduce_motion());
            }
            self.motion.presence.set(open, now);
            // Not through the list entity: confirm and cancel close the list from callbacks that
            // hold its lock.
            self.motion.opened.set(open.then_some(now));
        }
        self.open = open;
        self.deferred_context = open.then(|| GlobalState::register_deferred_popover(cx));

        cx.notify();
    }

    /// The chevron's rotation in degrees at `now`: it flips past half a turn as the list opens and
    /// turns back as it closes.
    fn chevron_angle(&self, now: Instant, reduced: bool) -> f32 {
        let presence = &self.motion.presence;
        if reduced {
            return if presence.is_open() { 180.0 } else { 0.0 };
        }
        match presence.phase(now, FLIP, BACK) {
            Phase::Opening(elapsed) => flip_angle(elapsed),
            Phase::Open => 180.0,
            Phase::Closing(elapsed) => back_angle(self.motion.back_from, elapsed),
            Phase::Closed => 0.0,
        }
    }

    fn clean(&mut self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
        self.set_selected_index(None, window, cx);
        cx.emit(SelectEvent::Confirm(None));
    }

    fn display_title(&mut self, _: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let default_title = div().text_color(cx.theme().muted_foreground).child(
            self.placeholder
                .clone()
                .unwrap_or_else(|| PLACEHOLDER.into()),
        );

        let Some(selected_index) = self.selected_index(cx) else {
            return default_title;
        };

        let Some(title) = self
            .list
            .read(cx)
            .delegate()
            .delegate
            .item(selected_index)
            .map(|item| {
                if let Some(el) = item.display_title() {
                    el
                } else if let Some(prefix) = self.title_prefix.as_ref() {
                    format!("{}{}", prefix, item.title()).into_any_element()
                } else {
                    item.title().into_any_element()
                }
            })
        else {
            return default_title;
        };

        div()
            .when(self.disabled, |this| {
                this.text_color(cx.theme().muted_foreground)
            })
            .child(title)
    }

    fn accessibility_value(&self) -> SharedString {
        let Some((_, item)) = self.selection.first() else {
            return self
                .placeholder
                .clone()
                .unwrap_or_else(|| PLACEHOLDER.into());
        };

        if let Some(prefix) = self.title_prefix.as_ref() {
            format!("{}{}", prefix, item.title()).into()
        } else {
            item.title()
        }
    }

    /// The dropdown, while it's open or playing its exit.
    fn render_popup(
        &mut self,
        phase: Phase,
        reduced: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let bounds = self.bounds;
        let interactive = self.open;
        // Sizes in rems, so the panel's contents scale with it.
        let rem = window.rem_size();
        let width = match self.menu_width {
            Length::Auto => rems((bounds.size.width + px(2.)) / rem).into(),
            Length::Definite(width) => in_rems(width, rem),
        };
        let max_h = match self.menu_max_h {
            Length::Definite(max_h) => Length::Definite(in_rems(max_h, rem)),
            auto => auto,
        };

        let mut pose = panel_pose(phase, reduced, cx.curves().r#in);
        let natural = self.motion.natural;
        if natural.is_none() {
            // The first frame of an open: lay out unscaled and hidden, to measure.
            pose.sx = 1.0;
            pose.sy = 1.0;
            pose.opacity = 0.0;
            window.request_animation_frame();
        }
        let side = self
            .motion
            .surface
            .and_then(|surface| Side::measure(bounds, surface))
            .unwrap_or(Side::Bottom);
        // From the middle of the trigger-facing edge, like the web's popper.
        let origin = side.origin(0.5);

        let content = div()
            .w(width)
            .text_color(cx.theme().popover_foreground)
            .child(
                List::new(&self.list)
                    .when_some(self.search_placeholder.clone(), |this, placeholder| {
                        this.search_placeholder(placeholder)
                    })
                    .with_size(self.size)
                    .max_h(max_h)
                    .p(rems(0.25)),
            );

        let state = cx.entity();
        let measure = pose.unscaled();
        let panel = panel(
            ("kk-select-list", cx.entity_id()),
            pose,
            natural,
            origin,
            content,
            cx,
        )
        .on_prepaint(move |bounds, _, cx| {
            state.update(cx, |this, _| {
                this.motion.surface = Some(bounds);
                if measure {
                    this.motion.natural = Some(bounds.size);
                }
            })
        });
        let panel = if interactive {
            div()
                .id("kk-select-panel")
                .occlude()
                .on_mouse_down_out(cx.listener(|this, _, window, cx| {
                    this.escape(&Cancel, window, cx);
                }))
                .child(panel)
                .into_any_element()
        } else {
            // Closing: painted, but the pointer goes through to whatever is underneath.
            Inert::new(panel).into_any_element()
        };

        deferred(
            Positioner::side(bounds)
                .placement(Placement::Bottom)
                .align(Align::Start)
                .offset(px(6.))
                .margin(px(8.))
                .child(panel),
        )
        .with_priority(POPUP_PRIORITY)
    }
}

/// `length` in rems if it's in pixels, so it scales with the rem size.
fn in_rems(length: DefiniteLength, rem: Pixels) -> DefiniteLength {
    match length {
        DefiniteLength::Absolute(AbsoluteLength::Pixels(pixels)) => rems(pixels / rem).into(),
        other => other,
    }
}

/// GPUI Component's input colours: background and text.
fn input_style(disabled: bool, cx: &App) -> (Hsla, Hsla) {
    if disabled {
        (
            cx.theme().input.mix_oklab(cx.theme().transparent, 0.8),
            cx.theme().muted_foreground,
        )
    } else {
        (cx.theme().input_background(), cx.theme().foreground)
    }
}

/// GPUI Component's caret size for a select of `size`.
fn caret_size(size: Size) -> Size {
    match size {
        Size::XSmall => Size::XSmall,
        Size::Small => Size::Small,
        _ => Size::Medium,
    }
}

impl<D> Render for SelectState<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let now = crate::motion::now();
        let reduced = cx.reduce_motion();
        let searchable = self.searchable;
        let is_focused = self.focus_handle.is_focused(window);
        let show_clean = self.cleanable && self.selected_index(cx).is_some();
        let allow_open = !(self.open || self.disabled);
        let outline_visible = self.open || (is_focused && !self.disabled);

        let (bg, fg) = input_style(self.disabled, cx);

        let list_searchable = self.list_searchable.replace(searchable);
        let size = self.size;
        self.list.update(cx, |list, cx| {
            if list_searchable != Some(searchable) {
                list.set_searchable(searchable, cx);
            }
            list.delegate_mut().size = size;
        });

        let (enter, exit) = durations(reduced);
        let phase = self.motion.presence.phase(now, enter, exit);
        let turning = !reduced && self.motion.presence.phase(now, FLIP, BACK).running();
        if phase.running() || turning {
            window.request_animation_frame();
        }
        let angle = self.chevron_angle(now, reduced);
        let rotation = Transformation::rotate(radians(angle.to_radians()));

        div().size_full().relative().child(
            div()
                .relative()
                .on_prepaint({
                    let state = cx.entity();
                    move |bounds, _, cx| state.update(cx, |r, _| r.bounds = bounds)
                })
                .child(
                    div()
                        .id("input")
                        .test_support()
                        .relative()
                        .flex()
                        .items_center()
                        .justify_between()
                        .border_1()
                        .border_color(cx.theme().transparent)
                        .when(self.appearance, |this| {
                            this.bg(bg)
                                .text_color(fg)
                                .when(self.disabled, |this| this.opacity(0.5))
                                .border_color(cx.theme().input)
                                .rounded(cx.theme().radius)
                        })
                        .input_size(self.size)
                        .input_text_size(self.size)
                        .refine_style(&self.style)
                        .when(outline_visible && self.appearance, |this| {
                            this.border_1().border_color(cx.theme().ring)
                        })
                        .when(
                            outline_visible && self.appearance && self.focus_ring_enabled,
                            |this| this.focus_ring_style(window, cx),
                        )
                        .when(allow_open, |this| {
                            this.on_click(cx.listener(Self::toggle_menu))
                        })
                        .child(
                            h_flex()
                                .id("inner")
                                .w_full()
                                .min_w_0()
                                .overflow_hidden()
                                .whitespace_nowrap()
                                .items_center()
                                .justify_between()
                                .gap_1()
                                .child(
                                    div()
                                        .id("title")
                                        .flex_1()
                                        .min_w_0()
                                        .overflow_hidden()
                                        .whitespace_nowrap()
                                        .truncate()
                                        .child(self.display_title(window, cx)),
                                )
                                .when(show_clean, |this| {
                                    let clean = Button::new("clean")
                                        .icon(Icon::new(IconName::Close))
                                        .text()
                                        .xsmall()
                                        .tab_stop(false);
                                    this.child(if self.disabled {
                                        clean.disabled(true)
                                    } else {
                                        clean.on_click(cx.listener(Self::clean))
                                    })
                                })
                                .when(!show_clean, |this| {
                                    let icon = match self.icon.clone() {
                                        Some(icon) => icon.xsmall(),
                                        None => Icon::new(IconName::ChevronDown)
                                            .with_size(caret_size(self.size)),
                                    };
                                    this.child(
                                        icon.text_color(cx.theme().muted_foreground)
                                            .transform(rotation),
                                    )
                                }),
                        ),
                )
                .when(phase != Phase::Closed, |this| {
                    this.child(self.render_popup(phase, reduced, window, cx))
                }),
        )
    }
}

impl<D> Select<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    pub fn new(state: &Entity<SelectState<D>>) -> Self {
        Self {
            id: ("select", state.entity_id()).into(),
            state: state.clone(),
            options: SelectOptions::default(),
            empty: None,
        }
    }

    /// Sets an explicit identity for the select root.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the width of the dropdown menu, default: `Length::Auto`.
    pub fn menu_width(mut self, width: impl Into<Length>) -> Self {
        self.options.menu_width = width.into();
        self
    }

    /// Set the max height of the dropdown menu, default: 20rem.
    pub fn menu_max_h(mut self, max_h: impl Into<Length>) -> Self {
        self.options.menu_max_h = max_h.into();
        self
    }

    /// Set the placeholder shown when no value is selected.
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.options.placeholder = Some(placeholder.into());
        self
    }

    /// Set the name a screen reader announces for the select.
    ///
    /// The placeholder and selected value are not used as the accessible name, because they
    /// describe the current value rather than the control itself.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.options.accessibility_label = Some(label.into());
        self
    }

    /// Override the trailing icon, replacing the default chevron. It flips the same way.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.options.icon = Some(icon.into());
        self
    }

    /// Set a label prefix shown before the selected title in the trigger.
    ///
    /// e.g. `title_prefix("Country: ")` → "Country: United States"
    pub fn title_prefix(mut self, prefix: impl Into<SharedString>) -> Self {
        self.options.title_prefix = Some(prefix.into());
        self
    }

    /// Show a clear button when a value is selected.
    pub fn cleanable(mut self, cleanable: bool) -> Self {
        self.options.cleanable = cleanable;
        self
    }

    /// Set the placeholder text for the search input.
    pub fn search_placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.options.search_placeholder = Some(placeholder.into());
        self
    }

    /// Set the disabled state.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.options.disabled = disabled;
        self
    }

    /// Set a custom closure that renders the empty-state element.
    pub fn empty<E: IntoElement + 'static>(
        mut self,
        builder: impl Fn(&mut Window, &App) -> E + 'static,
    ) -> Self {
        self.empty = Some(Box::new(move |window, cx| {
            builder(window, cx).into_any_element()
        }));
        self
    }

    /// Control whether the trigger shows a border and background (`true` by default).
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.options.appearance = appearance;
        self
    }
}

impl<D> Sizable for Select<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.options.size = size.into();
        self
    }
}

impl<D> FocusableExt for Select<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    fn focus_ring(mut self, enabled: bool) -> Self {
        self.options.focus_ring_enabled = enabled;
        self
    }

    fn is_focus_ring_enabled(&self) -> bool {
        self.options.focus_ring_enabled
    }
}

impl<D> EventEmitter<SelectEvent<D>> for SelectState<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
}

impl<D> EventEmitter<DismissEvent> for SelectState<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
}

impl<D> Focusable for SelectState<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        if self.open {
            self.list.focus_handle(cx)
        } else {
            self.focus_handle.clone()
        }
    }
}

impl<D> Styled for Select<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.options.style
    }
}

impl<D> RenderOnce for Select<D>
where
    D: SearchableListDelegate + 'static,
    <D::Item as SearchableListItem>::Value: PartialEq + Clone,
{
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let disabled = self.options.disabled;
        let accessibility_label = self.options.accessibility_label.clone();
        let focus_handle = self.state.read(cx).focus_handle.clone();
        let empty = self.empty;
        let opts = self.options;

        self.state.update(cx, |this, _| {
            this.style = opts.style;
            this.size = opts.size;
            this.cleanable = opts.cleanable;
            this.placeholder = opts.placeholder;
            this.search_placeholder = opts.search_placeholder;
            this.menu_width = opts.menu_width;
            this.menu_max_h = opts.menu_max_h;
            this.disabled = opts.disabled;
            this.appearance = opts.appearance;
            this.focus_ring_enabled = opts.focus_ring_enabled;
            this.icon = opts.icon;
            this.title_prefix = opts.title_prefix;

            if let Some(empty) = empty {
                this.empty = Some(empty);
            }
        });

        let is_open = self.state.read(cx).open;
        let accessibility_value = self.state.read(cx).accessibility_value();
        let content_focus_handle = self.state.read(cx).list.focus_handle(cx);
        let open_state = self.state.clone();

        BaseSelect::new(self.id)
            .open(is_open)
            .disabled(disabled)
            .when_some(accessibility_label, |this, label| {
                this.accessibility_label(label)
            })
            .focus_handle(&focus_handle)
            .content_focus_handle(&content_focus_handle)
            .accessibility_value(accessibility_value)
            .on_open_change(move |open, window, cx| {
                open_state.update(cx, |state, cx| {
                    if !open {
                        state.clear_query_and_restore_cursor(window, cx);
                    }
                    state.set_open(open, cx);
                });
            })
            .size_full()
            .child(self.state)
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::component::IndexPath;
    use gpui_kit::component::searchable_list::{SearchableListDelegate as _, SearchableVec};
    use gpui_kit::{AppContext as _, RenderOnce as _, TestAppContext};

    use super::{Select, SelectState};
    use crate::select::SelectGroup;

    fn init(cx: &mut TestAppContext) {
        cx.update(|cx| {
            gpui_kit::init(cx);
            crate::init(cx);
        });
    }

    type Languages = SelectState<SearchableVec<&'static str>>;

    struct Host {
        state: gpui_kit::Entity<Languages>,
    }

    impl gpui_kit::Render for Host {
        fn render(
            &mut self,
            _: &mut gpui_kit::Window,
            _: &mut gpui_kit::Context<Self>,
        ) -> impl gpui_kit::IntoElement {
            use gpui_kit::{ParentElement as _, Styled as _};
            gpui_kit::div().size_full().child(
                gpui_kit::div()
                    .w(gpui_kit::px(200.))
                    .child(Select::new(&self.state)),
            )
        }
    }

    /// Confirm and cancel close the list from inside the list's own callbacks, and the list
    /// stays painted for its exit: none of it may re-enter an entity that's being updated.
    #[gpui_kit::test]
    fn the_keyboard_opens_confirms_and_cancels(cx: &mut TestAppContext) {
        init(cx);
        let (host, cx) = cx.add_window_view(|window, cx| {
            let items = SearchableVec::new(vec!["Rust", "Go", "C++"]);
            Host {
                state: cx.new(|cx| SelectState::new(items, Some(IndexPath::new(0)), window, cx)),
            }
        });
        let state = host.read_with(cx, |host, _| host.state.clone());
        let draw = |cx: &mut gpui_kit::VisualTestContext| {
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear(cx));
        };
        cx.update(|window, cx| {
            let trigger = state.read(cx).focus_handle.clone();
            trigger.focus(window, cx);
        });
        draw(cx);

        cx.simulate_keystrokes("down");
        draw(cx);
        assert!(state.read_with(cx, |state, _| state.open));
        cx.simulate_keystrokes("down enter");
        draw(cx);
        state.read_with(cx, |state, _| {
            assert!(!state.open);
            assert_eq!(state.selected_value(), Some(&"Go"));
        });

        cx.simulate_keystrokes("down");
        draw(cx);
        cx.simulate_keystrokes("down escape");
        draw(cx);
        state.read_with(cx, |state, cx| {
            assert!(!state.open);
            assert_eq!(state.selected_value(), Some(&"Go"));
            assert_eq!(state.selected_index(cx), Some(IndexPath::new(1)));
        });
    }

    #[gpui_kit::test]
    fn an_explicit_accessibility_label_does_not_replace_the_placeholder(cx: &mut TestAppContext) {
        init(cx);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let items = SearchableVec::new(vec!["Rust", "Go", "C++"]);
            let state = cx.new(|cx| SelectState::new(items, None, window, cx));

            let named = Select::new(&state)
                .placeholder("Choose a language")
                .accessibility_label("Programming language");
            assert_eq!(
                named.options.accessibility_label.as_deref(),
                Some("Programming language")
            );
            assert_eq!(
                named.options.placeholder.as_deref(),
                Some("Choose a language")
            );
        });
    }

    #[gpui_kit::test]
    fn the_initial_selection_seeds_the_cursor(cx: &mut TestAppContext) {
        init(cx);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let items = SearchableVec::new(vec!["Rust", "Go", "C++"]);
            let state = cx.new(|cx| SelectState::new(items, Some(IndexPath::new(1)), window, cx));
            assert_eq!(state.read(cx).selected_index(cx), Some(IndexPath::new(1)));
            assert_eq!(state.read(cx).selected_value(), Some(&"Go"));

            let mut groups: SearchableVec<SelectGroup<&'static str>> = SearchableVec::new(vec![]);
            groups.push(SelectGroup::new("A").items(["Apple", "Avocado"]));
            groups.push(SelectGroup::new("B").items(["Banana", "Blueberry"]));
            let initial = IndexPath::new(1).section(1);
            let state = cx.new(|cx| SelectState::new(groups, Some(initial), window, cx));
            assert_eq!(state.read(cx).selected_index(cx), Some(initial));
            assert_eq!(state.read(cx).selected_value(), Some(&"Blueberry"));
        });
    }

    #[gpui_kit::test]
    fn set_selected_value_clears_the_search_query(cx: &mut TestAppContext) {
        init(cx);
        let cx = cx.add_empty_window();
        cx.update(|window, cx| {
            let mut groups: SearchableVec<SelectGroup<&'static str>> = SearchableVec::new(vec![]);
            groups.push(SelectGroup::new("A").items(["Apple", "Avocado"]));
            groups.push(SelectGroup::new("B").items(["Banana", "Blueberry"]));
            let state = cx.new(|cx| SelectState::new(groups, None, window, cx).searchable(true));
            let list = state.read(cx).list.clone();

            list.update(cx, |list, cx| list.set_query("Blue", window, cx));
            assert_eq!(list.read(cx).delegate().delegate.sections_count(cx), 1);
            state.update(cx, |state, cx| {
                state.set_selected_value(&"Banana", window, cx)
            });

            assert_eq!(state.read(cx).selected_value(), Some(&"Banana"));
            assert_eq!(
                state.read(cx).selected_index(cx),
                Some(IndexPath::new(0).section(1))
            );
            assert_eq!(list.read(cx).delegate().delegate.sections_count(cx), 2);
            assert!(!list.read(cx).delegate().searched);
        });
    }

    #[gpui_kit::test]
    fn the_accessibility_value_tracks_the_placeholder_and_selection(cx: &mut TestAppContext) {
        init(cx);
        let window = cx.add_empty_window();
        window.update(|window, cx| {
            let items = SearchableVec::new(vec!["Rust", "Go"]);
            let state = cx.new(|cx| SelectState::new(items, None, window, cx).searchable(true));

            _ = Select::new(&state)
                .placeholder("Choose a language")
                .render(window, cx);
            assert_eq!(state.read(cx).accessibility_value(), "Choose a language");

            state.update(cx, |state, cx| {
                state.set_selected_value(&"Rust", window, cx)
            });
            assert_eq!(state.read(cx).accessibility_value(), "Rust");

            _ = Select::new(&state)
                .title_prefix("Language: ")
                .render(window, cx);
            assert_eq!(state.read(cx).accessibility_value(), "Language: Rust");

            state.update(cx, |state, cx| state.set_selected_index(None, window, cx));
            _ = Select::new(&state).render(window, cx);
            assert_eq!(state.read(cx).accessibility_value(), "Please select");
        });
    }
}
