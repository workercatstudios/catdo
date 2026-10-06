//! Sparkles: four-point stars that twinkle around their content.
//!
//! Stars sit on an ellipse just outside the content, scattered by golden-angle steps, and each
//! twinkles in turn: it grows and turns in, then shrinks and turns out, and rests for the rest of
//! its cycle. Under reduced motion there are no stars.

use gpui_kit::base::StyledExt as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, ElementId, Hsla, InteractiveElement as _, IntoElement, ParentElement, Pixels,
    RenderOnce, StatefulInteractiveElement as _, StyleRefinement, Styled, Transformation, Window,
    div, radians, relative, size,
};

use crate::bounce_text::font_size;
use crate::icons::{self, icon};
use crate::motion::{
    Clock, Easing, IterationCount, Pose, Pulse, Timing, Track, Trigger, delay_ms, hash, ms,
};
use crate::theme::ActiveKira as _;

/// When the stars twinkle.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SparklesTrigger {
    /// All the time.
    #[default]
    Loop,
    /// Only while hovered.
    Hover,
}

/// Twinkling stars around their children.
#[derive(IntoElement)]
pub struct Sparkles {
    id: ElementId,
    count: usize,
    colors: Option<Vec<Hsla>>,
    size: Option<Pixels>,
    trigger: SparklesTrigger,
    duration: u64,
    stagger: u64,
    style: StyleRefinement,
    children: Vec<AnyElement>,
}

impl Sparkles {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            count: 6,
            colors: None,
            size: None,
            trigger: SparklesTrigger::Loop,
            duration: 600,
            stagger: 200,
            style: StyleRefinement::default(),
            children: Vec::new(),
        }
    }

    /// How many stars. 6 by default.
    pub fn count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }

    /// Star colours, cycled in order. Kirakira yellow and pink by default.
    pub fn colors(mut self, colors: impl IntoIterator<Item = impl Into<Hsla>>) -> Self {
        self.colors = Some(colors.into_iter().map(Into::into).collect());
        self
    }

    /// Size of the largest star. Defaults to 0.75 of the text size, so stars follow the text.
    pub fn star_size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }

    pub fn trigger(mut self, trigger: SparklesTrigger) -> Self {
        self.trigger = trigger;
        self
    }

    /// Length of one twinkle in ms. 600 by default.
    pub fn duration(mut self, duration_ms: u64) -> Self {
        self.duration = duration_ms;
        self
    }

    /// Time between one star's twinkle and the next in ms. 200 by default.
    pub fn stagger(mut self, stagger_ms: u64) -> Self {
        self.stagger = stagger_ms;
        self
    }
}

impl Styled for Sparkles {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for Sparkles {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

#[derive(Default)]
struct Hovered(bool);

/// The twinkle takes the first 40% of the cycle; the rest is a pause before the next one.
fn twinkle() -> crate::motion::Keyframes<Pose> {
    Track::new(Easing::EaseInOut)
        .at(0.0, Pose::new().scale(0.0).rotate(-30.0))
        .at(0.2, Pose::new())
        .at_each(&[0.4, 1.0], Pose::new().scale(0.0).rotate(30.0))
        .build()
}

impl RenderOnce for Sparkles {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // A loop runs on the clock, so it follows a timeline; hovering restarts the twinkle from
        // its first star each time, like the web version's `animation: none` until hover.
        let clock = Clock::new(self.id.clone(), Trigger::Mount, window, cx);
        let hover_start = Pulse::new((self.id.clone(), "kk-hover"), window, cx);
        let hovered = window.use_keyed_state((self.id.clone(), "kk-hovered"), cx, |_, _| {
            Hovered::default()
        });
        let elapsed = match self.trigger {
            SparklesTrigger::Loop => (!clock.reduced()).then(|| clock.elapsed()),
            SparklesTrigger::Hover if hovered.read(cx).0 => hover_start.elapsed(),
            SparklesTrigger::Hover => None,
        };
        let palette = cx.kira();
        let colors = self
            .colors
            .filter(|colors| !colors.is_empty())
            .unwrap_or_else(|| vec![palette.yellow, palette.pink]);
        let largest = self
            .size
            .unwrap_or_else(|| font_size(&self.style, window) * 0.75);

        let twinkle = twinkle();
        let cycle = ms(self.duration * 5 / 2);

        match self.trigger {
            SparklesTrigger::Loop => clock.animate(None, window),
            SparklesTrigger::Hover if elapsed.is_some() => window.request_animation_frame(),
            SparklesTrigger::Hover => {}
        }

        let stars = elapsed
            .into_iter()
            .flat_map(|elapsed| (0..self.count).map(move |i| (i, elapsed)))
            .map(|(i, elapsed)| {
                // Golden-angle steps scatter the stars evenly around an ellipse just outside the
                // content.
                let angle = (i as f32 * 137.508 - 30.0).to_radians();
                let rx = 48.0 + 14.0 * hash(i, 0.618_034);
                let ry = 58.0 + 30.0 * hash(i, 0.754_878);
                let x = (50.0 + angle.cos() * rx) / 100.0;
                let y = (50.0 + angle.sin() * ry) / 100.0;
                let star = largest * (0.55 + 0.45 * hash(i, 0.414_214));
                let timing = Timing::new(cycle)
                    .delay(delay_ms(i as u64 * self.stagger))
                    .iterations(IterationCount::Infinite);
                let pose = twinkle.sample(timing.sample(elapsed).directed_progress);
                icon(icons::SPARKLE)
                    .absolute()
                    .left(relative(x))
                    .top(relative(y))
                    .ml(star * -0.5)
                    .mt(star * -0.5)
                    .size(star)
                    .text_color(colors[i % colors.len()])
                    .with_transformation(
                        Transformation::scale(size(pose.sx, pose.sy))
                            .with_rotation(radians(pose.rotate.to_radians())),
                    )
            });

        let hover = hovered.clone();
        div()
            .id(self.id)
            .relative()
            .flex_none()
            .refine_style(&self.style)
            .when(self.trigger == SparklesTrigger::Hover, |this| {
                this.on_hover(move |hovering, _, cx| {
                    if *hovering {
                        hover_start.fire(cx);
                    }
                    hover.update(cx, |hovered, cx| {
                        hovered.0 = *hovering;
                        cx.notify();
                    });
                })
            })
            .children(self.children)
            .child(div().absolute().inset_0().children(stars))
    }
}

#[cfg(test)]
mod tests {
    use crate::parity::assert_pose_track;

    #[test]
    fn twinkle_matches_the_web() {
        assert_pose_track("sparkles", "kk-sparkles-twinkle", &super::twinkle(), &[]);
    }
}
