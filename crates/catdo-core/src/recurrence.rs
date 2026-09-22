use anyhow::{Result, bail};
use chrono::{DateTime, Datelike, Days, Months, NaiveDate, TimeDelta, TimeZone, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RepeatUnit {
    Days,
    Weeks,
    Months,
    Weekdays,
}

/// Preserve wall-clock time across local DST transitions. At an ambiguous
/// time choose the earlier occurrence; in a spring gap use the next valid minute.
pub fn shift_reminder<Tz: TimeZone>(
    at: DateTime<Utc>,
    shift: TimeDelta,
    timezone: &Tz,
) -> Result<DateTime<Utc>> {
    let local = at
        .with_timezone(timezone)
        .naive_local()
        .checked_add_signed(shift)
        .ok_or_else(|| anyhow::anyhow!("Reminder is outside the supported date range."))?;
    for minute in 0..=180 {
        let candidate = local
            .checked_add_signed(TimeDelta::minutes(minute))
            .ok_or_else(|| anyhow::anyhow!("Reminder is outside the supported date range."))?;
        if let Some(next) = timezone.from_local_datetime(&candidate).earliest() {
            return Ok(next.with_timezone(&Utc));
        }
    }
    bail!("The next reminder falls on an unavailable local date. Choose a new reminder time.")
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recurrence {
    pub unit: RepeatUnit,
    pub interval: u16,
    pub after_completion: bool,
    /// Monday = 0, Sunday = 6. Used only for selected weekdays.
    pub weekdays: Vec<u32>,
    /// Preserve e.g. January 31 → February 28 → March 31.
    pub month_day: u32,
}

impl Recurrence {
    pub fn validate(&self) -> Result<()> {
        if self.interval == 0 || self.interval > 999 {
            bail!("Repeat interval must be between 1 and 999.");
        }
        if !(1..=31).contains(&self.month_day) {
            bail!("Invalid monthly repeat day.");
        }
        if self.unit == RepeatUnit::Weekdays
            && (self.weekdays.is_empty() || self.weekdays.iter().any(|day| *day > 6))
        {
            bail!("Choose at least one weekday.");
        }
        Ok(())
    }

    /// Fixed schedules skip missed occurrences. Early completion advances once;
    /// completion-relative schedules advance from the actual completion date.
    pub fn next_date(&self, current: NaiveDate, completed: NaiveDate) -> Result<NaiveDate> {
        self.validate()?;
        let mut candidate = if self.after_completion {
            completed
        } else {
            current
        };
        let threshold = if self.after_completion {
            completed
        } else {
            current.max(completed)
        };
        loop {
            candidate = match self.unit {
                RepeatUnit::Days => candidate.checked_add_days(Days::new(self.interval.into())),
                RepeatUnit::Weeks => {
                    candidate.checked_add_days(Days::new(u64::from(self.interval) * 7))
                }
                RepeatUnit::Months => {
                    let first = candidate
                        .with_day(1)
                        .unwrap()
                        .checked_add_months(Months::new(self.interval.into()));
                    let target_day = if self.after_completion {
                        completed.day()
                    } else {
                        self.month_day
                    };
                    first.and_then(|first| {
                        (1..=target_day).rev().find_map(|day| first.with_day(day))
                    })
                }
                RepeatUnit::Weekdays => candidate.checked_add_days(Days::new(1)),
            }
            .ok_or_else(|| {
                anyhow::anyhow!("The next occurrence is outside the supported date range.")
            })?;
            let weekday_matches = self.unit != RepeatUnit::Weekdays
                || self
                    .weekdays
                    .contains(&candidate.weekday().num_days_from_monday());
            if candidate > threshold && weekday_matches {
                return Ok(candidate);
            }
        }
    }

    pub fn label(&self) -> String {
        if self.unit == RepeatUnit::Weekdays {
            let names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
            return self
                .weekdays
                .iter()
                .filter_map(|d| names.get(*d as usize))
                .copied()
                .collect::<Vec<_>>()
                .join(", ");
        }
        let unit = match self.unit {
            RepeatUnit::Days => "day",
            RepeatUnit::Weeks => "week",
            RepeatUnit::Months => "month",
            RepeatUnit::Weekdays => unreachable!(),
        };
        let span = format!(
            "{} {unit}{}",
            self.interval,
            if self.interval == 1 { "" } else { "s" }
        );
        if self.after_completion {
            format!("{span} after completion")
        } else {
            format!("Every {span}")
        }
    }
}
