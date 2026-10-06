//! Bounce Text: letters that rise, pop or drop in one after another.
//!
//! - `Rise` comes up out of the baseline, overshoots and settles, clipped below the row.
//! - `Pop` grows each letter from nothing past full size and back.
//! - `Drop` falls each letter from above and bounces it on the line twice.
//!
//! Composition-safe: inside a [`Timeline`](crate::timeline::Timeline) it plays on the timeline's
//! clock. Letters squash and stretch exactly as on the web: a moving letter is drawn as a
//! [vector glyph](crate::vector), and a letter at rest is GPUI's own text.

use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AbsoluteLength, App, ElementId, InteractiveElement as _, IntoElement, ParentElement as _,
    Pixels, RenderOnce, Role, SharedString, StatefulInteractiveElement as _, StyleRefinement,
    Styled, Window, div,
};

use crate::draw::{Inset, Side, clip};
use crate::motion::{Clock, Easing, Keyframes, Pose, Timing, Track, Trigger, delay_ms, ms};
use crate::text::{Order, Part, split, steps};
use crate::vector::{TextSetting, Vector};

/// How the letters arrive.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BounceVariant {
    /// Up out of the baseline with an overshoot.
    #[default]
    Rise,
    /// Grow from nothing past full size and back.
    Pop,
    /// Fall from above and bounce twice.
    Drop,
}

impl BounceVariant {
    /// The default length of one letter's animation.
    pub fn duration_ms(self) -> u64 {
        match self {
            Self::Rise => 550,
            Self::Pop => 600,
            Self::Drop => 800,
        }
    }

    /// Where letters scale from, as fractions of their box.
    fn origin(self) -> (f32, f32) {
        match self {
            Self::Pop => (0.5, 0.6),
            _ => (0.5, 1.0),
        }
    }

    /// The letter's motion and its opacity, as two tracks like the CSS keyframes.
    fn tracks(self) -> (Keyframes<Pose>, Keyframes<f32>) {
        let ease = Easing::EaseInOut;
        match self {
            Self::Rise => (
                Track::new(ease.clone())
                    .at(0.0, Pose::new().yp(1.0))
                    .at(0.6, Pose::new().yp(-0.08).scale_xy(0.95, 1.15))
                    .at(0.8, Pose::new().scale_xy(1.04, 0.96))
                    .at(1.0, Pose::new())
                    .build(),
                Track::new(Easing::Linear).at(0.0, 1.0).at(1.0, 1.0).build(),
            ),
            Self::Pop => (
                Track::new(ease.clone())
                    .at(0.0, Pose::new().scale(0.0))
                    .at(0.5, Pose::new().scale_xy(1.2, 1.25))
                    .at(0.75, Pose::new().scale_xy(0.9, 0.95))
                    .at(1.0, Pose::new())
                    .build(),
                Track::new(ease.clone())
                    .at(0.0, 0.0)
                    .at(0.2, 1.0)
                    .at(1.0, 1.0)
                    .build(),
            ),
            Self::Drop => {
                let fall = Easing::cubic_bezier(0.6, 0.0, 1.0, 0.7).expect("valid curve");
                let land = Easing::cubic_bezier(0.0, 0.4, 0.4, 1.0).expect("valid curve");
                (
                    Track::new(ease.clone())
                        .at_ease(0.0, Pose::new().yp(-1.2).scale_xy(0.9, 1.15), fall.clone())
                        .at_ease(0.45, Pose::new().scale_xy(1.18, 0.82), land.clone())
                        .at_ease(
                            0.62,
                            Pose::new().yp(-0.22).scale_xy(0.96, 1.05),
                            fall.clone(),
                        )
                        .at_ease(0.77, Pose::new().scale_xy(1.06, 0.94), land)
                        .at_ease(0.88, Pose::new().yp(-0.08), fall)
                        .at(1.0, Pose::new())
                        .build(),
                    Track::new(ease.clone())
                        .at(0.0, 0.0)
                        .at(0.15, 1.0)
                        .at(1.0, 1.0)
                        .build(),
                )
            }
        }
    }
}

/// Splits text into letters that bounce in one after another.
///
/// ```ignore
/// BounceText::new("title", "Kirakira!").variant(BounceVariant::Drop).text_3xl()
/// ```
#[derive(IntoElement)]
pub struct BounceText {
    id: ElementId,
    text: SharedString,
    variant: BounceVariant,
    order: Order,
    stagger: u64,
    duration: Option<u64>,
    delay: u64,
    trigger: Trigger,
    style: StyleRefinement,
}

impl BounceText {
    pub fn new(id: impl Into<ElementId>, text: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            text: text.into(),
            variant: BounceVariant::default(),
            order: Order::default(),
            stagger: 80,
            duration: None,
            delay: 200,
            trigger: Trigger::InView,
            style: StyleRefinement::default(),
        }
    }

    pub fn variant(mut self, variant: BounceVariant) -> Self {
        self.variant = variant;
        self
    }

    /// The order letters start in.
    pub fn order(mut self, order: Order) -> Self {
        self.order = order;
        self
    }

    /// Milliseconds between letters. 80 by default.
    pub fn stagger(mut self, stagger_ms: u64) -> Self {
        self.stagger = stagger_ms;
        self
    }

    /// Milliseconds per letter. 550 for rise, 600 for pop and 800 for drop by default.
    pub fn duration(mut self, duration_ms: u64) -> Self {
        self.duration = Some(duration_ms);
        self
    }

    /// Milliseconds before the first letter. 200 by default.
    pub fn delay(mut self, delay_ms: u64) -> Self {
        self.delay = delay_ms;
        self
    }

    /// When the letters start. [`Trigger::InView`] by default.
    pub fn trigger(mut self, trigger: Trigger) -> Self {
        self.trigger = trigger;
        self
    }
}

impl Styled for BounceText {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// The font size `style` sets, or the inherited one.
pub(crate) fn font_size(style: &StyleRefinement, window: &Window) -> Pixels {
    let rem = window.rem_size();
    style
        .text
        .font_size
        .map(|size: AbsoluteLength| size.to_pixels(rem))
        .unwrap_or_else(|| window.text_style().font_size.to_pixels(rem))
}

impl RenderOnce for BounceText {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let clock = Clock::new(self.id.clone(), self.trigger, window, cx);
        let parts = split(&self.text);
        let count = parts
            .iter()
            .map(|part| match part {
                Part::Word(letters) => letters.len(),
                Part::Space(_) => 0,
            })
            .sum();
        let step = steps(count, self.order);
        let duration = self.duration.unwrap_or(self.variant.duration_ms());
        let last_step = step.iter().copied().max().unwrap_or(0) as u64;
        clock.animate(
            Some(ms(self.delay + last_step * self.stagger + duration)),
            window,
        );

        let (motion, fade) = self.variant.tracks();
        let (origin_x, origin_y) = self.variant.origin();
        let rise = self.variant == BounceVariant::Rise;
        let size = font_size(&self.style, window);
        let setting = TextSetting::new(&self.style.text, window);

        let mut letter = 0;
        let mut words = Vec::with_capacity(parts.len());
        for part in parts {
            match part {
                Part::Space(space) => {
                    words.push(div().whitespace_nowrap().child(space).into_any_element())
                }
                Part::Word(letters) => {
                    let mut word = div().flex().flex_none().whitespace_nowrap();
                    for glyph in letters {
                        let timing = Timing::new(ms(duration))
                            .delay(delay_ms(self.delay + step[letter] as u64 * self.stagger));
                        let progress = timing.sample(clock.elapsed()).directed_progress;
                        let pose = Pose {
                            opacity: fade.sample(progress),
                            ..motion.sample(progress)
                        };
                        let moving = !pose.is_identity();
                        let line_box = setting.line_box(&glyph, window);
                        let vector = moving.then(|| {
                            div().absolute().top_0().left_0().child(
                                Vector::new(line_box.size)
                                    .layer(setting.layer(&glyph, line_box))
                                    .pose(pose)
                                    .origin(origin_x, origin_y),
                            )
                        });
                        word = word.child(
                            div()
                                .relative()
                                .flex_none()
                                // The real glyph holds the layout, and shows at rest.
                                .child(div().when(moving, |this| this.invisible()).child(glyph))
                                .children(vector),
                        );
                        letter += 1;
                    }
                    // Rise comes up out of the baseline: clip below the row only, with room
                    // above and beside the word for the overshoot. GPUI's line box already holds
                    // the descenders. A paint-time clip, not padding and negative margins:
                    // Taffy 0.13 scales a `flex_none` item's negative margin by the item's width
                    // when it sizes a flex row to its content, so a shrink-to-fit Bounce Text
                    // would collapse to its widest word and wrap.
                    words.push(if rise {
                        let side = Side::px(f32::from(size) * -0.5);
                        clip(
                            Inset::new(Side::px(-f32::from(size)), side, Side::EDGE, side),
                            word,
                        )
                        .into_any_element()
                    } else {
                        word.into_any_element()
                    });
                }
            }
        }

        clock.observe(
            div()
                .id(self.id)
                .role(Role::Label)
                .aria_label(self.text)
                .flex()
                .flex_wrap()
                .items_end()
                .refine_style(&self.style)
                .children(words),
        )
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Bounds, px};

    use super::*;
    use crate::parity::{assert_number_track, assert_pose_track};

    /// A Bounce Text centred in the window, in a box that shrinks to fit it unless `width` sets
    /// one.
    struct Host {
        variant: BounceVariant,
        width: Option<Pixels>,
    }

    impl gpui_kit::Render for Host {
        fn render(&mut self, _: &mut Window, _: &mut gpui_kit::Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .debug_selector(|| "probe".into())
                        .flex_none()
                        .when_some(self.width, |this, width| this.w(width))
                        .child(
                            BounceText::new("text", "for GPUI")
                                .variant(self.variant)
                                .trigger(Trigger::Mount)
                                .text_size(px(20.)),
                        ),
                )
        }
    }

    fn laid_out(
        cx: &mut gpui_kit::TestAppContext,
        variant: BounceVariant,
        width: Option<Pixels>,
    ) -> Bounds<Pixels> {
        let (_, cx) = cx.add_window_view(|_, _| Host { variant, width });
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.debug_bounds("probe").expect("the probe is drawn")
    }

    /// Split text sizes like inline text: its whole line where it shrinks to fit, wrapping only
    /// where the box is narrower.
    #[gpui_kit::test]
    fn split_text_shrinks_to_its_whole_line(cx: &mut gpui_kit::TestAppContext) {
        cx.update(|cx| {
            gpui_kit::component::init(cx);
            crate::init(cx);
        });
        // The test text system sets every character 0.6 em wide: 8 × 12 px.
        let line = px(8. * 12.);
        for variant in [BounceVariant::Rise, BounceVariant::Pop, BounceVariant::Drop] {
            let fit = laid_out(cx, variant, None);
            assert_eq!(fit.size.width, line, "{variant:?} shrink-to-fit width");
            let narrow = laid_out(cx, variant, Some(px(60.)));
            assert_eq!(
                narrow.size.height,
                fit.size.height * 2.,
                "{variant:?} wraps onto two lines in a narrow box"
            );
        }
    }

    #[test]
    fn keyframes_match_the_web() {
        for (variant, name) in [
            (BounceVariant::Rise, "kk-bounce-text-rise"),
            (BounceVariant::Pop, "kk-bounce-text-pop"),
            (BounceVariant::Drop, "kk-bounce-text-drop"),
        ] {
            let (motion, fade) = variant.tracks();
            assert_pose_track("bounce-text", name, &motion, &[]);
            if variant != BounceVariant::Rise {
                assert_number_track("bounce-text", name, "opacity", &fade);
            }
        }
    }
}
