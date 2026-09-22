use catdo_core::{
    Data, Store, Task,
    sync::{Choice, Snapshot, merge},
};
fn setup() -> Data {
    let mut d = Data::initial();
    d.save_task(Task::new(d.workspaces[0].id, "Plan")).unwrap();
    d.save_task(Task::new(d.workspaces[0].id, "Walk")).unwrap();
    d
}
#[test]
fn merge_preserves_deletions_and_independent_edits() {
    let base = setup();
    let mut local = base.clone();
    let mut remote = base.clone();
    local.tasks[0].title = "Local".into();
    remote.tasks.pop();
    let (data, conflicts) = merge(&base, &local, &remote, None).unwrap();
    assert!(conflicts.is_empty());
    assert_eq!(data.tasks.len(), 1);
    assert_eq!(data.tasks[0].title, "Local");
}
#[test]
fn outbox_survives_restart_and_inflight_edits() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catdo.sqlite3");
    let mut store = Store::open(&path).unwrap();
    let data = setup();
    store.save(&data).unwrap();
    store.bind_account("user_a").unwrap();
    let pending = store.prepare_sync(&data).unwrap().unwrap();
    drop(store);
    let mut store = Store::open(&path).unwrap();
    let mut current = store.load().unwrap();
    assert_eq!(
        store.prepare_sync(&current).unwrap().unwrap().id,
        pending.id
    );
    current.tasks[0].title = "During upload".into();
    store.save(&current).unwrap();
    let accepted = store
        .accept_sync(
            &current,
            Snapshot {
                revision: 1,
                data: pending.data,
            },
            true,
        )
        .unwrap()
        .unwrap();
    assert_eq!(accepted.tasks[0].title, "During upload");
    assert!(store.prepare_sync(&accepted).unwrap().is_some());
    assert!(store.bind_account("user_b").is_err());
}
#[test]
fn conflict_is_durable_and_requires_a_choice() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catdo.sqlite3");
    let mut store = Store::open(&path).unwrap();
    let base = setup();
    store.save(&base).unwrap();
    store.bind_account("user_a").unwrap();
    store.prepare_sync(&base).unwrap();
    store
        .accept_sync(
            &base,
            Snapshot {
                revision: 1,
                data: base.clone(),
            },
            true,
        )
        .unwrap();
    let mut local = base.clone();
    local.tasks[0].title = "Local".into();
    store.save(&local).unwrap();
    let mut remote = base.clone();
    remote.tasks[0].title = "Remote".into();
    assert!(
        store
            .accept_sync(
                &local,
                Snapshot {
                    revision: 2,
                    data: remote
                },
                false
            )
            .unwrap()
            .is_none()
    );
    drop(store);
    let mut store = Store::open(&path).unwrap();
    let local = store.load().unwrap();
    assert!(store.prepare_sync(&local).is_err());
    let resolved = store.resolve_sync(&local, Choice::Remote).unwrap();
    assert_eq!(resolved.tasks[0].title, "Remote");
    assert!(store.sync_state().unwrap().unwrap().conflict.is_none());
}
#[test]
fn empty_desktop_adopts_existing_workspaces() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("db")).unwrap();
    let local = store.load().unwrap();
    store.bind_account("user_a").unwrap();
    assert!(store.prepare_sync(&local).unwrap().is_none());
    let mut remote = setup();
    remote.workspaces[0].name = "Work".into();
    let next = store
        .accept_sync(
            &local,
            Snapshot {
                revision: 1,
                data: remote,
            },
            false,
        )
        .unwrap()
        .unwrap();
    assert_eq!(next.workspaces.len(), 1);
    assert_eq!(next.workspaces[0].name, "Work");
}
#[test]
fn stale_window_cannot_upload_old_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let mut first = Store::open(&path).unwrap();
    let data = setup();
    first.save(&data).unwrap();
    first.bind_account("user_a").unwrap();
    let mut second = Store::open(&path).unwrap();
    let mut next = second.load().unwrap();
    next.tasks[0].title = "Newer window".into();
    second.save(&next).unwrap();
    assert!(first.prepare_sync(&data).is_err());
}
#[test]
fn concurrent_recurring_completion_keeps_one_history_entry() {
    use catdo_core::{Recurrence, RepeatUnit};
    use chrono::{NaiveDate, Utc};
    let mut base = setup();
    base.tasks[0].scheduled = Some(NaiveDate::from_ymd_opt(2026, 9, 22).unwrap());
    base.tasks[0].recurrence = Some(Recurrence {
        unit: RepeatUnit::Days,
        interval: 1,
        after_completion: false,
        weekdays: vec![],
        month_day: 22,
    });
    let mut local = base.clone();
    let mut remote = base.clone();
    local
        .complete(
            base.tasks[0].id,
            Utc::now(),
            base.tasks[0].scheduled.unwrap(),
        )
        .unwrap();
    remote
        .complete(
            base.tasks[0].id,
            Utc::now(),
            base.tasks[0].scheduled.unwrap(),
        )
        .unwrap();
    let (merged, conflicts) = merge(&base, &local, &remote, Some(Choice::Remote)).unwrap();
    assert_eq!(conflicts.len(), 1);
    assert_eq!(merged.history, remote.history);
}
