//! Toggle and ToggleGroup: toggles that sink, pop and rock their icon.
//!
//! GPUI Component keeps `Toggle` and `ToggleGroup` in its `button` module, so these are
//! re-exported from [`crate::button`] in their place. Rebuilt on gpui-base with GPUI Component's
//! structure, sizes and colours; `ToggleVariant` and `ToggleVariants` are GPUI Component's own.
//!
//! A toggle sinks to 0.9 while pressed (0.1 s). Turning it on pops it from there, 0.9 → (1.08,
//! 1.1) → 0.97 → 1 in 0.34 s, and its icons rock -14° → 7° → -3.5° → 0 over 0.4 s, each swing half
//! the last. Turning it off only springs back, 0.9 → 1.03 → 1 in 0.24 s. In a group every press
//! pops the item, 0.92 → (1.08, 1.1) → 0.97 → 1 in 0.36 s, as the web group's `multiple` mode
//! does (GPUI Component's group is always multiple). Nothing plays on first render. Under reduced
//! motion nothing sinks, pops or rocks; the colours still change.
//!
//! Differences: GPUI scales the whole toggle evenly, so the (1.08, 1.1) keyframes play at their
//! geometric mean. Only icons added with [`Toggle::icon`] rock; arbitrary children can't rotate.

use std::{cell::Cell, rc::Rc, time::Duration};

use gpui_kit::base::{Toggle as BaseToggle, ToggleGroup as BaseToggleGroup};
use gpui_kit::component::button::{ToggleVariant, ToggleVariants};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{
    ActiveTheme as _, Disableable, Icon, Sizable, Size, StyledExt as _, h_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Axis, Corners, Edges, ElementId, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement, RenderOnce, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, relative,
};

use crate::button::Pressed;
use crate::motion::{Easing, Keyframes, Pose, Pulse, Timing, Track, ms, split_layout, transform};

fn on_track() -> Keyframes<Pose> {
    Track::new(Easing::EaseInOut)
        .at(0.0, Pose::new().scale(0.9))
        .at(0.4, Pose::new().scale_xy(1.08, 1.1))
        .at(0.7, Pose::new().scale(0.97))
        .at(1.0, Pose::new())
        .build()
}

fn off_track() -> Keyframes<Pose> {
    Track::new(Easing::EaseInOut)
        .at(0.0, Pose::new().scale(0.9))
        .at(0.5, Pose::new().scale(1.03))
        .at(1.0, Pose::new())
        .build()
}

fn group_track() -> Keyframes<Pose> {
    Track::new(Easing::EaseInOut)
        .at(0.0, Pose::new().scale(0.92))
        .at(0.4, Pose::new().scale_xy(1.08, 1.1))
        .at(0.7, Pose::new().scale(0.97))
        .at(1.0, Pose::new())
        .build()
}

/// The sink while held: `kk-pop-toggle-press`, to 0.9 in 0.1 s on ease-out.
fn press_track() -> Keyframes<f32> {
    Track::new(Easing::EaseOut)
        .at(0.0, 1.0)
        .at(1.0, 0.9)
        .build()
}

/// The icon's rock, in degrees.
fn rock_track() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.0)
        .at(0.3, -14.0)
        .at(0.55, 7.0)
        .at(0.8, -3.5)
        .at(1.0, 0.0)
        .build()
}

const ON: Duration = ms(340);
const OFF: Duration = ms(240);
const GROUP: Duration = ms(360);
const ROCK: Duration = ms(400);
const SINK: Duration = ms(100);

/// The toggle's scale and its icons' rotation (degrees) `elapsed` after its last change.
pub(crate) fn toggle_frame(on: bool, in_group: bool, elapsed: Option<Duration>) -> (f32, f32) {
    let Some(elapsed) = elapsed else {
        return (1.0, 0.0);
    };
    // GPUI scales the whole toggle evenly: uneven keyframes play at their geometric mean.
    let sample = |track: Keyframes<Pose>, duration: Duration| {
        if elapsed < duration {
            track
                .sample(Timing::new(duration).sample(elapsed).directed_progress)
                .uniform_scale()
        } else {
            1.0
        }
    };
    if in_group {
        return (sample(group_track(), GROUP), 0.0);
    }
    let scale = if on {
        sample(on_track(), ON)
    } else {
        sample(off_track(), OFF)
    };
    let rock = if on && elapsed < ROCK {
        rock_track().sample(elapsed.as_secs_f32() / ROCK.as_secs_f32())
    } else {
        0.0
    };
    (scale, rock)
}

enum ToggleChild {
    Icon(Box<Icon>),
    Element(AnyElement),
}

#[derive(IntoElement)]
pub struct Toggle {
    id: ElementId,
    style: StyleRefinement,
    checked: bool,
    size: Size,
    variant: ToggleVariant,
    disabled: bool,
    border_corners: Corners<bool>,
    border_edges: Edges<bool>,
    children: Vec<ToggleChild>,
    on_click: Option<Rc<dyn Fn(&bool, &mut Window, &mut App) + 'static>>,
    tooltip: Option<SharedString>,
    in_group: bool,
}

impl Toggle {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            checked: false,
            size: Size::default(),
            variant: ToggleVariant::default(),
            disabled: false,
            border_corners: Corners {
                top_left: true,
                top_right: true,
                bottom_left: true,
                bottom_right: true,
            },
            border_edges: Edges::all(true),
            children: Vec::new(),
            on_click: None,
            tooltip: None,
            in_group: false,
        }
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        let label: SharedString = label.into();
        self.children
            .push(ToggleChild::Element(label.into_any_element()));
        self
    }

    /// Add an icon. Icons rock when the toggle turns on.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.children.push(ToggleChild::Icon(Box::new(icon.into())));
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Called with the requested checked state.
    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    fn border_corners(mut self, corners: impl Into<Corners<bool>>) -> Self {
        self.border_corners = corners.into();
        self
    }

    fn border_edges(mut self, edges: impl Into<Edges<bool>>) -> Self {
        self.border_edges = edges.into();
        self
    }
}

impl ToggleVariants for Toggle {
    fn with_variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl ParentElement for Toggle {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children
            .extend(elements.into_iter().map(ToggleChild::Element));
    }
}

impl Disableable for Toggle {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Sizable for Toggle {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Toggle {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

struct Seen(bool);

impl RenderOnce for Toggle {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let checked = self.checked;
        let disabled = self.disabled;
        let id = self.id.clone();
        let in_group = self.in_group;

        // Standalone toggles pop when their value changes (armed by a new value, so toggles that
        // render on don't pop on load); group items pop on every press.
        let seen = window.use_keyed_state((id.clone(), "kk-seen"), cx, |_, _| Seen(checked));
        if seen.read(cx).0 != checked {
            seen.update(cx, |seen, _| seen.0 = checked);
            if !in_group {
                Pulse::new((id.clone(), "kk-pop"), window, cx).fire(cx);
            }
        }
        let pulse = Pulse::new((id.clone(), "kk-pop"), window, cx);
        pulse.animate(ROCK.max(GROUP), window);
        let pressed = window.use_keyed_state((id.clone(), "kk-pressed"), cx, |_, _| Pressed(false));
        let held = !disabled && !in_group && pressed.read(cx).0;
        // The sink runs from the press, like the web's `forwards` press animation; letting go
        // drops it at once. A pulse reports nothing under reduced motion, so nothing sinks.
        let press = Pulse::new((id.clone(), "kk-press"), window, cx);
        let sunk = match press.elapsed() {
            Some(elapsed) if held => {
                press.animate(SINK, window);
                press_track().sample((elapsed.as_secs_f32() / SINK.as_secs_f32()).min(1.0))
            }
            _ => 1.0,
        };
        let (pop, rock) = toggle_frame(checked, in_group, pulse.elapsed());
        let scale = if held { sunk } else { pop };

        let theme = cx.theme();
        let hoverable = !disabled && !checked;
        let rounding = theme.radius;
        let pressed_background = theme.tokens.accent;
        let pressed_foreground = theme.accent_foreground;
        let hover_background = theme.tokens.accent;
        let hover_foreground = theme.accent_foreground;
        let border = theme.border;
        let background = theme.tokens.background;

        let outer = split_layout(&mut self.style);
        let instance_style = self.style.clone();
        let accessibility_label = self.tooltip.clone();
        let tooltip = self.tooltip.clone();
        let children = self.children.into_iter().map(|child| match child {
            ToggleChild::Icon(icon) if rock != 0.0 => (*icon)
                .rotate(gpui_kit::radians(rock.to_radians()))
                .into_any_element(),
            ToggleChild::Icon(icon) => (*icon).into_any_element(),
            ToggleChild::Element(element) => element,
        });
        let on_click = self.on_click;
        let fire = pulse.clone();
        let (down, up, up_out) = (pressed.clone(), pressed.clone(), pressed);
        let press_pulse = press;

        let toggle = BaseToggle::new(id.clone())
            .pressed(checked)
            .disabled(disabled)
            .when_some(accessibility_label, |this, label| {
                this.accessibility_label(label)
            })
            .on_change(move |next, _, window, cx| {
                if in_group {
                    fire.fire(cx);
                }
                if let Some(on_click) = on_click.as_ref() {
                    on_click(&next, window, cx);
                }
            })
            .flex()
            .flex_row()
            .line_height(relative(1.25))
            .items_center()
            .justify_center()
            .map(|this| match self.size {
                Size::XSmall => this.min_w_5().h_5().px_0p5().text_xs(),
                Size::Small => this.min_w_6().h_6().px_1().text_sm(),
                Size::Large => this.min_w_9().h_9().px_3().text_lg(),
                _ => this.min_w_8().h_8().px_2(),
            })
            .when(self.border_corners.top_left, |this| {
                this.rounded_tl(rounding)
            })
            .when(self.border_corners.top_right, |this| {
                this.rounded_tr(rounding)
            })
            .when(self.border_corners.bottom_left, |this| {
                this.rounded_bl(rounding)
            })
            .when(self.border_corners.bottom_right, |this| {
                this.rounded_br(rounding)
            })
            .when(self.variant == ToggleVariant::Outline, |this| {
                this.when(self.border_edges.left, |this| this.border_l_1())
                    .when(self.border_edges.right, |this| this.border_r_1())
                    .when(self.border_edges.top, |this| this.border_t_1())
                    .when(self.border_edges.bottom, |this| this.border_b_1())
                    .border_color(border)
                    .bg(background)
            })
            .when(hoverable, |this| {
                this.hover(|this| this.bg(hover_background).text_color(hover_foreground))
            })
            .styles(|styles| {
                styles.pressed(|style| {
                    style
                        .bg(pressed_background)
                        .text_color(pressed_foreground)
                        .refine_style(&instance_style)
                })
            })
            .refine_style(&instance_style)
            .when(!disabled && !in_group, |this| {
                this.on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    press_pulse.fire(cx);
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
            })
            .children(children)
            .when_some(tooltip, |this, tooltip| {
                this.tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx))
            });

        transform(id, Pose::new().scale(scale), toggle).outer_style(outer)
    }
}

/// A group of toggles; each can be on or off.
#[derive(IntoElement)]
pub struct ToggleGroup {
    id: ElementId,
    style: StyleRefinement,
    size: Size,
    variant: ToggleVariant,
    disabled: bool,
    segmented: bool,
    items: Vec<Toggle>,
    on_click: Option<Rc<dyn Fn(&Vec<bool>, &mut Window, &mut App) + 'static>>,
}

impl ToggleGroup {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            size: Size::default(),
            variant: ToggleVariant::default(),
            disabled: false,
            segmented: false,
            items: Vec::new(),
            on_click: None,
        }
    }

    pub fn child(mut self, toggle: impl Into<Toggle>) -> Self {
        self.items.push(toggle.into());
        self
    }

    pub fn children(mut self, children: impl IntoIterator<Item = impl Into<Toggle>>) -> Self {
        self.items.extend(children.into_iter().map(Into::into));
        self
    }

    /// Called with the requested checked state of every toggle in the group.
    #[allow(clippy::ptr_arg)]
    pub fn on_click(
        mut self,
        on_click: impl Fn(&Vec<bool>, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    /// Join the toggles into one segmented control.
    pub fn segmented(mut self) -> Self {
        self.segmented = true;
        self
    }
}

impl Sizable for ToggleGroup {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl ToggleVariants for ToggleGroup {
    fn with_variant(mut self, variant: ToggleVariant) -> Self {
        self.variant = variant;
        self
    }
}

impl Disableable for ToggleGroup {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl Styled for ToggleGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for ToggleGroup {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let disabled = self.disabled;
        let items_len = self.items.len();
        let checks = self
            .items
            .iter()
            .map(|item| item.checked)
            .collect::<Vec<bool>>();
        let clicked_index = Rc::new(Cell::new(None));
        let segmented = self.segmented;
        let (size, variant) = (self.size, self.variant);

        BaseToggleGroup::new(self.id)
            .axis(Axis::Horizontal)
            .child(
                h_flex()
                    .items_center()
                    .when(!segmented, |this| this.gap_2())
                    .refine_style(&self.style)
                    .children(self.items.into_iter().enumerate().map(|(ix, item)| {
                        let corners = |left: bool, right: bool| Corners {
                            top_left: left,
                            top_right: right,
                            bottom_left: left,
                            bottom_right: right,
                        };
                        let edges = |left: bool| Edges {
                            left,
                            top: true,
                            right: true,
                            bottom: true,
                        };
                        let item = if !segmented || items_len == 1 {
                            item
                        } else if ix == 0 {
                            item.border_corners(corners(true, false))
                                .border_edges(edges(true))
                        } else if ix == items_len - 1 {
                            item.border_corners(corners(false, true))
                                .border_edges(edges(false))
                        } else {
                            item.border_corners(corners(false, false))
                                .border_edges(edges(false))
                        };
                        let effective_disabled = disabled || item.disabled;
                        let clicked_index = clicked_index.clone();
                        let mut item = item
                            .disabled(effective_disabled)
                            .with_size(size)
                            .with_variant(variant)
                            .on_click(move |_, _, cx| {
                                clicked_index.set(Some(ix));
                                cx.propagate();
                            });
                        item.in_group = true;
                        item
                    })),
            )
            .when_some(
                (!disabled).then_some(self.on_click).flatten(),
                |this, on_click| {
                    this.on_click(move |_, window, cx| {
                        let Some(ix) = clicked_index.take() else {
                            return;
                        };
                        let mut next = checks.clone();
                        next[ix] = !next[ix];
                        on_click(&next, window, cx);
                    })
                },
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turning_on_pops_and_rocks() {
        assert_eq!(toggle_frame(true, false, None), (1.0, 0.0));
        let (scale, rock) = toggle_frame(true, false, Some(ms(0)));
        assert_eq!((scale, rock), (0.9, 0.0));
        let (scale, _) = toggle_frame(true, false, Some(ms(136)));
        assert!((scale - (1.08_f32 * 1.1).sqrt()).abs() < 1e-4);
        let (_, rock) = toggle_frame(true, false, Some(ms(120)));
        assert!((rock + 14.0).abs() < 1e-3);
        assert_eq!(toggle_frame(true, false, Some(ms(500))), (1.0, 0.0));
    }

    #[test]
    fn turning_off_only_springs_back() {
        let (scale, rock) = toggle_frame(false, false, Some(ms(120)));
        assert!((scale - 1.03).abs() < 1e-4);
        assert_eq!(rock, 0.0);
    }

    #[test]
    fn tracks_match_the_web() {
        use crate::parity::{assert_number_track, assert_pose_track};
        assert_pose_track("pop-toggle", "kk-pop-toggle-on", &on_track(), &[]);
        assert_pose_track("pop-toggle", "kk-pop-toggle-off", &off_track(), &[]);
        assert_number_track("pop-toggle", "kk-pop-toggle-rock", "rotate", &rock_track());
        assert_number_track("pop-toggle", "kk-pop-toggle-press", "sx", &press_track());
        let group = group_track();
        assert_pose_track("pop-toggle-group", "kk-pop-toggle-group-pop-a", &group, &[]);
        assert_pose_track("pop-toggle-group", "kk-pop-toggle-group-pop-b", &group, &[]);
    }

    #[test]
    fn group_items_pop_from_092() {
        assert_eq!(toggle_frame(true, true, Some(ms(0))).0, 0.92);
        assert_eq!(toggle_frame(false, true, Some(ms(0))).0, 0.92);
    }
}
