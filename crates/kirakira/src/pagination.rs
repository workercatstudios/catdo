//! Pagination: page links that squash under the finger and a current page that pops.
//!
//! Replaces `gpui_kit::component::pagination`, with the same builder:
//! `Pagination::new("pages").current_page(4).total_pages(12).on_click(...)`. Rebuilt on gpui-base's
//! `Pagination` like GPUI Component's, with the same buttons, sizes, ellipsis menus and tooltips,
//! plus the motion of Kirakira's Pop Pagination:
//!
//! - A page link squashes to `scale: 1.08 0.9` while held (0.08 s, ease-out) and springs back when
//!   let go (0.35 s, spring curve).
//! - The page that becomes current pops: `0.85 → 1.12 → 0.96 → 1` in 0.36 s, ease-in-out. It plays
//!   on every change of page, and when the pagination first appears, as on the web.
//! - Previous and Next push their chevron 0.2 em outwards on hover (0.3 s, spring curve).
//!
//! Under reduced motion the current page still changes its outline at once; nothing squashes,
//! pops or nudges.
//!
//! How it's drawn: GPUI can't scale a button, so while a page link moves it turns transparent
//! (keeping its layout, hover and clicks) and a [vector](crate::vector) plate of its outline, fill
//! and number is drawn over it in the colours GPUI Component's button has in that state, so the
//! number squashes with the box exactly as on the web. At rest it is GPUI Component's button.
//! Previous and Next carry a label and a chevron: their box squashes exactly and their contents
//! scale evenly. Labels are English, where GPUI Component translates them.

use std::rc::Rc;

use gpui_kit::base::{
    Pagination as BasePagination, PaginationItem as PageItem, PaginationState, StyledExt as _,
    h_flex,
};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::{
    ActiveTheme as _, Colorize as _, Disableable, Icon, IconName, Sizable, Size,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, ElementId, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, RenderOnce, SharedString, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div, px, rems,
};

use crate::button::Pressed;
use crate::motion::{Easing, Keyframes, Pulse, Timing, Track, ms};
use crate::squash::{Plate, Squash, plated, squashed};
use crate::state_motion::glide;
use crate::theme::ActiveKira as _;

/// The current page's pop.
fn pop_track() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.85)
        .at(0.45, 1.12)
        .at(0.75, 0.96)
        .at(1.0, 1.0)
        .build()
}

const POP: std::time::Duration = ms(360);
const SQUASH: Squash = Squash::new(1.08, 0.9);

/// Pagination with page navigation, next and previous links.
#[derive(IntoElement)]
pub struct Pagination {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    current_page: usize,
    total_pages: usize,
    disabled: bool,
    compact: bool,
    visible_pages: usize,
    on_click: Option<Rc<dyn Fn(&usize, &mut Window, &mut App)>>,
}

impl Pagination {
    /// Create a new Pagination component with the given ID.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            size: Size::default(),
            current_page: 1,
            total_pages: 1,
            visible_pages: 5,
            disabled: false,
            compact: false,
            on_click: None,
        }
    }

    /// Set the current page number (1-based).
    ///
    /// The value will be clamped between 1 and total_pages when total_pages is set.
    pub fn current_page(mut self, page: usize) -> Self {
        self.current_page = page.max(1);
        self
    }

    /// Set the total number of pages.
    pub fn total_pages(mut self, pages: usize) -> Self {
        self.total_pages = pages.max(1);
        if self.current_page > self.total_pages {
            self.current_page = self.total_pages;
        }
        self
    }

    /// Set the handler for page change (when clicking on page numbers, prev, or next).
    ///
    /// This handler receives the new page number to navigate to.
    pub fn on_click(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Set to display as compact style.
    ///
    /// If true, only the prev, next buttons with only icon.
    pub fn compact(mut self) -> Self {
        self.compact = true;
        self
    }

    /// Set viewable maximum number of page buttons, default 5.
    pub fn visible_pages(mut self, max: usize) -> Self {
        self.visible_pages = max;
        self
    }
}

impl Disableable for Pagination {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Sizable for Pagination {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Pagination {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// Tracks the held state of `button`: sets `pressed` on mouse down, clears it on release.
fn track_press(button: Button, pressed: &gpui_kit::Entity<Pressed>) -> Button {
    let (down, up, up_out) = (pressed.clone(), pressed.clone(), pressed.clone());
    button
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            down.update(cx, |pressed, cx| {
                pressed.0 = true;
                cx.notify();
            })
        })
        .on_mouse_up(MouseButton::Left, move |_, _, cx| {
            up.update(cx, |pressed, cx| {
                pressed.0 = false;
                cx.notify();
            })
        })
        .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
            up_out.update(cx, |pressed, cx| {
                pressed.0 = false;
                cx.notify();
            })
        })
}

struct Hovered(bool);

/// Where a page link was last painted.
struct Spot(Option<gpui_kit::Bounds<gpui_kit::Pixels>>);

struct ShownPage(Option<usize>);

impl Pagination {
    /// Previous or Next: squashes while held. The box squashes exactly; its label and chevron
    /// scale evenly.
    fn squash(
        &self,
        id: ElementId,
        button: Button,
        interactive: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let pressed = window.use_keyed_state((id.clone(), "kk-pressed"), cx, |_, _| Pressed(false));
        let held = interactive && pressed.read(cx).0;
        let (sx, sy) = SQUASH.scale(&id, held, window, cx);
        let button = if interactive {
            track_press(button, &pressed)
        } else {
            button
        };
        squashed(id, (sx, sy), button)
            .text((sx * sy).sqrt())
            .into_any_element()
    }

    /// A page link: squashes while held, and pops by `pop` when it becomes current. While it
    /// moves it's drawn as a vector plate in the colours GPUI Component's button has in its
    /// state, so the number squashes with the outline.
    #[allow(clippy::too_many_arguments)]
    fn page_link(
        &self,
        id: ElementId,
        label: SharedString,
        button: Button,
        selected: bool,
        pop: f32,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        if self.disabled {
            return button.into_any_element();
        }
        let pressed = window.use_keyed_state((id.clone(), "kk-pressed"), cx, |_, _| Pressed(false));
        // Hit-tested each frame against where the link was last painted, like GPUI's own hover: a
        // flag set by mouse moves would stick when a click moves the links under a still pointer.
        let spot = window.use_keyed_state((id.clone(), "kk-spot"), cx, |_, _| Spot(None));
        let held = pressed.read(cx).0;
        let (sx, sy) = SQUASH.scale(&id, held, window, cx);
        let theme = cx.theme();
        let hover = spot
            .read(cx)
            .0
            .is_some_and(|bounds| bounds.contains(&window.mouse_position()));
        // GPUI Component's outline (current) and ghost (other) buttons, per state.
        let (fill, border, color) = if selected {
            let fill = if held {
                theme.input.mix_oklab(theme.transparent, 0.7)
            } else if hover {
                theme.input.mix_oklab(theme.transparent, 0.5)
            } else {
                theme.input_background()
            };
            (Some(fill), Some(theme.input), theme.button_foreground)
        } else if held {
            (Some(theme.button_active), None, theme.secondary_foreground)
        } else if hover {
            let accent = if theme.is_dark() {
                theme.accent.opacity(0.5)
            } else {
                theme.accent
            };
            (Some(accent), None, theme.accent_foreground)
        } else {
            (None, None, theme.secondary_foreground)
        };
        let text = gpui_kit::TextStyleRefinement {
            font_size: Some(match self.size {
                Size::XSmall => rems(0.75).into(),
                Size::Small => rems(0.875).into(),
                _ => rems(1.0).into(),
            }),
            ..Default::default()
        };
        let plate = Plate {
            fill,
            border,
            radius: theme.radius,
            label,
            color,
            text,
        };
        let pose = crate::motion::Pose::new().scale_xy(sx * pop, sy * pop);
        let record = spot.clone();
        div()
            .id((id.clone(), "kk-hover"))
            .relative()
            .flex_none()
            .on_hover(move |_, _, cx| spot.update(cx, |_, cx| cx.notify()))
            .child(plated(id, pose, plate, track_press(button, &pressed)))
            .child(
                gpui_kit::canvas(
                    move |bounds, window, cx| {
                        record.update(cx, |spot, _| spot.0 = Some(bounds));
                        // The links moved under the pointer: draw again with the right hover.
                        if bounds.contains(&window.mouse_position()) != hover {
                            window.refresh();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .into_any_element()
    }

    fn render_nav_button(
        &self,
        state: &PaginationState,
        is_prev: bool,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyElement {
        let (name, label, icon) = if is_prev {
            ("prev", "Previous", IconName::ChevronLeft)
        } else {
            ("next", "Next", IconName::ChevronRight)
        };
        let id: ElementId = (self.id.clone(), name).into();
        let target_page = if is_prev {
            state.previous_page()
        } else {
            state.next_page()
        };
        let enabled = target_page.is_some();

        let hovered = window.use_keyed_state((id.clone(), "kk-hovered"), cx, |_, _| Hovered(false));
        let is_hovered = enabled && hovered.read(cx).0;
        let em = rems(0.875).to_pixels(window.rem_size());
        let nudge = glide(
            (id.clone(), "kk-nudge"),
            if is_hovered && !cx.reduce_motion() {
                0.2_f32
            } else {
                0.0
            },
            ms(300),
            cx.curves().spring,
            window,
            cx,
        );
        let chevron = div()
            .relative()
            .left(em * if is_prev { -nudge } else { nudge })
            .flex()
            .child(Icon::new(icon));

        let button = Button::new(name)
            .ghost()
            .compact()
            .with_size(self.size)
            .disabled(!enabled)
            .tooltip(label)
            .accessibility_label(label)
            .child(
                h_flex()
                    .w_full()
                    .gap_2()
                    .flex_nowrap()
                    .when(is_prev, |this| this.flex_row_reverse())
                    .when(!self.compact, |this| this.child(SharedString::from(label)))
                    .child(chevron),
            )
            .when_some(
                target_page.filter(|_| state.has_on_change()),
                |this, target_page| {
                    let state = state.clone();
                    this.on_click(move |_, window, cx| {
                        state.request_page(target_page, window, cx);
                    })
                },
            );
        // The button's own hover drives its tooltip, so watch the pointer from a wrapper.
        div()
            .id((id.clone(), "kk-hover"))
            .flex_none()
            .on_hover(move |hovering, _, cx| {
                hovered.update(cx, |hovered, cx| {
                    hovered.0 = *hovering;
                    cx.notify();
                })
            })
            .child(self.squash(id, button, enabled, window, cx))
            .into_any_element()
    }
}

impl RenderOnce for Pagination {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let mut state = PaginationState::new(self.current_page, self.total_pages)
            .visible_pages(self.visible_pages)
            .disabled(self.disabled);
        if let Some(on_click) = self.on_click.clone() {
            state = state.on_change(move |page, window, cx| on_click(&page, window, cx));
        }
        let page_numbers = if self.compact {
            Vec::new()
        } else {
            state.items()
        };
        let current_page = state.current_page();
        let is_disabled = self.disabled;

        // The page that becomes current pops; so does the first one shown.
        let pulse = Pulse::new((self.id.clone(), "kk-pop"), window, cx);
        let shown =
            window.use_keyed_state((self.id.clone(), "kk-shown"), cx, |_, _| ShownPage(None));
        if shown.read(cx).0 != Some(current_page) {
            shown.update(cx, |shown, _| shown.0 = Some(current_page));
            pulse.fire(cx);
        }
        let pulse = Pulse::new((self.id.clone(), "kk-pop"), window, cx);
        let pop = match pulse.running(POP) {
            Some(elapsed) => {
                pulse.animate(POP, window);
                pop_track().sample(Timing::new(POP).sample(elapsed).directed_progress)
            }
            None => 1.0,
        };

        let prev = self.render_nav_button(&state, true, window, cx);
        let next = self.render_nav_button(&state, false, window, cx);
        let mut items = Vec::new();
        for item in page_numbers {
            match item {
                PageItem::Page(page) => {
                    let is_selected = page == current_page;
                    let id: ElementId =
                        (self.id.clone(), SharedString::from(page.to_string())).into();
                    let button = Button::new(page)
                        .with_size(self.size)
                        .map(|this| {
                            if is_selected {
                                this.outline()
                            } else {
                                this.ghost()
                            }
                        })
                        .label(page.to_string())
                        .compact()
                        .disabled(is_disabled)
                        .when(!is_selected && state.has_on_change(), |this| {
                            let state = state.clone();
                            this.on_click(move |_, window, cx| {
                                state.request_page(page, window, cx);
                            })
                        });
                    items.push(self.page_link(
                        id,
                        page.to_string().into(),
                        button,
                        is_selected,
                        if is_selected { pop } else { 1.0 },
                        window,
                        cx,
                    ));
                }
                PageItem::Ellipsis(range) => items.push(
                    Button::new(SharedString::from(format!(
                        "ellipsis-{}-{}",
                        range.start, range.end
                    )))
                    .ghost()
                    .with_size(self.size)
                    .compact()
                    .disabled(self.disabled)
                    .icon(IconName::Ellipsis)
                    .dropdown_menu({
                        let state = state.clone();
                        move |mut menu, _, _| {
                            for page in range.clone() {
                                menu = menu.item(
                                    PopupMenuItem::new(format!("{}", page))
                                        .checked(page == current_page)
                                        .on_click({
                                            let state = state.clone();
                                            move |_, window, cx| {
                                                state.request_page(page, window, cx);
                                            }
                                        }),
                                )
                            }
                            menu.min_w(px(55.)).max_h(px(240.)).scrollable(true)
                        }
                    })
                    .into_any_element(),
                ),
            }
        }

        BasePagination::new(self.id.clone(), state)
            .h_flex()
            .px_2()
            .py_2()
            .gap_1()
            .items_center()
            .refine_style(&self.style)
            .child(prev)
            .children(items)
            .child(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pop_matches_the_web() {
        use crate::parity::assert_number_track;
        assert_number_track(
            "pop-pagination",
            "kk-pop-pagination-pop",
            "sx",
            &pop_track(),
        );
        assert_number_track(
            "pop-pagination",
            "kk-pop-pagination-pop",
            "sy",
            &pop_track(),
        );
    }

    #[test]
    fn the_current_page_pops_past_full_size() {
        let pop = pop_track();
        assert_eq!(pop.sample(0.0), 0.85);
        assert!((pop.sample(0.45) - 1.12).abs() < 1e-6);
        assert!((pop.sample(0.75) - 0.96).abs() < 1e-6);
        assert_eq!(pop.sample(1.0), 1.0);
    }
}
