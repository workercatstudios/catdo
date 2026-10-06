//! Breadcrumb: links that underline themselves and a current page that rises into place.
//!
//! Replaces `gpui_kit::component::breadcrumb`, with the same builder: `Breadcrumb::new()` with
//! [`BreadcrumbItem`] children. Rebuilt on the same structure and sizes as GPUI Component's, with
//! the motion of Kirakira's Pop Breadcrumb:
//!
//! - A clickable item draws an underline in from the left on hover (0.25 s on the `out` curve) and
//!   pulls it off through the right end when the pointer leaves; its text turns to the foreground
//!   colour in 0.15 s.
//! - The separator after a hovered item nudges 0.2 em forward on the spring curve (0.3 s).
//! - The current page (the last item) rises into place when it appears: from 0.5 em below and
//!   transparent, opaque by 30 %, 0.1 em past its place at 60 %, settled at 0.4 s. A new last
//!   item rises again, so walking up or down a path replays it.
//!
//! Under reduced motion the underline and colour change in place; nothing moves.
//!
//! Differences from the web version: the underline is a 0.09 em bar drawn under the label rather
//! than a background gradient, which looks the same. GPUI Component's breadcrumb has no ellipsis
//! item, so neither has this one (the web kit's ellipsis hops its dots on hover). The current
//! page's rise runs on a [`Clock`](crate::motion::Clock), so a
//! [`Timeline`](crate::timeline::Timeline) can seek it; hover motion runs on real time, read from
//! [`motion::now`](crate::motion::now) so screenshots can pin it.

use std::rc::Rc;

use gpui_kit::base::{StyledExt as _, h_flex};
use gpui_kit::component::{ActiveTheme as _, Icon, IconName};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, ClickEvent, ElementId, Entity, Hsla, InteractiveElement as _, IntoElement,
    ParentElement as _, Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, relative, rems,
};

use crate::motion::{Clock, Easing, Keyframes, Pose, Timing, Track, Trigger, ms, transform};
use crate::state_motion::glide;
use crate::theme::ActiveKira as _;

/// The current page's rise, in ems: `translate: 0 0.5em` → `-0.1em` at 60 % → `0`.
fn rise_y() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.5, Easing::EaseOut)
        .at(0.6, -0.1)
        .at(1.0, 0.0)
        .build()
}

/// The current page's fade: transparent to opaque by 30 %.
fn rise_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.0, Easing::EaseOut)
        .at(0.3, 1.0)
        .at(1.0, 1.0)
        .build()
}

const RISE: std::time::Duration = ms(400);

/// A breadcrumb navigation element.
#[derive(IntoElement)]
pub struct Breadcrumb {
    id: ElementId,
    style: StyleRefinement,
    items: Vec<BreadcrumbItem>,
}

/// Item for the [`Breadcrumb`].
#[derive(IntoElement)]
pub struct BreadcrumbItem {
    /// Keys a standalone item's hover state. Inside a [`Breadcrumb`] the breadcrumb keys it.
    id: ElementId,
    style: StyleRefinement,
    label: SharedString,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    disabled: bool,
}

impl BreadcrumbItem {
    /// Create a new BreadcrumbItem with the given label.
    ///
    /// On its own, an item keys its hover state by where it's created and its label; give items
    /// made at the same call site with the same label their own [`BreadcrumbItem::id`].
    #[track_caller]
    pub fn new(label: impl Into<SharedString>) -> Self {
        let label: SharedString = label.into();
        Self {
            id: (
                ElementId::CodeLocation(*std::panic::Location::caller()),
                label.clone(),
            )
                .into(),
            style: StyleRefinement::default(),
            label,
            on_click: None,
            disabled: false,
        }
    }

    /// Key a standalone item's hover state by `id` instead of its call site and label. Not in
    /// GPUI Component.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    fn is_link(&self) -> bool {
        self.on_click.is_some() && !self.disabled
    }
}

impl Styled for BreadcrumbItem {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl From<&'static str> for BreadcrumbItem {
    #[track_caller]
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

impl From<String> for BreadcrumbItem {
    #[track_caller]
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<SharedString> for BreadcrumbItem {
    #[track_caller]
    fn from(value: SharedString) -> Self {
        Self::new(value)
    }
}

/// Rendered on its own, an item is a plain crumb; inside a [`Breadcrumb`] it gets the motion.
impl RenderOnce for BreadcrumbItem {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let hovered = window.use_keyed_state((id.clone(), "kk-hovered"), cx, |_, _| None);
        render_item(self, &id, 0, false, &hovered, window, cx)
    }
}

impl Breadcrumb {
    /// Create a new breadcrumb.
    ///
    /// Its hover and entrance state is keyed by where it's created; give breadcrumbs made at the
    /// same call site their own [`Breadcrumb::id`].
    #[track_caller]
    pub fn new() -> Self {
        Self {
            id: ElementId::CodeLocation(*std::panic::Location::caller()),
            items: Vec::new(),
            style: StyleRefinement::default(),
        }
    }

    /// Key the breadcrumb's motion state by `id` instead of its call site.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }

    /// Add an [`BreadcrumbItem`] to the breadcrumb.
    pub fn child(mut self, item: impl Into<BreadcrumbItem>) -> Self {
        self.items.push(item.into());
        self
    }

    /// Add multiple [`BreadcrumbItem`] items to the breadcrumb.
    pub fn children(mut self, items: impl IntoIterator<Item = impl Into<BreadcrumbItem>>) -> Self {
        self.items.extend(items.into_iter().map(Into::into));
        self
    }
}

impl Default for Breadcrumb {
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}

impl Styled for Breadcrumb {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// One `em` of the breadcrumb's `text_sm` labels.
fn em(window: &Window) -> Pixels {
    rems(0.875).to_pixels(window.rem_size())
}

fn render_item(
    item: BreadcrumbItem,
    breadcrumb: &ElementId,
    ix: usize,
    is_last: bool,
    hovered: &Entity<Option<usize>>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let id: ElementId = (breadcrumb.clone(), SharedString::from(format!("item-{ix}"))).into();
    let is_link = item.is_link();
    let is_hovered = is_link && *hovered.read(cx) == Some(ix);
    let theme = cx.theme();
    let (muted, foreground) = (theme.muted_foreground, theme.foreground);
    let rest = if is_last && !item.disabled {
        foreground
    } else {
        muted
    };
    let color: Hsla = glide(
        (id.clone(), "kk-color"),
        if is_hovered { foreground } else { rest },
        ms(150),
        Easing::EaseOut,
        window,
        cx,
    );
    // How much of the underline shows. It grows from the left and leaves through the right.
    let underline = glide(
        (id.clone(), "kk-underline"),
        if is_hovered { 1.0_f32 } else { 0.0 },
        ms(250),
        cx.curves().out,
        window,
        cx,
    );
    let em = em(window);
    let hover_state = hovered.clone();

    let crumb = div()
        .id(id)
        .relative()
        .role(if is_link { Role::Link } else { Role::ListItem })
        .child(item.label)
        .text_color(color)
        .refine_style(&item.style)
        .when(underline > 0.001, |this| {
            this.child(
                div()
                    .absolute()
                    .bottom(em * 0.05)
                    .h(em * 0.09)
                    .w(relative(underline.min(1.0)))
                    .map(|this| {
                        if is_hovered {
                            this.left_0()
                        } else {
                            this.right_0()
                        }
                    })
                    .bg(color),
            )
        })
        .when(is_link, |this| {
            this.when_some(item.on_click, |this, on_click| {
                this.cursor_pointer()
                    .on_click(move |event, window, cx| on_click(event, window, cx))
                    .on_hover(move |hovering, _, cx| {
                        hover_state.update(cx, |hovered, cx| {
                            if *hovering {
                                *hovered = Some(ix);
                            } else if *hovered == Some(ix) {
                                *hovered = None;
                            }
                            cx.notify();
                        })
                    })
            })
        });

    if !is_last {
        return crumb.into_any_element();
    }
    // The current page rises in as it appears; a new page gets a new clock.
    let label_key: ElementId = (
        breadcrumb.clone(),
        SharedString::from(format!("kk-rise-{ix}")),
    )
        .into();
    let clock = Clock::new(label_key.clone(), Trigger::Mount, window, cx);
    clock.animate(Some(RISE), window);
    let timing = Timing::new(RISE);
    let pose = Pose::new()
        .y(clock.sample(&rise_y(), &timing) * f32::from(em))
        .opacity(clock.sample(&rise_opacity(), &timing));
    transform(label_key, pose, crumb).into_any_element()
}

impl RenderOnce for Breadcrumb {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let hovered = window.use_keyed_state((id.clone(), "kk-hovered"), cx, |_, _| None);
        let items_count = self.items.len();
        // A new current page replays its rise: key the clock by the page's label too.
        let last_label = self.items.last().map(|item| item.label.clone());
        let page_id: ElementId = match last_label {
            Some(label) => (id.clone(), label).into(),
            None => id.clone(),
        };
        let muted = cx.theme().muted_foreground;
        let em = em(window);
        let spring = cx.curves().spring;
        let reduced = cx.reduce_motion();

        let mut children = vec![];
        for (ix, item) in self.items.into_iter().enumerate() {
            let is_last = ix == items_count - 1;
            let next_to_hover = item.is_link() && *hovered.read(cx) == Some(ix);
            children.push(render_item(
                item,
                if is_last { &page_id } else { &id },
                ix,
                is_last,
                &hovered,
                window,
                cx,
            ));
            if !is_last {
                let nudge = glide(
                    (id.clone(), SharedString::from(format!("kk-separator-{ix}"))),
                    if next_to_hover && !reduced {
                        0.2_f32
                    } else {
                        0.0
                    },
                    ms(300),
                    spring.clone(),
                    window,
                    cx,
                );
                children.push(
                    div()
                        .relative()
                        .left(em * nudge)
                        .flex()
                        .child(
                            Icon::new(IconName::ChevronRight)
                                .text_color(muted)
                                .size_3p5(),
                        )
                        .into_any_element(),
                );
            }
        }

        h_flex()
            .gap_1p5()
            .text_sm()
            .text_color(muted)
            .refine_style(&self.style)
            .children(children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rise_matches_the_web() {
        use crate::parity::assert_number_track;
        // The web translates in em; these tracks are in em too.
        assert_number_track("pop-breadcrumb", "kk-pop-breadcrumb-rise", "y", &rise_y());
        assert_number_track(
            "pop-breadcrumb",
            "kk-pop-breadcrumb-rise",
            "opacity",
            &rise_opacity(),
        );
    }

    #[test]
    fn the_page_rises_past_its_place() {
        let y = rise_y();
        assert_eq!(y.sample(0.0), 0.5);
        assert!((y.sample(0.6) + 0.1).abs() < 1e-6);
        assert_eq!(y.sample(1.0), 0.0);
        let opacity = rise_opacity();
        assert_eq!(opacity.sample(0.0), 0.0);
        assert_eq!(opacity.sample(0.3), 1.0);
    }

    #[test]
    fn only_clickable_enabled_items_are_links() {
        assert!(!BreadcrumbItem::new("Home").is_link());
        assert!(BreadcrumbItem::new("Home").on_click(|_, _, _| {}).is_link());
        assert!(
            !BreadcrumbItem::new("Home")
                .on_click(|_, _, _| {})
                .disabled(true)
                .is_link()
        );
    }
}
