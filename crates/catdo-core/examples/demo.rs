//! Generate a separate, disposable demo database; never seed the real user store.
use anyhow::{Context, Result, ensure};
use catdo_core::{Data, Recurrence, RepeatUnit, Store, Task};
use chrono::{Datelike, Days, Local};

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let directory = std::path::PathBuf::from(
        args.next()
            .context("Usage: cargo run -p catdo-core --example demo -- DIRECTORY [calendar]")?,
    );
    let calendar = args.next().as_deref() == Some("calendar");
    let path = directory.join("catdo.sqlite3");
    ensure!(
        !path.exists(),
        "A database already exists in this directory; choose a new demo directory."
    );
    let mut store = Store::open(&path)?;
    let mut data = Data::initial();
    let workspace = data.add_workspace("WorkerCat")?;
    let catdo = data.add_project(workspace, "CatDo")?;
    let studio = data.add_project(workspace, "Studio")?;
    let today = Local::now().date_naive();
    for (title, offset, due_offset, project, notes) in [
        (
            "Sketch the first CatDo screens",
            0,
            Some(3),
            catdo,
            "Keep the task list quiet. Make the important actions easy to find.",
        ),
        (
            "Try the calendar with a real week",
            0,
            None,
            catdo,
            "Leave some room between commitments.",
        ),
        (
            "Choose artwork for the studio page",
            0,
            Some(0),
            studio,
            "A small touch of WorkerCat personality.",
        ),
        (
            "Write a short product introduction",
            1,
            Some(4),
            catdo,
            "Plain language. Show what the app actually does.",
        ),
        ("Plan the next asset pack", 3, None, studio, ""),
        (
            "Review the week",
            4,
            None,
            studio,
            "What got done? What should move to next week?",
        ),
    ] {
        let mut task = Task::new(workspace, title);
        task.project_id = Some(project);
        task.notes = notes.into();
        task.scheduled = Some(today.checked_add_days(Days::new(offset)).unwrap());
        task.due = due_offset.map(|offset| today.checked_add_days(Days::new(offset)).unwrap());
        if title == "Review the week" {
            task.recurrence = Some(Recurrence {
                unit: RepeatUnit::Weeks,
                interval: 1,
                after_completion: false,
                weekdays: vec![],
                month_day: task.scheduled.unwrap().day(),
            });
        }
        data.save_task(task)?;
    }
    store.save(&data)?;
    store.set_preference("navigation", &serde_json::json!({
        "workspace_id": workspace,
        "views": { workspace.to_string(): [if calendar { "Calendar" } else { "Today" }, today.with_day(1).unwrap(), today] }
    }))?;
    println!(
        "Demo created. Run: cargo run -p catdo-desktop -- --data-dir {}",
        directory.display()
    );
    Ok(())
}
