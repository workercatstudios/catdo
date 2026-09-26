use catdo_core::Store;
use chrono::Local;
use gpui_kit::TestAppContext;

use crate::app::{CatDo, CreateKind, View};

#[gpui_kit::test]
fn initial_sign_in_requires_age_confirmation_without_changing_local_data(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("test.sqlite3")).unwrap();
    let data = store.load().unwrap();
    let original = data.clone();
    cx.update(gpui_kit::init);
    let window = cx.add_window(|window, cx| CatDo::new(store, data, window, cx));
    window
        .update(cx, |app, _, cx| {
            assert!(!app.sign_in_age_confirmed);
            app.sign_in(cx);
            assert!(!app.sync_busy);
            assert!(!app.sync_enabled);
            assert!(app.login_url.is_none());
            assert_eq!(app.data, original);
        })
        .unwrap();
}

#[gpui_kit::test]
fn capture_switch_complete_undo_and_restart(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("test.sqlite3");
    let mut store = Store::open(&path).unwrap();
    let data = store.load().unwrap();
    let personal = data.workspaces[0].id;
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::bind_keys(cx);
    });
    let window = cx.add_window(|window, cx| CatDo::new(store, data, window, cx));
    window
        .update(cx, |app, window, cx| {
            app.quick_add
                .update(cx, |input, cx| input.set_value("Plan CatDo", window, cx));
            app.quick_create(window, cx);
            assert_eq!(app.data.tasks.len(), 1);
            assert_eq!(app.data.tasks[0].scheduled, Some(Local::now().date_naive()));
            assert!(app.quick_add.read(cx).value().is_empty());
            let task_id = app.data.tasks[0].id;
            app.complete_task(task_id, cx);
            assert!(!app.data.tasks[0].active());
            app.undo(window, cx);
            assert!(app.data.tasks[0].active());
            assert!(app.data.history.is_empty());

            app.start_named(CreateKind::Workspace, window, cx);
            app.name_input
                .update(cx, |input, cx| input.set_value("WorkerCat", window, cx));
            app.create_named(window, cx);
            assert_ne!(app.workspace_id, personal);
            assert!(app.visible_tasks(cx).is_empty());
            app.start_named(CreateKind::Project, window, cx);
            app.name_input
                .update(cx, |input, cx| input.set_value("CatDo", window, cx));
            app.create_named(window, cx);
            assert!(matches!(app.view, View::Project(_)));
            app.quick_add.update(cx, |input, cx| {
                input.set_value("Build the desktop", window, cx)
            });
            app.quick_create(window, cx);
            assert_eq!(app.visible_tasks(cx).len(), 1);
            assert_eq!(app.visible_tasks(cx)[0].title, "Build the desktop");
        })
        .unwrap();
    let mut reopened = Store::open(&path).unwrap();
    let saved = reopened.load().unwrap();
    assert_eq!(saved.workspaces.len(), 2);
    assert_eq!(saved.projects.len(), 1);
    assert_eq!(saved.tasks.len(), 2);
}

#[gpui_kit::test]
fn invalid_draft_blocks_navigation_and_valid_draft_can_save(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("test.sqlite3")).unwrap();
    let data = store.load().unwrap();
    cx.update(|cx| {
        gpui_kit::init(cx);
    });
    let window = cx.add_window(|window, cx| CatDo::new(store, data, window, cx));
    window
        .update(cx, |app, window, cx| {
            app.new_task(window, cx);
            app.navigate(View::Calendar, window, cx);
            assert_eq!(app.view, View::Today);
            assert!(app.editor.is_some());
            assert!(app.message.as_ref().unwrap().1);
            app.editor = None;
            app.quick_add
                .update(cx, |input, cx| input.set_value("Calendar task", window, cx));
            app.quick_create(window, cx);
            let id = app.data.tasks[0].id;
            app.open_task(id, window, cx);
            assert!(app.commit_editor(cx));
            app.navigate(View::Calendar, window, cx);
            assert_eq!(app.view, View::Calendar);
            assert_eq!(app.visible_tasks(cx).len(), 1);
        })
        .unwrap();
}

#[gpui_kit::test]
fn keyboard_capture_and_save_render_through_the_real_root(cx: &mut TestAppContext) {
    use gpui_kit::test::TestWindowExt;
    use gpui_kit::{AppContext, SharedString};
    let temp = tempfile::tempdir().unwrap();
    let mut store = Store::open(&temp.path().join("test.sqlite3")).unwrap();
    let data = store.load().unwrap();
    cx.update(|cx| {
        gpui_kit::init(cx);
        crate::app::bind_keys(cx);
    });
    let mut app = None;
    let window = cx.add_window(|window, cx| {
        let entity = cx.new(|cx| CatDo::new(store, data, window, cx));
        app = Some(entity.clone());
        gpui_kit::component::Root::new(entity, window, cx)
    });
    let app = app.unwrap();
    cx.simulate_keystrokes(*window, "ctrl-n");
    cx.simulate_input(*window, "A task entered with the keyboard");
    cx.simulate_keystrokes(*window, "ctrl-s");
    window
        .update(cx, |_, window, cx| {
            app.update(cx, |app, cx| {
                assert!(app.editor.is_none(), "The editor should close after saving");
                assert_eq!(app.data.tasks.len(), 1);
                assert_eq!(app.data.tasks[0].title, "A task entered with the keyboard");
                app.navigate(View::Calendar, window, cx);
            });
        })
        .unwrap();
    cx.run_until_parked();
    cx.simulate_keystrokes(*window, "ctrl-1");
    window
        .update(cx, |_, _, cx| assert_eq!(app.read(cx).view, View::Today))
        .unwrap();
    let task_id = cx.read_entity(&app, |app, _| app.data.tasks[0].id);
    cx.update_window(window.into(), |_, window, cx| {
        window.render_frame(cx);
        window.click(SharedString::from(format!("complete-{task_id}")), cx);
        assert!(!app.read(cx).data.tasks[0].active());
    })
    .unwrap();
    cx.simulate_keystrokes(*window, "ctrl-z");
    cx.read_entity(&app, |app, _| assert!(app.data.tasks[0].active()));
}
