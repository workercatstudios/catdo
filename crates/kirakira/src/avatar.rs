//! Avatar: a picture that pops in, initials that fade in, and an optional beacon ring.
//!
//! Replaces `gpui_kit::component::avatar`, rebuilt on gpui-base with GPUI Component's sizes,
//! colours and initials, so `use kirakira::avatar::*` is a drop-in.
//!
//! - An avatar with a picture pops in when it mounts: from 60 % size and transparent, opaque by
//!   30 %, past full size (106 %) at 55 %, under (98 %) at 80 %, rest, in 0.4 s on `ease-in-out`.
//! - An avatar showing initials or the placeholder fades in over 0.3 s on `ease-out`.
//! - [`Avatar::ring`] adds a steady 2 px ring 2 px outside the avatar and a copy of it that swells
//!   to 145 % and fades from 80 % opacity, every 1.6 s on `ease-out`, like a beacon, for "online"
//!   or "live". [`Avatar::ring_pulse`] does it in Kirakira pink.
//!
//! Under reduced motion the avatar is simply there and the beacon stops; the steady ring stays.
//!
//! Composition-safe: the pop, the fade and the beacon run on a [`Clock`], so inside a
//! [`Timeline`](crate::timeline::Timeline) they follow the timeline's time.
//!
//! Differences from the web version: GPUI loads images itself, so the pop plays when the avatar
//! mounts rather than when its picture has loaded. GPUI Component's avatar has a background and a
//! border of its own, so the whole disc pops, not only the picture; it is an owned shape, so the
//! pop's size is exact. Avatars take no id, so each one's clock is keyed by [`Avatar::id`], by
//! default the place in the source where the avatar was made (`Avatar::new()`'s caller) and its
//! name; an [`AvatarGroup`] adds its own place and each avatar's position. Avatars without a name
//! made by the same line under one parent, in a loop say, share a clock: give them ids.

use std::panic::Location;

use gpui_kit::base::{Avatar as BaseAvatar, AvatarFallback, AvatarImage, StyledExt as _};
use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable, Size, ThemeStyled as _, oklch,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, DefiniteLength, Div, ElementId, Hsla, ImageSource, InteractiveElement, Interactivity,
    IntoElement, Length, ParentElement as _, Pixels, Refineable as _, RenderOnce, SharedString,
    StyleRefinement, Styled, Window, div, px, relative, rems,
};

use crate::motion::{Clock, Easing, IterationCount, Keyframes, Timing, Track, Trigger, ms};
use crate::theme::ActiveKira as _;

/// Returns the size of the avatar based on the given [`Size`].
fn avatar_size(size: Size) -> Pixels {
    match size {
        Size::Large => px(80.),
        Size::Medium => px(48.),
        Size::Small => px(24.),
        Size::XSmall => px(16.),
        Size::Size(size) => size,
    }
}

fn avatar_text_size(this: Div, size: Size) -> Div {
    match size {
        Size::Large => this.text_3xl().font_semibold(),
        Size::Medium => this.text_sm(),
        Size::Small => this.text_xs(),
        Size::XSmall => this.text_size(rems(0.65)),
        Size::Size(size) => this.size(size * 0.5),
    }
}

/// `kk-pop-avatar-pop`: the picture's scale and opacity.
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

/// `kk-pop-avatar-fade`: initials and the placeholder fading in, on `ease-out`.
pub(crate) fn fade_in_track() -> Keyframes<f32> {
    Track::new(Easing::EaseOut)
        .at(0.0, 0.0)
        .at(1.0, 1.0)
        .build()
}

/// `kk-pop-avatar-pulse`: the beacon's scale and opacity.
pub(crate) fn beacon_tracks() -> (Keyframes<f32>, Keyframes<f32>) {
    (
        Track::new(Easing::EaseOut)
            .at(0.0, 1.0)
            .at(0.8, 1.45)
            .at(1.0, 1.45)
            .build(),
        Track::new(Easing::EaseOut)
            .at(0.0, 0.8)
            .at(0.8, 0.0)
            .at(1.0, 0.0)
            .build(),
    )
}

/// The ring's gap outside the avatar, and its width.
const RING_GAP: f32 = 2.0;
const RING_WIDTH: f32 = 2.0;

/// User avatar element.
///
/// We can use [`Sizable`] trait to set the size of the avatar (16, 24, 48 or 80 px, or any size).
#[derive(IntoElement)]
pub struct Avatar {
    base: BaseAvatar,
    style: StyleRefinement,
    src: Option<ImageSource>,
    name: Option<SharedString>,
    short_name: SharedString,
    placeholder: Icon,
    size: Size,
    id: Option<ElementId>,
    location: &'static Location<'static>,
    ring: Option<Option<Hsla>>,
}

impl Default for Avatar {
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}

impl Avatar {
    #[track_caller]
    pub fn new() -> Self {
        Self {
            location: Location::caller(),
            base: BaseAvatar::new(),
            style: StyleRefinement::default(),
            src: None,
            name: None,
            short_name: SharedString::default(),
            placeholder: Icon::new(IconName::User),
            size: Size::Medium,
            id: None,
            ring: None,
        }
    }

    /// Set to use image source for the avatar.
    pub fn src(mut self, source: impl Into<ImageSource>) -> Self {
        self.src = Some(source.into());
        self
    }

    /// Set name of the avatar user, if `src` is none, will use this name as placeholder.
    pub fn name(mut self, name: impl Into<SharedString>) -> Self {
        let name: SharedString = name.into();
        let short: SharedString = extract_text_initials(&name).into();

        self.name = Some(name);
        self.short_name = short;
        self
    }

    /// Set placeholder icon, default: [`IconName::User`]
    pub fn placeholder(mut self, icon: impl Into<Icon>) -> Self {
        self.placeholder = icon.into();
        self
    }

    /// Keys the avatar's clock. By default, the place in the source where it was made and its name.
    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// The clock's key: the id, or where the avatar was made and its name.
    fn key(&self) -> ElementId {
        match (&self.id, &self.name) {
            (Some(id), _) => id.clone(),
            (None, Some(name)) => (ElementId::from(self.location), name.clone()).into(),
            (None, None) => self.location.into(),
        }
    }

    /// A steady ring outside the avatar and a copy that pulses outward, in `color`.
    pub fn ring(mut self, color: impl Into<Hsla>) -> Self {
        self.ring = Some(Some(color.into()));
        self
    }

    /// The pulsing ring in Kirakira pink.
    pub fn ring_pulse(mut self) -> Self {
        self.ring = Some(None);
        self
    }
}

impl Sizable for Avatar {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Avatar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl InteractiveElement for Avatar {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

/// The avatar's width in pixels: its style's, when that is absolute, or its size's.
fn resolved_size(style: &StyleRefinement, size: Size, rem: Pixels) -> Pixels {
    match style.size.width {
        Some(Length::Definite(DefiniteLength::Absolute(length))) => length.to_pixels(rem),
        _ => avatar_size(size),
    }
}

impl RenderOnce for Avatar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let key = self.key();
        let clock = Clock::new((key.clone(), "kk-pop"), Trigger::Mount, window, cx);
        let picture = self.src.is_some();
        let (scale, opacity) = if picture {
            clock.animate(Some(ms(400)), window);
            let timing = Timing::new(ms(400));
            let (scale, fade) = pop_tracks();
            (clock.sample(&scale, &timing), clock.sample(&fade, &timing))
        } else {
            clock.animate(Some(ms(300)), window);
            let fade = fade_in_track();
            (1.0, clock.sample(&fade, &Timing::new(ms(300))))
        };

        let identity = self
            .name
            .is_some()
            .then(|| IdentityColor::new(&self.short_name, cx));

        // The tinted outline belongs to the initials. An image, or the anonymous
        // placeholder, keeps the neutral border.
        let border_color = match (identity, &self.src) {
            (Some(identity), None) => identity.border,
            _ => cx.theme().border,
        };

        // Layout stays on the outer box; the disc inside it is drawn at the pop's size.
        let mut style = self.style;
        let inner_style = StyleRefinement {
            corner_radii: style.corner_radii.clone(),
            ..Default::default()
        };
        let disc_style = StyleRefinement {
            background: style.background.take(),
            border_color: style.border_color.take(),
            border_widths: std::mem::take(&mut style.border_widths),
            text: std::mem::take(&mut style.text),
            corner_radii: style.corner_radii.clone(),
            ..Default::default()
        };
        let outer_size = resolved_size(&style, self.size, window.rem_size());

        let fallback = AvatarFallback::new()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .rounded_full_style(cx)
            .overflow_hidden()
            .when_none(&identity, |this| {
                this.text_size(avatar_size(self.size) * 0.6)
                    .child(self.placeholder)
            })
            .when_some(identity, |this, identity| {
                this.bg(identity.background)
                    .text_color(identity.foreground)
                    .child(avatar_text_size(div(), self.size).child(self.short_name))
            })
            .refine_style(&inner_style);

        let disc = self
            .base
            .absolute()
            .left(relative((1.0 - scale) * 0.5))
            .top(relative((1.0 - scale) * 0.5))
            .w(relative(scale))
            .h(relative(scale))
            .rounded_full_style(cx)
            .overflow_hidden()
            .bg(cx.theme().tokens.secondary)
            .text_color(cx.theme().background)
            .border_1()
            .border_color(border_color)
            .opacity(opacity)
            .fallback(fallback)
            .when_some(self.src, |this, src| {
                this.image(
                    AvatarImage::new(src)
                        .size_full()
                        .rounded_full_style(cx)
                        .refine_style(&inner_style),
                )
            })
            .refine_style(&disc_style);

        let radii = style.corner_radii.clone();
        let ring = self.ring.map(|color| color.unwrap_or(cx.kira().pink));
        let beacon = ring.filter(|_| !clock.reduced()).map(|color| {
            clock.animate(None, window);
            let timing = Timing::new(ms(1600)).iterations(IterationCount::Infinite);
            let (scale, fade) = beacon_tracks();
            (
                color,
                clock.sample(&scale, &timing),
                clock.sample(&fade, &timing),
            )
        });
        let ring_box = outer_size + px(2. * (RING_GAP + RING_WIDTH));
        let ring_at = |scale: f32, color: Hsla| {
            let size = ring_box * scale;
            let inset = (outer_size - size) * 0.5;
            div()
                .absolute()
                .left(inset)
                .top(inset)
                .size(size)
                .border(px(RING_WIDTH) * scale)
                .border_color(color)
                .rounded_full_style(cx)
                .map(|mut this| {
                    this.style().corner_radii.refine(&radii);
                    this
                })
        };

        div()
            .relative()
            .size(avatar_size(self.size))
            .flex_shrink_0()
            .refine_style(&style)
            .child(disc)
            .when_some(ring, |this, color| this.child(ring_at(1.0, color)))
            .when_some(beacon, |this, (color, scale, alpha)| {
                this.child(ring_at(scale, color.opacity(alpha)))
            })
    }
}

/// A grouped avatars to display in a compact layout.
#[derive(IntoElement)]
pub struct AvatarGroup {
    base: Div,
    style: StyleRefinement,
    avatars: Vec<Avatar>,
    size: Size,
    limit: usize,
    ellipsis: bool,
    location: &'static Location<'static>,
}

impl Default for AvatarGroup {
    #[track_caller]
    fn default() -> Self {
        Self::new()
    }
}

impl AvatarGroup {
    /// Create a new AvatarGroup.
    #[track_caller]
    pub fn new() -> Self {
        Self {
            location: Location::caller(),
            base: div(),
            style: StyleRefinement::default(),
            avatars: Vec::new(),
            size: Size::default(),
            limit: 3,
            ellipsis: false,
        }
    }

    /// Add a child avatar to the group.
    pub fn child(mut self, avatar: Avatar) -> Self {
        self.avatars.push(avatar);
        self
    }

    /// Add multiple child avatars to the group.
    pub fn children(mut self, avatars: impl IntoIterator<Item = Avatar>) -> Self {
        self.avatars.extend(avatars);
        self
    }

    /// Set the maximum number of avatars to display before showing a "more" avatar.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set whether to show an ellipsis when the limit is reached, default: false
    pub fn ellipsis(mut self) -> Self {
        self.ellipsis = true;
        self
    }
}

impl Sizable for AvatarGroup {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for AvatarGroup {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl InteractiveElement for AvatarGroup {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl RenderOnce for AvatarGroup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let item_ml = -avatar_size(self.size) * 0.3;
        let avatars_len = self.avatars.len();
        let group = ElementId::from(self.location);

        self.base
            .h_flex()
            .flex_row_reverse()
            .refine_style(&self.style)
            .children(if self.ellipsis && avatars_len > self.limit {
                Some(
                    Avatar::new()
                        .id((group.clone(), "kk-more"))
                        .name("⋯")
                        .bg(cx.theme().tokens.secondary)
                        .text_color(cx.theme().muted_foreground)
                        .with_size(self.size)
                        .ml_1(),
                )
            } else {
                None
            })
            .children(
                self.avatars
                    .into_iter()
                    .take(self.limit)
                    .enumerate()
                    .rev()
                    .map(|(ix, item)| {
                        let item = match item.id {
                            Some(_) => item,
                            // Each avatar in the group keeps its own clock, apart from those of
                            // other groups.
                            None => {
                                let id = (group.clone(), format!("{ix}:{}", item.key()));
                                item.id(id)
                            }
                        };
                        item.with_size(self.size)
                            .when(ix > 0, |this| this.ml(item_ml))
                    }),
            )
    }
}

/// The colors a name-based fallback draws itself in, picked from the initials so
/// the same person always gets the same ones. GPUI Component's, unchanged.
#[derive(Debug, Clone, Copy, PartialEq)]
struct IdentityColor {
    background: Hsla,
    foreground: Hsla,
    border: Hsla,
}

impl IdentityColor {
    const HUES: u64 = 12;
    const HUE_STEP: f32 = 360. / Self::HUES as f32;

    fn new(short_name: &SharedString, cx: &App) -> Self {
        let hue = (gpui_kit::hash(short_name) % Self::HUES) as f32 * Self::HUE_STEP;
        Self::from_hue(hue, cx.theme().is_dark())
    }

    fn from_hue(hue: f32, is_dark: bool) -> Self {
        let (background, foreground, border) = if is_dark {
            (
                oklch(0.30, 0.05, hue),
                oklch(0.82, 0.11, hue),
                oklch(0.36, 0.06, hue),
            )
        } else {
            (
                oklch(0.97, 0.032, hue),
                oklch(0.50, 0.145, hue),
                oklch(0.89, 0.05, hue),
            )
        };

        Self {
            background,
            foreground,
            border,
        }
    }
}

fn extract_text_initials(text: &str) -> String {
    let mut result = text
        .split(" ")
        .flat_map(|word| word.chars().next().map(|c| c.to_string()))
        .take(2)
        .collect::<Vec<String>>()
        .join("");

    if result.len() == 1 {
        result = text.chars().take(2).collect::<String>();
    }

    result.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_match_gpui_component() {
        assert_eq!(extract_text_initials("Jason Lee"), "JL");
        assert_eq!(extract_text_initials("Foo Bar Dar"), "FB");
        assert_eq!(extract_text_initials("huacnlee"), "HU");
    }

    #[test]
    fn sibling_avatars_get_their_own_keys() {
        let first = Avatar::new();
        let second = Avatar::new();
        assert_ne!(first.key(), second.key());
        let named = |name: &'static str| Avatar::new().name(name);
        assert_ne!(named("Mika").key(), named("Sora").key());
        assert_eq!(named("Mika").key(), named("Mika").key());
        assert_eq!(Avatar::new().id("me").key(), ElementId::from("me"));
        assert_ne!(AvatarGroup::new().location, AvatarGroup::new().location);
    }

    #[test]
    fn keyframes_match_the_web() {
        use crate::parity::assert_number_track;
        let (scale, fade) = pop_tracks();
        assert_number_track("pop-avatar", "kk-pop-avatar-pop", "sx", &scale);
        assert_number_track("pop-avatar", "kk-pop-avatar-pop", "opacity", &fade);
        assert_number_track(
            "pop-avatar",
            "kk-pop-avatar-fade",
            "opacity",
            &fade_in_track(),
        );
        let (scale, fade) = beacon_tracks();
        assert_number_track("pop-avatar", "kk-pop-avatar-pulse", "sx", &scale);
        assert_number_track("pop-avatar", "kk-pop-avatar-pulse", "opacity", &fade);
    }

    #[test]
    fn pop_overshoots_and_beacon_fades_by_80_percent() {
        let (scale, fade) = pop_tracks();
        assert_eq!(scale.sample(0.0), 0.6);
        assert_eq!(scale.sample(0.55), 1.06);
        assert_eq!(scale.sample(0.8), 0.98);
        assert_eq!(fade.sample(0.3), 1.0);
        let (scale, fade) = beacon_tracks();
        assert_eq!(scale.sample(0.8), 1.45);
        assert_eq!(fade.sample(0.0), 0.8);
        assert_eq!(fade.sample(0.9), 0.0);
    }
}
