use std::collections::HashSet;

use anyhow::{Result, bail, ensure};
use chrono::{DateTime, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Recurrence;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub archived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub archived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub workspace_id: Uuid,
    pub project_id: Option<Uuid>,
    pub parent_id: Option<Uuid>,
    pub title: String,
    pub notes: String,
    pub scheduled: Option<NaiveDate>,
    pub due: Option<NaiveDate>,
    pub recurrence: Option<Recurrence>,
    #[serde(default)]
    pub reminder: Option<Reminder>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reminder {
    pub at: DateTime<Utc>,
    pub delivered: bool,
}

impl Task {
    pub fn new(workspace_id: Uuid, title: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            workspace_id,
            title: title.into(),
            notes: String::new(),
            project_id: None,
            parent_id: None,
            scheduled: None,
            due: None,
            recurrence: None,
            reminder: None,
            completed_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn active(&self) -> bool {
        self.completed_at.is_none()
    }

    pub fn on_day(&self, day: NaiveDate) -> bool {
        self.active() && (self.scheduled == Some(day) || self.due == Some(day))
    }

    pub fn overdue(&self, today: NaiveDate) -> bool {
        self.active() && self.due.is_some_and(|due| due < today)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Occurrence {
    pub id: Uuid,
    pub task: Task,
    pub completed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Data {
    pub workspaces: Vec<Workspace>,
    pub projects: Vec<Project>,
    pub tasks: Vec<Task>,
    pub history: Vec<Occurrence>,
}

impl Data {
    pub fn initial() -> Self {
        Self {
            workspaces: vec![Workspace {
                id: Uuid::new_v4(),
                name: "Personal".into(),
                archived: false,
            }],
            ..Self::default()
        }
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.workspaces.iter().any(|w| !w.archived),
            "At least one active workspace is required."
        );
        let mut ids = HashSet::new();
        for workspace in &self.workspaces {
            ensure!(ids.insert(workspace.id), "Duplicate workspace ID.");
            ensure!(!workspace.name.trim().is_empty(), "Workspace needs a name.");
        }
        for project in &self.projects {
            ensure!(ids.insert(project.id), "Duplicate project ID.");
            ensure!(!project.name.trim().is_empty(), "Project needs a name.");
            ensure!(
                self.workspaces.iter().any(|w| w.id == project.workspace_id),
                "Project workspace does not exist."
            );
        }
        for task in &self.tasks {
            ensure!(ids.insert(task.id), "Duplicate task ID.");
            ensure!(!task.title.trim().is_empty(), "Give your task a title.");
            ensure!(
                self.workspaces.iter().any(|w| w.id == task.workspace_id),
                "Task workspace does not exist."
            );
            if let Some(project_id) = task.project_id {
                ensure!(
                    self.projects
                        .iter()
                        .any(|p| p.id == project_id && p.workspace_id == task.workspace_id),
                    "Project must belong to the task's workspace."
                );
            }
            let mut parent_id = task.parent_id;
            let mut ancestors = HashSet::from([task.id]);
            while let Some(id) = parent_id {
                ensure!(ancestors.insert(id), "A task cannot be its own ancestor.");
                let parent = self
                    .tasks
                    .iter()
                    .find(|t| t.id == id)
                    .ok_or_else(|| anyhow::anyhow!("Parent task does not exist."))?;
                ensure!(
                    parent.workspace_id == task.workspace_id
                        && parent.project_id == task.project_id,
                    "Subtasks must stay with their parent's project and workspace."
                );
                parent_id = parent.parent_id;
            }
            if let Some(rule) = &task.recurrence {
                rule.validate()?;
                ensure!(
                    task.scheduled.is_some() || task.due.is_some(),
                    "Set a scheduled date or due date before repeating a task."
                );
            }
        }
        Ok(())
    }

    pub fn rename(&mut self, id: Uuid, name: &str) -> Result<()> {
        let name = name.trim();
        ensure!(!name.is_empty(), "Enter a name.");
        if let Some(workspace) = self.workspaces.iter_mut().find(|w| w.id == id) {
            workspace.name = name.into();
        } else if let Some(project) = self.projects.iter_mut().find(|p| p.id == id) {
            project.name = name.into();
        } else {
            bail!("Workspace or project no longer exists.");
        }
        Ok(())
    }

    pub fn archive(&mut self, id: Uuid, archived: bool) -> Result<()> {
        if let Some(index) = self.workspaces.iter().position(|w| w.id == id) {
            ensure!(
                !archived || self.workspaces.iter().any(|w| w.id != id && !w.archived),
                "Keep at least one active workspace."
            );
            self.workspaces[index].archived = archived;
        } else if let Some(project) = self.projects.iter_mut().find(|p| p.id == id) {
            project.archived = archived;
        } else {
            bail!("Workspace or project no longer exists.");
        }
        Ok(())
    }

    pub fn visible_task(&self, task: &Task) -> bool {
        !self
            .workspaces
            .iter()
            .any(|w| w.id == task.workspace_id && w.archived)
            && !self
                .projects
                .iter()
                .any(|p| Some(p.id) == task.project_id && p.archived)
    }

    pub fn add_workspace(&mut self, name: &str) -> Result<Uuid> {
        let name = name.trim();
        ensure!(!name.is_empty(), "Enter a workspace name.");
        ensure!(
            !self
                .workspaces
                .iter()
                .any(|w| w.name.eq_ignore_ascii_case(name)),
            "That workspace already exists."
        );
        let id = Uuid::new_v4();
        self.workspaces.push(Workspace {
            id,
            name: name.into(),
            archived: false,
        });
        Ok(id)
    }

    pub fn add_project(&mut self, workspace_id: Uuid, name: &str) -> Result<Uuid> {
        let name = name.trim();
        ensure!(!name.is_empty(), "Enter a project name.");
        ensure!(
            self.workspaces.iter().any(|w| w.id == workspace_id),
            "Workspace does not exist."
        );
        ensure!(
            !self
                .projects
                .iter()
                .any(|p| p.workspace_id == workspace_id && p.name.eq_ignore_ascii_case(name)),
            "That project already exists in this workspace."
        );
        let id = Uuid::new_v4();
        self.projects.push(Project {
            id,
            workspace_id,
            name: name.into(),
            archived: false,
        });
        Ok(id)
    }

    pub fn save_task(&mut self, mut task: Task) -> Result<()> {
        task.title = task.title.trim().to_string();
        let mut next = self.clone();
        // Move the entire subtree when changing a parent's location.
        let descendants = self.descendants(task.id);
        for child in &mut next.tasks {
            if descendants.contains(&child.id) {
                child.workspace_id = task.workspace_id;
                child.project_id = task.project_id;
            }
        }
        if let Some(existing) = next.tasks.iter_mut().find(|t| t.id == task.id) {
            *existing = task;
        } else {
            next.tasks.push(task);
        }
        next.validate()?;
        *self = next;
        Ok(())
    }

    pub fn descendants(&self, id: Uuid) -> HashSet<Uuid> {
        let mut result = HashSet::new();
        let mut pending = vec![id];
        while let Some(parent) = pending.pop() {
            for child in self.tasks.iter().filter(|t| t.parent_id == Some(parent)) {
                if result.insert(child.id) {
                    pending.push(child.id);
                }
            }
        }
        result
    }

    pub fn remove_task(&mut self, id: Uuid) {
        let descendants = self.descendants(id);
        self.tasks
            .retain(|t| t.id != id && !descendants.contains(&t.id));
    }

    pub fn complete(&mut self, id: Uuid, at: DateTime<Utc>, local_day: NaiveDate) -> Result<()> {
        let task = self
            .tasks
            .iter()
            .find(|t| t.id == id)
            .ok_or_else(|| anyhow::anyhow!("Task no longer exists."))?
            .clone();
        if !task.active() {
            bail!("Task is already completed.");
        }
        let mut next = task.clone();
        if let Some(rule) = &task.recurrence {
            let anchor = task
                .scheduled
                .or(task.due)
                .ok_or_else(|| anyhow::anyhow!("Repeating tasks need a date."))?;
            let next_date = rule.next_date(anchor, local_day)?;
            let shift = next_date - anchor;
            next.scheduled = task
                .scheduled
                .map(|d| {
                    d.checked_add_signed(shift)
                        .ok_or_else(|| anyhow::anyhow!("Scheduled date is out of range."))
                })
                .transpose()?;
            next.due = task
                .due
                .map(|d| {
                    d.checked_add_signed(shift)
                        .ok_or_else(|| anyhow::anyhow!("Due date is out of range."))
                })
                .transpose()?;
            if let Some(reminder) = &mut next.reminder {
                reminder.at = crate::shift_reminder(reminder.at, shift, &Local)?;
                reminder.delivered = false;
            }
        } else {
            next.completed_at = Some(at);
        }
        self.save_task(next)?;
        self.history.push(Occurrence {
            id: Uuid::new_v4(),
            task,
            completed_at: at,
        });
        Ok(())
    }
}
