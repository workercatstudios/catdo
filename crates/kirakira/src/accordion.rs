//! Accordion: panels that open past their height and settle, and a chevron that turns too far.
//!
//! Replaces `gpui_kit::component::accordion`, rebuilt on gpui-base with GPUI Component's sizes,
//! spacing and theme tokens, so `use kirakira::accordion::*` is a drop-in.
//!
//! Opening grows the panel to its measured height, runs 6 px past it and settles (55 %, then
//! 2 px short at 80 %) in 0.38 s; the text inside fades and rises half a rem, 0.08 s later, on
//! Kirakira's `out` curve. Closing is quicker: the panel falls shut in 0.2 s on the `in` curve
//! while the text fades. The chevron turns past its mark and settles back (0 → 195° → 175° →
//! 180°, 0.42 s), then turns home on the `in` curve in 0.2 s, from wherever it stands when the
//! panel closes (the web's transition), past 180° if the flip is still settling. Nothing plays on
//! first render.
//!
//! Under reduced motion panels open and close at once, the text fades in, and the chevron flips.
//!
//! Interactive, so not composition-safe.

use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::{cell::RefCell, collections::HashSet};

use gpui_kit::base::{
    Accordion as BaseAccordion, AccordionHeader as BaseAccordionHeader,
    AccordionItem as BaseAccordionItem, AccordionPanel as BaseAccordionPanel, AccordionTrigger,
    PlaybackDirection, StyledExt as _, h_flex,
};
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable, Size};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, ElementId, InteractiveElement as _, IntoElement, ParentElement, Pixels,
    RenderOnce, SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled,
    Transformation, Window, div, px, radians, rems,
};

use crate::motion::{Easing, Keyframes, Pose, Timing, Track, delay_ms, ms, transform};
use crate::state_motion::{Reveal, progress, running, since_change};
use crate::theme::ActiveKira as _;

/// How far open a panel is drawn this frame.
pub(crate) struct PanelFrame {
    /// Fraction of the measured content height.
    pub fraction: f32,
    /// Pixels added to it: the overshoot past full height.
    pub extra: Pixels,
    /// The content's own motion: the rise and the fades.
    pub inner: Pose,
}

impl PanelFrame {
    fn rest(open: bool) -> Self {
        Self {
            fraction: if open { 1.0 } else { 0.0 },
            extra: px(0.),
            inner: Pose::new().opacity(if open { 1.0 } else { 0.0 }),
        }
    }
}

const OPEN: std::time::Duration = ms(380);
const CLOSE: std::time::Duration = ms(200);
const RISE: std::time::Duration = ms(300);
const RISE_DELAY: u64 = 80;
const FADE: std::time::Duration = ms(200);

/// `kk-pop-accordion-open`: the height as a fraction of the content's and the pixels past it.
pub(crate) fn open_tracks() -> (Keyframes<f32>, Keyframes<f32>) {
    let ease = Easing::EaseInOut;
    (
        Track::new(ease.clone())
            .at(0.0, 0.0)
            .at(0.55, 1.0)
            .at(0.8, 1.0)
            .at(1.0, 1.0)
            .build(),
        Track::new(ease)
            .at(0.0, 0.0)
            .at(0.55, 6.0)
            .at(0.8, -2.0)
            .at(1.0, 0.0)
            .build(),
    )
}

/// `kk-pop-accordion-flip`: the chevron's turn as it opens, in degrees.
pub(crate) fn flip_track() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.0)
        .at(0.55, 195.0)
        .at(0.8, 175.0)
        .at(1.0, 180.0)
        .build()
}

/// `kk-pop-accordion-rise`: the content's offset in rems (`translate: 0 0.5rem`) and its opacity,
/// both to their natural values. Played on the `out` curve.
pub(crate) fn rise_tracks(ease: Easing) -> (Keyframes<f32>, Keyframes<f32>) {
    (
        Track::new(ease.clone()).at(0.0, 0.5).at(1.0, 0.0).build(),
        Track::new(ease).at(0.0, 0.0).at(1.0, 1.0).build(),
    )
}

/// `kk-pop-accordion-fade`: the content fading out.
pub(crate) fn fade_track(ease: Easing) -> Keyframes<f32> {
    Track::new(ease).at(0.0, 1.0).at(1.0, 0.0).build()
}

/// The panel motion shared with [`crate::collapsible`]: where a panel keyed by `id` is, `open` or
/// closing, this frame. Asks for frames while it moves.
pub(crate) fn panel_frame(
    id: &ElementId,
    open: bool,
    window: &mut Window,
    cx: &mut App,
) -> PanelFrame {
    let since = since_change((id.clone(), "kk-open"), open, window, cx);
    let reduced = cx.reduce_motion();
    let curves = cx.curves();
    if open {
        if reduced {
            // The text fades in (`kk-pop-accordion-fade 0.2s ease-out reverse`); nothing moves.
            let Some(elapsed) = running(since, FADE, window) else {
                return PanelFrame::rest(true);
            };
            let fade = fade_track(Easing::EaseOut);
            let timing = Timing::new(FADE).direction(PlaybackDirection::Reverse);
            let opacity = fade.sample(timing.sample(elapsed).directed_progress);
            return PanelFrame {
                inner: Pose::new().opacity(opacity),
                ..PanelFrame::rest(true)
            };
        }
        let Some(elapsed) = running(since, OPEN, window) else {
            return PanelFrame::rest(true);
        };
        let (fraction, extra) = open_tracks();
        let t = progress(elapsed, OPEN);
        let (rise, appear) = rise_tracks(curves.out);
        let rise_t = Timing::new(RISE)
            .delay(delay_ms(RISE_DELAY))
            .sample(elapsed)
            .directed_progress;
        let inner = Pose::new()
            .y(rise.sample(rise_t) * window.rem_size().as_f32())
            .opacity(appear.sample(rise_t));
        PanelFrame {
            fraction: fraction.sample(t),
            extra: px(extra.sample(t)),
            inner,
        }
    } else {
        if reduced {
            return PanelFrame::rest(false);
        }
        let Some(elapsed) = running(since, CLOSE, window) else {
            return PanelFrame::rest(false);
        };
        let t = progress(elapsed, CLOSE);
        let fraction = Track::new(curves.r#in).at(0.0, 1.0).at(1.0, 0.0).build();
        let fade = fade_track(Easing::EaseIn);
        PanelFrame {
            fraction: fraction.sample(t),
            extra: px(0.),
            inner: Pose::new().opacity(fade.sample(t)),
        }
    }
}

const FLIP: Duration = ms(420);

/// Where an open chevron stands `opened` after it opened: on its flip, then at rest. `None` is
/// open since the first render, at rest.
fn open_turn(opened: Option<Duration>) -> f32 {
    match opened {
        Some(elapsed) if elapsed < FLIP => flip_track().sample(progress(elapsed, FLIP)),
        _ => 180.0,
    }
}

/// When a chevron last opened or closed, and the turn it closed from.
struct Chevron {
    open: bool,
    changed: Option<Instant>,
    from: f32,
}

/// The chevron's turn in degrees this frame.
///
/// Closing is the web's `transition: rotate 0.2s`: it turns home from wherever it stands when it
/// closes, which is past 180° if the flip is still settling.
fn chevron_turn(id: &ElementId, open: bool, window: &mut Window, cx: &mut App) -> f32 {
    let now = crate::motion::now();
    let state = window.use_keyed_state((id.clone(), "kk-chevron"), cx, |_, _| Chevron {
        open,
        changed: None,
        from: 180.0,
    });
    if state.read(cx).open != open {
        state.update(cx, |chevron, _| {
            // The last change was the opening, if there was one.
            if !open {
                chevron.from =
                    open_turn(chevron.changed.map(|at| now.saturating_duration_since(at)));
            }
            chevron.open = open;
            chevron.changed = Some(now);
        });
    }
    let (since, from) = {
        let chevron = state.read(cx);
        (
            chevron.changed.map(|at| now.saturating_duration_since(at)),
            chevron.from,
        )
    };
    let rest = if open { 180.0 } else { 0.0 };
    if cx.reduce_motion() {
        return rest;
    }
    if open {
        match running(since, FLIP, window) {
            Some(elapsed) => open_turn(Some(elapsed)),
            None => rest,
        }
    } else {
        match running(since, CLOSE, window) {
            Some(elapsed) => close_track(from, cx.curves().r#in).sample(progress(elapsed, CLOSE)),
            None => rest,
        }
    }
}

/// The turn home, from `from` degrees on the `in` curve.
fn close_track(from: f32, ease: Easing) -> Keyframes<f32> {
    Track::new(ease).at(0.0, from).at(1.0, 0.0).build()
}

/// Accordion element.
#[derive(IntoElement)]
pub struct Accordion {
    id: ElementId,
    style: StyleRefinement,
    multiple: bool,
    size: Size,
    bordered: bool,
    disabled: bool,
    children: Vec<AccordionItem>,
    on_toggle_click: Option<Rc<dyn Fn(&[usize], &mut Window, &mut App)>>,
}

impl Accordion {
    /// Create a new Accordion with the given ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            multiple: false,
            size: Size::default(),
            bordered: true,
            children: Vec::new(),
            disabled: false,
            on_toggle_click: None,
        }
    }

    /// Set whether multiple accordion items can be opened simultaneously, default: false
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Set whether the accordion items have borders, default: true
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Set whether the accordion is disabled, default: false
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Adds an AccordionItem to the Accordion.
    pub fn item<F>(mut self, child: F) -> Self
    where
        F: FnOnce(AccordionItem) -> AccordionItem,
    {
        let item = child(AccordionItem::new());
        self.children.push(item);
        self
    }

    /// Sets the on_toggle_click callback for the AccordionGroup.
    ///
    /// The first argument `Vec<usize>` is the indices of the open accordions.
    pub fn on_toggle_click(
        mut self,
        on_toggle_click: impl Fn(&[usize], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle_click = Some(Rc::new(on_toggle_click));
        self
    }
}

impl Sizable for Accordion {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Accordion {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Accordion {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let open_indices = Rc::new(RefCell::new(HashSet::new()));
        let multiple = self.multiple;
        let last_ix = self.children.len().saturating_sub(1);

        BaseAccordion::new(self.id)
            .v_flex()
            .size_full()
            // The bordered accordion is a single rounded card, the items are
            // joined by their separators.
            .when(self.bordered, |this| {
                this.border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius_lg)
                    .overflow_hidden()
            })
            .refine_style(&self.style)
            .children(
                self.children
                    .into_iter()
                    .enumerate()
                    .map(|(ix, accordion)| {
                        if accordion.open {
                            open_indices.borrow_mut().insert(ix);
                        }

                        accordion
                            .index(ix)
                            .last(ix == last_ix)
                            .with_size(self.size)
                            .disabled(self.disabled)
                            .on_toggle_click({
                                let open_indices = open_indices.clone();
                                move |open, _, _| {
                                    let mut open_indices = open_indices.borrow_mut();
                                    if *open {
                                        if !multiple {
                                            open_indices.clear();
                                        }
                                        open_indices.insert(ix);
                                    } else {
                                        open_indices.remove(&ix);
                                    }
                                }
                            })
                    }),
            )
            .when_some(
                self.on_toggle_click.filter(|_| !self.disabled),
                |this, on_toggle| {
                    this.on_click(move |_, window, cx| {
                        let open_indices =
                            open_indices.borrow().iter().copied().collect::<Vec<_>>();
                        on_toggle(&open_indices, window, cx)
                    })
                },
            )
    }
}

/// An Accordion is a vertically stacked list of items, each of which can be expanded to reveal the content associated with it.
#[derive(IntoElement)]
pub struct AccordionItem {
    index: usize,
    last: bool,
    style: StyleRefinement,
    hover_style: Option<StyleRefinement>,
    title_style: StyleRefinement,
    content_style: StyleRefinement,
    icon: Option<Icon>,
    title: AnyElement,
    children: Vec<AnyElement>,
    open: bool,
    size: Size,
    disabled: bool,
    on_toggle_click: Option<Arc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl Default for AccordionItem {
    fn default() -> Self {
        Self::new()
    }
}

impl AccordionItem {
    /// Create a new AccordionItem.
    pub fn new() -> Self {
        Self {
            index: 0,
            last: false,
            style: StyleRefinement::default(),
            hover_style: None,
            title_style: StyleRefinement::default(),
            content_style: StyleRefinement::default(),
            icon: None,
            title: SharedString::default().into_any_element(),
            children: Vec::new(),
            open: false,
            disabled: false,
            on_toggle_click: None,
            size: Size::default(),
        }
    }

    /// Set the icon for the accordion item.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the title for the accordion item.
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = title.into_any_element();
        self
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set extra style for the title row.
    pub fn title_style(mut self, style: StyleRefinement) -> Self {
        self.title_style = style;
        self
    }

    /// Set the style of the title row while the mouse is over it.
    ///
    /// There is no hover style by default. The title row is the part that
    /// toggles the item, so the hover feedback belongs there, not on the
    /// whole item.
    pub fn hover(mut self, f: impl FnOnce(StyleRefinement) -> StyleRefinement) -> Self {
        self.hover_style = Some(f(StyleRefinement::default()));
        self
    }

    /// Set extra style for the content below the title.
    pub fn content_style(mut self, style: StyleRefinement) -> Self {
        self.content_style = style;
        self
    }

    fn index(mut self, index: usize) -> Self {
        self.index = index;
        self
    }

    fn last(mut self, last: bool) -> Self {
        self.last = last;
        self
    }

    fn on_toggle_click(
        mut self,
        on_toggle_click: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle_click = Some(Arc::new(on_toggle_click));
        self
    }
}

impl ParentElement for AccordionItem {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for AccordionItem {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for AccordionItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for AccordionItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let text_size = match self.size {
            Size::XSmall => rems(0.8125),
            Size::Large => rems(1.0),
            _ => rems(0.875),
        };
        let key: ElementId = ("kk-accordion-item", self.index).into();
        let frame = panel_frame(&key, self.open, window, cx);
        let turn = chevron_turn(&key, self.open, window, cx);

        let trigger = AccordionTrigger::new(("trigger", self.index))
            .open(self.open)
            .disabled(self.disabled)
            .h_flex()
            .justify_between()
            .gap_3()
            .font_medium()
            .map(|this| match self.size {
                Size::XSmall => this.py_1().px_1p5(),
                Size::Small => this.py_1p5().px_2(),
                Size::Large => this.py_3().px_4(),
                _ => this.py_2().px_3(),
            })
            .when(self.open, |this| this.text_color(cx.theme().foreground))
            .refine_style(&self.title_style)
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .map(|this| match self.size {
                        Size::XSmall | Size::Small => this.gap_1(),
                        _ => this.gap_2(),
                    })
                    .when_some(self.icon, |this, icon| {
                        this.child(icon.with_size(self.size))
                    })
                    .child(self.title),
            )
            .when(!self.disabled, |this| {
                this.when_some(self.hover_style, |this, hover_style| {
                    this.hover(move |this| this.refine_style(&hover_style))
                })
                .child(
                    Icon::new(IconName::ChevronDown)
                        .xsmall()
                        .flex_none()
                        .text_color(cx.theme().muted_foreground)
                        .transform(Transformation::rotate(radians(turn.to_radians()))),
                )
                .when_some(self.on_toggle_click, |this, on_toggle_click| {
                    this.on_change(move |open, _, window, cx| {
                        on_toggle_click(&open, window, cx);
                    })
                })
            });

        let content = div()
            .map(|this| match self.size {
                Size::XSmall => this.pb_1().px_1p5(),
                Size::Small => this.pb_1p5().px_2(),
                Size::Large => this.pb_3().px_4(),
                _ => this.pb_2().px_3(),
            })
            .refine_style(&self.content_style)
            .children(self.children);

        div().flex_1().child(
            BaseAccordionItem::new()
                .open(self.open)
                .disabled(self.disabled)
                .header(
                    BaseAccordionHeader::new(trigger)
                        .id(("header", self.index))
                        .w_full(),
                )
                .panel(
                    BaseAccordionPanel::new()
                        .id(("panel", self.index))
                        .open(self.open)
                        .keep_mounted(true)
                        .w_full()
                        .child(Reveal::new(
                            ("content", self.index),
                            frame.fraction,
                            frame.extra,
                            transform((key.clone(), "kk-inner"), frame.inner, content),
                        )),
                )
                .v_flex()
                .w_full()
                .bg(cx.theme().tokens.accordion)
                .overflow_hidden()
                .when(!self.last, |this| {
                    this.border_b_1().border_color(cx.theme().border)
                })
                .text_size(text_size)
                .refine_style(&self.style),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_runs_six_pixels_past_and_settles() {
        let (fraction, extra) = open_tracks();
        assert_eq!(fraction.sample(0.0), 0.0);
        assert_eq!(fraction.sample(0.55), 1.0);
        assert_eq!(extra.sample(0.55), 6.0);
        assert_eq!(extra.sample(0.8), -2.0);
        assert_eq!(fraction.sample(1.0), 1.0);
        assert_eq!(extra.sample(1.0), 0.0);
    }

    #[test]
    fn keyframes_match_the_web() {
        use crate::parity::assert_number_track;
        let ease = Easing::EaseInOut;
        assert_number_track(
            "pop-accordion",
            "kk-pop-accordion-flip",
            "rotate",
            &flip_track(),
        );
        let (rise, appear) = rise_tracks(ease.clone());
        // The parser reads `0.5rem` as 0.5, and the track is in rems.
        assert_number_track("pop-accordion", "kk-pop-accordion-rise", "y", &rise);
        assert_number_track("pop-accordion", "kk-pop-accordion-rise", "opacity", &appear);
        assert_number_track(
            "pop-accordion",
            "kk-pop-accordion-fade",
            "opacity",
            &fade_track(ease),
        );
        // `kk-pop-accordion-open` keys `height` on `calc(var(--accordion-panel-height))`,
        // which the parity parser can't read; `open_runs_six_pixels_past_and_settles` checks it.
    }

    #[test]
    fn chevron_turns_home_from_where_it_stands() {
        // Closed at the flip's peak, 55 % into its 0.42 s, it turns home from 195°.
        let from = super::open_turn(Some(ms(231)));
        assert!((from - 195.0).abs() < 1e-3, "{from}");
        // Settled, or open since the first render: from 180°.
        assert_eq!(super::open_turn(Some(ms(1000))), 180.0);
        assert_eq!(super::open_turn(None), 180.0);
        let home = super::close_track(from, Easing::Linear);
        assert_eq!(home.sample(0.0), from);
        assert_eq!(home.sample(1.0), 0.0);
    }

    #[test]
    fn chevron_turns_past_its_mark() {
        let flip = flip_track();
        assert_eq!(flip.sample(0.55), 195.0);
        assert_eq!(flip.sample(0.8), 175.0);
        assert_eq!(flip.sample(1.0), 180.0);
    }
}
