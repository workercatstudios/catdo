use std::time::{Duration, Instant};

use gpui_kit::{App, ElementId, Entity, Window};

struct PulseState {
    fired: Option<Instant>,
    count: usize,
}

/// A one-shot animation that restarts on an event: a click pop, a toggle bounce, a burst.
///
/// The web components restart a keyframe by flipping between two identical copies on each click.
/// A pulse records when it last fired instead. Create it while rendering, read
/// [`Pulse::elapsed`], and call [`Pulse::fire`] from an event handler.
///
/// Interactive motion runs on real time, never on a timeline's clock: a pulse can't be seeked.
#[derive(Clone)]
pub struct Pulse {
    state: Entity<PulseState>,
    elapsed: Option<Duration>,
    count: usize,
    reduced: bool,
}

impl Pulse {
    pub fn new(id: impl Into<ElementId>, window: &mut Window, cx: &mut App) -> Self {
        let state = window.use_keyed_state(id.into(), cx, |_, _| PulseState {
            fired: None,
            count: 0,
        });
        let now = super::now();
        let snapshot = state.read(cx);
        let elapsed = snapshot
            .fired
            .map(|fired| now.saturating_duration_since(fired));
        let count = snapshot.count;
        Self {
            state,
            elapsed,
            count,
            reduced: cx.reduce_motion(),
        }
    }

    /// Time since the pulse last fired, or `None` if it never has. `None` under reduced motion,
    /// so the pulse shows its resting state.
    pub fn elapsed(&self) -> Option<Duration> {
        if self.reduced { None } else { self.elapsed }
    }

    /// Time since the last fire, if that was less than `duration` ago.
    pub fn running(&self, duration: Duration) -> Option<Duration> {
        self.elapsed().filter(|elapsed| *elapsed < duration)
    }

    /// How many times the pulse has fired. Useful to vary each burst.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Restarts the pulse. Call it from an event handler.
    pub fn fire(&self, cx: &mut App) {
        self.state.update(cx, |state, cx| {
            state.fired = Some(super::now());
            state.count += 1;
            cx.notify();
        });
    }

    /// Asks for another frame while the pulse runs, for `duration` after it fired.
    pub fn animate(&self, duration: Duration, window: &mut Window) {
        if self.running(duration).is_some() {
            window.request_animation_frame();
        }
    }
}
