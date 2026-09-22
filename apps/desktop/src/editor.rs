use anyhow::{Result, ensure};
use catdo_core::{Data, Recurrence, Reminder, RepeatUnit, Task};
use chrono::{Datelike, Local, NaiveTime, TimeZone, Utc};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{
    IconName, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    calendar::Date,
    date_picker::{DatePicker, DatePickerEvent, DatePickerState},
    input::{Input, InputEvent, InputState, Textarea, TextareaState},
    select::{Select, SelectEvent, SelectState},
};
use gpui_kit::{prelude::*, *};
use uuid::Uuid;

#[cfg(test)]
#[path = "editor_tests.rs"]
mod tests;

type Choices = Vec<SharedString>;

pub enum EditorEvent {
    Save,
    Delete(Uuid),
    Discard,
    Open(Uuid),
    AddSubtask(Uuid),
    Complete(Uuid),
}

pub struct TaskEditor {
    pub original: Task,
    data: Data,
    is_new: bool,
    title: Entity<InputState>,
    notes: Entity<TextareaState>,
    scheduled: Entity<DatePickerState>,
    due: Entity<DatePickerState>,
    reminder_day: Entity<DatePickerState>,
    reminder_time: Entity<InputState>,
    workspace: Entity<SelectState<Choices>>,
    project: Entity<SelectState<Choices>>,
    repeat: Entity<SelectState<Choices>>,
    interval: Entity<InputState>,
    weekdays: Vec<u32>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<EditorEvent> for TaskEditor {}

pub fn input(
    value: &str,
    placeholder: &str,
    window: &mut Window,
    cx: &mut App,
) -> Entity<InputState> {
    cx.new(|cx| {
        InputState::new(window, cx)
            .default_value(value.to_string())
            .placeholder(placeholder.to_string())
    })
}

fn choices(
    items: Vec<String>,
    selected: usize,
    window: &mut Window,
    cx: &mut App,
) -> Entity<SelectState<Choices>> {
    cx.new(|cx| {
        SelectState::new(
            items.into_iter().map(Into::into).collect(),
            Some(gpui_kit::component::IndexPath::new(selected)),
            window,
            cx,
        )
    })
}

impl TaskEditor {
    pub fn new(task: Task, data: &Data, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let title = input(&task.title, "Task name", window, cx);
        let notes = cx.new(|cx| {
            TextareaState::new(window, cx)
                .default_value(task.notes.clone())
                .placeholder("Add a little context…")
        });
        let scheduled = cx.new(|cx| {
            let mut state = DatePickerState::new(window, cx).date_format("%b %-d, %Y");
            state.set_date(Date::Single(task.scheduled), window, cx);
            state
        });
        let due = cx.new(|cx| {
            let mut state = DatePickerState::new(window, cx).date_format("%b %-d, %Y");
            state.set_date(Date::Single(task.due), window, cx);
            state
        });
        let reminder_day = cx.new(|cx| {
            let mut state = DatePickerState::new(window, cx).date_format("%b %-d, %Y");
            state.set_date(
                Date::Single(
                    task.reminder
                        .as_ref()
                        .map(|r| r.at.with_timezone(&Local).date_naive()),
                ),
                window,
                cx,
            );
            state
        });
        let reminder_time = input(
            &task.reminder.as_ref().map_or("09:00".into(), |r| {
                r.at.with_timezone(&Local).format("%H:%M").to_string()
            }),
            "HH:MM",
            window,
            cx,
        );
        let workspace = choices(
            data.workspaces.iter().map(|w| w.name.clone()).collect(),
            data.workspaces
                .iter()
                .position(|w| w.id == task.workspace_id)
                .unwrap_or(0),
            window,
            cx,
        );
        let projects = data
            .projects
            .iter()
            .filter(|p| p.workspace_id == task.workspace_id)
            .collect::<Vec<_>>();
        let project = choices(
            std::iter::once("Inbox".into())
                .chain(projects.iter().map(|p| p.name.clone()))
                .collect(),
            projects
                .iter()
                .position(|p| Some(p.id) == task.project_id)
                .map_or(0, |i| i + 1),
            window,
            cx,
        );
        let repeat_ix =
            task.recurrence
                .as_ref()
                .map_or(0, |r| match (r.unit, r.after_completion) {
                    (RepeatUnit::Days, false) => 1,
                    (RepeatUnit::Weeks, false) => 2,
                    (RepeatUnit::Months, false) => 3,
                    (RepeatUnit::Weekdays, _) => 4,
                    (RepeatUnit::Days, true) => 5,
                    (RepeatUnit::Weeks, true) => 6,
                    (RepeatUnit::Months, true) => 7,
                });
        let repeat = choices(
            [
                "Does not repeat",
                "Days · fixed schedule",
                "Weeks · fixed schedule",
                "Months · fixed schedule",
                "Selected weekdays",
                "Days after completion",
                "Weeks after completion",
                "Months after completion",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            repeat_ix,
            window,
            cx,
        );
        let interval = input(
            &task
                .recurrence
                .as_ref()
                .map_or(1, |r| r.interval)
                .to_string(),
            "1",
            window,
            cx,
        );
        let weekdays = task
            .recurrence
            .as_ref()
            .map_or(vec![0, 1, 2, 3, 4], |r| r.weekdays.clone());
        let subscriptions = vec![
            cx.subscribe_in(
                &workspace,
                window,
                |this, _, _: &SelectEvent<Choices>, window, cx| {
                    let workspace_id = this.workspace_id(cx);
                    let items: Choices = std::iter::once("Inbox".into())
                        .chain(
                            this.data
                                .projects
                                .iter()
                                .filter(|p| p.workspace_id == workspace_id)
                                .map(|p| p.name.clone().into()),
                        )
                        .collect();
                    this.project.update(cx, |p, cx| {
                        p.set_items(items, window, cx);
                        p.set_selected_index(
                            Some(gpui_kit::component::IndexPath::new(0)),
                            window,
                            cx,
                        );
                    });
                    cx.notify();
                },
            ),
            cx.subscribe(&repeat, |_, _, _: &SelectEvent<Choices>, cx| cx.notify()),
            cx.subscribe(&interval, |_, _, _: &InputEvent, cx| cx.notify()),
            cx.subscribe(&scheduled, |_, _, _: &DatePickerEvent, cx| cx.notify()),
            cx.subscribe(&due, |_, _, _: &DatePickerEvent, cx| cx.notify()),
        ];
        let is_new = !data.tasks.iter().any(|t| t.id == task.id);
        Self {
            original: task,
            data: data.clone(),
            is_new,
            title,
            notes,
            scheduled,
            due,
            reminder_day,
            reminder_time,
            workspace,
            project,
            repeat,
            interval,
            weekdays,
            _subscriptions: subscriptions,
        }
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.title.read(cx).focus_handle(cx).focus(window, cx);
    }

    fn workspace_id(&self, cx: &App) -> Uuid {
        self.workspace
            .read(cx)
            .selected_index(cx)
            .and_then(|index| self.data.workspaces.get(index.row))
            .map_or(self.original.workspace_id, |w| w.id)
    }

    fn repeat_index(&self, cx: &App) -> usize {
        self.repeat.read(cx).selected_index(cx).map_or(0, |p| p.row)
    }

    pub fn task(&self, cx: &App) -> Result<Task> {
        let mut task = self.original.clone();
        task.title = self.title.read(cx).value().trim().into();
        ensure!(
            !task.title.is_empty(),
            "Give your task a title before saving."
        );
        task.notes = self.notes.read(cx).value().to_string();
        task.workspace_id = self.workspace_id(cx);
        task.project_id = self
            .project
            .read(cx)
            .selected_index(cx)
            .and_then(|index| index.row.checked_sub(1))
            .and_then(|index| {
                self.data
                    .projects
                    .iter()
                    .filter(|p| p.workspace_id == task.workspace_id)
                    .nth(index)
            })
            .map(|p| p.id);
        if task.workspace_id != self.original.workspace_id
            || task.project_id != self.original.project_id
        {
            task.parent_id = None;
        }
        task.scheduled = match self.scheduled.read(cx).date() {
            Date::Single(date) => date,
            _ => None,
        };
        task.due = match self.due.read(cx).date() {
            Date::Single(date) => date,
            _ => None,
        };
        task.reminder = match self.reminder_day.read(cx).date() {
            Date::Single(Some(day)) => {
                let time = NaiveTime::parse_from_str(&self.reminder_time.read(cx).value(), "%H:%M")
                    .map_err(|_| {
                        anyhow::anyhow!("Use a reminder time like 09:30 (24-hour time).")
                    })?;
                let at = Local.from_local_datetime(&day.and_time(time)).single().ok_or_else(|| anyhow::anyhow!("That reminder time is ambiguous or unavailable because the clocks change. Choose another time."))?.with_timezone(&Utc);
                let delivered = self
                    .original
                    .reminder
                    .as_ref()
                    .is_some_and(|r| r.at == at && r.delivered);
                Some(Reminder { at, delivered })
            }
            _ => None,
        };
        let repeat = self.repeat_index(cx);
        task.recurrence = if repeat == 0 {
            None
        } else {
            let interval = if repeat == 4 {
                1
            } else {
                self.interval.read(cx).value().parse::<u16>().map_err(|_| {
                    anyhow::anyhow!("Repeat interval must be a whole number from 1 to 999.")
                })?
            };
            let anchor = task.scheduled.or(task.due).ok_or_else(|| {
                anyhow::anyhow!("Choose a scheduled date or due date for this repeating task.")
            })?;
            let same_anchor =
                task.scheduled.or(task.due) == self.original.scheduled.or(self.original.due);
            let rule = Recurrence {
                unit: match repeat {
                    1 | 5 => RepeatUnit::Days,
                    2 | 6 => RepeatUnit::Weeks,
                    3 | 7 => RepeatUnit::Months,
                    _ => RepeatUnit::Weekdays,
                },
                interval,
                after_completion: repeat >= 5,
                weekdays: self.weekdays.clone(),
                month_day: if same_anchor {
                    self.original
                        .recurrence
                        .as_ref()
                        .map_or(anchor.day(), |r| r.month_day)
                } else {
                    anchor.day()
                },
            };
            rule.validate()?;
            Some(rule)
        };
        Ok(task)
    }

    pub fn has_changes(&self, cx: &App) -> bool {
        self.task(cx)
            .map_or(true, |task| task != self.original || self.is_new)
    }

    fn field(&self, label: &str, control: impl IntoElement, cx: &App) -> impl IntoElement {
        div()
            .v_flex()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().muted_foreground)
                    .child(label.to_string()),
            )
            .child(control)
    }
}

impl Render for TaskEditor {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.theme().color_tokens();
        let repeat = self.repeat_index(cx);
        let next_occurrence = self.task(cx).ok().and_then(|task| {
            let rule = task.recurrence.as_ref()?;
            rule.next_date(task.scheduled.or(task.due)?, Local::now().date_naive())
                .ok()
        });
        let children = self
            .data
            .tasks
            .iter()
            .filter(|t| t.parent_id == Some(self.original.id))
            .cloned()
            .collect::<Vec<_>>();
        div().v_flex().w(px(350.)).min_w(px(350.)).h_full().bg(p.background).border_l_1().border_color(p.border)
            .child(div().h_flex().items_center().justify_between().px_5().h(px(62.)).border_b_1().border_color(p.border)
                .child(div().font_weight(FontWeight::SEMIBOLD).child(if self.is_new { "New task" } else { "Task details" }))
                .child(Button::new("discard").ghost().small().label("Discard").tooltip("Discard changes and close").on_click(cx.listener(|_, _, _, cx| cx.emit(EditorEvent::Discard)))))
            .child(div().id("editor-scroll").v_flex().flex_1().min_h_0().overflow_y_scrollbar().p_5().gap_5()
                .child(Input::new(&self.title))
                .when(!self.is_new, |el| el.child(Button::new("complete-editor").ghost().small().icon(IconName::CircleCheck).label(if self.original.active() { "Complete task" } else { "Reopen task" }).on_click(cx.listener(|this, _, _, cx| cx.emit(EditorEvent::Complete(this.original.id))))))
                .child(Textarea::new(&self.notes).h(px(100.)))
                .child(self.field("Workspace", Select::new(&self.workspace), cx))
                .child(self.field("Project", Select::new(&self.project), cx))
                .child(self.field("Scheduled · when you'll work on it", DatePicker::new(&self.scheduled).placeholder("Choose a day").cleanable(true), cx))
                .child(self.field("Due · when it must be finished", DatePicker::new(&self.due).placeholder("No deadline").cleanable(true), cx))
                .child(self.field("Reminder", DatePicker::new(&self.reminder_day).placeholder("No reminder").cleanable(true), cx))
                .child(self.field("Reminder time · your local time", Input::new(&self.reminder_time).w(px(100.)), cx))
                .child(div().text_xs().text_color(p.muted_foreground).child("Desktop reminders work while CatDo is open."))
                .child(self.field("Repeat", Select::new(&self.repeat), cx))
                .when(repeat != 0 && repeat != 4, |el| el.child(self.field("Every", Input::new(&self.interval).w(px(80.)), cx)))
                .when(repeat == 4, |el| el.child(div().h_flex().gap_1().children(["M", "T", "W", "T", "F", "S", "S"].into_iter().enumerate().map(|(i, label)| {
                    Button::new(("weekday", i)).small().label(label).when(self.weekdays.contains(&(i as u32)), |b| b.primary()).on_click(cx.listener(move |this, _, _, cx| {
                        if this.weekdays.contains(&(i as u32)) { this.weekdays.retain(|d| *d != i as u32); } else { this.weekdays.push(i as u32); this.weekdays.sort(); }
                        cx.notify();
                    }))
                }))))
                .when(repeat != 0, |el| el.child(div().text_xs().text_color(p.muted_foreground).child("Fixed schedules skip missed dates. Completing a task keeps its history and creates the next occurrence.")))
                .when_some(next_occurrence, |el, day| el.child(div().text_xs().text_color(p.primary).child(format!("If completed today, next: {}", day.format("%b %-d, %Y")))))
                .when(!self.is_new, |el| el.child(div().v_flex().gap_2().pt_3().border_t_1().border_color(p.border)
                    .child(div().text_xs().text_color(p.muted_foreground).child("SUBTASKS"))
                    .children(children.into_iter().map(|task| Button::new(SharedString::from(format!("child-{}", task.id))).ghost().label(format!("{} {}", if task.active() { "○" } else { "✓" }, task.title)).on_click(cx.listener(move |_, _, _, cx| cx.emit(EditorEvent::Open(task.id))))))
                    .child(Button::new("subtask").ghost().small().icon(IconName::Plus).label("Add subtask").on_click(cx.listener(|this, _, _, cx| cx.emit(EditorEvent::AddSubtask(this.original.id)))))))
            )
            .child(div().h_flex().justify_between().p_4().border_t_1().border_color(p.border)
                .child(Button::new("delete").ghost().label(if self.is_new { "Cancel" } else { "Delete task" }).on_click(cx.listener(|this, _, _, cx| {
                    if this.is_new { cx.emit(EditorEvent::Discard) } else { cx.emit(EditorEvent::Delete(this.original.id)) }
                })))
                .child(Button::new("save").primary().label("Save task").on_click(cx.listener(|_, _, _, cx| {
                    // The parent also validates and displays errors from the draft.
                    cx.emit(EditorEvent::Save);
                }))))
    }
}
