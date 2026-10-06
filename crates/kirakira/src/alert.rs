//! Alert: a box that pops up from below, squashed, and an icon that lands a beat later.
//!
//! Replaces `gpui_kit::component::alert`, rebuilt with GPUI Component's sizes, colours and layout,
//! so `use kirakira::alert::*` is a drop-in.
//!
//! The box rises from 0.75 em below, squashed to 97 % × 80 %, stretches past its height (100.5 %
//! × 105 %, 0.1 em high, at 55 %), dips to 99 % tall at 80 % and settles, in 0.4 s; it is opaque by
//! 30 %. Its icon lands 150 ms later in 0.45 s: from nothing, past full size with a turn (1.3,
//! 10°), back under (0.9, −4°), rest. An error alert's icon jolts side to side instead (−14°, 10°,
//! −5°). It waits until it scrolls into view by default. Under reduced motion it is simply there.
//!
//! Composition-safe: inside a [`Timeline`](crate::timeline::Timeline) it plays on the timeline's
//! clock.
//!
//! How it's drawn: the box's background and border are an owned plate, so its squash is exact.
//! Text can only scale evenly, so the content follows the plate's horizontal scale (97–100.5 %)
//! and keeps its height while the plate squashes behind it. Background, border colour, border
//! widths and corner radii set through `Styled` go to the plate; gap and alignment go to the row
//! of icon, text and close button. The icon inherits the alert's text colour, as in GPUI
//! Component, and fades in through an opacity wrapper.

use std::rc::Rc;

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::text::{Text, TextViewStyle};
use gpui_kit::component::{ActiveTheme as _, Colorize as _, Icon, IconName, Sizable, Size};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AbsoluteLength, App, ClickEvent, ElementId, Empty, Hsla, InteractiveElement as _, IntoElement,
    ParentElement as _, Refineable as _, RenderOnce, Role, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Transformation, Window, div, px,
    radians, relative, rems, size, transparent_white,
};

pub use gpui_kit::component::alert::AlertVariant;

use crate::motion::{
    Clock, Easing, Keyframes, Pose, Timing, Track, Trigger, delay_ms, ms, split_layout, transform,
};

fn fg(variant: AlertVariant, cx: &App) -> Hsla {
    match variant {
        AlertVariant::Default => cx.theme().foreground,
        AlertVariant::Info => cx.theme().info,
        AlertVariant::Success => cx.theme().success,
        AlertVariant::Warning => cx.theme().warning,
        AlertVariant::Error => cx.theme().danger,
    }
}

fn bg(variant: AlertVariant, cx: &App) -> Hsla {
    match variant {
        AlertVariant::Default => cx.theme().background,
        AlertVariant::Info => cx.theme().info.mix_oklab(transparent_white(), 0.04),
        AlertVariant::Success => cx.theme().success.mix_oklab(transparent_white(), 0.04),
        AlertVariant::Warning => cx.theme().warning.mix_oklab(transparent_white(), 0.04),
        AlertVariant::Error => cx.theme().danger.mix_oklab(transparent_white(), 0.04),
    }
}

fn border_color(variant: AlertVariant, cx: &App) -> Hsla {
    match variant {
        AlertVariant::Default => cx.theme().border,
        AlertVariant::Info => cx.theme().info.mix_oklab(transparent_white(), 0.3),
        AlertVariant::Success => cx.theme().success.mix_oklab(transparent_white(), 0.3),
        AlertVariant::Warning => cx.theme().warning.mix_oklab(transparent_white(), 0.3),
        AlertVariant::Error => cx.theme().danger.mix_oklab(transparent_white(), 0.3),
    }
}

/// `kk-pop-alert-in`: the box's motion, with `y` in em, and its opacity.
pub(crate) fn box_tracks() -> (Keyframes<Pose>, Keyframes<f32>) {
    (
        Track::new(Easing::EaseInOut)
            .at_ease(
                0.0,
                Pose::new().y(0.75).scale_xy(0.97, 0.8),
                Easing::EaseOut,
            )
            .at(0.55, Pose::new().y(-0.1).scale_xy(1.005, 1.05))
            .at(0.8, Pose::new().scale_xy(1.0, 0.99))
            .at(1.0, Pose::new())
            .build(),
        Track::new(Easing::EaseInOut)
            .at_ease(0.0, 0.0, Easing::EaseOut)
            .at(0.3, 1.0)
            .at(1.0, 1.0)
            .build(),
    )
}

/// The icon's landing: scale in `sx`, turn in `rotate`, and its opacity. `kk-pop-alert-jolt` for
/// errors, `kk-pop-alert-icon` otherwise.
pub(crate) fn icon_tracks(jolt: bool) -> (Keyframes<f32>, Keyframes<f32>, Keyframes<f32>) {
    let ease = Easing::EaseInOut;
    let fade = Track::new(ease.clone())
        .at(0.0, 0.0)
        .at(0.2, 1.0)
        .at(1.0, 1.0)
        .build();
    if jolt {
        (
            Track::new(ease.clone())
                .at(0.0, 0.0)
                .at(0.4, 1.3)
                .at(0.55, 1.0)
                .at(1.0, 1.0)
                .build(),
            Track::new(ease)
                .at(0.0, 0.0)
                .at(0.4, 0.0)
                .at(0.55, -14.0)
                .at(0.7, 10.0)
                .at(0.85, -5.0)
                .at(1.0, 0.0)
                .build(),
            fade,
        )
    } else {
        (
            Track::new(ease.clone())
                .at(0.0, 0.0)
                .at(0.5, 1.3)
                .at(0.75, 0.9)
                .at(1.0, 1.0)
                .build(),
            Track::new(ease)
                .at(0.0, -30.0)
                .at(0.5, 10.0)
                .at(0.75, -4.0)
                .at(1.0, 0.0)
                .build(),
            fade,
        )
    }
}

/// Alert used to display a message to the user.
#[derive(IntoElement)]
pub struct Alert {
    id: ElementId,
    style: StyleRefinement,
    variant: AlertVariant,
    icon: Icon,
    title: Option<SharedString>,
    message: Text,
    size: Size,
    banner: bool,
    on_close: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    visible: bool,
    trigger: Trigger,
    delay: u64,
}

impl Alert {
    /// Create a new alert with the given message.
    pub fn new(id: impl Into<ElementId>, message: impl Into<Text>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            variant: AlertVariant::default(),
            icon: Icon::new(IconName::Info),
            title: None,
            message: message.into(),
            size: Size::default(),
            banner: false,
            visible: true,
            on_close: None,
            trigger: Trigger::InView,
            delay: 0,
        }
    }

    /// Create a new info [`AlertVariant::Info`] with the given message.
    pub fn info(id: impl Into<ElementId>, message: impl Into<Text>) -> Self {
        Self::new(id, message)
            .with_variant(AlertVariant::Info)
            .icon(IconName::Info)
    }

    /// Create a new [`AlertVariant::Success`] alert with the given message.
    pub fn success(id: impl Into<ElementId>, message: impl Into<Text>) -> Self {
        Self::new(id, message)
            .with_variant(AlertVariant::Success)
            .icon(IconName::CircleCheck)
    }

    /// Create a new [`AlertVariant::Warning`] alert with the given message.
    pub fn warning(id: impl Into<ElementId>, message: impl Into<Text>) -> Self {
        Self::new(id, message)
            .with_variant(AlertVariant::Warning)
            .icon(IconName::TriangleAlert)
    }

    /// Create a new [`AlertVariant::Error`] alert with the given message.
    pub fn error(id: impl Into<ElementId>, message: impl Into<Text>) -> Self {
        Self::new(id, message)
            .with_variant(AlertVariant::Error)
            .icon(IconName::CircleX)
    }

    /// Sets the [`AlertVariant`] of the alert.
    pub fn with_variant(mut self, variant: AlertVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set the icon for the alert.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = icon.into();
        self
    }

    /// Set the title for the alert.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set alert as banner style.
    ///
    /// The `banner` style will make the alert take the full width of the container and not border and radius.
    /// This mode will not display `title`.
    pub fn banner(mut self) -> Self {
        self.banner = true;
        self
    }

    /// Set the visibility of the alert.
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Set alert as closable, true will show Close icon.
    pub fn on_close(
        mut self,
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Some(Rc::new(on_close));
        self
    }

    /// When the alert pops in. [`Trigger::InView`] by default.
    pub fn trigger(mut self, trigger: Trigger) -> Self {
        self.trigger = trigger;
        self
    }

    /// Milliseconds before the pop; the icon lands 150 ms after the box. 0 by default.
    pub fn delay(mut self, delay_ms: u64) -> Self {
        self.delay = delay_ms;
        self
    }
}

impl Sizable for Alert {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Alert {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Alert {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if !self.visible {
            return Empty.into_any_element();
        }

        let (radius, padding_x, padding_y, gap) = match self.size {
            Size::XSmall => (cx.theme().radius, px(12.), px(6.), px(6.)),
            Size::Small => (cx.theme().radius, px(12.), px(8.), px(6.)),
            Size::Large => (cx.theme().radius_lg, px(20.), px(14.), px(12.)),
            _ => (cx.theme().radius, px(16.), px(10.), px(12.)),
        };

        let bg = bg(self.variant, cx);
        let fg = fg(self.variant, cx);
        let border_color = border_color(self.variant, cx);

        let id = self.id.clone();
        let clock = Clock::new((id.clone(), "kk-pop"), self.trigger, window, cx);
        clock.animate(Some(ms(self.delay + 600)), window);
        let em = window.rem_size().as_f32() * 0.875;
        let (motion, fade) = box_tracks();
        let timing = Timing::new(ms(400)).delay(delay_ms(self.delay));
        let pose = clock.sample(&motion, &timing);
        let opacity = clock.sample(&fade, &timing);
        let (scale, turn, icon_fade) = icon_tracks(self.variant == AlertVariant::Error);
        let icon_timing = Timing::new(ms(450)).delay(delay_ms(self.delay + 150));
        let icon_scale = clock.sample(&scale, &icon_timing);
        let icon_turn = clock.sample(&turn, &icon_timing);
        let icon_opacity = clock.sample(&icon_fade, &icon_timing);

        // Layout goes on the motion wrapper, the box's look on the plate, the rest on the row.
        let mut style = self.style;
        let outer = split_layout(&mut style);
        let plate_bg = style.background.take();
        let plate_border = style.border_color.take();
        let plate_widths = std::mem::take(&mut style.border_widths);
        let rem = window.rem_size();
        let edge = |width: Option<AbsoluteLength>| width.map_or(px(1.), |w| w.to_pixels(rem));
        let plate_radii = std::mem::take(&mut style.corner_radii);
        // The row lays out the content, so the user's gap and alignment go to it.
        let row_style = StyleRefinement {
            gap: std::mem::take(&mut style.gap),
            align_items: style.align_items.take(),
            justify_content: style.justify_content.take(),
            ..Default::default()
        };
        let banner = self.banner;

        let plate = div()
            .absolute()
            .w(relative(pose.sx))
            .h(relative(pose.sy))
            .left(relative((1.0 - pose.sx) * 0.5))
            .top(relative((1.0 - pose.sy) * 0.5))
            .bg(bg)
            .border_1()
            .border_color(border_color)
            .when(!banner, |this| this.rounded(radius))
            .when_some(plate_bg, |this, fill| {
                this.map(|mut this| {
                    this.style().background = Some(fill);
                    this
                })
            })
            .when_some(plate_border, |this, color| this.border_color(color))
            .map(|mut this| {
                this.style().border_widths.refine(&plate_widths);
                this.style().corner_radii.refine(&plate_radii);
                this
            });

        // The icon keeps the colour it inherits (the variant's, or the user's) and fades in.
        let icon = div().opacity(icon_opacity).child(
            self.icon.transform(
                Transformation::scale(size(icon_scale, icon_scale))
                    .with_rotation(radians(icon_turn.to_radians())),
            ),
        );

        let row = div()
            .flex()
            .w_full()
            .gap(gap)
            .justify_between()
            .map(|this| {
                if banner {
                    this.items_center()
                } else {
                    this.items_start()
                }
            })
            .refine_style(&row_style)
            .child(
                div()
                    .flex()
                    .flex_1()
                    .when(banner, |this| this.items_center())
                    .overflow_hidden()
                    .gap(gap)
                    .child(div().when(!banner, |this| this.mt(px(5.))).child(icon))
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .gap_3()
                            .when(!banner, |this| {
                                this.when_some(self.title, |this, title| {
                                    this.child(
                                        div().w_full().truncate().font_semibold().child(title),
                                    )
                                })
                            })
                            .child(
                                self.message
                                    .style(TextViewStyle::default().paragraph_gap(rems(0.2))),
                            ),
                    ),
            )
            .when_some(self.on_close, |this, on_close| {
                this.child(
                    div()
                        .id("close")
                        .p_0p5()
                        .rounded(cx.theme().radius)
                        .hover(|this| this.bg(bg.opacity(0.8)))
                        .active(|this| this.bg(bg.opacity(0.9)))
                        .on_click(move |ev, window, cx| {
                            on_close(ev, window, cx);
                        })
                        .child(
                            Icon::new(IconName::Close)
                                .with_size(self.size.max(Size::Medium))
                                .flex_shrink_0(),
                        ),
                )
            });

        let mut content_layout = StyleRefinement {
            flex_grow: Some(1.0),
            flex_shrink: Some(1.0),
            ..Default::default()
        };
        content_layout.min_size.width = Some(px(0.).into());

        let alert = div()
            .id(id.clone())
            .role(Role::Alert)
            .relative()
            .flex()
            .w_full()
            .text_color(fg)
            // The plate draws the border, so the padding takes its place.
            .pl(padding_x + edge(plate_widths.left))
            .pr(padding_x + edge(plate_widths.right))
            .pt(padding_y + edge(plate_widths.top))
            .pb(padding_y + edge(plate_widths.bottom))
            .text_sm()
            .refine_style(&style)
            .child(plate)
            .child(
                transform((id.clone(), "kk-content"), Pose::new().scale(pose.sx), row)
                    .outer_style(content_layout),
            );

        transform(
            (id, "kk-alert"),
            Pose::new().y(pose.y * em).opacity(opacity),
            clock.observe(alert),
        )
        .outer_style(outer)
        .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn box_rises_squashed_and_settles() {
        let (motion, fade) = box_tracks();
        let start = motion.sample(0.0);
        assert_eq!((start.y, start.sx, start.sy), (0.75, 0.97, 0.8));
        let peak = motion.sample(0.55);
        assert_eq!((peak.y, peak.sy), (-0.1, 1.05));
        assert_eq!(motion.sample(1.0), Pose::new());
        assert_eq!(fade.sample(0.3), 1.0);
    }

    #[test]
    fn keyframes_match_the_web() {
        use crate::parity::{assert_number_track, assert_pose_track};
        let (motion, fade) = box_tracks();
        // The parser reads `0.75em` as 0.75, and the track's `y` is in ems.
        assert_pose_track("pop-alert", "kk-pop-alert-in", &motion, &[]);
        assert_number_track("pop-alert", "kk-pop-alert-in", "opacity", &fade);
        for (name, jolt) in [("kk-pop-alert-icon", false), ("kk-pop-alert-jolt", true)] {
            let (scale, turn, fade) = icon_tracks(jolt);
            assert_number_track("pop-alert", name, "sx", &scale);
            assert_number_track("pop-alert", name, "rotate", &turn);
            assert_number_track("pop-alert", name, "opacity", &fade);
        }
    }

    #[test]
    fn error_icon_jolts_instead_of_turning() {
        let (scale, turn, _) = icon_tracks(true);
        assert_eq!(scale.sample(0.4), 1.3);
        assert_eq!(turn.sample(0.55), -14.0);
        assert_eq!(turn.sample(0.7), 10.0);
        let (scale, turn, _) = icon_tracks(false);
        assert_eq!(scale.sample(0.5), 1.3);
        assert_eq!(turn.sample(0.0), -30.0);
    }
}
