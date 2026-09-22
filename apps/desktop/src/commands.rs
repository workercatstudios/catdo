use crate::{
    app::{CatDo, View},
    editor::{EditorEvent, TaskEditor},
};
use catdo_core::Task;
use chrono::{Local, Utc};
use gpui_kit::*;
use uuid::Uuid;

impl CatDo {
    fn task_for_view(&self, title: String) -> Task {
        let mut task = Task::new(self.workspace_id, title);
        if let View::Project(id) = self.view {
            task.project_id = Some(id);
        }
        task.scheduled = match self.view {
            View::Today => Some(Local::now().date_naive()),
            View::Calendar => Some(self.selected_day),
            _ => None,
        };
        task
    }

    pub fn quick_create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let title = self.quick_add.read(cx).value().to_string();
        if title.trim().is_empty() {
            return;
        }
        let task = self.task_for_view(title);
        if self.change("Task added", |data| data.save_task(task), cx) {
            self.quick_add
                .update(cx, |s, cx| s.set_value("", window, cx));
        }
    }

    pub fn new_task(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.commit_editor(cx) {
            return;
        }
        self.show_editor(self.task_for_view(String::new()), window, cx);
    }

    pub fn open_task(&mut self, id: Uuid, window: &mut Window, cx: &mut Context<Self>) {
        if !self.commit_editor(cx) {
            return;
        }
        if let Some(task) = self.data.tasks.iter().find(|t| t.id == id).cloned() {
            self.show_editor(task, window, cx);
        }
    }

    fn show_editor(&mut self, task: Task, window: &mut Window, cx: &mut Context<Self>) {
        let editor = cx.new(|cx| TaskEditor::new(task, &self.data, window, cx));
        self.editor_subscription =
            Some(
                cx.subscribe_in(&editor, window, |this, _, event, window, cx| match event {
                    EditorEvent::Save => {
                        if this.commit_editor(cx) {
                            this.editor = None;
                            this.focus.focus(window, cx);
                            cx.notify();
                        }
                    }
                    EditorEvent::Discard => {
                        this.editor = None;
                        this.focus.focus(window, cx);
                        cx.notify();
                    }
                    EditorEvent::Delete(id) => {
                        if this.change(
                            "Task deleted",
                            |data| {
                                data.remove_task(*id);
                                Ok(())
                            },
                            cx,
                        ) {
                            this.editor = None;
                        }
                    }
                    EditorEvent::Open(id) => this.open_task(*id, window, cx),
                    EditorEvent::Complete(id) => this.complete_task(*id, cx),
                    EditorEvent::AddSubtask(id) => {
                        if !this.commit_editor(cx) {
                            return;
                        }
                        if let Some(parent) = this.data.tasks.iter().find(|t| t.id == *id) {
                            let mut task = Task::new(parent.workspace_id, "");
                            task.project_id = parent.project_id;
                            task.parent_id = Some(*id);
                            this.show_editor(task, window, cx);
                        }
                    }
                }),
            );
        editor.update(cx, |editor, cx| editor.focus(window, cx));
        self.editor = Some(editor);
        cx.notify();
    }

    pub fn commit_editor(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(editor) = &self.editor else {
            return true;
        };
        if !editor.read(cx).has_changes(cx) {
            return true;
        }
        match editor.read(cx).task(cx) {
            Ok(task) => {
                if self.change("Task saved", |data| data.save_task(task), cx) {
                    self.editor = None;
                    true
                } else {
                    false
                }
            }
            Err(error) => {
                self.error(error.to_string(), cx);
                false
            }
        }
    }

    pub fn complete_task(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if !self.commit_editor(cx) {
            return;
        }
        self.editor = None;
        let completed = self
            .data
            .tasks
            .iter()
            .find(|t| t.id == id)
            .is_some_and(|t| !t.active());
        self.change(
            if completed {
                "Task reopened"
            } else {
                "Task completed"
            },
            |data| {
                if completed {
                    if let Some(task) = data.tasks.iter_mut().find(|t| t.id == id) {
                        task.completed_at = None;
                    }
                    Ok(())
                } else {
                    data.complete(id, Utc::now(), Local::now().date_naive())
                }
            },
            cx,
        );
    }

    pub fn undo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .editor
            .as_ref()
            .is_some_and(|e| e.read(cx).has_changes(cx))
        {
            self.error("Save or discard your task edits before undoing.".into(), cx);
            return;
        }
        let Some((previous, label)) = self.undo.last().cloned() else {
            return;
        };
        match self.store.save(&previous) {
            Ok(()) => {
                self.data = previous;
                self.undo.pop();
                self.editor = None;
                if !self
                    .data
                    .workspaces
                    .iter()
                    .any(|w| w.id == self.workspace_id)
                {
                    self.workspace_id = self.data.workspaces[0].id;
                    self.view = View::Today;
                }
                if let View::Project(id) = self.view
                    && !self.data.projects.iter().any(|p| p.id == id)
                {
                    self.view = View::Inbox;
                }
                self.refresh_workspace_select(window, cx);
                self.message = Some((format!("Undone: {}", label.to_lowercase()), false));
                cx.notify();
            }
            Err(error) => self.error(error.to_string(), cx),
        }
    }
}
