use catdo_core::{Data, RepeatUnit, Task};
use chrono::NaiveDate;
use gpui_kit::TestAppContext;
use gpui_kit::component::{IndexPath, calendar::Date};

use super::TaskEditor;

#[gpui_kit::test]
fn editor_keeps_dates_separate_and_builds_a_valid_recurrence(cx: &mut TestAppContext) {
    let mut data = Data::initial();
    let workspace = data.workspaces[0].id;
    let project = data.add_project(workspace, "Inbox").unwrap();
    let mut task = Task::new(workspace, "Prepare release");
    task.project_id = Some(project);
    data.save_task(task.clone()).unwrap();
    cx.update(gpui_kit::init);
    let window = cx.add_window(|window, cx| TaskEditor::new(task, &data, window, cx));
    window
        .update(cx, |editor, window, cx| {
            let scheduled: NaiveDate = "2026-09-22".parse().unwrap();
            let due: NaiveDate = "2026-09-25".parse().unwrap();
            editor.scheduled.update(cx, |state, cx| {
                state.set_date(Date::Single(Some(scheduled)), window, cx)
            });
            editor.due.update(cx, |state, cx| {
                state.set_date(Date::Single(Some(due)), window, cx)
            });
            editor.repeat.update(cx, |state, cx| {
                state.set_selected_index(Some(IndexPath::new(2)), window, cx)
            });
            editor.notes.update(cx, |state, cx| {
                state.set_value("First line\nSecond line", window, cx);
            });
            let saved = editor.task(cx).unwrap();
            assert_eq!(saved.notes, "First line\nSecond line");
            assert_eq!(saved.scheduled, Some(scheduled));
            assert_eq!(saved.due, Some(due));
            assert_eq!(saved.project_id, Some(project));
            assert_eq!(saved.recurrence.unwrap().unit, RepeatUnit::Weeks);
            editor.project.update(cx, |state, cx| {
                state.set_selected_index(Some(IndexPath::new(0)), window, cx)
            });
            assert_eq!(editor.task(cx).unwrap().project_id, None);
            editor
                .interval
                .update(cx, |state, cx| state.set_value("0", window, cx));
            assert!(editor.task(cx).is_err());
        })
        .unwrap();
}
