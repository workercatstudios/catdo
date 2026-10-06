//! Table: the header fades in, then the rows rise one by one.
//!
//! Replaces `gpui_kit::component::table`. Kirakira's Pop Table is shadcn's simple table, so this
//! module rebuilds GPUI Component's simple table parts with the same builders and sizes and adds
//! the entrance: [`Table`], [`TableHeader`], [`TableBody`], [`TableFooter`] and [`TableCaption`].
//! [`TableRow`], [`TableHead`] and [`TableCell`] are GPUI Component's own, as is everything else
//! the module exports: the stateful, virtualised `DataTable` with its `TableState`,
//! `TableDelegate` and columns comes through unchanged. Its rows are drawn by your delegate, out
//! of reach of a table-wide entrance, and a data table is a tool for scrolling through data, where
//! rows rising on every scroll would get in the way.
//!
//! When the table comes into view (or mounts, with [`Trigger::Mount`]) the header, footer and
//! caption fade in (0.3 s, ease-out, after `delay`), then the body rows rise one by one from
//! 0.1 s later, `stagger` (50 ms) apart: up from 0.9 em below, opaque by 40 %, 0.12 em past their
//! place at 65 %, settled after `duration` (400 ms). From the 12th row on they all go with the
//! 12th, so a long table is never kept waiting. Under reduced motion everything shows at once.
//!
//! The entrance runs on a [`Clock`](crate::motion::Clock), so a
//! [`Timeline`](crate::timeline::Timeline) can seek it. It plays once per table: the web version
//! also rises a row that mounts later (the next page), but GPUI rows have no identity to tell a
//! new row from an old one, so give the table a new [`Table::id`] to play it again, for example
//! one per page.

use std::cell::RefCell;
use std::time::Duration;

use gpui_kit::base::{StyledExt as _, Table as BaseTable, TableBody as BaseTableBody};
use gpui_kit::component::table::{
    TableCaption as UiCaption, TableFooter as UiFooter, TableHeader as UiHeader,
};
use gpui_kit::component::{ActiveTheme as _, AnyChildElement, ChildElement, Sizable, Size};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement,
    LayoutId, ParentElement, Pixels, RenderOnce, SharedString, StyleRefinement, Styled, Window,
    relative, rems,
};

pub use gpui_kit::component::table::*;

use crate::motion::{
    Clock, Easing, Keyframes, Pose, Timing, Track, Trigger, child_id, delay_ms, ms, transform,
};

/// Rows past this one rise together with it.
const STAGGERED: usize = 12;

/// A body row's rise, in ems below its place.
fn rise_y() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.9, Easing::EaseOut)
        .at(0.65, -0.12)
        .at(1.0, 0.0)
        .build()
}

/// A body row's fade: opaque by 40 %.
fn rise_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.0, Easing::EaseOut)
        .at(0.4, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// The header, footer and caption fade: `from { opacity: 0 }`, eased out over the whole run.
fn fade_track() -> Keyframes<f32> {
    Track::new(Easing::EaseOut)
        .at(0.0, 0.0)
        .at(1.0, 1.0)
        .build()
}

/// The table's entrance timing, shared with the sections laid out inside it.
#[derive(Clone, Debug)]
struct Entrance {
    table: ElementId,
    elapsed: Duration,
    delay: u64,
    stagger: u64,
    duration: u64,
    em: Pixels,
}

impl Entrance {
    /// When the `index`th body row starts rising, in milliseconds.
    fn row_delay(&self, index: usize) -> u64 {
        self.delay + 100 + index.min(STAGGERED - 1) as u64 * self.stagger
    }

    /// Opacity of the header, footer and caption.
    fn fade(&self) -> f32 {
        let timing = Timing::new(ms(300)).delay(delay_ms(self.delay));
        fade_track().sample(timing.sample(self.elapsed).directed_progress)
    }

    /// The pose of the `index`th body row.
    fn row(&self, index: usize) -> Pose {
        let timing = Timing::new(ms(self.duration)).delay(delay_ms(self.row_delay(index)));
        let progress = timing.sample(self.elapsed).directed_progress;
        Pose::new()
            .y(rise_y().sample(progress) * f32::from(self.em))
            .opacity(rise_opacity().sample(progress))
    }

    /// When the last thing has landed, for `rows` body rows.
    fn end(&self, rows: usize) -> Duration {
        let last_row = self.row_delay(rows.saturating_sub(1)) + self.duration;
        ms(last_row.max(self.delay + 300))
    }
}

thread_local! {
    static ENTRANCE: RefCell<Vec<Entrance>> = const { RefCell::new(Vec::new()) };
}

fn current_entrance() -> Option<Entrance> {
    ENTRANCE.with(|entrance| entrance.borrow().last().cloned())
}

/// Lays out `child` with `entrance` published to the sections inside it, which render then.
struct WithEntrance {
    entrance: Entrance,
    child: AnyElement,
}

impl IntoElement for WithEntrance {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for WithEntrance {
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
                ENTRANCE.with(|entrance| entrance.borrow_mut().pop());
            }
        }
        ENTRANCE.with(|entrance| entrance.borrow_mut().push(self.entrance.clone()));
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

/// Fades `child` in with the table's header, if it is inside a Kirakira [`Table`].
fn faded(part: &'static str, ix: usize, child: impl IntoElement) -> AnyElement {
    match current_entrance() {
        Some(entrance) => {
            let id = child_id(&(entrance.table.clone(), part).into(), ix);
            transform(id, Pose::new().opacity(entrance.fade()), child)
                .outer_style(full_width())
                .into_any_element()
        }
        None => child.into_any_element(),
    }
}

fn full_width() -> StyleRefinement {
    let mut style = StyleRefinement::default();
    style.size.width = Some(relative(1.0).into());
    style
}

/// A basic table component for directly rendering tabular data, whose rows rise into place.
///
/// The same builder as GPUI Component's `Table`. Size set via [`Sizable`] is propagated to all
/// children.
///
/// ```rust,ignore
/// Table::new()
///     .child(TableHeader::new().child(
///         TableRow::new()
///             .child(TableHead::new().child("Name"))
///             .child(TableHead::new().child("Email"))
///     ))
///     .child(TableBody::new()
///         .child(TableRow::new()
///             .child(TableCell::new().child("John"))
///             .child(TableCell::new().child("john@example.com")))
///     )
///     .child(TableCaption::new().child("A list of recent invoices."))
/// ```
#[derive(IntoElement)]
pub struct Table {
    ix: usize,
    id: ElementId,
    style: StyleRefinement,
    children: Vec<AnyChildElement>,
    size: Size,
    accessibility_label: Option<SharedString>,
    trigger: Trigger,
    delay: u64,
    stagger: u64,
    duration: u64,
}

impl Table {
    /// A table. Its entrance state is keyed by where it's created; give tables made at the same
    /// call site their own [`Table::id`].
    #[track_caller]
    pub fn new() -> Self {
        Self {
            ix: 0,
            id: ElementId::CodeLocation(*std::panic::Location::caller()),
            style: StyleRefinement::default(),
            children: Vec::new(),
            size: Size::default(),
            accessibility_label: None,
            trigger: Trigger::InView,
            delay: 0,
            stagger: 50,
            duration: 400,
        }
    }

    /// Key the entrance by `id`. A new id plays the entrance again.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Set the name a screen reader announces for the table.
    ///
    /// A [`TableCaption`] is visible descriptive content and is not used
    /// automatically as the table's accessible name.
    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    /// Play when the table first scrolls into view (the default) or when it mounts.
    pub fn trigger(mut self, trigger: Trigger) -> Self {
        self.trigger = trigger;
        self
    }

    /// Milliseconds before the header fades in. The first row follows 100 ms later.
    pub fn delay(mut self, delay: u64) -> Self {
        self.delay = delay;
        self
    }

    /// Milliseconds between one row and the next. 50 by default.
    pub fn stagger(mut self, stagger: u64) -> Self {
        self.stagger = stagger;
        self
    }

    /// Milliseconds for each row's rise. 400 by default.
    pub fn duration(mut self, duration: u64) -> Self {
        self.duration = duration;
        self
    }

    pub fn child(mut self, child: impl ChildElement + 'static) -> Self {
        self.children.push(AnyChildElement::new(child));
        self
    }

    pub fn children<E: ChildElement + 'static>(
        mut self,
        children: impl IntoIterator<Item = E>,
    ) -> Self {
        self.children
            .extend(children.into_iter().map(AnyChildElement::new));
        self
    }
}

impl Default for Table {
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for Table {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for Table {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ChildElement for Table {
    fn with_ix(mut self, ix: usize) -> Self {
        self.ix = ix;
        self
    }
}

impl RenderOnce for Table {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let clock = Clock::new((self.id.clone(), "kk-entrance"), self.trigger, window, cx);
        let entrance = Entrance {
            table: self.id.clone(),
            elapsed: clock.elapsed(),
            delay: self.delay,
            stagger: self.stagger,
            duration: self.duration,
            em: rems(0.875).to_pixels(window.rem_size()),
        };
        // Ask for frames until the longest table we could hold has landed.
        clock.animate(Some(entrance.end(STAGGERED)), window);

        let size = self.size;
        let root = BaseTable::new(("table", self.ix))
            .when_some(self.accessibility_label, |this, label| {
                this.accessibility_label(label)
            })
            .w_full()
            .text_sm()
            .overflow_hidden()
            .bg(cx.theme().tokens.table)
            .refine_style(&self.style)
            .children(
                self.children
                    .into_iter()
                    .enumerate()
                    .map(|(ix, child)| WithEntrance {
                        entrance: entrance.clone(),
                        child: child.into_any(ix, size),
                    }),
            );
        clock.observe(root)
    }
}

/// The header section of a [`Table`], wrapping header rows. Fades in first.
#[derive(IntoElement)]
pub struct TableHeader {
    ix: usize,
    inner: UiHeader,
}

impl TableHeader {
    pub fn new() -> Self {
        Self {
            ix: 0,
            inner: UiHeader::new(),
        }
    }

    pub fn child(mut self, child: impl ChildElement + 'static) -> Self {
        self.inner = self.inner.child(child);
        self
    }

    pub fn children<E: ChildElement + 'static>(
        mut self,
        children: impl IntoIterator<Item = E>,
    ) -> Self {
        self.inner = self.inner.children(children);
        self
    }
}

impl Default for TableHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for TableHeader {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}

impl Sizable for TableHeader {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.inner = self.inner.with_size(size);
        self
    }
}

impl ChildElement for TableHeader {
    fn with_ix(mut self, ix: usize) -> Self {
        self.ix = ix;
        self.inner = self.inner.with_ix(ix);
        self
    }
}

impl RenderOnce for TableHeader {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        faded("kk-header", self.ix, self.inner)
    }
}

/// The body section of a [`Table`], wrapping data rows. Its rows rise one by one.
#[derive(IntoElement)]
pub struct TableBody {
    ix: usize,
    style: StyleRefinement,
    children: Vec<AnyChildElement>,
    size: Size,
}

impl TableBody {
    pub fn new() -> Self {
        Self {
            ix: 0,
            style: StyleRefinement::default(),
            children: Vec::new(),
            size: Size::default(),
        }
    }

    pub fn child(mut self, child: impl ChildElement + 'static) -> Self {
        self.children.push(AnyChildElement::new(child));
        self
    }

    pub fn children<E: ChildElement + 'static>(
        mut self,
        children: impl IntoIterator<Item = E>,
    ) -> Self {
        self.children
            .extend(children.into_iter().map(AnyChildElement::new));
        self
    }
}

impl Default for TableBody {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for TableBody {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for TableBody {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ChildElement for TableBody {
    fn with_ix(mut self, ix: usize) -> Self {
        self.ix = ix;
        self
    }
}

impl RenderOnce for TableBody {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let entrance = current_entrance();
        let size = self.size;
        let body_ix = self.ix;
        BaseTableBody::new(("table-body", self.ix))
            .w_full()
            .refine_style(&self.style)
            .children(
                self.children
                    .into_iter()
                    .enumerate()
                    .map(move |(ix, child)| {
                        let row = child.into_any(ix, size);
                        match &entrance {
                            Some(entrance) => {
                                let body =
                                    child_id(&(entrance.table.clone(), "kk-body").into(), body_ix);
                                transform(child_id(&body, ix), entrance.row(ix), row)
                                    .outer_style(full_width())
                                    .into_any_element()
                            }
                            None => row,
                        }
                    }),
            )
    }
}

/// The footer section of a [`Table`], wrapping footer rows. Fades in with the header.
#[derive(IntoElement)]
pub struct TableFooter {
    ix: usize,
    inner: UiFooter,
}

impl TableFooter {
    pub fn new() -> Self {
        Self {
            ix: 0,
            inner: UiFooter::new(),
        }
    }

    pub fn child(mut self, child: impl ChildElement + 'static) -> Self {
        self.inner = self.inner.child(child);
        self
    }

    pub fn children<E: ChildElement + 'static>(
        mut self,
        children: impl IntoIterator<Item = E>,
    ) -> Self {
        self.inner = self.inner.children(children);
        self
    }
}

impl Default for TableFooter {
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for TableFooter {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}

impl Sizable for TableFooter {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.inner = self.inner.with_size(size);
        self
    }
}

impl ChildElement for TableFooter {
    fn with_ix(mut self, ix: usize) -> Self {
        self.ix = ix;
        self.inner = self.inner.with_ix(ix);
        self
    }
}

impl RenderOnce for TableFooter {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        faded("kk-footer", self.ix, self.inner)
    }
}

/// A caption displayed below the [`Table`]. Fades in with the header.
#[derive(IntoElement)]
pub struct TableCaption {
    ix: usize,
    inner: UiCaption,
}

impl TableCaption {
    pub fn new() -> Self {
        Self {
            ix: 0,
            inner: UiCaption::new(),
        }
    }
}

impl Default for TableCaption {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for TableCaption {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.inner.extend(elements);
    }
}

impl Sizable for TableCaption {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.inner = self.inner.with_size(size);
        self
    }
}

impl ChildElement for TableCaption {
    fn with_ix(mut self, ix: usize) -> Self {
        self.ix = ix;
        self.inner = self.inner.with_ix(ix);
        self
    }
}

impl Styled for TableCaption {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}

impl RenderOnce for TableCaption {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        faded("kk-caption", self.ix, self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entrance(elapsed: u64) -> Entrance {
        Entrance {
            table: "t".into(),
            elapsed: ms(elapsed),
            delay: 200,
            stagger: 50,
            duration: 400,
            em: gpui_kit::px(14.),
        }
    }

    #[test]
    fn keyframes_match_the_web() {
        use crate::parity::assert_number_track;
        // The web translates in em; the rise track is in em too.
        assert_number_track("pop-table", "kk-pop-table-rise", "y", &rise_y());
        assert_number_track("pop-table", "kk-pop-table-rise", "opacity", &rise_opacity());
        assert_number_track("pop-table", "kk-pop-table-fade", "opacity", &fade_track());
    }

    #[test]
    fn rows_follow_the_header_and_stop_staggering_at_twelve() {
        let e = entrance(0);
        assert_eq!(e.row_delay(0), 300);
        assert_eq!(e.row_delay(1), 350);
        assert_eq!(e.row_delay(11), 850);
        assert_eq!(e.row_delay(30), 850);
    }

    #[test]
    fn a_row_waits_hidden_below_then_lands() {
        let waiting = entrance(250).row(0);
        assert_eq!(waiting.opacity, 0.0);
        assert!((waiting.y - 0.9 * 14.0).abs() < 1e-4);
        let landed = entrance(300 + 400).row(0);
        assert_eq!(landed, Pose::new());
        // 65 % of the way: just past its place.
        let past = entrance(300 + 260).row(0);
        assert!((past.y + 0.12 * 14.0).abs() < 1e-3);
    }

    #[test]
    fn the_header_fades_after_the_delay() {
        assert_eq!(entrance(100).fade(), 0.0);
        assert_eq!(entrance(500).fade(), 1.0);
    }
}
