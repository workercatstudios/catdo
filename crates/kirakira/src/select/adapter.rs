//! GPUI Component's searchable-list adapter, ported so the rows are Kirakira's: each row and group
//! label drops into place and the selected row's check pops.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::component::list::{ListDelegate, ListState};
use gpui_kit::component::searchable_list::{SearchableListDelegate, SearchableListItem as _};
use gpui_kit::component::select::SelectListItem;
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, IndexPath, Sizable as _, Size,
    StyleSized as _,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Context, IntoElement, ParentElement as _, Styled as _, Task, Transformation,
    Window, div, px, size,
};

use super::motion::{check_scale, row_pose, rows_end, step};

type OnConfirm<A> = Box<dyn Fn(Option<IndexPath>, bool, &mut Window, &mut Context<ListState<A>>)>;
type OnCancel<A> = Box<dyn Fn(Option<IndexPath>, &mut Window, &mut Context<ListState<A>>)>;

/// Bridges a [`SearchableListDelegate`] into GPUI Component's [`ListDelegate`], as GPUI
/// Component's own (private) adapter does, and moves the rows it renders.
pub(crate) struct Adapter<D: SearchableListDelegate + 'static> {
    pub(crate) delegate: D,
    /// The keyboard cursor row.
    selected_index: Option<IndexPath>,
    /// The parent's committed selection, kept in sync after every change, so rows never read the
    /// parent entity while the list is rendering.
    pub(crate) selection_snapshot: Vec<(IndexPath, D::Item)>,
    on_confirm: OnConfirm<Self>,
    on_cancel: OnCancel<Self>,
    on_render_empty: Box<dyn Fn(&mut Window, &mut App) -> AnyElement>,
    pub(crate) size: Size,
    /// Whether a search has run since the query was last cleared.
    pub(crate) searched: bool,
    /// When the list last opened: the rows drop in and the check pops from then. `None` while
    /// it's closed, so the rows rest through the exit, as they do on the web. Shared with the
    /// state, which sets it from callbacks that run while this list is locked.
    opened: Rc<Cell<Option<Instant>>>,
}

impl<D: SearchableListDelegate + 'static> Adapter<D> {
    pub(crate) fn new(
        delegate: D,
        on_confirm: impl Fn(Option<IndexPath>, bool, &mut Window, &mut Context<ListState<Self>>)
        + 'static,
        on_cancel: impl Fn(Option<IndexPath>, &mut Window, &mut Context<ListState<Self>>) + 'static,
        on_render_empty: impl Fn(&mut Window, &mut App) -> AnyElement + 'static,
        opened: Rc<Cell<Option<Instant>>>,
    ) -> Self {
        Self {
            delegate,
            selected_index: None,
            selection_snapshot: Vec::new(),
            on_confirm: Box::new(on_confirm),
            on_cancel: Box::new(on_cancel),
            on_render_empty: Box::new(on_render_empty),
            size: Size::default(),
            searched: false,
            opened,
        }
    }

    pub(crate) fn update_selection_snapshot(&mut self, snapshot: Vec<(IndexPath, D::Item)>) {
        self.selection_snapshot = snapshot;
    }

    /// How long ago the list opened, while anything in it still moves.
    fn since_open(&self, window: &mut Window, cx: &App) -> Option<Duration> {
        if cx.reduce_motion() {
            return None;
        }
        let elapsed = crate::motion::now().saturating_duration_since(self.opened.get()?);
        if elapsed >= rows_end() {
            return None;
        }
        window.request_animation_frame();
        Some(elapsed)
    }

    /// Whether `section` has a label, which takes the first place in its group.
    fn has_header(&self, section: usize, window: &mut Window, cx: &mut App) -> bool {
        #[allow(deprecated)]
        let label = self.delegate.section(section).is_some();
        label
            || self
                .delegate
                .render_section_header(section, window, cx)
                .is_some()
    }
}

fn check_icon(scale: Option<f32>) -> Icon {
    let icon = Icon::new(IconName::Check);
    match scale {
        Some(scale) => {
            let scale = scale.max(1e-3);
            icon.transform(Transformation::scale(size(scale, scale)))
        }
        None => icon,
    }
}

impl<D: SearchableListDelegate + 'static> ListDelegate for Adapter<D> {
    type Item = SelectListItem;

    fn sections_count(&self, cx: &App) -> usize {
        self.delegate.sections_count(cx)
    }

    fn items_count(&self, section: usize, _: &App) -> usize {
        self.delegate.items_count(section)
    }

    fn render_section_header(
        &mut self,
        section: usize,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<impl IntoElement> {
        let header = match self.delegate.render_section_header(section, window, cx) {
            Some(header) => header,
            None => {
                #[allow(deprecated)]
                let title = self.delegate.section(section)?;
                div()
                    .py_0p5()
                    .px_2()
                    .list_size(self.size)
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(title)
                    .into_any_element()
            }
        };
        // A group's label drops first, in its group's place.
        let pose = self
            .since_open(window, cx)
            .and_then(|elapsed| row_pose(step(section, 0), elapsed));
        let rem = f32::from(window.rem_size());
        Some(
            div()
                .relative()
                .when_some(pose, |this, (y, opacity)| {
                    this.top(px(y * rem)).opacity(opacity)
                })
                .child(header),
        )
    }

    fn render_item(
        &mut self,
        ix: IndexPath,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Option<Self::Item> {
        self.delegate.item(ix)?;
        let (pose, check) = match self.since_open(window, cx) {
            Some(elapsed) => {
                let place = ix.row + usize::from(self.has_header(ix.section, window, cx));
                let step = step(ix.section, place);
                (row_pose(step, elapsed), check_scale(step, elapsed))
            }
            None => (None, None),
        };

        let item = self.delegate.item(ix)?;
        let is_checked = self
            .delegate
            .is_item_checked(ix, item, &self.selection_snapshot, cx);
        let disabled = !self.delegate.is_item_enabled(ix, item, cx);
        let size = self.size;

        let row = match self.delegate.render_item(ix, item, is_checked, window, cx) {
            Some(el) => SelectListItem::new(ix.row)
                .disabled(disabled)
                .with_size(size)
                .child(el),
            None => {
                let content = div()
                    .whitespace_nowrap()
                    .child(item.render(window, cx).into_any_element());
                // A check waiting at scale 0 is hidden: at any size the SVG leaves a speck.
                let shown = check.is_none_or(|scale| scale > 0.01);
                SelectListItem::new(ix.row)
                    .checked(is_checked && shown)
                    .check_icon(check_icon(check))
                    .disabled(disabled)
                    .with_size(size)
                    .child(content.into_any_element())
            }
        };
        let rem = f32::from(window.rem_size());
        Some(match pose {
            Some((y, opacity)) => row.top(px(y * rem)).opacity(opacity),
            None => row,
        })
    }

    fn cancel(&mut self, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        let saved = self.selected_index;
        (self.on_cancel)(saved, window, cx);
    }

    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<ListState<Self>>) {
        (self.on_confirm)(self.selected_index, secondary, window, cx);
    }

    fn perform_search(
        &mut self,
        query: &str,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> Task<()> {
        self.searched = true;
        self.delegate.perform_search(query, window, cx)
    }

    fn set_selected_index(
        &mut self,
        ix: Option<IndexPath>,
        _: &mut Window,
        _: &mut Context<ListState<Self>>,
    ) {
        self.selected_index = ix;
    }

    fn render_empty(
        &mut self,
        window: &mut Window,
        cx: &mut Context<ListState<Self>>,
    ) -> impl IntoElement {
        (self.on_render_empty)(window, cx)
    }
}
