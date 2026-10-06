//! Tabs: an indicator that springs to the new tab and squashes as it lands.
//!
//! Replaces `gpui_kit::component::tab`, rebuilt on gpui-base with GPUI Component's variants, sizes
//! and theme tokens, so `use kirakira::tab::*` is a drop-in. [`TabPanel`] is an extra: a panel
//! that rises in when the selected tab changes.
//!
//! The sliding indicator of the segmented, pill and underline bars moves to the new tab on
//! Kirakira's `spring` curve (`cubic-bezier(0.34, 1.56, 0.64, 1)`, a little overshoot) in 0.42 s,
//! resizing as it goes. Heading for a tab at the end of the bar, where an overshoot would poke out
//! of the track, it eases on the `out` curve instead. Each change restarts a squash on the
//! indicator: stretched along the travel (106 % × 92 %) at 30 %, squashed as it lands (96 % ×
//! 106 %) at 60 %, a small rebound (101.5 % × 98.5 %) at 82 %, settled, in 0.46 s on
//! `ease-in-out`. The indicator fades in over 0.15 s when it first finds its tab. A [`TabPanel`]
//! rises half a rem and fades in over 0.25 s on the `out` curve when it mounts or its tab changes.
//!
//! Under reduced motion the indicator jumps to the new tab without squashing, and the panel only
//! fades in, over 0.2 s.
//!
//! The indicator is an owned shape, so its squash is exact (its corner radius keeps its size,
//! where CSS `scale` would scale it too). Interactive, so not composition-safe.
//!
//! Differences from the web version: the web pill's slide is a CSS transition of `translate` and
//! `width`; here it is the same curve on Kirakira's clock, retargeted from where the indicator is.
//! GPUI Component's tab bar is horizontal only, so there is no vertical variant.

use std::rc::Rc;
use std::time::{Duration, Instant};
use std::{cell::RefCell, time::Duration as StdDuration};

use gpui_kit::base::{
    ElementExt as _, InteractiveElementExt as _, StyledExt as _, Tab as BaseTab, Tabs as BaseTabs,
    h_flex,
};
use gpui_kit::component::animation::{Lerp, ease_in_out_cubic};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::{ActiveTheme, Icon, IconName, Selectable, Sizable, Size};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    Anchor, Animation, AnimationExt as _, AnyElement, App, Background, Bounds, BoxShadow,
    ClickEvent, Edges, ElementId, Entity, Hsla, InteractiveElement, Interactivity, IntoElement,
    ParentElement, Pixels, RenderOnce, ScrollHandle, SharedString, StatefulInteractiveElement,
    StyleRefinement, Styled, Window, div, hsla, px, relative, transparent_white,
};

use crate::motion::{Easing, Keyframes, Pose, Track, ms, now, transform};
use crate::state_motion::{glide, progress, running, since_change};
use crate::theme::ActiveKira as _;

/// Tab variants.
#[derive(Debug, Clone, Default, Copy, PartialEq, Eq, Hash)]
pub enum TabVariant {
    #[default]
    Tab,
    Outline,
    Pill,
    Segmented,
    Underline,
}

impl TabVariant {
    fn height(&self, size: Size) -> Pixels {
        match size {
            Size::XSmall => match self {
                TabVariant::Underline => px(26.),
                _ => px(20.),
            },
            Size::Small => match self {
                TabVariant::Underline => px(30.),
                _ => px(24.),
            },
            Size::Large => match self {
                TabVariant::Underline => px(44.),
                _ => px(36.),
            },
            _ => match self {
                TabVariant::Underline => px(36.),
                _ => px(32.),
            },
        }
    }

    pub(super) fn inner_height(&self, size: Size) -> Pixels {
        match size {
            Size::XSmall => match self {
                TabVariant::Tab | TabVariant::Outline | TabVariant::Pill => px(18.),
                TabVariant::Segmented => px(16.),
                TabVariant::Underline => px(20.),
            },
            Size::Small => match self {
                TabVariant::Tab | TabVariant::Outline | TabVariant::Pill => px(22.),
                TabVariant::Segmented => px(18.),
                TabVariant::Underline => px(22.),
            },
            Size::Large => match self {
                TabVariant::Tab | TabVariant::Outline | TabVariant::Pill => px(36.),
                TabVariant::Segmented => px(28.),
                TabVariant::Underline => px(32.),
            },
            _ => match self {
                TabVariant::Tab => px(30.),
                TabVariant::Outline | TabVariant::Pill => px(26.),
                TabVariant::Segmented => px(24.),
                TabVariant::Underline => px(26.),
            },
        }
    }

    /// Default px(12) to match a dock tab bar's px_3
    fn inner_paddings(&self, size: Size) -> Edges<Pixels> {
        let mut padding_x = match size {
            Size::XSmall => px(8.),
            Size::Small => px(10.),
            Size::Large => px(16.),
            _ => px(12.),
        };

        if matches!(self, TabVariant::Underline) {
            padding_x = px(0.);
        }

        Edges {
            left: padding_x,
            right: padding_x,
            ..Default::default()
        }
    }

    fn inner_margins(&self, size: Size) -> Edges<Pixels> {
        match size {
            Size::XSmall => match self {
                TabVariant::Underline => Edges {
                    top: px(1.),
                    bottom: px(2.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
            Size::Small => match self {
                TabVariant::Underline => Edges {
                    top: px(2.),
                    bottom: px(3.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
            Size::Large => match self {
                TabVariant::Underline => Edges {
                    top: px(5.),
                    bottom: px(6.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
            _ => match self {
                TabVariant::Underline => Edges {
                    top: px(3.),
                    bottom: px(4.),
                    ..Default::default()
                },
                _ => Edges::all(px(0.)),
            },
        }
    }

    fn normal(&self, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges::all(px(1.)),
                border_color: cx.theme().border,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().foreground,
                bg: cx.theme().transparent.into(),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().tab_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: cx.theme().transparent.into(),
                borders: Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
        }
    }

    fn hovered(&self, selected: bool, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: cx.theme().secondary_foreground,
                bg: cx.theme().tokens.secondary_hover.into(),
                borders: Edges::all(px(1.)),
                border_color: cx.theme().border,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().secondary_foreground,
                bg: cx.theme().tokens.secondary.into(),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: if selected {
                    cx.theme().tokens.background.into()
                } else {
                    cx.theme().transparent.into()
                },
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: cx.theme().transparent.into(),
                borders: Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
                border_color: cx.theme().transparent,
                ..Default::default()
            },
        }
    }

    fn selected(&self, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().tokens.tab_active.into(),
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                border_color: cx.theme().border,
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: cx.theme().primary,
                bg: cx.theme().transparent.into(),
                borders: Edges::all(px(1.)),
                border_color: cx.theme().primary,
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: cx.theme().primary_foreground,
                bg: cx.theme().tokens.primary.into(),
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                inner_bg: cx.theme().tokens.background.into(),
                shadow: true,
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().tab_active_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
                border_color: cx.theme().primary,
                ..Default::default()
            },
        }
    }

    fn disabled(&self, selected: bool, cx: &App) -> TabStyle {
        match self {
            TabVariant::Tab => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().transparent.into(),
                border_color: if selected {
                    cx.theme().border
                } else {
                    cx.theme().transparent
                },
                borders: Edges {
                    left: px(1.),
                    right: px(1.),
                    ..Default::default()
                },
                ..Default::default()
            },
            TabVariant::Outline => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().transparent.into(),
                borders: Edges::all(px(1.)),
                border_color: if selected {
                    cx.theme().primary
                } else {
                    cx.theme().border
                },
                ..Default::default()
            },
            TabVariant::Pill => TabStyle {
                fg: if selected {
                    cx.theme().primary_foreground.opacity(0.5)
                } else {
                    cx.theme().muted_foreground
                },
                bg: if selected {
                    cx.theme().primary.opacity(0.5).into()
                } else {
                    cx.theme().transparent.into()
                },
                ..Default::default()
            },
            TabVariant::Segmented => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().tokens.tab_bar.into(),
                inner_bg: if selected {
                    cx.theme().tokens.background.into()
                } else {
                    cx.theme().transparent.into()
                },
                ..Default::default()
            },
            TabVariant::Underline => TabStyle {
                fg: cx.theme().muted_foreground,
                bg: cx.theme().transparent.into(),
                border_color: if selected {
                    cx.theme().border
                } else {
                    cx.theme().transparent
                },
                borders: Edges {
                    bottom: px(2.),
                    ..Default::default()
                },
                ..Default::default()
            },
        }
    }

    pub(super) fn tab_bar_radius(&self, size: Size, cx: &App) -> Pixels {
        if *self != TabVariant::Segmented {
            return px(0.);
        }

        match size {
            Size::XSmall | Size::Small => cx.theme().radius,
            Size::Large => cx.theme().radius_lg,
            _ => cx.theme().radius_lg,
        }
    }

    fn radius(&self, size: Size, cx: &App) -> Pixels {
        match self {
            TabVariant::Outline | TabVariant::Pill => cx.theme().radius_full(),
            TabVariant::Segmented => match size {
                Size::XSmall | Size::Small => cx.theme().radius,
                Size::Large => cx.theme().radius_lg,
                _ => cx.theme().radius_lg,
            },
            _ => px(0.),
        }
    }

    pub(super) fn inner_radius(&self, size: Size, cx: &App) -> Pixels {
        match self {
            // The inset the active pill sits at, taken off the bar's own radius
            // so the two curves stay concentric. Floored at zero: a square bar
            // has nothing to inset from.
            TabVariant::Segmented => match size {
                Size::Large => (self.tab_bar_radius(size, cx) - px(3.)).max(px(0.)),
                _ => (self.tab_bar_radius(size, cx) - px(2.)).max(px(0.)),
            },
            _ => px(0.),
        }
    }
}

#[allow(dead_code)]
struct TabStyle {
    borders: Edges<Pixels>,
    border_color: Hsla,
    bg: Background,
    fg: Hsla,
    shadow: bool,
    inner_bg: Background,
}

impl Default for TabStyle {
    fn default() -> Self {
        TabStyle {
            borders: Edges::all(px(0.)),
            border_color: transparent_white(),
            bg: transparent_white().into(),
            fg: transparent_white(),
            shadow: false,
            inner_bg: transparent_white().into(),
        }
    }
}

/// A Tab element for the [`TabBar`].
#[derive(IntoElement)]
pub struct Tab {
    ix: usize,
    base: BaseTab,
    pub(super) label: Option<SharedString>,
    aria_label: Option<SharedString>,
    pub(super) icon: Option<Icon>,
    prefix: Option<AnyElement>,
    pub(super) tab_bar_prefix: Option<bool>,
    suffix: Option<AnyElement>,
    children: Vec<AnyElement>,
    variant: TabVariant,
    size: Size,
    pub(super) disabled: bool,
    pub(super) selected: bool,
    pub(super) indicator_active: bool,
    pub(super) indicator_ready: bool,
    /// Animation epoch of the [`TabBar`] indicator; increments on every
    /// tab switch. Used to key the selected tab's text color fade so it
    /// restarts in sync with the indicator slide.
    pub(super) indicator_epoch: u64,
    pub(super) max_width: Option<Pixels>,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
}

impl From<&'static str> for Tab {
    fn from(label: &'static str) -> Self {
        Self::new().label(label)
    }
}

impl From<String> for Tab {
    fn from(label: String) -> Self {
        Self::new().label(label)
    }
}

impl From<SharedString> for Tab {
    fn from(label: SharedString) -> Self {
        Self::new().label(label)
    }
}

impl From<Icon> for Tab {
    fn from(icon: Icon) -> Self {
        Self::default().icon(icon)
    }
}

impl From<IconName> for Tab {
    fn from(icon_name: IconName) -> Self {
        Self::default().icon(Icon::new(icon_name))
    }
}

impl Default for Tab {
    fn default() -> Self {
        Self {
            ix: 0,
            base: BaseTab::new(0usize),
            label: None,
            aria_label: None,
            icon: None,
            tab_bar_prefix: None,
            children: Vec::new(),
            disabled: false,
            selected: false,
            indicator_active: false,
            indicator_ready: true,
            indicator_epoch: 0,
            prefix: None,
            suffix: None,
            variant: TabVariant::default(),
            size: Size::default(),
            max_width: None,
            on_click: None,
        }
    }
}

impl Tab {
    /// Create a new tab with a label.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set label for the tab.
    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Set the accessible label for the tab.
    pub fn aria_label(mut self, label: impl Into<SharedString>) -> Self {
        self.aria_label = Some(label.into());
        self
    }

    fn a11y_label(&self) -> Option<SharedString> {
        self.aria_label.clone().or_else(|| self.label.clone())
    }

    /// Set icon for the tab.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set Tab Variant.
    pub fn with_variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Use Pill variant.
    pub fn pill(mut self) -> Self {
        self.variant = TabVariant::Pill;
        self
    }

    /// Use outline variant.
    pub fn outline(mut self) -> Self {
        self.variant = TabVariant::Outline;
        self
    }

    /// Use Segmented variant.
    pub fn segmented(mut self) -> Self {
        self.variant = TabVariant::Segmented;
        self
    }

    /// Use Underline variant.
    pub fn underline(mut self) -> Self {
        self.variant = TabVariant::Underline;
        self
    }

    /// Set the left side of the tab
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    /// Set the right side of the tab
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Set disabled state to the tab, default false.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Set the click handler for the tab.
    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    /// Set index to the tab.
    pub(crate) fn ix(mut self, ix: usize) -> Self {
        self.ix = ix;
        self.base = self.base.id(ix);
        self
    }

    /// Set if the tab bar has a prefix.
    pub(crate) fn tab_bar_prefix(mut self, tab_bar_prefix: bool) -> Self {
        self.tab_bar_prefix = Some(tab_bar_prefix);
        self
    }

    /// Set the maximum width of the tab, see [`TabBar::max_width`].
    pub(super) fn max_width(mut self, max_width: Option<Pixels>) -> Self {
        self.max_width = max_width;
        self
    }
}

impl ParentElement for Tab {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Selectable for Tab {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl InteractiveElement for Tab {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl StatefulInteractiveElement for Tab {}

impl Styled for Tab {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl Sizable for Tab {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for Tab {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut normal_style = self.variant.normal(cx);
        let mut selected_style = self.variant.selected(cx);
        let mut disabled_style = self.variant.disabled(self.selected, cx);
        let mut hover_style = self.variant.hovered(self.selected, cx);
        if self.disabled {
            hover_style = self.variant.disabled(self.selected, cx);
        }
        let tab_bar_prefix = self.tab_bar_prefix.unwrap_or_default();
        if !tab_bar_prefix && self.ix == 0 && self.variant == TabVariant::Tab {
            normal_style.borders.left = px(0.);
            selected_style.borders.left = px(0.);
            disabled_style.borders.left = px(0.);
            hover_style.borders.left = px(0.);
        }
        let tab_style = if self.disabled {
            &disabled_style
        } else if self.selected {
            &selected_style
        } else {
            &normal_style
        };
        let radius = self.variant.radius(self.size, cx);
        let inner_radius = self.variant.inner_radius(self.size, cx);
        let inner_paddings = self.variant.inner_paddings(self.size);
        let inner_margins = self.variant.inner_margins(self.size);
        let inner_height = self.variant.inner_height(self.size);
        let height = self.variant.height(self.size);
        let aria_label = self.a11y_label();

        let segmented_indicator_active =
            self.variant == TabVariant::Segmented && self.indicator_active;
        let has_inline_inner_bg =
            self.selected && segmented_indicator_active && !self.indicator_ready;
        let inline_inner_bg = tab_style.inner_bg;
        let (inner_bg, hover_inner_bg) = if segmented_indicator_active && self.indicator_ready {
            (cx.theme().transparent.into(), cx.theme().transparent.into())
        } else if has_inline_inner_bg {
            (inline_inner_bg, inline_inner_bg)
        } else {
            (tab_style.inner_bg, hover_style.inner_bg)
        };
        let inner_shadow = tab_style.shadow && !segmented_indicator_active;

        // When a sliding indicator is active and ready, it alone represents the
        // selected state. Suppress the selected tab's own active background/border
        // so the two don't overlap during the switch animation (Segmented already
        // does this for its `inner_bg` above). Skip disabled tabs so a
        // disabled-selected tab keeps its dimmed styling instead of the
        // full-strength indicator color.
        let suppress_active_visual =
            self.selected && !self.disabled && self.indicator_active && self.indicator_ready;
        // Pill paints its active state via the outer `bg`.
        let selected_outer_bg = if suppress_active_visual && self.variant == TabVariant::Pill {
            cx.theme().transparent.into()
        } else {
            selected_style.bg
        };
        // Underline paints its active state via the bottom `border_color`.
        let selected_outer_border_color =
            if suppress_active_visual && self.variant == TabVariant::Underline {
                cx.theme().transparent
            } else {
                selected_style.border_color
            };

        // For Pill, the newly selected tab's text color (`primary_foreground`)
        // would otherwise snap to white instantly while the indicator is still
        // sliding into place. Fade it from the normal color in sync with the
        // indicator slide (keyed on the indicator epoch so it restarts on each
        // switch). `epoch == 0` is the initial layout (no slide), so we skip it.
        let animate_fg = self.selected
            && !self.disabled
            && self.variant == TabVariant::Pill
            && self.indicator_active
            && self.indicator_ready
            && self.indicator_epoch > 0;
        let fg_from = self.variant.normal(cx).fg;
        let fg_to = tab_style.fg;
        // Icon-only tabs are fixed-size and exempt from `max_width`.
        let max_width = self.max_width.filter(|_| self.icon.is_none());

        let inner_content = h_flex()
            .flex_1()
            .h(inner_height)
            .line_height(relative(1.25))
            .whitespace_nowrap()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .margins(inner_margins)
            // Normally the label decides the tab width, so it never shrinks. With
            // `max_width` it is the one part that gives way.
            .map(|this| match max_width {
                Some(_) => this.flex_auto(),
                None => this.flex_shrink_0(),
            })
            .map(|this| match self.icon {
                Some(icon) => this
                    .w(inner_height * 1.25)
                    .child(icon.map(|this| match self.size {
                        Size::XSmall => this.size_2p5(),
                        Size::Small => this.size_3p5(),
                        Size::Large => this.size_4(),
                        _ => this.size_4(),
                    })),
                None => this
                    .paddings(inner_paddings)
                    .map(|this| match (self.label, max_width) {
                        // Text always takes its natural width, so it needs a box
                        // that is allowed to shrink to ellipsize inside of.
                        (Some(label), Some(_)) => this.child(
                            div()
                                .min_w_0()
                                .whitespace_nowrap()
                                .text_ellipsis()
                                .child(label),
                        ),
                        (Some(label), None) => this.child(label),
                        (None, _) => this,
                    })
                    .children(self.children),
            })
            .bg(inner_bg)
            .rounded(inner_radius)
            .when(inner_shadow, |this| this.shadow_xs())
            .hover(|this| this.bg(hover_inner_bg).rounded(inner_radius));

        let inner_element = if animate_fg {
            inner_content
                .with_animation(
                    ElementId::NamedInteger("tab-fg".into(), self.indicator_epoch),
                    Animation::new(Duration::from_millis(200)).with_easing(ease_in_out_cubic),
                    move |this, delta| this.text_color(Lerp::lerp(&fg_from, &fg_to, delta)),
                )
                .into_any_element()
        } else {
            inner_content.into_any_element()
        };

        self.base
            .id(self.ix)
            .selected(self.selected)
            .disabled(self.disabled)
            .when_some(aria_label, |this, label| this.accessibility_label(label))
            .styles(|styles| {
                styles
                    .selected(|style| {
                        style
                            .text_color(selected_style.fg)
                            .bg(selected_outer_bg)
                            .border_l(selected_style.borders.left)
                            .border_r(selected_style.borders.right)
                            .border_t(selected_style.borders.top)
                            .border_b(selected_style.borders.bottom)
                            .border_color(selected_outer_border_color)
                    })
                    .disabled(|style| {
                        style
                            .text_color(disabled_style.fg)
                            .bg(disabled_style.bg)
                            .border_l(disabled_style.borders.left)
                            .border_r(disabled_style.borders.right)
                            .border_t(disabled_style.borders.top)
                            .border_b(disabled_style.borders.bottom)
                            .border_color(disabled_style.border_color)
                    })
            })
            .relative()
            .flex()
            // Wrapping would move the overflow onto a clipped second line instead
            // of letting the label shrink, so a capped tab lays out on one line.
            .map(|this| match max_width {
                Some(max_width) => this.flex_nowrap().max_w(max_width),
                None => this.flex_wrap(),
            })
            .gap_1()
            .items_center()
            .flex_shrink_0()
            .h(height)
            .overflow_hidden()
            .map(|this| match self.size {
                Size::XSmall => this.text_xs(),
                Size::Large => this.text_base(),
                _ => this.text_sm(),
            })
            .rounded(radius)
            .when(!self.selected && !self.disabled, |this| {
                this.text_color(normal_style.fg)
                    .bg(normal_style.bg)
                    .border_l(normal_style.borders.left)
                    .border_r(normal_style.borders.right)
                    .border_t(normal_style.borders.top)
                    .border_b(normal_style.borders.bottom)
                    .border_color(normal_style.border_color)
            })
            .hover(|this| {
                // Always register the hover style: GPUI only refreshes the cached
                // hover state while one is present. If the selected tab skipped it,
                // the stale state would keep hover colors after unselecting.
                if self.selected || self.disabled {
                    return this;
                }
                this.text_color(hover_style.fg)
                    .bg(hover_style.bg)
                    .border_l(hover_style.borders.left)
                    .border_r(hover_style.borders.right)
                    .border_t(hover_style.borders.top)
                    .border_b(hover_style.borders.bottom)
                    .border_color(hover_style.border_color)
                    .rounded(radius)
            })
            .when(has_inline_inner_bg, |this| {
                this.child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top_0()
                        .bottom_0()
                        .flex()
                        .items_center()
                        .child(
                            div()
                                .w_full()
                                .h(inner_height)
                                .bg(inline_inner_bg)
                                .rounded(inner_radius)
                                .when(tab_style.shadow, |this| this.shadow_sm()),
                        ),
                )
            })
            // Under `max_width` the label is the only part that gives way, so
            // hold the prefix and suffix (e.g. a close button) at their full size.
            .when_some(self.prefix, |this, prefix| {
                this.child(
                    div()
                        .when_some(max_width, |this, _| this.flex_shrink_0())
                        .child(prefix),
                )
            })
            .child(inner_element)
            .when_some(self.suffix, |this, suffix| {
                this.child(
                    div()
                        .when_some(max_width, |this, _| this.flex_shrink_0())
                        .child(suffix),
                )
            })
            .when_some(self.on_click.clone(), |this, on_click| {
                this.on_click(move |event, window, cx| on_click(event, window, cx))
            })
    }
}

/// shadcn/ui's `shadow-sm` with GPUI's halved blur, as GPUI Component draws a segmented pill.
fn raised_shadow() -> Vec<BoxShadow> {
    let ink = hsla(0., 0., 0., 0.1);
    vec![
        BoxShadow::new(px(0.), px(1.), ink).blur_radius(px(1.5)),
        BoxShadow::new(px(0.), px(1.), ink)
            .blur_radius(px(1.))
            .spread_radius(px(-1.)),
    ]
}

/// `kk-pop-tabs-land`: the indicator's squash, as `sx` and `sy`, for a horizontal bar.
pub(crate) fn land_track() -> Keyframes<Pose> {
    Track::new(Easing::EaseInOut)
        .at(0.0, Pose::new())
        .at(0.3, Pose::new().scale_xy(1.06, 0.92))
        .at(0.6, Pose::new().scale_xy(0.96, 1.06))
        .at(0.82, Pose::new().scale_xy(1.015, 0.985))
        .at(1.0, Pose::new())
        .build()
}

const SLIDE: Duration = ms(420);
const LAND: Duration = ms(460);
const APPEAR: Duration = ms(150);

/// Whether the move from `from` to the tab at `to` heads into an end of the bar, where the
/// spring's overshoot would poke out: the web version's `data-wall`.
pub(crate) fn hits_wall(from: Pixels, to: Bounds<Pixels>, start: Pixels, end: Pixels) -> bool {
    let slack = px(4.);
    (to.left() > from && to.right() >= end - slack)
        || (to.left() < from && to.left() <= start + slack)
}

struct TabIndicatorBounds {
    container: Bounds<Pixels>,
    tabs: Vec<Bounds<Pixels>>,
}

impl TabIndicatorBounds {
    fn new(num_tabs: usize) -> Self {
        Self {
            container: Bounds::default(),
            tabs: vec![Bounds::default(); num_tabs],
        }
    }

    fn resize(&mut self, num_tabs: usize) {
        self.tabs.resize(num_tabs, Bounds::default());
    }
}

/// Where the indicator is heading: left and width relative to the first tab, the switch epoch,
/// and whether it hits a wall.
#[derive(Clone, Copy, Default)]
struct IndicatorTarget {
    left: Pixels,
    width: Pixels,
    epoch: u64,
    wall: bool,
}

/// A TabBar element that contains multiple [`Tab`] items.
#[derive(IntoElement)]
pub struct TabBar {
    id: ElementId,
    base: BaseTabs,
    style: StyleRefinement,
    scroll_handle: Option<ScrollHandle>,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    children: Vec<Tab>,
    last_empty_space: AnyElement,
    selected_index: Option<usize>,
    variant: TabVariant,
    size: Size,
    menu: bool,
    max_width: Option<Pixels>,
    on_click: Option<Rc<dyn Fn(&usize, &mut Window, &mut App) + 'static>>,
}

impl TabBar {
    /// Create a new TabBar.
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            id: id.clone(),
            base: BaseTabs::new(id).px(px(-1.)),
            style: StyleRefinement::default(),
            children: Vec::new(),
            scroll_handle: None,
            prefix: None,
            suffix: None,
            variant: TabVariant::default(),
            size: Size::default(),
            last_empty_space: div().w_3().into_any_element(),
            selected_index: None,
            on_click: None,
            menu: false,
            max_width: None,
        }
    }

    /// Set the Tab variant, all children will inherit the variant.
    pub fn with_variant(mut self, variant: TabVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the Tab variant to Pill, all children will inherit the variant.
    pub fn pill(mut self) -> Self {
        self.variant = TabVariant::Pill;
        self
    }

    /// Set the Tab variant to Outline, all children will inherit the variant.
    pub fn outline(mut self) -> Self {
        self.variant = TabVariant::Outline;
        self
    }

    /// Set the Tab variant to Segmented, all children will inherit the variant.
    pub fn segmented(mut self) -> Self {
        self.variant = TabVariant::Segmented;
        self
    }

    /// Set the Tab variant to Underline, all children will inherit the variant.
    pub fn underline(mut self) -> Self {
        self.variant = TabVariant::Underline;
        self
    }

    /// Set whether to show the menu button when tabs overflow, default is false.
    pub fn menu(mut self, menu: bool) -> Self {
        self.menu = menu;
        self
    }

    /// Set the maximum width of each tab. Labels longer than this width are
    /// truncated with an ellipsis. Does not apply to icon-only tabs. The
    /// overflow menu still shows the full label.
    pub fn max_width(mut self, width: impl Into<Pixels>) -> Self {
        self.max_width = Some(width.into());
        self
    }

    /// Track the scroll of the TabBar.
    ///
    /// This does not automatically reveal the selected tab. Use the tracked
    /// [`ScrollHandle`] to request an explicit reveal when needed.
    pub fn track_scroll(mut self, scroll_handle: &ScrollHandle) -> Self {
        self.scroll_handle = Some(scroll_handle.clone());
        self
    }

    /// Set the prefix element of the TabBar
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    /// Set the suffix element of the TabBar
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Add children of the TabBar, all children will inherit the variant.
    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Tab>>) -> Self {
        self.children.extend(children.into_iter().map(Into::into));
        self
    }

    /// Add child of the TabBar, tab will inherit the variant.
    pub fn child(mut self, child: impl Into<Tab>) -> Self {
        self.children.push(child.into());
        self
    }

    /// Set the selected index of the TabBar.
    pub fn selected_index(mut self, index: usize) -> Self {
        self.selected_index = Some(index);
        self
    }

    /// Set the last empty space element of the TabBar.
    pub fn last_empty_space(mut self, last_empty_space: impl IntoElement) -> Self {
        self.last_empty_space = last_empty_space.into_any_element();
        self
    }

    /// Set the on_click callback of the TabBar, the first parameter is the index of the clicked tab.
    ///
    /// When this is set, the children's on_click will be ignored.
    pub fn on_click<F>(mut self, on_click: F) -> Self
    where
        F: Fn(&usize, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    /// Render the sliding indicator, with the switch epoch the tabs key their colour fades on.
    fn render_indicator(
        &self,
        bounds_rc: &Option<Rc<RefCell<TabIndicatorBounds>>>,
        padding_x: Pixels,
        window: &mut Window,
        cx: &mut App,
    ) -> Option<(AnyElement, u64)> {
        let has_indicator = matches!(
            self.variant,
            TabVariant::Segmented | TabVariant::Pill | TabVariant::Underline
        );
        let num_tabs = self.children.len();
        let selected_ix = self.selected_index.unwrap_or(usize::MAX);

        if !(has_indicator && num_tabs > 0 && selected_ix < num_tabs) {
            return None;
        }

        let id = self.id.clone();
        let prev_selected =
            window.use_keyed_state((id.clone(), "kk-tab-prev"), cx, |_, _| selected_ix);
        let target_state = window.use_keyed_state((id.clone(), "kk-tab-target"), cx, |_, _| {
            IndicatorTarget::default()
        });

        self.update_target(
            selected_ix,
            bounds_rc,
            padding_x,
            &prev_selected,
            &target_state,
            cx,
        );

        let target = *target_state.read(cx);
        let ready = target.width > px(0.);
        // The first placement fades in where it lands, as the web pill does.
        let appeared = since_change((id.clone(), "kk-tab-ready"), ready, window, cx);
        if !ready {
            return None;
        }
        let opacity = match running(appeared, APPEAR, window) {
            Some(elapsed) => Easing::EaseOut.sample(progress(elapsed, APPEAR)),
            None => 1.0,
        };

        let curves = cx.curves();
        let easing = if target.wall {
            curves.out
        } else {
            curves.spring
        };
        let left = glide(
            (id.clone(), "kk-tab-left"),
            target.left,
            SLIDE,
            easing.clone(),
            window,
            cx,
        );
        let width = glide(
            (id.clone(), "kk-tab-width"),
            target.width,
            SLIDE,
            easing,
            window,
            cx,
        );
        let landed = since_change((id.clone(), "kk-tab-land"), target.epoch, window, cx);
        let squash = match running(landed.filter(|_| !cx.reduce_motion()), LAND, window) {
            Some(elapsed) => land_track().sample(progress(elapsed, LAND)),
            None => Pose::new(),
        };

        let variant = self.variant;
        let size = self.size;
        let inner_height = variant.inner_height(size);
        let inner_radius = variant.inner_radius(size, cx);
        let theme = cx.theme();

        let indicator = div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(left)
            .w(width)
            .opacity(opacity)
            .map(|el| match variant {
                TabVariant::Segmented => el.flex().items_center().justify_center().child(
                    div()
                        .flex_none()
                        .w(relative(squash.sx))
                        .h(inner_height * squash.sy)
                        .bg(theme.tokens.background)
                        .rounded(inner_radius)
                        .shadow(raised_shadow()),
                ),
                TabVariant::Pill => el.flex().items_center().justify_center().child(
                    div()
                        .flex_none()
                        .w(relative(squash.sx))
                        .h(relative(squash.sy))
                        .bg(theme.tokens.primary)
                        .rounded(theme.radius_full()),
                ),
                TabVariant::Underline => el.child(
                    div()
                        .absolute()
                        .left(relative((1. - squash.sx) * 0.5))
                        .w(relative(squash.sx))
                        .bottom_0()
                        .h(px(2.) * squash.sy)
                        .bg(theme.tokens.primary),
                ),
                _ => el,
            });

        Some((indicator.into_any_element(), target.epoch))
    }

    /// Update the indicator's target from the selection and the tabs' bounds.
    fn update_target(
        &self,
        selected_ix: usize,
        bounds_rc: &Option<Rc<RefCell<TabIndicatorBounds>>>,
        padding_x: Pixels,
        prev_selected: &Entity<usize>,
        target_state: &Entity<IndicatorTarget>,
        cx: &mut App,
    ) {
        let Some(rc) = bounds_rc else {
            return;
        };

        let prev_ix = *prev_selected.read(cx);
        let bounds = rc.borrow();
        let container = bounds.container;

        if container.size.width == px(0.) {
            if prev_ix != selected_ix {
                prev_selected.update(cx, |v, _| *v = selected_ix);
            }
            return;
        }

        // The indicator is nested in the first tab wrapper, so its position is
        // relative to that wrapper rather than the scroll container.
        let first_tab_origin = bounds
            .tabs
            .first()
            .map(|tab| tab.origin.x)
            .unwrap_or(container.origin.x);

        let current = *target_state.read(cx);
        if prev_ix != selected_ix {
            if let Some(to_b) = bounds.tabs.get(selected_ix) {
                let left = to_b.origin.x - first_tab_origin;
                let width = to_b.size.width;
                // Only a switch away from a tab that still exists restarts the
                // tabs' own epoch-keyed transitions and the landing squash.
                let moved = bounds.tabs.get(prev_ix).is_some() && current.width > px(0.);
                let wall = moved
                    && hits_wall(
                        current.left + first_tab_origin,
                        *to_b,
                        container.left() + padding_x,
                        container.right() - padding_x,
                    );
                let epoch = if moved {
                    current.epoch + 1
                } else {
                    current.epoch
                };
                target_state.update(cx, |v, _| {
                    *v = IndicatorTarget {
                        left,
                        width,
                        epoch,
                        wall,
                    }
                });
            }
            drop(bounds);
            prev_selected.update(cx, |v, _| *v = selected_ix);
            return;
        }

        if let Some(to_b) = bounds.tabs.get(selected_ix) {
            let left = to_b.origin.x - first_tab_origin;
            let width = to_b.size.width;
            if left != current.left || width != current.width {
                target_state.update(cx, |v, _| {
                    v.left = left;
                    v.width = width;
                });
            }
        }
    }
}

impl Styled for TabBar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Sizable for TabBar {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for TabBar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let default_gap = match self.size {
            Size::Small | Size::XSmall => px(8.),
            Size::Large => px(16.),
            _ => px(12.),
        };
        let (bg, paddings, gap): (Background, _, _) = match self.variant {
            TabVariant::Tab => {
                let padding = Edges::all(px(0.));
                (cx.theme().tokens.tab_bar.into(), padding, px(0.))
            }
            TabVariant::Outline => {
                let padding = Edges::all(px(0.));
                (cx.theme().transparent.into(), padding, default_gap)
            }
            TabVariant::Pill => {
                let padding = Edges::all(px(0.));
                (cx.theme().transparent.into(), padding, px(4.))
            }
            TabVariant::Segmented => {
                let padding_x = match self.size {
                    Size::XSmall => px(2.),
                    Size::Small => px(3.),
                    _ => px(4.),
                };
                let padding = Edges {
                    left: padding_x,
                    right: padding_x,
                    ..Default::default()
                };

                (cx.theme().tokens.tab_bar_segmented.into(), padding, px(2.))
            }
            TabVariant::Underline => {
                // This gap is same as the tab inner_paddings
                let gap = match self.size {
                    Size::XSmall => px(10.),
                    Size::Small => px(12.),
                    Size::Large => px(20.),
                    _ => px(16.),
                };

                (cx.theme().transparent.into(), Edges::all(px(0.)), gap)
            }
        };

        let has_indicator = matches!(
            self.variant,
            TabVariant::Segmented | TabVariant::Pill | TabVariant::Underline
        );
        let num_tabs = self.children.len();

        // Bounds tracking for tab indicator animation.
        // Uses Rc<RefCell> to avoid triggering re-renders from prepaint writes.
        let bounds_rc = if has_indicator && num_tabs > 0 {
            let rc: Rc<RefCell<TabIndicatorBounds>> = window
                .use_keyed_state((self.id.clone(), "kk-tab-bounds"), cx, |_, _| {
                    Rc::new(RefCell::new(TabIndicatorBounds::new(num_tabs)))
                })
                .read(cx)
                .clone();
            rc.borrow_mut().resize(num_tabs);
            Some(rc)
        } else {
            None
        };

        let padding_x = paddings.left;
        let indicator = self.render_indicator(&bounds_rc, padding_x, window, cx);
        let indicator_epoch = indicator.as_ref().map(|(_, epoch)| *epoch).unwrap_or(0);
        let mut indicator_element = indicator.map(|(el, _)| el);
        let indicator_ready = indicator_element.is_some();

        let has_suffix_or_menu = self.suffix.is_some() || self.menu;
        let mut item_metas: Vec<(Option<SharedString>, Option<Icon>, bool)> = Vec::new();
        let selected_index = self.selected_index;
        let on_click = self.on_click.clone();
        let tabs = self.base;
        let mut rendered_tabs = Vec::with_capacity(self.children.len());
        let max_width = self.max_width;

        for (ix, child) in self.children.into_iter().enumerate() {
            item_metas.push((child.label.clone(), child.icon.clone(), child.disabled));
            let tab_bar_prefix = child.tab_bar_prefix.unwrap_or(true);
            let mut tab = child
                .ix(ix)
                .tab_bar_prefix(tab_bar_prefix)
                .max_width(max_width)
                .with_variant(self.variant)
                .with_size(self.size);
            tab.indicator_active = has_indicator;
            tab.indicator_ready = indicator_ready;
            tab.indicator_epoch = indicator_epoch;
            let mut tab = tab
                .when_some(selected_index, |tab, selected_index| {
                    tab.selected(selected_index == ix)
                })
                .when_some(self.on_click.clone(), move |tab, on_click| {
                    tab.on_click(move |_, window, cx| on_click(&ix, window, cx))
                });
            // The wrapper below is the flex item the bar lays out, so a tab's
            // own `flex_grow` / `flex_basis` (e.g. `flex_1()`) must size it.
            let flex_grow = tab.style().flex_grow;
            let flex_basis = tab.style().flex_basis;

            rendered_tabs.push(if let Some(ref rc) = bounds_rc {
                let rc = rc.clone();
                // `tabs-inner` is tracked by `ScrollHandle`, which indexes its
                // direct children. Keep the indicator inside the first tab so
                // only logical tabs occupy those indices.
                div()
                    .flex_shrink_0()
                    .map(|mut this| {
                        this.style().flex_grow = flex_grow;
                        this.style().flex_basis = flex_basis;
                        this
                    })
                    .on_prepaint(move |bounds, _, _| {
                        if let Some(slot) = rc.borrow_mut().tabs.get_mut(ix) {
                            *slot = bounds;
                        }
                    })
                    .relative()
                    .when(ix == 0, |this| {
                        this.when_some(indicator_element.take(), |this, indicator| {
                            this.child(indicator)
                        })
                    })
                    .child(tab)
                    .into_any_element()
            } else {
                tab.into_any_element()
            });
        }

        tabs.group("tab-bar")
            .relative()
            .flex()
            .items_center()
            .bg(bg)
            .text_color(cx.theme().tab_foreground)
            .when(
                self.variant == TabVariant::Underline || self.variant == TabVariant::Tab,
                |this| {
                    this.child(
                        div()
                            .id("border-b")
                            .absolute()
                            .left_0()
                            .bottom_0()
                            .size_full()
                            .border_b_1()
                            .border_color(cx.theme().border),
                    )
                },
            )
            .rounded(self.variant.tab_bar_radius(self.size, cx))
            .paddings(paddings)
            .refine_style(&self.style)
            .when_some(self.prefix, |this, prefix| this.child(prefix))
            .child(
                h_flex()
                    .id("tabs")
                    .flex_1()
                    .min_w_0()
                    .mx(-padding_x)
                    .px(padding_x)
                    .overflow_x_hidden()
                    // `on_prepaint` adds a canvas child. Keep that helper on
                    // the non-scrolling wrapper so it cannot shift tab indices.
                    .when_some(bounds_rc.clone(), |this, rc| {
                        this.on_prepaint(move |bounds, _, _| {
                            rc.borrow_mut().container = bounds;
                        })
                    })
                    .child(
                        h_flex()
                            .id("tabs-inner")
                            // Fill the bar so tabs can grow into the free space;
                            // as a scroll container it still shrinks below its content.
                            .flex_1()
                            // Keep the scroll viewport inside the wrapper padding so
                            // explicit reveals leave space at both ends of the bar.
                            .relative()
                            .gap(gap)
                            .overflow_x_scroll()
                            .lock_scroll_axis()
                            .when_some(self.scroll_handle, |this, scroll_handle| {
                                this.track_scroll(&scroll_handle)
                            })
                            .children(rendered_tabs)
                            .when(has_suffix_or_menu, |this| this.child(self.last_empty_space)),
                    ),
            )
            .when(self.menu, |this| {
                this.child(
                    Button::new("more")
                        .xsmall()
                        .ghost()
                        .dropdown_caret(true)
                        .dropdown_menu(move |mut this, _, _| {
                            this = this.scrollable(true);
                            for (ix, (label, icon, disabled)) in item_metas.iter().enumerate() {
                                let base = if let Some(label) = label.clone() {
                                    PopupMenuItem::new(label)
                                } else if let Some(icon) = icon.clone() {
                                    PopupMenuItem::element(move |_, _| icon.clone())
                                } else {
                                    PopupMenuItem::new("Unnamed")
                                };
                                this = this.item(
                                    base.checked(selected_index == Some(ix))
                                        .disabled(*disabled)
                                        .when_some(on_click.clone(), |this, on_click| {
                                            this.on_click(move |_, window, cx| {
                                                on_click(&ix, window, cx)
                                            })
                                        }),
                                );
                            }

                            this
                        })
                        .anchor(Anchor::TopRight),
                )
            })
            .when_some(self.suffix, |this, suffix| this.child(suffix))
    }
}

struct Shown {
    key: usize,
    at: Instant,
}

/// The content of the selected tab, rising in when it mounts and when the tab changes. The web
/// version's `TabsContent`; GPUI Component leaves panels to the app, so this is an extra.
#[derive(IntoElement)]
pub struct TabPanel {
    id: ElementId,
    selected: usize,
    base: gpui_kit::Div,
}

impl TabPanel {
    /// A panel for the tab at `selected`. It rises in again whenever `selected` changes.
    pub fn new(id: impl Into<ElementId>, selected: usize) -> Self {
        Self {
            id: id.into(),
            selected,
            base: div(),
        }
    }
}

impl Styled for TabPanel {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for TabPanel {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

/// `kk-pop-tabs-content`: up from `rise` pixels below, fading in.
pub(crate) fn content_track(rise: f32) -> Keyframes<Pose> {
    Track::new(Easing::Linear)
        .at(0.0, Pose::new().y(rise).opacity(0.0))
        .at(1.0, Pose::new())
        .build()
}

impl RenderOnce for TabPanel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let now = now();
        let state = window.use_keyed_state((self.id.clone(), "kk-panel"), cx, |_, _| Shown {
            key: self.selected,
            at: now,
        });
        if state.read(cx).key != self.selected {
            state.update(cx, |shown, _| {
                shown.key = self.selected;
                shown.at = now;
            });
        }
        let elapsed = now.saturating_duration_since(state.read(cx).at);
        let pose = if cx.reduce_motion() {
            // `kk-pop-tabs-fade 0.2s ease-out`.
            match running(Some(elapsed), ms(200), window) {
                Some(elapsed) => {
                    Pose::new().opacity(Easing::EaseOut.sample(progress(elapsed, ms(200))))
                }
                None => Pose::new(),
            }
        } else {
            // `kk-pop-tabs-content 0.25s` on the `out` curve.
            let duration = StdDuration::from_millis(250);
            match running(Some(elapsed), duration, window) {
                Some(elapsed) => content_track(window.rem_size().as_f32() * 0.5)
                    .sample(cx.curves().out.sample(progress(elapsed, duration))),
                None => Pose::new(),
            }
        };
        let mut base = self.base;
        let outer = crate::motion::split_layout(base.style());
        transform((self.id, "kk-panel-motion"), pose, base.flex_1()).outer_style(outer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn land_stretches_squashes_and_settles() {
        let land = land_track();
        let stretch = land.sample(0.3);
        assert_eq!((stretch.sx, stretch.sy), (1.06, 0.92));
        let squash = land.sample(0.6);
        assert_eq!((squash.sx, squash.sy), (0.96, 1.06));
        let rebound = land.sample(0.82);
        assert_eq!((rebound.sx, rebound.sy), (1.015, 0.985));
        assert_eq!(land.sample(1.0), Pose::new());
    }

    #[test]
    fn walls_are_the_ends_of_the_bar() {
        let tab = |x: f32| {
            Bounds::new(
                gpui_kit::point(px(x), px(0.)),
                gpui_kit::size(px(40.), px(20.)),
            )
        };
        // Moving right onto the last tab, which ends at the bar's end.
        assert!(hits_wall(px(0.), tab(60.), px(0.), px(100.)));
        // Moving right onto a middle tab.
        assert!(!hits_wall(px(0.), tab(30.), px(0.), px(100.)));
        // Moving left onto the first tab.
        assert!(hits_wall(px(60.), tab(0.), px(0.), px(100.)));
    }

    #[test]
    fn panel_rises_from_half_a_rem() {
        let track = content_track(8.0);
        assert_eq!(track.sample(0.0), Pose::new().y(8.0).opacity(0.0));
        assert_eq!(track.sample(1.0), Pose::new());
    }

    #[test]
    fn keyframes_match_the_web() {
        // In rems: the parser reads `0.5rem` as 0.5.
        crate::parity::assert_pose_track(
            "pop-tabs",
            "kk-pop-tabs-content",
            &content_track(0.5),
            &[],
        );
        // `kk-pop-tabs-land-a`/`-b` key `scale` on `var(--kk-pop-tabs-stretch)` and friends, which
        // the parity parser can't read; `land_stretches_squashes_and_settles` checks the values of
        // those custom properties for a horizontal list.
    }
}
