//! Card: a shadcn-style card that lifts on hover and can pop in.
//!
//! GPUI Component has no card (its nearest piece is `group_box`), so this follows shadcn/ui's card:
//! [`Card`], [`CardHeader`], [`CardTitle`], [`CardDescription`], [`CardAction`], [`CardContent`]
//! and [`CardFooter`], drawn with GPUI Component's theme tokens and shadcn's spacing. GPUI
//! Component has no `card` token, so the surface is `popover` (a raised surface in both Kirakira
//! themes), with `border`, `muted_foreground` and `radius_lg`.
//!
//! - **Lift** (on by default): under the pointer, or while something inside it has focus (the
//!   web's `:focus-within`), the card rises 4 px and its shadow softens and spreads, in 0.16 s on
//!   `ease-out`.
//! - **Pop** (off by default, [`Card::pop`]): the card comes up from 16 px below at 75 % size,
//!   overshoots to 103 % 2 px high at 55 %, dips to 99 % at 80 % and rests, in 0.5 s on
//!   `ease-in-out`; it is opaque by 30 %. It waits until it scrolls into view by default.
//!
//! Under reduced motion the card neither lifts nor pops; the shadow still changes on hover.
//!
//! The pop is composition-safe: inside a [`Timeline`](crate::timeline::Timeline) it plays on the
//! timeline's clock. The lift follows the pointer, so it isn't.
//!
//! Differences from the web version: the pop scales evenly by drawing the card at a scaled rem
//! size (see [`crate::motion::Transform`]), so its rem-based padding, gaps and text follow while
//! its border, corner radius and shadow keep their pixel sizes. A lifting card tracks a focus
//! handle to see focus inside it, so a click on its bare surface focuses the card itself (where a
//! web page's focus would go to the body); that focus doesn't lift it.

use gpui_kit::base::{StyledExt as _, h_flex};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, BoxShadow, Div, ElementId, FontWeight, InteractiveElement as _, IntoElement,
    ParentElement, RenderOnce, StatefulInteractiveElement as _, StyleRefinement, Styled, Window,
    div, hsla, px, relative,
};

use crate::motion::{
    Clock, Easing, Keyframes, Pose, Timing, Track, Trigger, delay_ms, ms, split_layout, transform,
};
use crate::state_motion::glide;

/// `kk-pop-card-pop`: the card's motion (`y` in pixels) and its opacity.
pub(crate) fn pop_tracks() -> (Keyframes<Pose>, Keyframes<f32>) {
    let ease = Easing::EaseInOut;
    (
        Track::new(ease.clone())
            .at(0.0, Pose::new().y(16.0).scale(0.75))
            .at(0.55, Pose::new().y(-2.0).scale(1.03))
            .at(0.8, Pose::new().scale(0.99))
            .at(1.0, Pose::new())
            .build(),
        Track::new(ease)
            .at(0.0, 0.0)
            .at(0.3, 1.0)
            .at(1.0, 1.0)
            .build(),
    )
}

/// The resting shadow and the lifted one, with CSS blur radii halved for GPUI, whose shader takes
/// the blur as the gaussian's deviation where CSS takes twice it.
fn shadow(lift: f32) -> Vec<BoxShadow> {
    let mix = |a: f32, b: f32| a + (b - a) * lift;
    let layer = |y: f32, blur: f32, spread: f32, alpha: f32| {
        BoxShadow::new(px(0.), px(y), hsla(0., 0., 0., alpha))
            .blur_radius(px(blur))
            .spread_radius(px(spread))
    };
    vec![
        // 0 1px 3px 0 / .1  →  0 14px 28px -12px / .2
        layer(mix(1., 14.), mix(1.5, 14.), mix(0., -12.), mix(0.1, 0.2)),
        // 0 1px 2px -1px / .1  →  0 3px 8px -3px / .06
        layer(mix(1., 3.), mix(1., 4.), mix(-1., -3.), mix(0.1, 0.06)),
    ]
}

struct Hovered(bool);

/// A card: a bordered, rounded surface for one subject.
#[derive(IntoElement)]
pub struct Card {
    id: ElementId,
    base: Div,
    lift: bool,
    pop: bool,
    trigger: Trigger,
    delay: u64,
    duration: u64,
}

impl Card {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            lift: true,
            pop: false,
            trigger: Trigger::InView,
            delay: 0,
            duration: 500,
        }
    }

    /// Lift a few pixels with a softer shadow on hover or focus inside. On by default.
    pub fn lift(mut self, lift: bool) -> Self {
        self.lift = lift;
        self
    }

    /// Pop in with an overshoot when it mounts or scrolls into view. Off by default.
    pub fn pop(mut self, pop: bool) -> Self {
        self.pop = pop;
        self
    }

    /// When the pop plays. [`Trigger::InView`] by default.
    pub fn trigger(mut self, trigger: Trigger) -> Self {
        self.trigger = trigger;
        self
    }

    /// Milliseconds before the pop, to stagger a grid. 0 by default.
    pub fn delay(mut self, delay_ms: u64) -> Self {
        self.delay = delay_ms;
        self
    }

    /// Milliseconds for the pop. 500 by default.
    pub fn duration(mut self, duration_ms: u64) -> Self {
        self.duration = duration_ms;
        self
    }
}

impl Styled for Card {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Card {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for Card {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id.clone();
        let hovered = window.use_keyed_state((id.clone(), "kk-hover"), cx, |_, _| Hovered(false));
        let focus = self.lift.then(|| {
            window
                .use_keyed_state((id.clone(), "kk-focus"), cx, |_, cx| cx.focus_handle())
                .read(cx)
                .clone()
        });
        // `:focus-within`: something inside has focus. The card itself takes focus when clicked
        // (where a web page's would go to the body), so that doesn't count.
        let focus_within = focus
            .as_ref()
            .is_some_and(|focus| focus.contains_focused(window, cx) && !focus.is_focused(window));
        let lifted = self.lift && (hovered.read(cx).0 || focus_within);
        let lift = glide(
            (id.clone(), "kk-lift"),
            if lifted { 1.0_f32 } else { 0.0 },
            ms(160),
            Easing::EaseOut,
            window,
            cx,
        );
        // Reduced motion keeps the shadow change but not the rise.
        let rise = if cx.reduce_motion() { 0.0 } else { -4.0 * lift };

        let mut pose = Pose::new();
        let clock = self.pop.then(|| {
            let clock = Clock::new((id.clone(), "kk-pop"), self.trigger, window, cx);
            clock.animate(Some(ms(self.delay + self.duration)), window);
            let timing = Timing::new(ms(self.duration)).delay(delay_ms(self.delay));
            let (motion, fade) = pop_tracks();
            pose = clock.sample(&motion, &timing);
            pose.opacity = clock.sample(&fade, &timing);
            clock
        });
        pose.y += rise;

        let theme = cx.theme();
        let mut base = self.base;
        let outer = split_layout(base.style());
        // The card's own look goes first and the user's style last, so the user's wins.
        let user = std::mem::take(base.style());
        let card = base
            .id((id.clone(), "kk-card"))
            .flex()
            .flex_col()
            .gap_6()
            .py_6()
            .rounded(theme.radius_lg)
            .border_1()
            .border_color(theme.border)
            .bg(theme.popover)
            .text_color(theme.popover_foreground)
            .map(|mut this| {
                this.style().box_shadow = Some(shadow(lift));
                this
            })
            .refine_style(&user)
            .when_some(focus, |this, focus| this.track_focus(&focus))
            .when(self.lift, |this| {
                this.on_hover(move |hover, _, cx| {
                    hovered.update(cx, |hovered, cx| {
                        hovered.0 = *hover;
                        cx.notify();
                    })
                })
            });
        let card = match &clock {
            Some(clock) => clock.observe(card),
            None => card,
        };
        transform(id, pose, card).outer_style(outer)
    }
}

macro_rules! part {
    ($(#[$doc:meta])* $name:ident, |$this:ident, $cx:ident| $build:expr) => {
        $(#[$doc])*
        #[derive(IntoElement)]
        pub struct $name {
            style: StyleRefinement,
            children: Vec<AnyElement>,
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $name {
            pub fn new() -> Self {
                Self {
                    style: StyleRefinement::default(),
                    children: Vec::new(),
                }
            }
        }

        impl Styled for $name {
            fn style(&mut self) -> &mut StyleRefinement {
                &mut self.style
            }
        }

        impl ParentElement for $name {
            fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
                self.children.extend(elements);
            }
        }

        impl RenderOnce for $name {
            #[allow(unused_variables)]
            fn render(self, _: &mut Window, $cx: &mut App) -> impl IntoElement {
                let $this = div();
                let built: Div = $build;
                built.refine_style(&self.style).children(self.children)
            }
        }
    };
}

part!(
    /// The title, the description and an optional [`CardAction`], with the card's side padding.
    /// Put the action in with [`CardHeader::action`] so it sits at the top right.
    CardHeaderBody,
    |this, _cx| this.flex().flex_col().gap_2().flex_1().min_w_0()
);

part!(
    /// The card's title: semibold, tight leading.
    CardTitle,
    |this, _cx| this.font_weight(FontWeight::SEMIBOLD).line_height(relative(1.))
);

part!(
    /// A line of muted text under the title.
    CardDescription,
    |this, cx| this.text_sm().text_color(cx.theme().muted_foreground)
);

part!(
    /// Something at the top right of the header: a badge, a button.
    CardAction,
    |this, _cx| this.flex_none().self_start()
);

part!(
    /// The card's body, with its side padding.
    CardContent,
    |this, _cx| this.px_6()
);

part!(
    /// A row along the bottom of the card, with its side padding.
    CardFooter,
    |this, _cx| this.flex().items_center().px_6()
);

/// The top of a card: a title, a description and an optional action at the top right.
#[derive(IntoElement)]
pub struct CardHeader {
    body: CardHeaderBody,
    action: Option<CardAction>,
}

impl Default for CardHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl CardHeader {
    pub fn new() -> Self {
        Self {
            body: CardHeaderBody::new(),
            action: None,
        }
    }

    /// The header's action, at the top right beside the title and description.
    pub fn action(mut self, action: CardAction) -> Self {
        self.action = Some(action);
        self
    }
}

impl Styled for CardHeader {
    fn style(&mut self) -> &mut StyleRefinement {
        self.body.style()
    }
}

impl ParentElement for CardHeader {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.body.extend(elements);
    }
}

impl RenderOnce for CardHeader {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        h_flex()
            .items_start()
            .gap_2()
            .px_6()
            .child(self.body)
            .children(self.action)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Host {
        inner: gpui_kit::FocusHandle,
    }

    impl gpui_kit::Render for Host {
        fn render(&mut self, _: &mut Window, _: &mut gpui_kit::Context<Self>) -> impl IntoElement {
            div().size_full().p_16().child(
                Card::new("card").child(
                    div()
                        .id("inner")
                        .track_focus(&self.inner)
                        .debug_selector(|| "inner".into())
                        .size(gpui_kit::px(20.)),
                ),
            )
        }
    }

    /// Focus inside the card lifts it, as the web's `:focus-within` does.
    #[gpui_kit::test]
    fn focus_inside_lifts_the_card(cx: &mut gpui_kit::TestAppContext) {
        use std::time::Instant;
        cx.update(|cx| {
            gpui_kit::component::init(cx);
            crate::init(cx);
        });
        let start = Instant::now();
        crate::motion::freeze_time(Some(start));
        let (host, cx) = cx.add_window_view(|_, cx| Host {
            inner: cx.focus_handle(),
        });
        let top = |cx: &mut gpui_kit::VisualTestContext| {
            cx.update(|window, cx| window.draw(cx).clear(cx));
            cx.debug_bounds("inner").expect("drawn").top()
        };
        let rest = top(cx);
        cx.update(|window, cx| {
            let inner = host.read(cx).inner.clone();
            window.focus(&inner, cx);
        });
        top(cx);
        crate::motion::freeze_time(Some(start + ms(1000)));
        let lifted = top(cx);
        crate::motion::freeze_time(None);
        assert_eq!(lifted, rest - gpui_kit::px(4.));
    }

    #[test]
    fn pop_overshoots_and_rests() {
        let (motion, fade) = pop_tracks();
        let start = motion.sample(0.0);
        assert_eq!((start.y, start.sx, start.opacity), (16.0, 0.75, 1.0));
        assert_eq!(fade.sample(0.0), 0.0);
        assert_eq!(fade.sample(0.3), 1.0);
        let peak = motion.sample(0.55);
        assert_eq!((peak.y, peak.sx), (-2.0, 1.03));
        assert_eq!(motion.sample(1.0), Pose::new());
    }

    #[test]
    fn keyframes_match_the_web() {
        use crate::parity::{assert_number_track, assert_pose_track};
        let (motion, fade) = pop_tracks();
        assert_pose_track("pop-card", "kk-pop-card-pop", &motion, &[]);
        assert_number_track("pop-card", "kk-pop-card-pop", "opacity", &fade);
    }

    #[test]
    fn shadow_lifts() {
        let rest = shadow(0.0);
        let lifted = shadow(1.0);
        assert_eq!(rest[0].offset.y, px(1.));
        assert_eq!(lifted[0].offset.y, px(14.));
        assert_eq!(lifted[0].spread_radius, px(-12.));
    }
}
