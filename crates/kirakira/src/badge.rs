//! Badge: a count, dot or icon that pops onto its element, and a strip of tape that slaps on.
//!
//! Replaces `gpui_kit::component::badge`, rebuilt with GPUI Component's sizes, colours and
//! placement, so `use kirakira::badge::*` is a drop-in.
//!
//! - [`Badge`] is GPUI Component's corner badge: a count, a dot or an icon on an avatar or a
//!   button. Its indicator pops on the way the web kit's corner badge does (Pop Avatar's
//!   `AvatarBadge`, which is this piece): from 60 % size and transparent, opaque by 30 %, past full
//!   size (106 %) at 55 %, under (98 %) at 80 %, rest, in 0.4 s on `ease-in-out`, 0.15 s after it
//!   appears. A count that drops to zero hides it; the next count pops it in again.
//! - [`Tape`] is the web Pop Badge's `tape` variant, which GPUI Component has no counterpart for:
//!   a strip of ink tape with torn ends and paper-coloured text, tilted (−3° by default), that slaps
//!   on: big (120 %) and turned 8° past its tilt, squashed under the hand (95 %, 2° the other way)
//!   at 40 %, 102 % at 70 %, settled, in 0.4 s; the first stretch falls on `cubic-bezier(0.8, 0,
//!   1, 1)`, the rest on `ease-in-out`, and it is opaque by 20 %. It waits until it scrolls into
//!   view by default.
//!
//! Under reduced motion both are simply there.
//!
//! Composition-safe: both run on a [`Clock`], so inside a
//! [`Timeline`](crate::timeline::Timeline) they follow the timeline's time.
//!
//! How it's drawn: the tape is a [`Vector`]: its torn strip (the web version's clip-path polygon)
//! and its label are two single-colour layers under one transform, so it scales and turns exactly
//! like the web keyframes. It rests tilted, so unlike other components it stays a vector at rest,
//! and its label is drawn by resvg, a touch softer than GPUI's own text. The indicator scales
//! evenly by drawing at a scaled rem size, so its sizes are in rems (the same pixels at the default
//! rem size).
//!
//! Differences from the web version: the web badge also grows to 106 % under the pointer; GPUI Component's badges take no id,
//! so there is no per-badge hover state to hang that on, and it is left out. The web badge's
//! colour variants (secondary, outline, ...) are GPUI Component's `tag`, not its badge.
//! Badges take no id, so each indicator's clock is keyed by [`Badge::id`], by default the place in
//! the source where the badge was made (`Badge::new()`'s caller). Badges made by the same line
//! under one parent, in a loop say, share a clock: give them ids.

use gpui_kit::base::{StyledExt as _, h_flex};
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable, Size, ThemeStyled as _, white};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, ElementId, FontWeight, Hsla, InteractiveElement as _, IntoElement,
    ParentElement, Pixels, Refineable as _, RenderOnce, Role, SharedString,
    StatefulInteractiveElement as _, StyleRefinement, Styled, TextStyleRefinement, Window, div, px,
    relative, rems, size,
};

use crate::motion::{
    Clock, Easing, Keyframes, Pose, Timing, Track, Trigger, delay_ms, ms, split_layout, transform,
};
use crate::vector::{Layer, TextSetting, Vector};

#[derive(Default, Clone)]
enum BadgeVariant {
    #[default]
    Number,
    Dot,
    Icon(Box<Icon>),
}

/// A badge for displaying a count, dot, or icon on an element.
#[derive(IntoElement)]
pub struct Badge {
    style: StyleRefinement,
    count: usize,
    max: usize,
    variant: BadgeVariant,
    children: Vec<AnyElement>,
    color: Option<Hsla>,
    size: Size,
    id: ElementId,
}

impl Default for Badge {
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}

impl Badge {
    /// Create a new badge.
    #[track_caller]
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            count: 0,
            max: 99,
            variant: Default::default(),
            color: None,
            children: Vec::new(),
            size: Size::default(),
            id: std::panic::Location::caller().into(),
        }
    }

    /// Set to use a dot.
    pub fn dot(mut self) -> Self {
        self.variant = BadgeVariant::Dot;
        self
    }

    /// Set to show a count.
    ///
    /// If count is 0, the badge will be hidden.
    pub fn count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }

    /// Set to show an icon.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.variant = BadgeVariant::Icon(Box::new(icon.into()));
        self
    }

    /// Set the maximum count to show (only for counts).
    pub fn max(mut self, max: usize) -> Self {
        self.max = max;
        self
    }

    /// Set the color (background) of the badge.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Keys the indicator's clock. The place in the source where the badge was made by default.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = id.into();
        self
    }
}

impl ParentElement for Badge {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Sizable for Badge {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Badge {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// `kk-pop-avatar-pop`, as the corner badge plays it: scale and opacity.
pub(crate) fn pop_tracks() -> (Keyframes<f32>, Keyframes<f32>) {
    let ease = Easing::EaseInOut;
    (
        Track::new(ease.clone())
            .at(0.0, 0.6)
            .at(0.55, 1.06)
            .at(0.8, 0.98)
            .at(1.0, 1.0)
            .build(),
        Track::new(ease)
            .at(0.0, 0.0)
            .at(0.3, 1.0)
            .at(1.0, 1.0)
            .build(),
    )
}

/// Pixels as rems at the default 16 px rem, so a scaled rem size scales them.
fn r(pixels: f32) -> gpui_kit::Rems {
    rems(pixels / 16.0)
}

impl RenderOnce for Badge {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let visible = match self.variant {
            BadgeVariant::Number => self.count > 0,
            BadgeVariant::Dot | BadgeVariant::Icon(_) => true,
        };

        let (size, text_size) = match self.size {
            Size::Large => (24., 14.),
            Size::Medium | Size::Size(_) => (16., 10.),
            Size::Small | Size::XSmall => (10., 8.),
        };

        let count = if self.count > self.max {
            format!("{}+", self.max)
        } else {
            self.count.to_string()
        };
        // The indicator's measured size belongs to what it shows: a dot never borrows a count's.
        let shape: SharedString = match self.variant {
            BadgeVariant::Number => format!("kk-indicator-{count}").into(),
            BadgeVariant::Dot => "kk-indicator-dot".into(),
            BadgeVariant::Icon(_) => "kk-indicator-icon".into(),
        };

        let indicator = visible.then(|| {
            let clock = Clock::new((self.id.clone(), "kk-pop"), Trigger::Mount, window, cx);
            clock.animate(Some(ms(550)), window);
            let timing = Timing::new(ms(400)).delay(delay_ms(150));
            let (scale, fade) = pop_tracks();
            let pose = Pose::new()
                .scale(clock.sample(&scale, &timing))
                .opacity(clock.sample(&fade, &timing));

            let mut indicator = h_flex()
                .absolute()
                .justify_center()
                .items_center()
                .rounded_full_style(cx)
                .bg(self.color.unwrap_or(cx.theme().red))
                .text_color(white())
                .text_size(r(text_size))
                .map(|this| match self.variant.clone() {
                    BadgeVariant::Dot => this.top_0().right_0().size(r(6.)),
                    BadgeVariant::Number => {
                        let (top, left) = match self.size {
                            Size::Large => (px(2.), -px(count.len() as f32)),
                            Size::Medium | Size::Size(_) => (-px(3.), -px(3.) * count.len()),
                            Size::Small | Size::XSmall => (-px(4.), -px(4.) * count.len()),
                        };

                        this.top(top)
                            .right(left)
                            .py_0p5()
                            .px_0p5()
                            .min_w_3p5()
                            .text_size(r(10.))
                            .line_height(relative(1.))
                            .child(count)
                    }
                    BadgeVariant::Icon(icon) => this
                        .right_0()
                        .bottom_0()
                        .size(r(size))
                        .border_1()
                        .border_color(cx.theme().background)
                        .child(*icon),
                });
            let outer = split_layout(indicator.style());
            transform((self.id.clone(), shape), pose, indicator).outer_style(outer)
        });

        div()
            .relative()
            .refine_style(&self.style)
            .children(self.children)
            .children(indicator)
    }
}

/// The tape's torn outline, as points in a `width × height` box with `em` the text size: the
/// clip-path polygon of the web version.
pub(crate) fn tape_outline(width: f32, height: f32, em: f32) -> [(f32, f32); 14] {
    let (w, h) = (width, height);
    [
        (0.15 * em, 0.0),
        (w - 0.2 * em, 0.0),
        (w, 0.18 * h),
        (w - 0.25 * em, 0.36 * h),
        (w - 0.05 * em, 0.55 * h),
        (w - 0.3 * em, 0.74 * h),
        (w, 0.9 * h),
        (w - 0.15 * em, h),
        (0.1 * em, h),
        (0.25 * em, 0.84 * h),
        (0.0, 0.66 * h),
        (0.2 * em, 0.47 * h),
        (0.05 * em, 0.3 * h),
        (0.3 * em, 0.14 * h),
    ]
}

/// `kk-pop-badge-slap` for a tape tilted `tilt` degrees: scale, turn (degrees) and opacity.
pub(crate) fn slap_tracks(tilt: f32) -> (Keyframes<f32>, Keyframes<f32>, Keyframes<f32>) {
    let fall = Easing::cubic_bezier(0.8, 0.0, 1.0, 1.0).expect("valid curve");
    let ease = Easing::EaseInOut;
    (
        Track::new(ease.clone())
            .at_ease(0.0, 1.2, fall.clone())
            .at(0.4, 0.95)
            .at(0.7, 1.02)
            .at(1.0, 1.0)
            .build(),
        Track::new(ease.clone())
            .at_ease(0.0, tilt - 8.0, fall.clone())
            .at(0.4, tilt + 2.0)
            .at(0.7, tilt - 1.0)
            .at(1.0, tilt)
            .build(),
        Track::new(ease)
            .at_ease(0.0, 0.0, fall)
            .at(0.2, 1.0)
            .at(1.0, 1.0)
            .build(),
    )
}

/// The torn strip as SVG markup for a box of `box_size`, drawn in one colour by a vector layer.
fn strip_markup(box_size: gpui_kit::Size<Pixels>, em: f32) -> String {
    let (w, h) = (box_size.width.as_f32(), box_size.height.as_f32());
    let points = tape_outline(w, h, em)
        .iter()
        .map(|(x, y)| format!("{x:.2},{y:.2}"))
        .collect::<Vec<_>>()
        .join(" ");
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}"><polygon points="{points}" fill="black"/></svg>"#
    )
}

/// A strip of ink tape with torn ends that slaps on, tilted. The web Pop Badge's `tape` variant.
#[derive(IntoElement)]
pub struct Tape {
    id: ElementId,
    label: SharedString,
    style: StyleRefinement,
    tilt: f32,
    delay: u64,
    trigger: Trigger,
}

impl Tape {
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            style: StyleRefinement::default(),
            tilt: -3.0,
            delay: 0,
            trigger: Trigger::InView,
        }
    }

    /// The resting tilt in degrees. −3 by default.
    pub fn tilt(mut self, degrees: f32) -> Self {
        self.tilt = degrees;
        self
    }

    /// Milliseconds before it slaps on, to stagger several. 0 by default.
    pub fn delay(mut self, delay_ms: u64) -> Self {
        self.delay = delay_ms;
        self
    }

    /// When it slaps on. [`Trigger::InView`] by default.
    pub fn trigger(mut self, trigger: Trigger) -> Self {
        self.trigger = trigger;
        self
    }
}

impl Styled for Tape {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Tape {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let clock = Clock::new((self.id.clone(), "kk-slap"), self.trigger, window, cx);
        clock.animate(Some(ms(self.delay + 400)), window);
        let timing = Timing::new(ms(400)).delay(delay_ms(self.delay));
        let (scale, turn, fade) = slap_tracks(self.tilt);
        let pose = Pose::new()
            .scale(clock.sample(&scale, &timing))
            .rotate(clock.sample(&turn, &timing))
            .opacity(clock.sample(&fade, &timing));

        // `text-xs font-medium`, paper on ink, under the user's own text styles.
        let mut text = TextStyleRefinement {
            font_size: Some(rems(0.75).into()),
            font_weight: Some(FontWeight::MEDIUM),
            color: Some(cx.theme().background),
            ..Default::default()
        };
        text.refine(&self.style.text);
        let setting = TextSetting::new(&text, window);
        let line = setting.line_box(&self.label, window);
        // `px-2.5 pt-0.5 pb-1`.
        let rem = window.rem_size();
        let (pad_x, pad_top, pad_bottom) = (rem * 0.625, rem * 0.125, rem * 0.25);
        let box_size = size(
            line.size.width + pad_x * 2.,
            line.size.height + pad_top + pad_bottom,
        );
        let strip = Layer::svg(
            strip_markup(box_size, setting.face.size.as_f32()),
            cx.theme().foreground,
        );
        let label = Layer::text(
            &self.label,
            &setting.face,
            box_size,
            pad_top + line.baseline,
            setting.color,
        );

        let mut style = self.style;
        style.text = Default::default();
        clock.observe(
            div()
                .id((self.id, "kk-tape"))
                .role(Role::Label)
                .aria_label(self.label)
                .flex_none()
                .refine_style(&style)
                .child(Vector::new(box_size).layer(strip).layer(label).pose(pose)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sibling_badges_get_their_own_keys() {
        let count = Badge::new().count(120);
        let dot = Badge::new().dot();
        assert_ne!(count.id, dot.id);
        let made = || Badge::new();
        assert_eq!(made().id, made().id);
    }

    #[test]
    fn tape_outline_stays_in_its_box() {
        for (x, y) in tape_outline(80.0, 20.0, 12.0) {
            assert!((0.0..=80.0).contains(&x));
            assert!((0.0..=20.0).contains(&y));
        }
    }

    #[test]
    fn indicator_pop_matches_the_web() {
        use crate::parity::assert_number_track;
        let (scale, fade) = pop_tracks();
        assert_number_track("pop-avatar", "kk-pop-avatar-pop", "sx", &scale);
        assert_number_track("pop-avatar", "kk-pop-avatar-pop", "opacity", &fade);
        // `kk-pop-badge-slap` keys `rotate` on `calc(var(--kk-pop-badge-tilt) - 8deg)`, which the
        // parity parser can't read, so the whole rule is checked by hand below instead.
    }

    #[test]
    fn slap_overshoots_the_tilt_and_settles_on_it() {
        let (scale, turn, fade) = slap_tracks(-3.0);
        assert_eq!(scale.sample(0.0), 1.2);
        assert_eq!(turn.sample(0.0), -11.0);
        assert_eq!(turn.sample(0.4), -1.0);
        assert_eq!(scale.sample(0.4), 0.95);
        assert_eq!(turn.sample(1.0), -3.0);
        assert_eq!(fade.sample(0.2), 1.0);
    }
}
