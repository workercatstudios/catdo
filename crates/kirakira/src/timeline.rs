//! One clock for a whole composition: [`Timeline`], [`Scene`], [`TimelineControls`] and [`Stage`].
//!
//! A timeline publishes its time to every [`Clock`](crate::motion::Clock) laid out inside it, so
//! the whole subtree is a pure function of time: it plays, pauses, scrubs backwards and loops.
//! Components that follow the composition rules in the crate docs are seekable this way; Pulse
//! driven controls (buttons, switches) run on real time and aren't.
//!
//! ```ignore
//! let timeline = cx.new(|_| TimelineState::new(ms(4000)));
//! Timeline::new(&timeline)
//!     .child(Scene::new(0).duration(2000).child(BounceText::new("title", "Hello")))
//!     .child(Scene::new(1600).child(PlateWipe::new("wipe")))
//! ```

use std::{rc::Rc, time::Duration, time::Instant};

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Bounds, Context, Div, ElementId, Entity, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement, Pixels, RenderOnce, SharedString, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Window, div, px, relative,
};

use crate::icons::{self, icon};
use crate::motion::{ScopeTime, rescaled, scope_time, scoped};

/// How long the Skip button stays up.
const SKIP_WINDOW: Duration = Duration::from_secs(5);

/// A timeline's clock. Keep it in an [`Entity`] and hand it to [`Timeline::new`] and
/// [`TimelineControls::new`].
pub struct TimelineState {
    duration: Duration,
    time: Duration,
    looping: bool,
    speed: f32,
    playing: bool,
    poster: Option<Duration>,
    skippable: bool,
    /// When a one-shot finished. From then on the timeline keeps free time so loops carry on.
    finished_at: Option<Instant>,
    last_tick: Option<Instant>,
    on_end: Option<Rc<dyn Fn(&mut App)>>,
}

impl TimelineState {
    /// A looping timeline of `duration` that plays as soon as it renders.
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            time: Duration::ZERO,
            looping: true,
            speed: 1.0,
            playing: true,
            poster: None,
            skippable: false,
            finished_at: None,
            last_tick: None,
            on_end: None,
        }
    }

    /// Start again from 0 at the end. On by default.
    pub fn looping(mut self, looping: bool) -> Self {
        self.looping = looping;
        self
    }

    /// Play on first render. On by default; off shows the [`poster`](Self::poster) frame.
    pub fn autoplay(mut self, autoplay: bool) -> Self {
        self.playing = autoplay;
        if !autoplay {
            self.time = self.poster.unwrap_or(self.duration);
        }
        self
    }

    /// Playback rate: 0.5 is half speed.
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed.max(0.0);
        self
    }

    /// The frame shown before playing and under reduced motion. Defaults to the last frame.
    pub fn poster(mut self, poster: Duration) -> Self {
        self.poster = Some(poster.min(self.duration));
        if !self.playing {
            self.time = poster.min(self.duration);
        }
        self
    }

    /// Show a Skip button for the first 5 s. Skipping jumps to the last frame.
    pub fn skippable(mut self, skippable: bool) -> Self {
        self.skippable = skippable;
        self
    }

    /// Called on stopping at the last frame: the end of a one-shot, or Skip.
    pub fn on_end(mut self, on_end: impl Fn(&mut App) + 'static) -> Self {
        self.on_end = Some(Rc::new(on_end));
        self
    }

    pub fn time(&self) -> Duration {
        self.time
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn is_looping(&self) -> bool {
        self.looping
    }

    /// Whether a one-shot reached its end and handed its last frame back to the page.
    pub fn is_finished(&self) -> bool {
        self.finished_at.is_some()
    }

    pub fn play(&mut self, cx: &mut Context<Self>) {
        if self.time >= self.duration {
            self.time = Duration::ZERO;
        }
        self.finished_at = None;
        self.playing = true;
        self.last_tick = None;
        cx.notify();
    }

    pub fn pause(&mut self, cx: &mut Context<Self>) {
        self.playing = false;
        cx.notify();
    }

    pub fn toggle(&mut self, cx: &mut Context<Self>) {
        if self.playing {
            self.pause(cx);
        } else {
            self.play(cx);
        }
    }

    /// Jumps to `time`, clamped to the timeline. Seeking doesn't change whether it plays.
    pub fn seek(&mut self, time: Duration, cx: &mut Context<Self>) {
        self.time = time.min(self.duration);
        self.finished_at = None;
        self.last_tick = None;
        cx.notify();
    }

    pub fn set_looping(&mut self, looping: bool, cx: &mut Context<Self>) {
        self.looping = looping;
        cx.notify();
    }

    /// Stops on the last frame and hands it back to the page, so loops carry on. Calls `on_end`.
    pub fn finish(&mut self, cx: &mut Context<Self>) {
        if self.finished_at.is_some() {
            return;
        }
        self.playing = false;
        self.time = self.duration;
        self.finished_at = Some(crate::motion::now());
        if let Some(on_end) = self.on_end.clone() {
            cx.defer(move |cx| on_end(cx));
        }
        cx.notify();
    }

    /// Advances the clock to `now`, and returns the time to publish.
    fn tick(&mut self, now: Instant, reduced: bool, cx: &mut Context<Self>) -> ScopeTime {
        if reduced {
            self.playing = false;
            let time = self.poster.unwrap_or(self.duration);
            return ScopeTime {
                time,
                free: false,
                end: self.duration,
            };
        }
        if let Some(finished_at) = self.finished_at {
            return ScopeTime {
                time: self.duration + now.saturating_duration_since(finished_at),
                free: true,
                end: self.duration,
            };
        }
        if self.playing {
            if let Some(last) = self.last_tick {
                self.time += now.saturating_duration_since(last).mul_f32(self.speed);
            }
            self.last_tick = Some(now);
            if self.time >= self.duration {
                if self.looping && !self.duration.is_zero() {
                    self.time = Duration::from_nanos(
                        (self.time.as_nanos() % self.duration.as_nanos()) as u64,
                    );
                } else {
                    self.finish(cx);
                    return ScopeTime {
                        time: self.duration,
                        free: true,
                        end: self.duration,
                    };
                }
            }
        } else {
            self.last_tick = None;
        }
        ScopeTime {
            time: self.time,
            free: false,
            end: self.duration,
        }
    }
}

/// The current time of the timeline being laid out, for text that counts or types along.
/// `None` outside a timeline.
pub fn timeline_time() -> Option<Duration> {
    scope_time().map(|scope| scope.time)
}

/// Drives every Kirakira animation inside it from one clock.
///
/// Wrap scenes in [`Scene`] to place them on the timeline; animations inside a scene count from
/// the scene's start. A one-shot timeline that reaches its end hands the last frame back to the
/// page: entrances stay finished, loops carry on.
#[derive(IntoElement)]
pub struct Timeline {
    state: Entity<TimelineState>,
    base: Div,
}

impl Timeline {
    pub fn new(state: &Entity<TimelineState>) -> Self {
        Self {
            state: state.clone(),
            base: div(),
        }
    }
}

impl Styled for Timeline {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Timeline {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for Timeline {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let reduced = cx.reduce_motion();
        let now = crate::motion::now();
        let (scope, playing, skippable) = self.state.update(cx, |state, cx| {
            let scope = state.tick(now, reduced, cx);
            (scope, state.playing, state.skippable)
        });
        if playing {
            window.request_animation_frame();
        }
        let skip = skippable && playing && scope.time < SKIP_WINDOW.min(scope.end);
        let state = self.state.clone();
        scoped(
            scope,
            self.base.relative().when(skip, |this| {
                this.child(
                    div()
                        .id("kk-timeline-skip")
                        .absolute()
                        .right_3()
                        .bottom_3()
                        .rounded_full()
                        .px_3()
                        .py_1()
                        .text_xs()
                        .font_weight(gpui_kit::FontWeight::BOLD)
                        .bg(cx.theme().background.opacity(0.85))
                        .text_color(cx.theme().foreground)
                        .border_1()
                        .border_color(cx.theme().border)
                        .shadow_sm()
                        .cursor_pointer()
                        .hover(|this| this.bg(cx.theme().background))
                        .on_click(move |_, _, cx| state.update(cx, |state, cx| state.finish(cx)))
                        .child("Skip ›"),
                )
            }),
        )
    }
}

/// A layer that is visible from `start` for `duration` ms.
///
/// Delays on animations inside it count from the scene's start, so a scene can be moved along the
/// timeline without retiming it. Finished scenes stay mounted and hidden, so seeking back finds
/// them. A scene fills its timeline (`absolute inset-0`); restyle it to change that.
#[derive(IntoElement)]
pub struct Scene {
    start: Duration,
    duration: Option<Duration>,
    base: Div,
}

impl Scene {
    /// A scene that appears `start_ms` after the timeline starts and stays to the end.
    pub fn new(start_ms: u64) -> Self {
        Self {
            start: Duration::from_millis(start_ms),
            duration: None,
            base: div().absolute().inset_0(),
        }
    }

    /// How long the scene stays, in ms.
    pub fn duration(mut self, duration_ms: u64) -> Self {
        self.duration = Some(Duration::from_millis(duration_ms));
        self
    }
}

impl Styled for Scene {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Scene {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

impl RenderOnce for Scene {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let Some(parent) = scope_time() else {
            // Outside a timeline a scene is just a layer.
            return self.base.into_any_element();
        };
        let shown = parent.time.min(parent.end);
        let end = self.duration.map(|duration| self.start + duration);
        let hidden = shown < self.start
            || end
                .is_some_and(|end| shown >= end && (end < parent.end || parent.time > parent.end));
        let local = ScopeTime {
            time: parent.time.saturating_sub(self.start),
            free: parent.free,
            end: parent.end.saturating_sub(self.start),
        };
        scoped(local, self.base.when(hidden, |this| this.invisible())).into_any_element()
    }
}

#[derive(Clone, Copy, Default)]
struct ScrubState {
    bounds: Bounds<Pixels>,
    resume: bool,
}

/// Play, pause, scrub and loop for a timeline. Keep it outside the [`Stage`].
#[derive(IntoElement)]
pub struct TimelineControls {
    id: ElementId,
    state: Entity<TimelineState>,
    style: StyleRefinement,
}

impl TimelineControls {
    pub fn new(id: impl Into<ElementId>, state: &Entity<TimelineState>) -> Self {
        Self {
            id: id.into(),
            state: state.clone(),
            style: StyleRefinement::default(),
        }
    }
}

impl Styled for TimelineControls {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

fn seconds(time: Duration) -> SharedString {
    format!("{:.2}", time.as_secs_f32()).into()
}

impl RenderOnce for TimelineControls {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (time, duration, playing, looping) = {
            let state = self.state.read(cx);
            (state.time, state.duration, state.playing, state.looping)
        };
        let progress = if duration.is_zero() {
            0.0
        } else {
            time.as_secs_f32() / duration.as_secs_f32()
        };
        let scrub =
            window.use_keyed_state((self.id.clone(), "scrub"), cx, |_, _| ScrubState::default());
        let theme = cx.theme();

        let seek_to = {
            let state = self.state.clone();
            let scrub = scrub.clone();
            move |x: Pixels, cx: &mut App| {
                let bounds = scrub.read(cx).bounds;
                if bounds.size.width <= px(0.0) {
                    return;
                }
                let fraction = ((x - bounds.origin.x) / bounds.size.width).clamp(0.0, 1.0);
                state.update(cx, |state, cx| {
                    let time = state.duration.mul_f32(fraction);
                    state.seek(time, cx);
                });
            }
        };

        let toggle = self.state.clone();
        let loop_state = self.state.clone();
        let down_state = self.state.clone();
        let down_scrub = scrub.clone();
        let up_state = self.state.clone();
        let up_scrub = scrub.clone();
        let seek_down = seek_to.clone();
        let measure = scrub.clone();

        div()
            .id(self.id.clone())
            .flex()
            .items_center()
            .gap_3()
            .rounded_full()
            .bg(theme.muted)
            .px_2()
            .py_1p5()
            .text_sm()
            .text_color(theme.foreground)
            .refine_style(&self.style)
            .child(
                div()
                    .id("play")
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .size_8()
                    .rounded_full()
                    .bg(theme.primary)
                    .text_color(theme.primary_foreground)
                    .cursor_pointer()
                    .on_click(move |_, _, cx| toggle.update(cx, |state, cx| state.toggle(cx)))
                    .child(icon(if playing { icons::PAUSE } else { icons::PLAY }).size_3p5()),
            )
            .child(
                div()
                    .id("track")
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .h_4()
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, move |event, _, cx| {
                        let resume = down_state.read(cx).playing;
                        down_scrub.update(cx, |scrub, _| scrub.resume = resume);
                        down_state.update(cx, |state, cx| state.pause(cx));
                        seek_down(event.position.x, cx);
                    })
                    .on_mouse_move(move |event, _, cx| {
                        if event.dragging() {
                            seek_to(event.position.x, cx);
                        }
                    })
                    .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                        let resume =
                            up_scrub.update(cx, |scrub, _| std::mem::take(&mut scrub.resume));
                        if resume {
                            up_state.update(cx, |state, cx| state.play(cx));
                        }
                    })
                    .child(
                        gpui_kit::canvas(
                            move |bounds, _, cx| {
                                measure.update(cx, |scrub, _| scrub.bounds = bounds)
                            },
                            |_, _, _, _| {},
                        )
                        .absolute()
                        .size_full(),
                    )
                    .child(
                        div()
                            .w_full()
                            .h_1p5()
                            .rounded_full()
                            .bg(theme.border)
                            .child(
                                div()
                                    .h_full()
                                    .rounded_full()
                                    .bg(theme.primary)
                                    .w(relative(progress.clamp(0.0, 1.0))),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .font_family(theme.mono_font_family.clone())
                    .text_xs()
                    .text_color(theme.muted_foreground)
                    .child(format!("{} / {}", seconds(time), seconds(duration))),
            )
            .child(
                div()
                    .id("loop")
                    .flex_none()
                    .rounded_full()
                    .px_2p5()
                    .py_1()
                    .text_xs()
                    .font_weight(gpui_kit::FontWeight::BOLD)
                    .cursor_pointer()
                    .text_color(theme.muted_foreground)
                    .when(looping, |this| {
                        this.bg(theme.background).text_color(theme.foreground)
                    })
                    .on_click(move |_, _, cx| {
                        loop_state.update(cx, |state, cx| {
                            let looping = !state.looping;
                            state.set_looping(looping, cx)
                        })
                    })
                    .child("Loop"),
            )
    }
}

#[derive(Clone, Copy, Default)]
struct StageState {
    width: Option<Pixels>,
}

/// A fixed-size canvas scaled to fit its width: build a scene once at 1920 × 1080 and it looks the
/// same in a small preview and a large window.
///
/// GPUI scales rems, not pixels: lay out stage content in rems, at 1rem = 16 design pixels
/// (`w(rems(120.))` is 1920 px), and text and rem spacing scale with the stage. Pixel sizes don't.
#[derive(IntoElement)]
pub struct Stage {
    id: ElementId,
    width: f32,
    height: f32,
    base: Div,
}

impl Stage {
    /// A 1920 × 1080 stage.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            width: 1920.0,
            height: 1080.0,
            base: div(),
        }
    }

    /// The design size in pixels.
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.width = width.max(1.0);
        self.height = height.max(1.0);
        self
    }
}

impl Styled for Stage {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl ParentElement for Stage {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.base.extend(elements);
    }
}

/// The rem size a stage's design is laid out at.
const DESIGN_REM: f32 = 16.0;

impl RenderOnce for Stage {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_keyed_state(self.id.clone(), cx, |_, _| StageState::default());
        let width = state.read(cx).width;
        let scale = width.map_or(1.0, |width| f32::from(width) / self.width);
        let rem = window.rem_size();
        let measure = state.clone();
        div()
            .id(self.id)
            .relative()
            .w_full()
            .overflow_hidden()
            .aspect_ratio(self.width / self.height)
            .child(
                gpui_kit::canvas(
                    move |bounds, window, cx| {
                        let changed = measure.update(cx, |state, _| {
                            let changed = state.width != Some(bounds.size.width);
                            state.width = Some(bounds.size.width);
                            changed
                        });
                        if changed {
                            window.refresh();
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .child(
                div().absolute().top_0().left_0().child(rescaled(
                    scale * DESIGN_REM / f32::from(rem),
                    self.base
                        .relative()
                        .w(px(self.width * scale))
                        .h(px(self.height * scale)),
                )),
            )
    }
}
