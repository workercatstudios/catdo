use catdo_core::*;
use chrono::{Datelike, NaiveDate, TimeZone, Utc};

fn day(s: &str) -> NaiveDate {
    s.parse().unwrap()
}
fn rule(unit: RepeatUnit, interval: u16, after_completion: bool, anchor: u32) -> Recurrence {
    Recurrence {
        unit,
        interval,
        after_completion,
        month_day: anchor,
        weekdays: vec![0, 2, 4],
    }
}

#[test]
fn month_end_recovers_original_day_and_skips_missed_occurrences() {
    let repeat = rule(RepeatUnit::Months, 1, false, 31);
    let feb = repeat
        .next_date(day("2026-01-31"), day("2026-01-31"))
        .unwrap();
    assert_eq!(feb, day("2026-02-28"));
    assert_eq!(repeat.next_date(feb, feb).unwrap(), day("2026-03-31"));
    assert_eq!(
        repeat.next_date(feb, day("2026-05-04")).unwrap(),
        day("2026-05-31")
    );
    assert_eq!(
        repeat
            .next_date(day("2028-01-31"), day("2028-01-31"))
            .unwrap(),
        day("2028-02-29")
    );
}

#[test]
fn fixed_and_completion_relative_schedules_are_different() {
    let fixed = rule(RepeatUnit::Weeks, 1, false, 1);
    let relative = rule(RepeatUnit::Weeks, 1, true, 1);
    assert_eq!(
        fixed
            .next_date(day("2026-09-21"), day("2026-09-23"))
            .unwrap(),
        day("2026-09-28")
    );
    assert_eq!(
        relative
            .next_date(day("2026-09-21"), day("2026-09-23"))
            .unwrap(),
        day("2026-09-30")
    );
    assert_eq!(
        fixed
            .next_date(day("2026-09-21"), day("2026-09-19"))
            .unwrap(),
        day("2026-09-28")
    );
}

#[test]
fn completion_relative_months_follow_the_actual_completion_day() {
    let repeat = rule(RepeatUnit::Months, 1, true, 31);
    assert_eq!(
        repeat
            .next_date(day("2026-01-31"), day("2026-02-10"))
            .unwrap(),
        day("2026-03-10")
    );
    assert_eq!(
        repeat
            .next_date(day("2026-01-01"), day("2026-01-31"))
            .unwrap(),
        day("2026-02-28")
    );
}

#[test]
fn recurring_reminders_preserve_wall_time_across_dst() {
    use chrono::TimeDelta;
    use chrono_tz::America::New_York;
    let before = New_York.with_ymd_and_hms(2026, 3, 7, 9, 0, 0).unwrap();
    let after = shift_reminder(before.with_timezone(&Utc), TimeDelta::days(1), &New_York).unwrap();
    assert_eq!(
        after.with_timezone(&New_York),
        New_York.with_ymd_and_hms(2026, 3, 8, 9, 0, 0).unwrap()
    );
    assert_eq!(after - before.with_timezone(&Utc), TimeDelta::hours(23));
    let before_gap = New_York.with_ymd_and_hms(2026, 3, 7, 2, 30, 0).unwrap();
    let after_gap = shift_reminder(
        before_gap.with_timezone(&Utc),
        TimeDelta::days(1),
        &New_York,
    )
    .unwrap();
    assert_eq!(
        after_gap.with_timezone(&New_York),
        New_York.with_ymd_and_hms(2026, 3, 8, 3, 0, 0).unwrap()
    );
}

#[test]
fn selected_weekdays_and_invalid_intervals() {
    let mut repeat = rule(RepeatUnit::Weekdays, 1, false, 1);
    let next = repeat
        .next_date(day("2026-09-25"), day("2026-09-25"))
        .unwrap();
    assert_eq!(next.weekday().num_days_from_monday(), 0);
    repeat.weekdays.clear();
    assert!(repeat.validate().is_err());
    repeat.unit = RepeatUnit::Days;
    repeat.interval = 0;
    assert!(repeat.validate().is_err());
}

#[test]
fn recurring_completion_preserves_deadline_offset_and_history() {
    let mut data = Data::initial();
    let mut task = Task::new(data.workspaces[0].id, "Release");
    task.scheduled = Some(day("2026-09-22"));
    task.due = Some(day("2026-09-25"));
    task.recurrence = Some(rule(RepeatUnit::Weeks, 1, false, 22));
    data.save_task(task.clone()).unwrap();
    let before = data.clone();
    data.complete(
        task.id,
        Utc.with_ymd_and_hms(2026, 9, 22, 18, 0, 0).unwrap(),
        day("2026-09-22"),
    )
    .unwrap();
    assert_eq!(data.tasks[0].scheduled, Some(day("2026-09-29")));
    assert_eq!(data.tasks[0].due, Some(day("2026-10-02")));
    assert!(data.tasks[0].active());
    assert_eq!(data.history[0].task, task);
    // Undo restores the whole command, including its completion history.
    data = before;
    assert!(data.history.is_empty());
    assert_eq!(data.tasks[0].scheduled, Some(day("2026-09-22")));
}

#[test]
fn moving_a_project_task_moves_its_subtasks_and_rejects_cycles() {
    let mut data = Data::initial();
    let personal = data.workspaces[0].id;
    let work = data.add_workspace("WorkerCat").unwrap();
    let project = data.add_project(work, "CatDo").unwrap();
    let parent = Task::new(personal, "Parent");
    let mut child = Task::new(personal, "Child");
    child.parent_id = Some(parent.id);
    data.save_task(parent.clone()).unwrap();
    data.save_task(child.clone()).unwrap();
    let mut moved = parent;
    moved.workspace_id = work;
    moved.project_id = Some(project);
    data.save_task(moved.clone()).unwrap();
    assert!(
        data.tasks
            .iter()
            .all(|t| t.workspace_id == work && t.project_id == Some(project))
    );
    moved.parent_id = Some(child.id);
    assert!(data.save_task(moved).is_err());
    let mut invalid = Task::new(personal, "Wrong workspace");
    invalid.project_id = Some(project);
    assert!(data.save_task(invalid).is_err());
}

#[test]
fn database_roundtrip_deletion_and_concurrent_writer_protection() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("catdo.sqlite3");
    let mut store = Store::open(&path).unwrap();
    let mut data = store.load().unwrap();
    let task = Task::new(data.workspaces[0].id, "Persist me");
    data.save_task(task.clone()).unwrap();
    store.save(&data).unwrap();
    let mut other = Store::open(&path).unwrap();
    let stale = other.load().unwrap();
    assert_eq!(data, stale);
    data.remove_task(task.id);
    store.save(&data).unwrap();
    assert!(other.save(&stale).is_err());
    assert!(other.load().unwrap().tasks.is_empty());
    let mut reopened = Store::open(&path).unwrap();
    assert_eq!(reopened.load().unwrap(), data);
}

#[test]
fn failed_validation_cannot_replace_saved_data() {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("db")).unwrap();
    let good = store.load().unwrap();
    let mut bad = good.clone();
    bad.tasks.push(Task::new(good.workspaces[0].id, "  "));
    assert!(store.save(&bad).is_err());
    assert_eq!(store.load().unwrap(), good);
}
