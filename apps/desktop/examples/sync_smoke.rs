//! Run against local Wrangler with a disposable Clerk development user:
//! CATDO_API_URL=http://127.0.0.1:8787 cargo run -p catdo-desktop --example sync_smoke
//! Authorize the printed Clerk device code in the same development browser session.
#[allow(dead_code)]
#[path = "../src/cloud.rs"]
mod cloud;
use anyhow::{Result, ensure};
use catdo_core::{Recurrence, RepeatUnit, Store, Task};
use chrono::{NaiveDate, Utc};
fn main() -> Result<()> {
    let api = cloud::api_url();
    ensure!(
        api == "http://127.0.0.1:8787",
        "This smoke test is restricted to the local development server."
    );
    // After editing the created task offline in the web app and reconnecting:
    // ... -- --verify-web USER_ID TASK_ID
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|arg| arg == "--verify-web") {
        ensure!(
            args.len() == 4,
            "Provide the development user ID and task ID."
        );
        let (remote, _) = cloud::sync(&api, &args[2], None)?;
        let id: uuid::Uuid = args[3].parse()?;
        let task = remote
            .data
            .tasks
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| anyhow::anyhow!("Task missing from server"))?;
        ensure!(
            task.title == "Web offline acceptance",
            "Web title did not sync"
        );
        ensure!(
            task.notes.contains("Edited after an offline reload"),
            "Offline notes did not sync"
        );
        ensure!(
            task.scheduled == Some(NaiveDate::from_ymd_opt(2026, 9, 23).unwrap())
                && task.due == Some(NaiveDate::from_ymd_opt(2026, 9, 25).unwrap())
                && task.recurrence.is_some(),
            "Web edits lost dates or recurrence"
        );
        ensure!(
            remote
                .data
                .history
                .iter()
                .filter(|h| h.task.id == id)
                .count()
                == 1,
            "Web edits lost completion history"
        );
        println!(
            "PASS: native client retrieved web offline edits with dates, recurrence and history intact."
        );
        return Ok(());
    }
    let login = cloud::begin_login(&api)?;
    println!(
        "Authorize code {} at {}",
        login.code.user_code,
        login
            .code
            .verification_uri_complete
            .as_deref()
            .unwrap_or(&login.code.verification_uri)
    );
    let owner = cloud::finish_login(login)?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("smoke.sqlite3");
    let mut store = Store::open(&path)?;
    let initial = store.load()?;
    store.bind_account(&owner)?;
    let (remote, _) = cloud::sync(&api, &owner, None)?;
    let mut data = store.accept_sync(&initial, remote, false)?.unwrap();
    let mut task = Task::new(data.workspaces[0].id, "Desktop sync acceptance");
    let id = task.id;
    task.scheduled = Some(NaiveDate::from_ymd_opt(2026, 9, 22).unwrap());
    task.due = Some(NaiveDate::from_ymd_opt(2026, 9, 24).unwrap());
    task.recurrence = Some(Recurrence {
        unit: RepeatUnit::Days,
        interval: 1,
        after_completion: false,
        weekdays: vec![],
        month_day: 22,
    });
    data.save_task(task)?;
    store.save(&data)?;
    let pending = store.prepare_sync(&data)?;
    let (remote, accepted) = cloud::sync(&api, &owner, pending)?;
    ensure!(accepted, "Unexpected conflict");
    data = store.accept_sync(&data, remote, true)?.unwrap();
    // Edit with no network calls, then reopen the database as after an offline restart.
    data.tasks.iter_mut().find(|t| t.id == id).unwrap().title = "Desktop offline acceptance".into();
    store.save(&data)?;
    drop(store);
    let mut store = Store::open(&path)?;
    let mut data = store.load()?;
    let pending = store.prepare_sync(&data)?.unwrap();
    let retry = pending.clone();
    let (remote, accepted) = cloud::sync(&api, &owner, Some(pending))?;
    ensure!(accepted, "Offline edit conflicted");
    data = store.accept_sync(&data, remote, true)?.unwrap();
    let (once, _) = cloud::sync(&api, &owner, Some(retry.clone()))?;
    let (twice, _) = cloud::sync(&api, &owner, Some(retry))?;
    ensure!(once.revision == twice.revision, "Retry duplicated a write");
    data.complete(
        id,
        Utc::now(),
        NaiveDate::from_ymd_opt(2026, 9, 22).unwrap(),
    )?;
    store.save(&data)?;
    let pending = store.prepare_sync(&data)?;
    let (remote, accepted) = cloud::sync(&api, &owner, pending)?;
    ensure!(accepted, "Completion conflicted");
    let data = store.accept_sync(&data, remote, true)?.unwrap();
    let task = data.tasks.iter().find(|t| t.id == id).unwrap();
    ensure!(
        task.scheduled == Some(NaiveDate::from_ymd_opt(2026, 9, 23).unwrap())
            && task.due == Some(NaiveDate::from_ymd_opt(2026, 9, 25).unwrap()),
        "Dates did not survive sync"
    );
    ensure!(
        data.history.iter().filter(|h| h.task.id == id).count() == 1,
        "History was lost or duplicated"
    );
    println!(
        "PASS: browser-authorized desktop sync, offline restart, lost-response retry, dates and recurring history. Task: {id}"
    );
    Ok(())
}
