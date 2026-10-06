use gpui_kit::base::TestSupportExt as _;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use gpui_kit::base::actions::{Cancel, Confirm, SelectDown, SelectUp};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::kbd::Kbd;
use gpui_kit::component::scroll::Scrollbar;
use gpui_kit::component::{
    ActiveTheme as _, ElementExt as _, Icon, IconName, IndexPath, StyledExt as _,
    VirtualListScrollHandle, h_flex, v_flex, v_virtual_list,
};
use gpui_kit::{
    AbsoluteLength, AnyElement, App, AppContext as _, AvailableSpace, Context, Entity, FocusHandle,
    Focusable, FontFallbacks, FontFeatures, FontStyle, FontWeight, InteractiveElement, IntoElement,
    ListSizingBehavior, ParentElement, Pixels, Render, Role, ScrollStrategy, SharedString, Size,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Subscription, TextOverflow,
    WhiteSpace, Window, canvas, div, fill, point, prelude::FluentBuilder as _, px, size,
};

use crate::command::{
    command::CommandOptions,
    item::{CommandEntry, CommandItem},
    motion::{Glide, Highlight, LIST_RESIZE, empty_pop, out},
};
use crate::motion::{now, transform};

/// GPUI Component's key context for its palette. Its `init` binds Escape, Enter, Up and Down here,
/// so Kirakira's palette gets the same keys without binding them again.
pub(crate) const CONTEXT: &str = "Command";

/// GPUI Component's English strings; its translations are private to it.
const PLACEHOLDER: &str = "Type a command or search...";
const EMPTY: &str = "No results found.";

/// The row a separator occupies: a one-pixel rule with a little air on
/// either side. Fixed, so that only the item and heading rows need measuring.
const SEPARATOR_ROW_HEIGHT: f32 = 9.;

pub(crate) type OnQuery = dyn Fn(&str, &mut Window, &mut App);
pub(crate) type OnIndex = dyn Fn(IndexPath, &mut Window, &mut App);
pub(crate) type OnCancel = dyn Fn(&mut Window, &mut App);

pub(crate) struct CommandModel {
    pub(crate) entries: Vec<CommandEntry>,
    pub(crate) searchable: bool,
    pub(crate) filterable: bool,
    pub(crate) on_query: Option<Rc<OnQuery>>,
    pub(crate) on_select: Option<Rc<OnIndex>>,
    pub(crate) on_confirm: Option<Rc<OnIndex>>,
    pub(crate) on_cancel: Option<Rc<OnCancel>>,
}

impl Default for CommandModel {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            searchable: true,
            filterable: true,
            on_query: None,
            on_select: None,
            on_confirm: None,
            on_cancel: None,
        }
    }
}

/// One rendered line of the list.
///
/// Groups are flattened into headings and items so the list is a single
/// sequence of rows, which is what the virtual list scrolls over.
#[derive(Clone, PartialEq)]
enum CommandRow {
    Heading(SharedString),
    /// Holds the index into [`CommandState::matched`].
    Item(usize),
    Separator,
}

#[derive(Clone, PartialEq)]
struct TextShapeKey {
    font_family: SharedString,
    font_features: FontFeatures,
    font_fallbacks: Option<FontFallbacks>,
    font_size: AbsoluteLength,
    font_weight: FontWeight,
    font_style: FontStyle,
    white_space: WhiteSpace,
    text_overflow: Option<TextOverflow>,
    line_clamp: Option<usize>,
}

#[derive(Clone, PartialEq)]
struct ListMeasurementKey {
    content_width: Pixels,
    rem_size: Pixels,
    line_height: Pixels,
    text_shape: TextShapeKey,
}

/// An item that survived the current query, and where it landed.
#[derive(Clone)]
struct MatchedItem {
    entry_ix: usize,
    item_ix: usize,
    index_path: IndexPath,
    row_ix: usize,
    disabled: bool,
}

/// The interaction state of a [`crate::command::Command`] palette: its query,
/// focus, scrolling, and highlighted command.
pub struct CommandState {
    focus_handle: FocusHandle,
    query_input: Entity<InputState>,
    scroll_handle: VirtualListScrollHandle,
    model: CommandModel,
    rows: Vec<CommandRow>,
    row_sizes: Rc<Vec<Size<Pixels>>>,
    list_measurement_key: Option<ListMeasurementKey>,
    needs_measure: bool,
    matched: Vec<MatchedItem>,
    selected_index: Option<usize>,
    preserve_no_selection: bool,
    loading: bool,
    pending_scroll: Option<usize>,
    /// The placeholder last written to the query input, so that `render` only
    /// writes when it changed — `set_placeholder` notifies, and an
    /// unconditional notify from `render` would redraw every frame.
    applied_placeholder: SharedString,
    applied_query: SharedString,
    pub(crate) options: CommandOptions,
    _subscriptions: Vec<Subscription>,
    /// The plate behind the selected row.
    highlight: Highlight,
    /// The list's height, easing to its results.
    list_height: Glide,
    /// When the empty message last appeared, for its pop.
    empty_since: Option<Instant>,
    /// The empty message's height as last laid out.
    empty_height: Rc<Cell<Pixels>>,
}

impl CommandState {
    /// Create an empty palette.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let query_input = cx.new(|cx| InputState::new(window, cx));

        let _subscriptions =
            vec![cx.subscribe_in(&query_input, window, Self::on_query_input_event)];

        Self {
            focus_handle: cx.focus_handle(),
            query_input,
            scroll_handle: VirtualListScrollHandle::new(),
            model: CommandModel::default(),
            rows: Vec::new(),
            row_sizes: Rc::new(Vec::new()),
            list_measurement_key: None,
            needs_measure: true,
            matched: Vec::new(),
            selected_index: None,
            preserve_no_selection: false,
            loading: false,
            pending_scroll: None,
            applied_placeholder: SharedString::default(),
            applied_query: SharedString::default(),
            options: CommandOptions::default(),
            _subscriptions,
            highlight: Highlight::default(),
            list_height: Glide::new(LIST_RESIZE, out()),
            empty_since: None,
            empty_height: Rc::new(Cell::new(px(0.))),
        }
    }

    pub(crate) fn install_model(&mut self, model: CommandModel, cx: &mut Context<Self>) {
        let selected_index_path = self.selected_index();
        self.model = model;
        self.update_matches(cx);

        let preserved_selection = selected_index_path.and_then(|selected_index_path| {
            self.matched
                .iter()
                .enumerate()
                .find_map(|(matched_ix, matched)| {
                    (!matched.disabled && matched.index_path == selected_index_path)
                        .then_some(matched_ix)
                })
        });

        if let Some(matched_ix) = preserved_selection {
            // Preserving the selection is not a navigation: the model
            // reinstalls on every host re-render, so scrolling here would
            // move the list one frame after a hover selection.
            self.selected_index = Some(matched_ix);
            self.preserve_no_selection = false;
        } else if self.preserve_no_selection {
            self.selected_index = None;
            self.pending_scroll = None;
        } else {
            self.reset_selection();
        }

        self.needs_measure = true;
    }

    /// The current search query.
    pub fn query(&self, cx: &App) -> SharedString {
        self.query_input.read(cx).value()
    }

    /// Replace the search query, as if it had been typed.
    ///
    /// The input suppresses its own change event for a programmatic write, so
    /// the re-filter and query callback happen here instead.
    pub fn set_query(
        &mut self,
        query: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let query = query.into();
        if self.query(cx) == query {
            return;
        }

        self.query_input
            .update(cx, |input, cx| input.set_value(query, window, cx));
        self.on_query_changed(window, cx);
    }

    /// The highlighted item's path in the model installed by the latest
    /// [`crate::command::Command`] render, before local filtering.
    ///
    /// Ungrouped items occupy section 0 and use their input position as the
    /// row. Explicit groups use their group and item positions; when a model
    /// mixes both forms, the implicit ungrouped section comes first.
    pub fn selected_index(&self) -> Option<IndexPath> {
        self.selected_index
            .and_then(|selected_index| self.matched.get(selected_index))
            .filter(|matched| !matched.disabled)
            .map(|matched| matched.index_path)
    }

    /// Highlight an item by its original, unfiltered model path, or clear the
    /// highlight with `None`.
    ///
    /// A path that is currently filtered out or disabled clears the
    /// highlight. A visible selection is scrolled into view.
    pub fn set_selected_index(
        &mut self,
        index: Option<IndexPath>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let matched_ix = index.and_then(|index| {
            self.matched
                .iter()
                .position(|matched| matched.index_path == index && !matched.disabled)
        });

        let preserve_no_selection = matched_ix.is_none();
        if self.selected_index == matched_ix {
            self.preserve_no_selection = preserve_no_selection;
            return;
        }

        let previous_index = self.selected_index();
        self.selected_index = matched_ix;
        self.preserve_no_selection = preserve_no_selection;
        self.pending_scroll = matched_ix
            .and_then(|matched_ix| self.matched.get(matched_ix))
            .map(|matched| matched.row_ix);

        if let Some((on_select, index)) = self.on_select_if_changed(previous_index) {
            window.defer(cx, move |window, cx| on_select(index, window, cx));
        }

        cx.notify();
    }

    /// The number of items matching the current query.
    pub fn matched_count(&self) -> usize {
        self.matched.len()
    }

    /// Move focus to the palette's active control.
    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        if self.model.searchable {
            self.query_input.focus_handle(cx).focus(window, cx);
        } else {
            self.focus_handle.focus(window, cx);
        }
    }

    /// Show or hide the search field's spinner, and suppress the empty message
    /// while it spins.
    ///
    /// Turn it on while an `on_query` callback is being answered.
    pub fn set_loading(&mut self, loading: bool, window: &mut Window, cx: &mut Context<Self>) {
        self.loading = loading;
        self.query_input
            .update(cx, |input, cx| input.set_loading(loading, window, cx));
        cx.notify();
    }

    /// Whether the search field is showing its spinner.
    pub fn is_loading(&self) -> bool {
        self.loading
    }

    // MARK: Matching

    fn item_matches(&self, item: &CommandItem, query: &str) -> bool {
        if !self.model.searchable || !self.model.filterable || query.is_empty() {
            true
        } else {
            item.matches(query)
        }
    }

    fn item_at(&self, matched_ix: usize) -> Option<&CommandItem> {
        let matched = self.matched.get(matched_ix)?;

        match self.model.entries.get(matched.entry_ix)? {
            CommandEntry::Item(item) => Some(item),
            CommandEntry::Group(group) => group.items.get(matched.item_ix),
            CommandEntry::Separator => None,
        }
    }

    /// Recompute the visible rows and the matching items for the current query.
    ///
    fn update_matches(&mut self, cx: &App) {
        let query = self.query(cx);
        let query = query.trim();

        let mut rows: Vec<CommandRow> = Vec::new();
        let mut matched: Vec<MatchedItem> = Vec::new();
        let has_ungrouped_items = self
            .model
            .entries
            .iter()
            .any(|entry| matches!(entry, CommandEntry::Item(_)));
        let mut ungrouped_item_ix = 0;
        let mut group_ix = 0;
        // A separator is only drawn once something follows it, which drops the
        // leading, trailing and doubled separators a filtered list leaves behind.
        let mut pending_separator = false;

        for (entry_ix, entry) in self.model.entries.iter().enumerate() {
            match entry {
                CommandEntry::Separator => pending_separator = !rows.is_empty(),
                CommandEntry::Item(item) => {
                    let item_ix = ungrouped_item_ix;
                    ungrouped_item_ix += 1;
                    if !self.item_matches(item, query) {
                        continue;
                    }

                    if pending_separator {
                        rows.push(CommandRow::Separator);
                        pending_separator = false;
                    }

                    let index_path = IndexPath::new(item_ix).section(0);
                    matched.push(MatchedItem {
                        entry_ix,
                        item_ix: 0,
                        index_path,
                        row_ix: rows.len(),
                        disabled: item.is_disabled(),
                    });
                    rows.push(CommandRow::Item(matched.len() - 1));
                }
                CommandEntry::Group(group) => {
                    let section_ix = group_ix + usize::from(has_ungrouped_items);
                    group_ix += 1;
                    let visible = group
                        .items
                        .iter()
                        .enumerate()
                        .filter(|(_, item)| self.item_matches(item, query))
                        .map(|(item_ix, item)| (item_ix, item.is_disabled()))
                        .collect::<Vec<_>>();

                    if visible.is_empty() {
                        continue;
                    }

                    if pending_separator {
                        rows.push(CommandRow::Separator);
                        pending_separator = false;
                    }

                    if let Some(heading) = group.heading() {
                        rows.push(CommandRow::Heading(heading.clone()));
                    }

                    for (item_ix, disabled) in visible {
                        let index_path = IndexPath::new(item_ix).section(section_ix);
                        matched.push(MatchedItem {
                            entry_ix,
                            item_ix,
                            index_path,
                            row_ix: rows.len(),
                            disabled,
                        });
                        rows.push(CommandRow::Item(matched.len() - 1));
                    }
                }
            }
        }

        self.rows = rows;
        self.matched = matched;
        self.needs_measure = true;
        self.selected_index = self.selected_index.and_then(|selected_index| {
            (selected_index < self.matched.len()).then_some(selected_index)
        });
    }

    /// Move the highlight to the first item that can be confirmed.
    fn reset_selection(&mut self) {
        self.selected_index = self.matched.iter().position(|matched| !matched.disabled);
        self.preserve_no_selection = false;
        self.pending_scroll = self
            .selected_index
            .and_then(|selected_index| self.matched.get(selected_index))
            .map(|matched| matched.row_ix)
            .or(Some(0));
    }

    fn on_query_input_event(
        &mut self,
        _: &Entity<InputState>,
        event: &InputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !matches!(event, InputEvent::Change) {
            return;
        }

        self.on_query_changed(window, cx);
    }

    /// Re-filter for the query that is now in the field, and report it.
    fn on_query_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.query(cx);
        if query == self.applied_query {
            return;
        }

        let previous_selection = self.selected_index();
        self.applied_query = query.clone();
        self.update_matches(cx);
        self.reset_selection();
        let selection_callback = self.on_select_if_changed(previous_selection);
        let query_callback = self
            .model
            .searchable
            .then(|| self.model.on_query.clone())
            .flatten();

        if selection_callback.is_some() || query_callback.is_some() {
            window.defer(cx, move |window, cx| {
                if let Some((on_select, index)) = selection_callback {
                    on_select(index, window, cx);
                }
                if let Some(on_query) = query_callback {
                    on_query(query.as_ref(), window, cx);
                }
            });
        }

        cx.notify();
    }

    fn set_list_measurement_key(
        &mut self,
        measurement_key: ListMeasurementKey,
        cx: &mut Context<Self>,
    ) {
        if self.list_measurement_key.as_ref() == Some(&measurement_key) {
            return;
        }

        self.list_measurement_key = Some(measurement_key);
        self.needs_measure = true;
        cx.notify();
    }

    // MARK: Actions

    fn on_select_if_changed(
        &self,
        previous_index: Option<IndexPath>,
    ) -> Option<(Rc<OnIndex>, IndexPath)> {
        let index = self.selected_index();
        if index == previous_index {
            return None;
        }

        self.model.on_select.clone().zip(index)
    }

    /// Highlight an item without scrolling it into view. Hover goes through
    /// here, and revealing a half-clipped edge row would slide the next row
    /// under the resting cursor, hover-selecting and scrolling in a loop.
    fn select(&mut self, matched_ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_index == Some(matched_ix) {
            return;
        }

        let previous_index = self.selected_index();
        self.selected_index = Some(matched_ix);
        self.preserve_no_selection = false;

        if let Some((on_select, index)) = self.on_select_if_changed(previous_index) {
            window.defer(cx, move |window, cx| on_select(index, window, cx));
        }

        cx.notify();
    }

    /// Move the highlight by `step` items, wrapping around and skipping the
    /// disabled ones.
    fn select_by(&mut self, step: isize, window: &mut Window, cx: &mut Context<Self>) {
        let len = self.matched.len();
        if len == 0 {
            return;
        }

        let mut next = self
            .selected_index
            .unwrap_or_else(|| if step >= 0 { len.saturating_sub(1) } else { 0 });
        let mut enabled = None;
        for _ in 0..len {
            next = (next as isize + step).rem_euclid(len as isize) as usize;
            if !self.matched[next].disabled {
                enabled = Some(next);
                break;
            }
        }

        if let Some(next) = enabled
            && self.selected_index != Some(next)
        {
            self.pending_scroll = self.matched.get(next).map(|matched| matched.row_ix);
            self.select(next, window, cx);
        }
    }

    fn on_action_select_up(&mut self, _: &SelectUp, window: &mut Window, cx: &mut Context<Self>) {
        self.select_by(-1, window, cx);
    }

    fn on_action_select_down(
        &mut self,
        _: &SelectDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_by(1, window, cx);
    }

    fn on_action_confirm(&mut self, _: &Confirm, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(selected_index) = self.selected_index {
            self.confirm(selected_index, window, cx);
        }
    }

    /// Escape clears a non-empty query first, and only then leaves the palette
    /// — the dialog that hosts it closes on the second press.
    fn on_action_cancel(&mut self, _: &Cancel, window: &mut Window, cx: &mut Context<Self>) {
        if self.model.searchable && !self.query(cx).is_empty() {
            self.set_query("", window, cx);
            return;
        }

        // Cancel is the one synchronous callback: propagation must continue in
        // this dispatch so a hosting Dialog observes it once and owns the pop.
        if let Some(on_cancel) = self.model.on_cancel.clone() {
            on_cancel(window, cx);
        }

        cx.propagate();
    }

    fn confirm(&mut self, matched_ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item) = self.item_at(matched_ix) else {
            return;
        };
        if item.is_disabled() {
            return;
        }

        let index_path = self.matched[matched_ix].index_path;
        let action = item.action.as_ref().map(|action| action.boxed_clone());
        let on_confirm = self.model.on_confirm.clone();

        if let Some(action) = action {
            window.dispatch_action(action, cx);
        }
        if let Some(on_confirm) = on_confirm {
            window.defer(cx, move |window, cx| {
                on_confirm(index_path, window, cx);
            });
        }
    }

    // MARK: Row sizing

    /// Measure each row before passing the sizes to the virtual list. Custom
    /// item elements can have independent intrinsic heights.
    fn measure_row_sizes(&self, window: &mut Window, cx: &mut Context<Self>) -> Vec<Size<Pixels>> {
        let available = size(
            self.list_measurement_key
                .as_ref()
                .map_or(AvailableSpace::MinContent, |key| {
                    AvailableSpace::Definite(key.content_width)
                }),
            AvailableSpace::MinContent,
        );
        let text_style = StyleRefinement {
            text: self.options.style.text.clone(),
            ..Default::default()
        };

        self.rows
            .iter()
            .enumerate()
            .map(|(row_ix, row)| match row {
                CommandRow::Separator => size(px(0.), px(SEPARATOR_ROW_HEIGHT)),
                CommandRow::Heading(_) | CommandRow::Item(_) => {
                    let row_size = div()
                        .refine_style(&text_style)
                        .child(self.render_row(row_ix, window, cx))
                        .into_any_element()
                        .layout_as_root(available, window, cx);
                    size(px(0.), row_size.height)
                }
            })
            .collect()
    }

    // MARK: Rendering

    fn sync_placeholder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let placeholder = self
            .options
            .placeholder
            .as_ref()
            .cloned()
            .unwrap_or_else(|| PLACEHOLDER.into());

        if self.applied_placeholder == placeholder {
            return;
        }

        self.applied_placeholder = placeholder.clone();
        self.query_input.update(cx, |input, cx| {
            input.set_placeholder(placeholder, window, cx)
        });
    }

    /// The frame every item row shares, so that the measured height matches the
    /// rendered one.
    fn item_row(&self, selected: bool, cx: &App) -> gpui_kit::Div {
        div()
            .flex()
            .flex_row()
            .items_center()
            .w_full()
            .gap_2()
            .px_2()
            .py_1p5()
            .text_sm()
            .rounded(cx.theme().radius)
            // The highlight is one plate behind the rows that springs between them.
            .when(selected, |this| {
                this.text_color(cx.theme().accent_foreground)
            })
    }

    fn heading_row(&self, heading: SharedString, cx: &App) -> gpui_kit::Div {
        div()
            .w_full()
            .px_2()
            .py_1p5()
            .text_xs()
            .font_medium()
            .text_color(cx.theme().muted_foreground)
            .child(heading)
    }

    fn render_row(&self, row_ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        match self.rows.get(row_ix) {
            None => div().into_any_element(),
            Some(CommandRow::Separator) => div()
                .w_full()
                .py(px(4.))
                .child(div().h(px(1.)).w_full().bg(cx.theme().border))
                .into_any_element(),
            Some(CommandRow::Heading(heading)) => {
                self.heading_row(heading.clone(), cx).into_any_element()
            }
            Some(CommandRow::Item(matched_ix)) => self.render_item(*matched_ix, window, cx),
        }
    }

    fn render_item(
        &self,
        matched_ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(item) = self.item_at(matched_ix) else {
            return div().into_any_element();
        };

        let disabled = item.is_disabled();
        let selected = self.selected_index == Some(matched_ix) && !disabled;
        let muted_foreground = cx.theme().muted_foreground;
        let icon_color = if selected {
            cx.theme().accent_foreground
        } else {
            muted_foreground
        };
        let binding = if item.content.is_none() {
            item.action.as_ref().and_then(|action| {
                Kbd::binding_for_action_in(action.as_ref(), &self.focus_handle(cx), window)
                    .or_else(|| Kbd::binding_for_action(action.as_ref(), None, window))
            })
        } else {
            None
        };

        let content = match &item.content {
            Some(render) => render(window, cx),
            None => h_flex()
                .flex_1()
                .gap_2()
                .items_center()
                .when_some(item.icon.clone(), |this, icon| {
                    this.child(icon.size_4().text_color(icon_color))
                })
                .when_some(item.label_text().cloned(), |this, label| this.child(label))
                .into_any_element(),
        };

        self.item_row(selected, cx)
            .id(self.matched[matched_ix].index_path)
            .test_support()
            .role(Role::ListBoxOption)
            .aria_selected(selected)
            .when(disabled, |this| this.text_color(muted_foreground))
            .when(!disabled, |this| {
                this.cursor_default()
                    .on_hover(cx.listener(move |this, hovered: &bool, window, cx| {
                        if *hovered {
                            this.select(matched_ix, window, cx);
                        }
                    }))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.confirm(matched_ix, window, cx);
                    }))
            })
            .child(content)
            .map(|this| match binding {
                Some(binding) => this.child(binding.ml_auto()),
                // The binding owns the trailing slot, so only an item without
                // one can show its check there.
                None => this.when(item.checked, |this| {
                    this.child(gpui_kit::component::Sizable::xsmall(
                        Icon::new(IconName::Check).ml_auto(),
                    ))
                }),
            })
            .into_any_element()
    }

    fn render_empty(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        let empty = match self.options.empty.as_ref() {
            Some(empty) => empty(self, window, cx),
            None => {
                let message: SharedString = EMPTY.into();
                div()
                    .py_6()
                    .w_full()
                    .text_center()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(message)
                    .into_any_element()
            }
        };
        // It pops in each time the results run out.
        let since = self.empty_since.unwrap_or_else(now);
        let (pose, running) = empty_pop(now().saturating_duration_since(since), cx.reduce_motion());
        if running {
            window.request_animation_frame();
        }
        let measured = self.empty_height.clone();
        div()
            .w_full()
            .on_prepaint(move |bounds, _, _| measured.set(bounds.size.height))
            .child(transform("kk-command-empty", pose, empty))
            .into_any_element()
    }

    /// Where the selected row sits in the list's content: its top and height, if one is.
    fn highlight_target(&self) -> Option<(f32, f32)> {
        let matched = self.matched.get(self.selected_index?)?;
        if matched.disabled {
            return None;
        }
        let sizes = self.row_sizes.get(..=matched.row_ix)?;
        let (last, before) = sizes.split_last()?;
        // The virtual list's `p_1` inset.
        let top = 4.0 + before.iter().map(|s| f32::from(s.height)).sum::<f32>();
        Some((top, f32::from(last.height)))
    }

    /// The list's height for its results, when it can be known: the rows and the list's inset,
    /// up to the maximum, or the empty message and its inset.
    fn list_target(&self, window: &Window) -> Option<f32> {
        let max = f32::from(
            self.options
                .max_h
                .to_pixels(window.viewport_size().height.into(), window.rem_size()),
        );
        if self.rows.is_empty() {
            if self.loading {
                return Some(8.0);
            }
            let empty = f32::from(self.empty_height.get());
            return (empty > 0.0).then_some((empty + 8.0).min(max));
        }
        let rows: f32 = self.row_sizes.iter().map(|s| f32::from(s.height)).sum();
        Some((rows + 8.0).min(max))
    }
}

impl Focusable for CommandState {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        if self.model.searchable {
            self.query_input.focus_handle(cx)
        } else {
            self.focus_handle.clone()
        }
    }
}

impl Render for CommandState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.sync_placeholder(window, cx);

        if self.needs_measure {
            self.needs_measure = false;
            self.row_sizes = Rc::new(self.measure_row_sizes(window, cx));
        }

        if let Some(row_ix) = self.pending_scroll.take() {
            self.scroll_handle
                .scroll_to_item(row_ix, ScrollStrategy::Nearest);
        }

        let rows_count = self.rows.len();
        let row_sizes = self.row_sizes.clone();
        let command_state = cx.entity();

        // Pop Command's motion: the highlight springs to the selected row, the list eases to
        // its results' height, and the empty message pops in when they run out.
        let now = now();
        let reduced = cx.reduce_motion();
        let empty = rows_count == 0 && !self.loading;
        if empty && self.empty_since.is_none() {
            self.empty_since = Some(now);
        } else if !empty {
            self.empty_since = None;
        }
        let target = self.highlight_target();
        self.highlight.update(target, now, reduced);
        let highlight = self.highlight.frame(now, reduced);
        let known = self.list_height.target_value() > 0.0;
        let list_height = match self.list_target(window) {
            Some(height) => {
                self.list_height.target(height, now, reduced || !known);
                Some(self.list_height.value(now))
            }
            // The empty message hasn't been measured yet: hold the height for that frame.
            None => known.then(|| self.list_height.value(now)),
        };
        if highlight.running || self.list_height.running(now) {
            window.request_animation_frame();
        }
        let plate = (cx.theme().accent, cx.theme().radius);
        let scroll = self.scroll_handle.clone();

        v_flex()
            .id("command")
            .test_support()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::on_action_select_up))
            .on_action(cx.listener(Self::on_action_select_down))
            .on_action(cx.listener(Self::on_action_confirm))
            .on_action(cx.listener(Self::on_action_cancel))
            .w_full()
            .overflow_hidden()
            .bg(cx.theme().popover)
            .text_color(cx.theme().popover_foreground)
            .when(self.options.bordered, |this| {
                this.rounded(cx.theme().radius_lg)
                    .border_1()
                    .border_color(cx.theme().border)
            })
            .refine_style(&self.options.style)
            .when_some(self.options.header.as_ref(), |this, header| {
                this.child(header(self, window, cx))
            })
            .when(self.model.searchable, |this| {
                this.child(
                    div()
                        .flex_none()
                        .px_3()
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child(
                            Input::new(&self.query_input)
                                .prefix(
                                    Icon::new(IconName::Search)
                                        .text_color(cx.theme().muted_foreground),
                                )
                                .appearance(false)
                                .p_0(),
                        ),
                )
            })
            .child(
                v_flex()
                    .id("command-list-container")
                    .role(Role::ListBox)
                    .relative()
                    .flex_1()
                    // The rows carry their inset on the virtual list itself so
                    // that a mid-scroll clip edge sits flush against the
                    // surrounding dividers; only the empty slot needs the
                    // container padding.
                    .when(rows_count == 0, |this| this.p_1())
                    .on_prepaint({
                        let measure_state = command_state.clone();
                        move |bounds, window, cx| {
                            measure_state.update(cx, |state, cx| {
                                // The list's `p_1` is one quarter rem on each
                                // side. Its rem-dependent padding and inherited
                                // layout-relevant text style participate in
                                // the row-size cache key.
                                let text_style = window.text_style();
                                state.set_list_measurement_key(
                                    ListMeasurementKey {
                                        content_width: (bounds.size.width
                                            - window.rem_size() * 0.5)
                                            .max(px(0.)),
                                        rem_size: window.rem_size(),
                                        line_height: window.line_height(),
                                        text_shape: TextShapeKey {
                                            font_family: text_style.font_family,
                                            font_features: text_style.font_features,
                                            font_fallbacks: text_style.font_fallbacks,
                                            font_size: text_style.font_size,
                                            font_weight: text_style.font_weight,
                                            font_style: text_style.font_style,
                                            white_space: text_style.white_space,
                                            text_overflow: text_style.text_overflow,
                                            line_clamp: text_style.line_clamp,
                                        },
                                    },
                                    cx,
                                )
                            })
                        }
                    })
                    .max_h(self.options.max_h)
                    .when_some(list_height, |this, height| this.h(px(height)))
                    .overflow_hidden()
                    // While a search is in flight the list is empty because the
                    // answer has not arrived, which is not the same as no match.
                    .when(rows_count == 0 && !self.loading, |this| {
                        this.child(self.render_empty(window, cx))
                    })
                    .when(rows_count > 0 && highlight.opacity > 0.0, |this| {
                        // The plate, painted behind the rows where the list has scrolled it.
                        this.child(
                            canvas(
                                |_, _, _| {},
                                move |bounds, _, window, _| {
                                    let (color, radius) = plate;
                                    let offset = scroll.base_handle().offset();
                                    let inset = px(4.);
                                    let width = (bounds.size.width - inset * 2.).max(px(0.));
                                    let height = px(highlight.height);
                                    let scaled = gpui_kit::size(
                                        width * highlight.scale,
                                        height * highlight.scale,
                                    );
                                    let origin = point(
                                        bounds.left() + inset + (width - scaled.width) / 2.,
                                        bounds.top()
                                            + offset.y
                                            + px(highlight.top)
                                            + (height - scaled.height) / 2.,
                                    );
                                    window.paint_quad(
                                        fill(
                                            gpui_kit::Bounds::new(origin, scaled),
                                            color.opacity(highlight.opacity),
                                        )
                                        .corner_radii(radius * highlight.scale),
                                    );
                                },
                            )
                            .absolute()
                            .inset_0(),
                        )
                    })
                    .when(rows_count > 0, |this| {
                        this.child(
                            v_virtual_list(
                                command_state.clone(),
                                "command-list",
                                row_sizes,
                                move |this, visible_range, window, cx| {
                                    visible_range
                                        .map(|row_ix| this.render_row(row_ix, window, cx))
                                        .collect::<Vec<_>>()
                                },
                            )
                            // Padding on the virtual list acts like CSS
                            // scroll-padding: the scroll ends keep their inset
                            // while scrolled-under rows paint and clip at the
                            // list edge.
                            .p_1()
                            .with_sizing_behavior(ListSizingBehavior::Infer)
                            .track_scroll(&self.scroll_handle),
                        )
                        .child(Scrollbar::vertical(&self.scroll_handle))
                    }),
            )
            .when_some(self.options.footer.as_ref(), |this, footer| {
                this.child(footer(self, window, cx))
            })
    }
}
