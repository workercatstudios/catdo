//! Calendar: picked days pop into their pill, ranges fill in a wave, months slide in.
//!
//! Replaces `gpui_kit::component::calendar` (GPUI Component's `time::calendar`), with the same
//! builder: `Calendar::new(&state)` with `number_of_months`, `first_day_of_week`, [`Sizable`] and
//! [`Styled`]. Every other item of that module is re-exported. Rebuilt on gpui-base's `Calendar`
//! like GPUI Component's, with the same sizes, colours and views, plus the motion of Kirakira's
//! Pop Calendar:
//!
//! - A day that becomes selected pops into its pill: `scale` 0.55 → (1.16, 1.2) at 45 % →
//!   (0.94, 0.96) at 72 % → 1 in 0.4 s, ease-in-out; so do a range's two ends.
//! - The days between a range's ends fill in a wave: (0.8, 0.7) → (1.04, 1.1) → 1 in 0.3 s, 25 ms
//!   apart along the week.
//! - Changing month slides the new weeks in from the side you moved towards: from 40 % of the
//!   grid's width, opaque between 25 % and 55 %, 3 % past their place at 65 %, settled at 0.4 s.
//!   The caption comes in the same way from 1.5 em in 0.36 s.
//! - Days, the month and year grids and the arrows squash to 0.86 while held (0.08 s) and spring
//!   back (0.35 s).
//!
//! Under reduced motion picks change colour at once and a new month fades in (0.24 s, opaque from
//! halfway); nothing moves or scales.
//!
//! Differences from the web version: react-day-picker keeps a copy of the old month on top and
//! slides it out (0.15 s) while the new one comes in; gpui-base's calendar renders only the current
//! month, so the old one is gone at once. While a day pops it is drawn as a [vector](crate::vector)
//! pill and number, so both squash exactly; at rest it is the real item. The 0.86 press is even, so
//! it scales the item's box and its rem size. A pop overrides the press, as a CSS animation
//! overrides a transition. Labels are English, where GPUI Component translates them. A selected day also pops when its month comes back into view, as on the web, where the
//! new month's days mount with their animation.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use chrono::{Datelike as _, NaiveDate, Weekday};
use gpui_kit::base::{
    Calendar as BaseCalendar, CalendarItem, CalendarItemKind, CalendarItemState, StyledExt as _,
};
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable, Size};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, ElementId, Entity, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _,
    Pixels, RenderOnce, SharedString, StyleRefinement, Styled, Window, px, rems,
};

pub use gpui_kit::component::calendar::*;

use crate::motion::{Easing, Keyframes, Pose, Timing, Track, ms, now, transform};
use crate::squash::{Plate, Squash, plated, squashed};

const PICK: Duration = ms(400);
const FILL: Duration = ms(300);
const WEEKS_IN: Duration = ms(400);
const CAPTION_IN: Duration = ms(360);
const FADE_IN: Duration = ms(240);
const PRESS: Squash = Squash::new(0.86, 0.86);

fn pick_x() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.55)
        .at(0.45, 1.16)
        .at(0.72, 0.94)
        .at(1.0, 1.0)
        .build()
}

fn pick_y() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.55)
        .at(0.45, 1.2)
        .at(0.72, 0.96)
        .at(1.0, 1.0)
        .build()
}

fn fill_x() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.8)
        .at(0.5, 1.04)
        .at(1.0, 1.0)
        .build()
}

fn fill_y() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.7)
        .at(0.5, 1.1)
        .at(1.0, 1.0)
        .build()
}

/// The new weeks' offset, as a fraction of the grid's width, for a move towards the end.
fn weeks_x() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.4, Easing::EaseOut)
        .at(0.65, -0.03)
        .at(1.0, 0.0)
        .build()
}

fn weeks_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.0, Easing::EaseOut)
        .at(0.25, 0.0)
        .at(0.55, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// The new caption's offset in ems, for a move towards the end.
fn caption_x() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 1.5, Easing::EaseOut)
        .at(0.6, -0.15)
        .at(1.0, 0.0)
        .build()
}

fn caption_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.0, Easing::EaseOut)
        .at(0.3, 0.0)
        .at(0.6, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// Reduced motion: a new month fades in, opaque from halfway.
fn fade_in() -> Keyframes<f32> {
    Track::new(Easing::Linear)
        .at(0.0, 0.0)
        .at(0.5, 0.0)
        .at(1.0, 1.0)
        .build()
}

fn month_name(month: i32) -> &'static str {
    const NAMES: [&str; 12] = [
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    NAMES
        .get((month - 1).clamp(0, 11) as usize)
        .copied()
        .unwrap_or("")
}

fn weekday_name(day: i32) -> &'static str {
    ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"][day.rem_euclid(7) as usize]
}

/// The weeks gpui-base lays out for `month` of `year`, starting on `first_day`.
fn days_in_month(year: i32, month: u32, first_day: Weekday) -> Vec<NaiveDate> {
    let total = year as i64 * 12 + month as i64 - 1;
    let year = total.div_euclid(12) as i32;
    let month = total.rem_euclid(12) as u32 + 1;
    let Some(first) = NaiveDate::from_ymd_opt(year, month, 1) else {
        return Vec::new();
    };
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    let Some(next) = next else {
        return Vec::new();
    };
    let offset =
        (first.weekday().num_days_from_sunday() + 7 - first_day.num_days_from_sunday()) % 7;
    let start = first - chrono::Duration::days(offset as i64);
    let count = ((next - start).num_days() as usize).div_ceil(7) * 7;
    (0..count)
        .map(|n| start + chrono::Duration::days(n as i64))
        .collect()
}

/// How a day is drawn as selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Look {
    /// A single pick or one end of a range: pops into its pill.
    Picked,
    /// Between a range's ends: fills in.
    Between,
}

fn look(date: &Date, day: &NaiveDate) -> Option<Look> {
    if date.is_active(day) {
        Some(Look::Picked)
    } else if date.is_in_range(day) {
        Some(Look::Between)
    } else {
        None
    }
}

/// The squash of a day's pill `elapsed` into its pop.
fn pop_scale(look: Look, elapsed: Duration, column: usize) -> (f32, f32) {
    let (x, y, timing) = match look {
        Look::Picked => (pick_x(), pick_y(), Timing::new(PICK)),
        Look::Between => (
            fill_x(),
            fill_y(),
            Timing::new(FILL).delay(crate::motion::delay_ms(column as u64 * 25)),
        ),
    };
    let progress = timing.sample(elapsed).directed_progress;
    (x.sample(progress), y.sample(progress))
}

/// Where a day came from: which month of the calendar and which date.
type DayKey = (NaiveDate, usize);

struct Motion {
    /// Each day's look on the last render.
    shown: HashMap<DayKey, Look>,
    /// Pops in flight: when each started.
    pops: HashMap<DayKey, (Instant, Look)>,
    /// The month shown, and when and which way it last changed.
    month: Option<(i32, u8)>,
    moved: Option<(Instant, f32)>,
    /// The item held down.
    pressed: Option<ElementId>,
}

/// What a day item needs to draw its motion.
struct DayMotion {
    date: NaiveDate,
    offset: usize,
    /// The pop in flight: the pill's scale and how it is drawn.
    pop: Option<((f32, f32), Look)>,
}

/// Styled facade for the complete behavior and structure in `gpui-base`, with motion.
#[derive(IntoElement)]
pub struct Calendar {
    id: ElementId,
    size: Size,
    state: Entity<CalendarState>,
    style: StyleRefinement,
    number_of_months: usize,
    first_day_of_week: Weekday,
}

impl Calendar {
    pub fn new(state: &Entity<CalendarState>) -> Self {
        Self {
            id: ("calendar", state.entity_id()).into(),
            size: Size::default(),
            state: state.clone(),
            style: StyleRefinement::default(),
            number_of_months: 1,
            first_day_of_week: Weekday::Sun,
        }
    }

    pub fn number_of_months(mut self, count: usize) -> Self {
        self.number_of_months = count;
        self
    }

    /// Set the first day of the week for the calendar.
    pub fn first_day_of_week(mut self, day: Weekday) -> Self {
        self.first_day_of_week = day;
        self
    }
}

impl Sizable for Calendar {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl Styled for Calendar {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

fn cell_size(size: Size) -> Pixels {
    match size {
        Size::Small => px(28.),
        Size::Large => px(40.),
        _ => px(32.),
    }
}

impl RenderOnce for Calendar {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let size = self.size;
        let month_count = self.number_of_months.max(1);
        let motion = window.use_keyed_state((self.id.clone(), "kk-motion"), cx, |_, _| Motion {
            shown: HashMap::new(),
            pops: HashMap::new(),
            month: None,
            moved: None,
            pressed: None,
        });
        let reduced = cx.reduce_motion();
        let now = now();

        // Which days show, in the order gpui-base asks for their items.
        let (date, view, shown_month, months) = {
            let state = self.state.read(cx);
            let months: Vec<(i32, u32)> = (0..month_count)
                .map(|offset| state.offset_year_month(offset))
                .collect();
            (
                state.date(),
                state.view(),
                (state.current_year(), state.current_month()),
                months,
            )
        };
        let days: Vec<(NaiveDate, usize, usize)> = months
            .iter()
            .enumerate()
            .flat_map(|(offset, (year, month))| {
                days_in_month(*year, *month, self.first_day_of_week)
                    .into_iter()
                    .enumerate()
                    .map(move |(ix, day)| (day, offset, ix % 7))
            })
            .collect();

        // Start a pop for every day whose look is new, and note a change of month.
        let looks: HashMap<DayKey, Look> = days
            .iter()
            .filter_map(|(day, offset, _)| look(&date, day).map(|look| ((*day, *offset), look)))
            .collect();
        motion.update(cx, |motion, _| {
            for (key, look) in &looks {
                if motion.shown.get(key) != Some(look) {
                    motion.pops.insert(*key, (now, *look));
                }
            }
            motion.pops.retain(|key, (at, _)| {
                looks.contains_key(key) && now.saturating_duration_since(*at) < ms(600)
            });
            motion.shown = looks;
            if view.is_day() {
                let current = (shown_month.0, shown_month.1);
                if let Some(previous) = motion.month.filter(|previous| *previous != current) {
                    let towards_end = (current.0, current.1) > (previous.0, previous.1);
                    motion.moved = Some((now, if towards_end { 1.0 } else { -1.0 }));
                }
                motion.month = Some(current);
            }
        });

        let snapshot = motion.read(cx);
        let mut running = false;
        let day_motion: Vec<DayMotion> = days
            .iter()
            .map(|(day, offset, column)| {
                let pop = match snapshot.pops.get(&(*day, *offset)) {
                    Some((at, look)) if !reduced => {
                        let scale = pop_scale(*look, now.saturating_duration_since(*at), *column);
                        (scale != (1.0, 1.0)).then_some((scale, *look))
                    }
                    _ => None,
                };
                running |= pop.is_some();
                DayMotion {
                    date: *day,
                    offset: *offset,
                    pop,
                }
            })
            .collect();
        let moved = snapshot
            .moved
            .map(|(at, dir)| (now.saturating_duration_since(at), dir));
        let pressed = snapshot.pressed.clone();
        if moved.is_some_and(|(elapsed, _)| elapsed < WEEKS_IN) {
            running = true;
        }
        if running {
            window.request_animation_frame();
        }

        let cell = cell_size(size);
        let em = rems(0.875).to_pixels(window.rem_size());
        // The month change: how far the new weeks and caption have come, and how opaque they are.
        let (weeks, caption) = match moved {
            Some((elapsed, dir)) if elapsed < WEEKS_IN => {
                if reduced {
                    let fade =
                        fade_in().sample(Timing::new(FADE_IN).sample(elapsed).directed_progress);
                    (Pose::new().opacity(fade), Pose::new().opacity(fade))
                } else {
                    let weeks = Timing::new(WEEKS_IN).sample(elapsed).directed_progress;
                    let caption = Timing::new(CAPTION_IN).sample(elapsed).directed_progress;
                    (
                        Pose::new()
                            .x(dir * weeks_x().sample(weeks) * f32::from(cell) * 7.0)
                            .opacity(weeks_opacity().sample(weeks)),
                        Pose::new()
                            .x(dir * caption_x().sample(caption) * f32::from(em))
                            .opacity(caption_opacity().sample(caption)),
                    )
                }
            }
            _ => (Pose::new(), Pose::new()),
        };

        let day_motion = Rc::new(day_motion);
        let next_day = Rc::new(Cell::new(0usize));
        let next_item = Rc::new(Cell::new(0usize));
        let calendar_id = self.id.clone();

        BaseCalendar::new(self.id.clone(), &self.state)
            .number_of_months(self.number_of_months)
            .first_day_of_week(self.first_day_of_week)
            .label(|kind, value| match kind {
                CalendarItemKind::Previous => "‹".into(),
                CalendarItemKind::Next => "›".into(),
                CalendarItemKind::MonthToggle | CalendarItemKind::Month => month_name(value).into(),
                CalendarItemKind::Weekday => weekday_name(value).into(),
                _ => value.to_string().into(),
            })
            .item(move |item, state, window, cx| {
                let kind = state.kind();
                // Days come in order; match each to its date.
                let day = (kind == CalendarItemKind::Day).then(|| {
                    let ix = next_day.get();
                    next_day.set(ix + 1);
                    day_motion.get(ix)
                });
                let day = day.flatten();
                let ix = next_item.get();
                next_item.set(ix + 1);
                let item_id: ElementId = match (kind, day) {
                    (CalendarItemKind::Day, Some(day)) => (
                        calendar_id.clone(),
                        SharedString::from(format!("day-{}-{}", day.date, day.offset)),
                    )
                        .into(),
                    _ => (
                        calendar_id.clone(),
                        SharedString::from(format!("{kind:?}-{ix}")),
                    )
                        .into(),
                };
                let pressable = !state.is_disabled()
                    && matches!(
                        kind,
                        CalendarItemKind::Day
                            | CalendarItemKind::Previous
                            | CalendarItemKind::Next
                            | CalendarItemKind::Month
                            | CalendarItemKind::Year
                    );
                let item = style_item(item, state, size, cx);
                if !pressable {
                    return match kind {
                        CalendarItemKind::MonthToggle | CalendarItemKind::YearToggle => {
                            transform((item_id, "kk-slide"), caption, item).into_any_element()
                        }
                        _ => item.into_any_element(),
                    };
                }

                let held = pressed.as_ref() == Some(&item_id);
                let (x, y) = PRESS.scale(&item_id, held, window, cx);
                let pop = day.and_then(|day| day.pop.map(|pop| (day, pop)));
                let (down, up, up_out) = (motion.clone(), motion.clone(), motion.clone());
                let press_id = item_id.clone();
                let item = item
                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                        down.update(cx, |motion, cx| {
                            motion.pressed = Some(press_id.clone());
                            cx.notify();
                        })
                    })
                    .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                        up.update(cx, |motion, cx| {
                            motion.pressed = None;
                            cx.notify();
                        })
                    })
                    .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                        up_out.update(cx, |motion, cx| {
                            motion.pressed = None;
                            cx.notify();
                        })
                    });
                let fixed = matches!(
                    kind,
                    CalendarItemKind::Day | CalendarItemKind::Previous | CalendarItemKind::Next
                );
                let natural = gpui_kit::size(cell, cell);
                // A pop overrides the press, as a CSS animation overrides a transition. While it
                // plays the day is a vector pill and number, so both squash exactly.
                let drawn = match pop {
                    Some((day, ((sx, sy), look))) => {
                        let theme = cx.theme();
                        let (fill, color) = match look {
                            Look::Picked => (theme.primary, theme.primary_foreground),
                            Look::Between => (theme.accent, theme.accent_foreground),
                        };
                        let text = gpui_kit::TextStyleRefinement {
                            font_size: Some(rems(0.875).into()),
                            ..Default::default()
                        };
                        let plate = Plate {
                            fill: Some(fill),
                            border: None,
                            radius: item_radius(size, cx),
                            label: day.date.day().to_string().into(),
                            color,
                            text,
                        };
                        plated(item_id.clone(), Pose::new().scale_xy(sx, sy), plate, item)
                            .natural(natural)
                            .into_any_element()
                    }
                    None => {
                        let mut squashed =
                            squashed(item_id.clone(), (x, y), item).text((x * y).sqrt());
                        if fixed {
                            squashed = squashed.natural(natural);
                        }
                        squashed.into_any_element()
                    }
                };
                if kind == CalendarItemKind::Day {
                    transform((item_id, "kk-slide"), weeks, drawn).into_any_element()
                } else {
                    drawn
                }
            })
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius_lg)
            .p_3()
            .gap_0p5()
            .map(|this| match size {
                Size::Small => this.w(px(220.) * month_count as f32),
                Size::Large => this.w(px(304.) * month_count as f32),
                _ => this.w(px(248.) * month_count as f32),
            })
            .refine_style(&self.style)
    }
}

fn item_radius(size: Size, cx: &App) -> Pixels {
    let radius = cx.theme().radius;
    match size {
        Size::Small => radius / 2.,
        Size::Large => radius * 2.,
        _ => radius,
    }
}

/// GPUI Component's look for a calendar item.
fn style_item(item: CalendarItem, state: CalendarItemState, size: Size, cx: &App) -> CalendarItem {
    let theme = cx.theme();
    let kind = state.kind();
    let item = match kind {
        CalendarItemKind::Previous => item
            .clear_children()
            .child(Icon::new(IconName::ChevronLeft).size_4()),
        CalendarItemKind::Next => item
            .clear_children()
            .child(Icon::new(IconName::ChevronRight).size_4()),
        _ => item,
    };
    item.map(|this| match size {
        Size::Small => this.size_7(),
        Size::Large => this.size_10(),
        _ => this.size_8(),
    })
    .rounded(item_radius(size, cx))
    .flex()
    .flex_none()
    .items_center()
    .justify_center()
    .when(kind != CalendarItemKind::Weekday, |this| this.text_sm())
    .when(kind == CalendarItemKind::Weekday, |this| {
        this.text_xs()
            .font_normal()
            .text_color(theme.muted_foreground)
    })
    .when(kind == CalendarItemKind::Month, |this| this.text_xs())
    .when(
        matches!(
            kind,
            CalendarItemKind::MonthToggle | CalendarItemKind::YearToggle
        ),
        |this| this.text_sm().font_medium(),
    )
    .when(
        matches!(kind, CalendarItemKind::Month | CalendarItemKind::Year),
        |this| this.w_full().my_1(),
    )
    .when(
        matches!(
            kind,
            CalendarItemKind::MonthToggle | CalendarItemKind::YearToggle
        ),
        |this| this.w_auto().px_2(),
    )
    .when(state.is_muted(), |this| {
        this.text_color(theme.muted_foreground)
            .when(state.is_disabled(), |this| this.opacity(0.5))
    })
    .when(state.is_in_range(), |this| {
        this.bg(theme.accent).text_color(theme.accent_foreground)
    })
    .when(
        !state.is_active() && !state.is_disabled() && kind != CalendarItemKind::Weekday,
        |this| {
            this.hover(|this| {
                this.bg(theme.tokens.secondary_hover)
                    .text_color(theme.foreground)
            })
        },
    )
    .when(state.is_active(), |this| {
        this.bg(theme.tokens.primary)
            .text_color(theme.primary_foreground)
    })
    .when(state.is_today() && !state.is_active(), |this| {
        this.bg(theme.accent).text_color(theme.accent_foreground)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    #[test]
    fn picks_match_the_web() {
        use crate::parity::assert_number_track;
        let c = "pop-calendar";
        assert_number_track(c, "kk-pop-calendar-pick", "sx", &pick_x());
        assert_number_track(c, "kk-pop-calendar-pick", "sy", &pick_y());
        assert_number_track(c, "kk-pop-calendar-fill", "sx", &fill_x());
        assert_number_track(c, "kk-pop-calendar-fill", "sy", &fill_y());
        assert_number_track(c, "kk-pop-calendar-fade-in", "opacity", &fade_in());
        // The month slides (kk-pop-calendar-weeks-in, -caption-in) translate by
        // `calc(var(...) * 40%)`, which the parity parser can't read, so those two rules are
        // checked by hand in `the_new_month_overshoots_then_settles`.
    }

    #[test]
    fn weeks_match_gpui_base() {
        // August 2025 starts on a Friday and needs six weeks.
        let days = days_in_month(2025, 8, Weekday::Sun);
        assert_eq!(days.len(), 42);
        assert_eq!(days[0], day(2025, 7, 27));
        assert_eq!(days[5], day(2025, 8, 1));
        // Starting the week on Monday shifts the grid.
        let days = days_in_month(2025, 8, Weekday::Mon);
        assert_eq!(days[0], day(2025, 7, 28));
        // Month 13 is January of the next year.
        assert_eq!(days_in_month(2025, 13, Weekday::Sun)[4], day(2026, 1, 1));
    }

    #[test]
    fn a_range_has_picked_ends_and_a_filled_middle() {
        let range = Date::Range(Some(day(2025, 2, 10)), Some(day(2025, 2, 14)));
        assert_eq!(look(&range, &day(2025, 2, 10)), Some(Look::Picked));
        assert_eq!(look(&range, &day(2025, 2, 12)), Some(Look::Between));
        assert_eq!(look(&range, &day(2025, 2, 14)), Some(Look::Picked));
        assert_eq!(look(&range, &day(2025, 2, 15)), None);
    }

    #[test]
    fn a_pick_pops_taller_than_wide() {
        let (x, y) = pop_scale(Look::Picked, ms(180), 0);
        assert!((x - 1.16).abs() < 1e-3 && (y - 1.2).abs() < 1e-3);
        assert_eq!(pop_scale(Look::Picked, ms(400), 0), (1.0, 1.0));
    }

    #[test]
    fn the_fill_waves_along_the_week() {
        // Column 4 waits 100 ms at its starting squash.
        assert_eq!(pop_scale(Look::Between, ms(50), 4), (0.8, 0.7));
        assert_eq!(pop_scale(Look::Between, ms(250), 4).0, 1.04);
    }

    #[test]
    fn the_new_month_overshoots_then_settles() {
        assert_eq!(weeks_x().sample(0.0), 0.4);
        assert!((weeks_x().sample(0.65) + 0.03).abs() < 1e-6);
        assert_eq!(weeks_opacity().sample(0.2), 0.0);
        assert_eq!(weeks_opacity().sample(0.25), 0.0);
        assert_eq!(weeks_opacity().sample(0.55), 1.0);
        assert_eq!(caption_x().sample(0.0), 1.5);
        assert!((caption_x().sample(0.6) + 0.15).abs() < 1e-6);
        assert_eq!(caption_opacity().sample(0.3), 0.0);
        assert_eq!(caption_opacity().sample(0.6), 1.0);
    }
}
