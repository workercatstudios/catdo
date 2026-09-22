use std::collections::{HashMap, HashSet};

use catdo_core::{Data, Store};
use chrono::{Datelike, Local, NaiveDate};
use gpui_kit::component::{
    input::{InputEvent, InputState},
    select::{SelectEvent, SelectState},
};
use gpui_kit::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::editor::{TaskEditor, input};

actions!(
    catdo,
    [NewTask, Find, Undo, SaveTask, CloseEditor, TodayView, Quit]
);

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("ctrl-n", NewTask, Some("CatDo")),
        KeyBinding::new("ctrl-q", Quit, Some("CatDo")),
        KeyBinding::new("ctrl-f", Find, Some("CatDo")),
        KeyBinding::new("ctrl-z", Undo, Some("CatDo")),
        KeyBinding::new("ctrl-s", SaveTask, Some("CatDo")),
        KeyBinding::new("escape", CloseEditor, Some("CatDo")),
        KeyBinding::new("ctrl-1", TodayView, Some("CatDo")),
    ]);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum View {
    Inbox,
    Today,
    Upcoming,
    Calendar,
    Project(Uuid),
    Completed,
    Manage,
}

#[derive(Clone, Copy)]
pub enum CreateKind {
    Workspace,
    Project,
    Rename(Uuid),
}

#[derive(Serialize, Deserialize)]
struct Preferences {
    workspace_id: Uuid,
    views: HashMap<Uuid, (View, NaiveDate, NaiveDate)>,
}

pub struct CatDo {
    pub data: Data,
    pub(crate) update_state: crate::update_ui::UpdateState,
    pub(crate) appearance: crate::theme::Appearance,
    pub(crate) sync_status: String,
    pub(crate) sync_enabled: bool,
    pub(crate) sync_busy: bool,
    pub(crate) login_url: Option<String>,
    pub(crate) workspace_select_dirty: bool,
    pub(crate) store: Store,
    pub(crate) reminders_in_flight: HashSet<Uuid>,
    pub workspace_id: Uuid,
    pub view: View,
    views: HashMap<Uuid, (View, NaiveDate, NaiveDate)>,
    pub month: NaiveDate,
    pub selected_day: NaiveDate,
    pub search: Entity<InputState>,
    pub quick_add: Entity<InputState>,
    pub workspace_select: Entity<SelectState<Vec<SharedString>>>,
    pub editor: Option<Entity<TaskEditor>>,
    pub(crate) editor_subscription: Option<Subscription>,
    pub create_kind: Option<CreateKind>,
    pub name_input: Entity<InputState>,
    pub(crate) undo: Vec<(Data, String)>,
    pub message: Option<(String, bool)>,
    pub(crate) focus: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl CatDo {
    pub fn new(store: Store, data: Data, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let appearance = store
            .preference::<crate::theme::Appearance>("appearance")
            .ok()
            .flatten()
            .unwrap_or_default();
        appearance.apply(window, cx);
        let search = input("", "Search this workspace", window, cx);
        let quick_add = input("", "Add a task…", window, cx);
        let name_input = input("", "Name", window, cx);
        let workspace_select = cx.new(|cx| {
            SelectState::new(
                data.workspaces
                    .iter()
                    .filter(|w| !w.archived)
                    .map(|w| SharedString::from(w.name.clone()))
                    .collect::<Vec<_>>(),
                Some(gpui_kit::component::IndexPath::new(0)),
                window,
                cx,
            )
        });
        let today = Local::now().date_naive();
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        Self::start_reminders(cx);
        Self::start_sync(cx);
        let sync_enabled = store
            .preference::<bool>("sync_enabled")
            .ok()
            .flatten()
            .unwrap_or(false);
        let mut app = Self {
            appearance,
            update_state: Default::default(),
            workspace_id: data.workspaces.iter().find(|w| !w.archived).unwrap().id,
            data,
            sync_status: "Saved on this device".into(),
            sync_enabled,
            sync_busy: false,
            login_url: None,
            workspace_select_dirty: false,
            store,
            reminders_in_flight: HashSet::new(),
            view: View::Today,
            views: HashMap::new(),
            month: today.with_day(1).unwrap(),
            selected_day: today,
            search,
            quick_add,
            workspace_select,
            editor: None,
            editor_subscription: None,
            create_kind: None,
            name_input,
            undo: Vec::new(),
            message: None,
            focus,
            _subscriptions: Vec::new(),
        };
        if let Ok(Some(preferences)) = app.store.preference::<Preferences>("navigation") {
            app.views = preferences.views;
            if app
                .data
                .workspaces
                .iter()
                .any(|w| w.id == preferences.workspace_id)
            {
                app.workspace_id = preferences.workspace_id;
                if let Some((view, month, selected)) = app.views.get(&app.workspace_id).copied() {
                    app.view = match view {
                        View::Project(id)
                            if !app
                                .data
                                .projects
                                .iter()
                                .any(|p| p.id == id && p.workspace_id == app.workspace_id) =>
                        {
                            View::Inbox
                        }
                        other => other,
                    };
                    app.month = month;
                    app.selected_day = selected;
                }
                app.refresh_workspace_select(window, cx);
            }
        }
        app.bind_window(window, cx);
        app
    }

    fn bind_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self._subscriptions = vec![
            cx.observe_window_appearance(window, |this, window, cx| {
                if this.appearance == crate::theme::Appearance::System {
                    this.appearance.apply(window, cx);
                }
            }),
            cx.subscribe(&self.search, |_, _, _: &InputEvent, cx| cx.notify()),
            cx.subscribe_in(&self.quick_add, window, |this, _, event, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.quick_create(window, cx);
                }
            }),
            cx.subscribe_in(&self.name_input, window, |this, _, event, window, cx| {
                if matches!(event, InputEvent::PressEnter { .. }) {
                    this.create_named(window, cx);
                }
            }),
            cx.subscribe_in(
                &self.workspace_select,
                window,
                |this, _, event: &SelectEvent<Vec<SharedString>>, window, cx| {
                    if let SelectEvent::Confirm(Some(_)) = event
                        && let Some(id) = this
                            .workspace_select
                            .read(cx)
                            .selected_index(cx)
                            .and_then(|index| {
                                this.data
                                    .workspaces
                                    .iter()
                                    .filter(|w| !w.archived)
                                    .nth(index.row)
                            })
                            .map(|w| w.id)
                    {
                        this.switch_workspace(id, window, cx);
                    }
                },
            ),
        ];
        let entity = cx.entity().downgrade();
        window.on_window_should_close(cx, move |_, cx| {
            entity
                .update(cx, |this, cx| this.save_before_close(cx))
                .unwrap_or(true)
        });
    }

    pub(crate) fn reopen(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // GPUI subscriptions and input state belong to a window. Recreate those,
        // while retaining tasks, undo, sync work, reminders, and the staged update.
        let search = self.search.read(cx).value();
        let quick = self.quick_add.read(cx).value();
        let name = self.name_input.read(cx).value();
        self.search = input(&search, "Search this workspace", window, cx);
        self.quick_add = input(&quick, "Add a task…", window, cx);
        self.name_input = input(&name, "Name", window, cx);
        self.workspace_select =
            cx.new(|cx| SelectState::new(Vec::<SharedString>::new(), None, window, cx));
        self.refresh_workspace_select(window, cx);
        self.appearance.apply(window, cx);
        self.bind_window(window, cx);
        self.focus.focus(window, cx);
    }

    pub(crate) fn save_before_close(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.commit_editor(cx) {
            return false;
        }
        self.views.insert(
            self.workspace_id,
            (self.view, self.month, self.selected_day),
        );
        if let Err(error) = self.store.set_preference(
            "navigation",
            &Preferences {
                workspace_id: self.workspace_id,
                views: self.views.clone(),
            },
        ) {
            self.error(error.to_string(), cx);
            return false;
        }
        self.editor = None;
        self.editor_subscription = None;
        true
    }

    pub fn change(
        &mut self,
        label: &str,
        operation: impl FnOnce(&mut Data) -> anyhow::Result<()>,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut next = self.data.clone();
        let result = operation(&mut next).and_then(|_| self.store.save(&next));
        match result {
            Ok(()) => {
                self.undo.push((self.data.clone(), label.into()));
                if self.undo.len() > 30 {
                    self.undo.remove(0);
                }
                self.data = next;
                self.message = Some((label.into(), false));
                cx.notify();
                true
            }
            Err(error) => {
                self.error(error.to_string(), cx);
                false
            }
        }
    }

    pub fn error(&mut self, message: String, cx: &mut Context<Self>) {
        self.message = Some((message, true));
        cx.notify();
    }

    pub fn navigate(&mut self, view: View, window: &mut Window, cx: &mut Context<Self>) {
        if !self.commit_editor(cx) {
            return;
        }
        self.editor = None;
        self.view = view;
        self.search.update(cx, |s, cx| s.set_value("", window, cx));
        cx.notify();
    }

    fn switch_workspace(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        if id == self.workspace_id {
            return;
        }
        if !self.commit_editor(cx) {
            self.refresh_workspace_select(window, cx);
            return;
        }
        self.views.insert(
            self.workspace_id,
            (self.view, self.month, self.selected_day),
        );
        self.workspace_id = id;
        let today = Local::now().date_naive();
        (self.view, self.month, self.selected_day) = self.views.get(&id).copied().unwrap_or((
            View::Today,
            today.with_day(1).unwrap(),
            today,
        ));
        self.editor = None;
        self.search.update(cx, |s, cx| s.set_value("", window, cx));
        self.quick_add
            .update(cx, |s, cx| s.set_value("", window, cx));
        self.message = None;
        cx.notify();
    }

    pub(crate) fn refresh_workspace_select(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let items = self
            .data
            .workspaces
            .iter()
            .filter(|w| !w.archived)
            .map(|w| SharedString::from(w.name.clone()))
            .collect::<Vec<_>>();
        let index = self
            .data
            .workspaces
            .iter()
            .filter(|w| !w.archived)
            .position(|w| w.id == self.workspace_id)
            .unwrap_or(0);
        self.workspace_select.update(cx, |s, cx| {
            s.set_items(items, window, cx);
            s.set_selected_index(Some(gpui_kit::component::IndexPath::new(index)), window, cx);
        });
    }

    pub fn start_named(&mut self, kind: CreateKind, window: &mut Window, cx: &mut Context<Self>) {
        if !self.commit_editor(cx) {
            return;
        }
        self.editor = None;
        self.create_kind = Some(kind);
        let initial = match kind {
            CreateKind::Rename(id) => self
                .data
                .workspaces
                .iter()
                .find(|w| w.id == id)
                .map(|w| w.name.clone())
                .or_else(|| {
                    self.data
                        .projects
                        .iter()
                        .find(|p| p.id == id)
                        .map(|p| p.name.clone())
                })
                .unwrap_or_default(),
            _ => String::new(),
        };
        self.name_input.update(cx, |s, cx| {
            s.set_value(initial, window, cx);
            s.focus(window, cx);
        });
        cx.notify();
    }

    pub fn create_named(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(kind) = self.create_kind else {
            return;
        };
        let name = self.name_input.read(cx).value().to_string();
        let workspace = self.workspace_id;
        let mut created = None;
        if self.change(
            match kind {
                CreateKind::Workspace => "Workspace created",
                CreateKind::Project => "Project created",
                CreateKind::Rename(_) => "Name updated",
            },
            |data| {
                created = Some(match kind {
                    CreateKind::Workspace => data.add_workspace(&name)?,
                    CreateKind::Project => data.add_project(workspace, &name)?,
                    CreateKind::Rename(id) => {
                        data.rename(id, &name)?;
                        id
                    }
                });
                Ok(())
            },
            cx,
        ) {
            self.create_kind = None;
            let id = created.unwrap();
            match kind {
                CreateKind::Workspace => {
                    self.switch_workspace(id, window, cx);
                    self.refresh_workspace_select(window, cx);
                }
                CreateKind::Project => self.view = View::Project(id),
                CreateKind::Rename(_) => self.refresh_workspace_select(window, cx),
            }
            self.focus.focus(window, cx);
        }
    }

    pub fn title(&self) -> String {
        match self.view {
            View::Inbox => "Inbox".into(),
            View::Today => "Today".into(),
            View::Upcoming => "Upcoming".into(),
            View::Calendar => self.month.format("%B %Y").to_string(),
            View::Completed => "Completed".into(),
            View::Manage => "Workspaces & projects".into(),
            View::Project(id) => self
                .data
                .projects
                .iter()
                .find(|p| p.id == id)
                .map_or("Project".into(), |p| p.name.clone()),
        }
    }
}
