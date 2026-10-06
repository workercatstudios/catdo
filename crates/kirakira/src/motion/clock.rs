use std::{
    cell::{Cell, RefCell},
    time::Duration,
    time::Instant,
};

use gpui_kit::{
    App, Bounds, ElementId, Entity, ParentElement, Pixels, Styled as _, Window, px, size,
};

use super::{Keyframes, Timing};
use gpui_kit::base::Interpolate;

/// When an entrance starts.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Trigger {
    /// When the element first renders.
    #[default]
    Mount,
    /// When the element first scrolls into view. Until then it holds its first frame.
    ///
    /// Inside a [`Timeline`](crate::timeline::Timeline) the timeline's clock decides, so it plays
    /// as if it were [`Trigger::Mount`].
    InView,
}

/// A time far enough past any entrance that it has finished. What a reduced-motion clock reports.
const FOREVER: Duration = Duration::from_secs(60 * 60 * 24);

/// The time a [`Timeline`](crate::timeline::Timeline) or scene publishes to the clocks under it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScopeTime {
    /// The time in the scope.
    pub time: Duration,
    /// The timeline has finished and keeps time on its own, so loops carry on: clocks under it
    /// ask for their own frames again.
    pub free: bool,
    /// The end of the timeline, in the same units as `time`.
    pub end: Duration,
}

thread_local! {
    /// Times pushed by the timelines and scenes being laid out, innermost last.
    static SCOPE: RefCell<Vec<ScopeTime>> = const { RefCell::new(Vec::new()) };
    /// A pinned "now" for screenshots and tests.
    static FROZEN: Cell<Option<Instant>> = const { Cell::new(None) };
}

/// The time Kirakira's clocks, pulses and timelines read: the real time, unless
/// [`freeze_time`] pinned it.
pub fn now() -> Instant {
    FROZEN.with(Cell::get).unwrap_or_else(Instant::now)
}

/// Pins [`now`] to `at`, or releases it with `None`. For screenshot tools and tests that need an
/// exact frame of real-time motion such as a click pop; the gallery's shot mode uses it.
pub fn freeze_time(at: Option<Instant>) {
    FROZEN.with(|frozen| frozen.set(at));
}

/// The time of the innermost [`Timeline`](crate::timeline::Timeline) or scene being laid out, if
/// any. Components render during their parent's layout, so this is how a timeline reaches every
/// clock under it without a context object.
pub fn scope_time() -> Option<ScopeTime> {
    SCOPE.with(|scope| scope.borrow().last().copied())
}

/// Runs `f` with `time` as the scope time, so every [`Clock`] created inside reads it.
pub fn with_scope_time<R>(time: ScopeTime, f: impl FnOnce() -> R) -> R {
    struct Pop;
    impl Drop for Pop {
        fn drop(&mut self) {
            SCOPE.with(|scope| scope.borrow_mut().pop());
        }
    }
    SCOPE.with(|scope| scope.borrow_mut().push(time));
    let _pop = Pop;
    f()
}

/// Whether an element painted at `bounds` shows inside `area`. A side of zero size counts as one
/// pixel past its origin, which is what a hairline drawn there covers: a 0 px tall separator
/// sitting on the top edge of a mask shows, one on its bottom edge doesn't. `Bounds::intersects`
/// alone would call both hidden.
fn in_view(bounds: Bounds<Pixels>, area: Bounds<Pixels>) -> bool {
    let thin = |extent: Pixels| if extent > px(0.) { extent } else { px(1.) };
    let bounds = Bounds::new(
        bounds.origin,
        size(thin(bounds.size.width), thin(bounds.size.height)),
    );
    bounds.intersects(&area)
}

struct ClockState {
    started: Option<Instant>,
}

/// How long an animation has been running.
///
/// Create one while rendering, sample your tracks with it, then call [`Clock::animate`] so GPUI
/// draws the next frame while the animation runs. The start time is keyed state, so re-rendering
/// continues the animation; include a generation in the id to replay it, e.g. `("title", 2)`.
#[derive(Clone)]
pub struct Clock {
    elapsed: Duration,
    scoped: bool,
    free: bool,
    reduced: bool,
    state: Option<Entity<ClockState>>,
    started: bool,
}

impl Clock {
    pub fn new(
        id: impl Into<ElementId>,
        trigger: Trigger,
        window: &mut Window,
        cx: &mut App,
    ) -> Self {
        let reduced = cx.reduce_motion();
        if let Some(scope) = scope_time() {
            return Self {
                elapsed: scope.time,
                scoped: true,
                free: scope.free,
                reduced,
                state: None,
                started: true,
            };
        }
        let now = now();
        let state = window.use_keyed_state(id.into(), cx, |_, _| ClockState {
            started: (trigger == Trigger::Mount).then_some(now),
        });
        let started = state.read(cx).started;
        Self {
            elapsed: started.map_or(Duration::ZERO, |started| {
                now.saturating_duration_since(started)
            }),
            scoped: false,
            free: false,
            reduced,
            state: Some(state),
            started: started.is_some(),
        }
    }

    /// A clock at a fixed time, for previews and tests.
    pub fn at(elapsed: Duration) -> Self {
        Self {
            elapsed,
            scoped: true,
            free: false,
            reduced: false,
            state: None,
            started: true,
        }
    }

    /// Time since the animation started. Zero while an [`Trigger::InView`] entrance waits to be
    /// seen; far past the end under reduced motion, so entrances show their final state.
    pub fn elapsed(&self) -> Duration {
        if self.reduced { FOREVER } else { self.elapsed }
    }

    /// Whether the user asked for reduced motion. Loops should stop and show a still state.
    pub fn reduced(&self) -> bool {
        self.reduced
    }

    /// Whether a timeline drives this clock.
    pub fn scoped(&self) -> bool {
        self.scoped
    }

    /// Whether the animation has started. False only while an `InView` entrance waits.
    pub fn started(&self) -> bool {
        self.started
    }

    /// Samples `keyframes` under `timing` at the clock's time.
    pub fn sample<T: Interpolate>(&self, keyframes: &Keyframes<T>, timing: &Timing) -> T {
        keyframes.sample(timing.sample(self.elapsed()).directed_progress)
    }

    /// Asks GPUI for another frame while the animation runs: until `end`, or forever when `end` is
    /// `None` (a loop). A timeline asks for its own frames, and a reduced-motion clock asks for none.
    pub fn animate(&self, end: Option<Duration>, window: &mut Window) {
        if (self.scoped && !self.free) || self.reduced || !self.started {
            return;
        }
        if end.is_none_or(|end| self.elapsed < end) {
            window.request_animation_frame();
        }
    }

    /// Wires up an [`Trigger::InView`] clock: once `element` is painted inside the visible part of
    /// the window, the clock starts. Other clocks return the element unchanged.
    pub fn observe<E: ParentElement>(&self, element: E) -> E {
        let Some(state) = self.state.clone().filter(|_| !self.started) else {
            return element;
        };
        // Pinned to the top left: gpui-base's `on_prepaint` canvas has no insets, so it would sit
        // at its static position, below the element's content, and report shifted bounds.
        element.child(
            gpui_kit::canvas(
                move |bounds, window, cx| {
                    let visible = window.content_mask().bounds;
                    let viewport =
                        gpui_kit::Bounds::new(gpui_kit::Point::default(), window.viewport_size());
                    if in_view(bounds, visible) && in_view(bounds, viewport) {
                        state.update(cx, |state, cx| {
                            if state.started.is_none() {
                                state.started = Some(now());
                                cx.notify();
                            }
                        });
                    }
                },
                |_, _, _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full(),
        )
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{Bounds, point, px, size};

    use super::in_view;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Bounds<gpui_kit::Pixels> {
        Bounds::new(point(px(x), px(y)), size(px(w), px(h)))
    }

    #[test]
    fn overlapping_boxes_are_in_view() {
        let area = rect(0., 0., 100., 100.);
        assert!(in_view(rect(10., 10., 20., 20.), area));
        assert!(in_view(rect(-10., -10., 20., 20.), area));
        // Touching from outside isn't seeing.
        assert!(!in_view(rect(0., 100., 20., 20.), area));
        assert!(!in_view(rect(-20., 0., 20., 20.), area));
        assert!(!in_view(rect(0., 200., 20., 20.), area));
    }

    #[test]
    fn hairlines_count_the_pixel_they_cover() {
        let area = rect(0., 0., 100., 100.);
        // A 0 px tall separator inside, and on the top edge.
        assert!(in_view(rect(0., 50., 100., 0.), area));
        assert!(in_view(rect(0., 0., 100., 0.), area));
        // On the bottom edge its pixel is below the area.
        assert!(!in_view(rect(0., 100., 100., 0.), area));
        // A 0 px wide vertical one on the left edge, and a 0 × 0 point.
        assert!(in_view(rect(0., 0., 0., 100.), area));
        assert!(in_view(rect(0., 0., 0., 0.), area));
        assert!(!in_view(rect(100., 0., 0., 100.), area));
    }
}
