use catdo_core::Task;
use chrono::{Local, NaiveDate};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{
    Icon, IconName, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    input::Input,
};
use gpui_kit::{prelude::*, *};

use crate::app::{CatDo, View};

impl CatDo {
    pub fn visible_tasks(&self, cx: &App) -> Vec<Task> {
        let today = Local::now().date_naive();
        let query = self.search.read(cx).value().to_lowercase();
        let mut tasks = self
            .data
            .tasks
            .iter()
            .filter(|task| {
                if task.workspace_id != self.workspace_id {
                    return false;
                }
                if self.view != View::Completed && !self.data.visible_task(task) {
                    return false;
                }
                if !query.is_empty() {
                    return task.active()
                        && (task.title.to_lowercase().contains(&query)
                            || task.notes.to_lowercase().contains(&query));
                }
                if self.view == View::Completed {
                    return !task.active();
                }
                if !task.active() {
                    return false;
                }
                match self.view {
                    View::Inbox => task.project_id.is_none() && task.parent_id.is_none(),
                    View::Today => {
                        task.scheduled.is_some_and(|d| d <= today)
                            || task.due.is_some_and(|d| d <= today)
                    }
                    View::Upcoming => {
                        task.scheduled.is_some_and(|d| d > today)
                            || task.due.is_some_and(|d| d > today)
                    }
                    View::Project(id) => task.project_id == Some(id) && task.parent_id.is_none(),
                    View::Calendar => task.on_day(self.selected_day),
                    View::Completed | View::Manage => false,
                }
            })
            .cloned()
            .collect::<Vec<_>>();
        tasks.sort_by_key(|t| {
            (
                !t.overdue(today),
                t.scheduled.or(t.due).unwrap_or(NaiveDate::MAX),
                t.created_at,
            )
        });
        tasks
    }

    pub fn task_row(&self, task: Task, cx: &Context<Self>) -> impl IntoElement {
        let p = cx.theme().color_tokens();
        let today = Local::now().date_naive();
        let id = task.id;
        let project = task
            .project_id
            .and_then(|id| self.data.projects.iter().find(|p| p.id == id))
            .map(|p| p.name.clone());
        let children = self
            .data
            .tasks
            .iter()
            .filter(|t| t.parent_id == Some(id))
            .count();
        let completed_children = self
            .data
            .tasks
            .iter()
            .filter(|t| t.parent_id == Some(id) && !t.active())
            .count();
        div()
            .id(SharedString::from(format!("task-{id}")))
            .h_flex()
            .items_start()
            .gap_3()
            .px_2()
            .py_3()
            .border_b_1()
            .border_color(p.border)
            .hover(move |el| el.bg(p.muted))
            .rounded_sm()
            .child(
                Checkbox::new(SharedString::from(format!("complete-{id}")))
                    .small()
                    .mt_1()
                    .checked(!task.active())
                    .accessibility_label(format!(
                        "{} {}",
                        if task.active() { "Complete" } else { "Reopen" },
                        task.title
                    ))
                    .tooltip(if task.active() {
                        "Complete task"
                    } else {
                        "Reopen task"
                    })
                    .on_click(cx.listener(move |this, _, _, cx| this.complete_task(id, cx))),
            )
            .child(
                div()
                    .id(SharedString::from(format!("open-{id}")))
                    .v_flex()
                    .gap_1p5()
                    .flex_1()
                    .min_w_0()
                    .cursor_pointer()
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.open_task(id, window, cx)),
                    )
                    .child(
                        div()
                            .text_color(if task.active() {
                                p.foreground
                            } else {
                                p.muted_foreground
                            })
                            .child(task.title.clone()),
                    )
                    .child(
                        div()
                            .h_flex()
                            .flex_wrap()
                            .gap_3()
                            .text_xs()
                            .text_color(p.muted_foreground)
                            .when_some(project, |el, name| el.child(name))
                            .when_some(task.scheduled, |el, day| {
                                el.child(format!("Scheduled {}", friendly_date(day, today)))
                            })
                            .when_some(task.due, |el, due| {
                                el.child(
                                    div()
                                        .text_color(if due < today && task.active() {
                                            p.destructive
                                        } else {
                                            p.muted_foreground
                                        })
                                        .child(format!("Due {}", friendly_date(due, today))),
                                )
                            })
                            .when_some(task.recurrence, |el, repeat| {
                                el.child(format!("↻ {}", repeat.label()))
                            })
                            .when(children > 0, |el| {
                                el.child(format!("{completed_children}/{children} subtasks"))
                            })
                            .when(task.parent_id.is_some(), |el| el.child("Subtask")),
                    ),
            )
    }

    pub fn render_task_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.theme().color_tokens();
        let today = Local::now().date_naive();
        let tasks = self.visible_tasks(cx);
        let query = self.search.read(cx).value().to_string();
        let searching = !query.is_empty();
        let overdue = tasks
            .iter()
            .filter(|t| t.overdue(today))
            .cloned()
            .collect::<Vec<_>>();
        let rest = tasks
            .iter()
            .filter(|t| !t.overdue(today))
            .cloned()
            .collect::<Vec<_>>();
        let title = if searching {
            "Search results".into()
        } else {
            self.title()
        };
        let subtitle = if searching {
            format!(
                "{} {} matching “{}”",
                tasks.len(),
                if tasks.len() == 1 { "task" } else { "tasks" },
                query
            )
        } else {
            match self.view {
                View::Today => today.format("%A, %B %-d").to_string(),
                View::Inbox => "Capture now. Find a place for it later.".into(),
                View::Upcoming => "A little room to plan ahead.".into(),
                View::Completed => "A record of what you've taken care of.".into(),
                _ => format!(
                    "{} active {}",
                    tasks.len(),
                    if tasks.len() == 1 { "task" } else { "tasks" }
                ),
            }
        };
        div()
            .id("task-list-scroll")
            .size_full()
            .overflow_y_scrollbar()
            .px_8()
            .py_8()
            .child(
                div()
                    .v_flex()
                    .max_w(px(820.))
                    .mx_auto()
                    .child(div().text_3xl().font_weight(FontWeight::BOLD).child(title))
                    .child(
                        div()
                            .mt_2()
                            .mb_7()
                            .text_color(p.muted_foreground)
                            .child(subtitle),
                    )
                    .when(!overdue.is_empty(), |el| {
                        el.child(
                            div()
                                .v_flex()
                                .mb_5()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .text_color(p.destructive)
                                        .mb_2()
                                        .child("OVERDUE"),
                                )
                                .children(overdue.into_iter().map(|task| self.task_row(task, cx))),
                        )
                    })
                    .children(rest.into_iter().map(|task| self.task_row(task, cx)))
                    .when(self.view != View::Completed && !searching, |el| {
                        el.child(
                            div()
                                .h_flex()
                                .items_center()
                                .gap_3()
                                .mt_4()
                                .child(Icon::new(IconName::Plus).size_4().text_color(p.primary))
                                .child(Input::new(&self.quick_add).appearance(false))
                                .child(
                                    Button::new("quick-add")
                                        .ghost()
                                        .small()
                                        .label("Add")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.quick_create(window, cx)
                                        })),
                                ),
                        )
                    })
                    .when(tasks.is_empty() && self.view != View::Completed, |el| {
                        el.child(
                            div()
                                .v_flex()
                                .items_center()
                                .gap_3()
                                .py_16()
                                .text_color(p.muted_foreground)
                                .child(
                                    Icon::new(if searching {
                                        IconName::Search
                                    } else {
                                        IconName::Sun
                                    })
                                    .size_8(),
                                )
                                .child(div().text_base().text_color(p.foreground).child(
                                    if searching {
                                        "No matching tasks"
                                    } else {
                                        "A little space to breathe."
                                    },
                                ))
                                .child(if searching {
                                    "Try another word, or switch workspaces."
                                } else {
                                    "Start with one thing you'd like to get done."
                                }),
                        )
                    })
                    .when(self.view == View::Completed && !searching, |el| {
                        el.child(self.render_history(cx))
                    }),
            )
    }

    fn render_history(&self, cx: &Context<Self>) -> impl IntoElement {
        let p = cx.theme().color_tokens();
        let history = self
            .data
            .history
            .iter()
            .filter(|h| h.task.workspace_id == self.workspace_id && h.task.recurrence.is_some())
            .rev()
            .take(100)
            .collect::<Vec<_>>();
        div()
            .v_flex()
            .mt_8()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .text_color(p.muted_foreground)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("RECURRING COMPLETIONS"),
            )
            .when(history.is_empty(), |el| {
                el.child(
                    div()
                        .py_4()
                        .text_color(p.muted_foreground)
                        .child("Completed recurring tasks will appear here."),
                )
            })
            .children(history.into_iter().map(|h| {
                div()
                    .v_flex()
                    .py_3()
                    .gap_1()
                    .border_b_1()
                    .border_color(p.border)
                    .child(format!("✓  {}", h.task.title))
                    .child(
                        div().text_xs().text_color(p.muted_foreground).child(
                            h.completed_at
                                .with_timezone(&Local)
                                .format("%b %-d, %Y · %H:%M")
                                .to_string(),
                        ),
                    )
            }))
    }
}

pub fn friendly_date(date: NaiveDate, today: NaiveDate) -> String {
    if date == today {
        "today".into()
    } else if date == today.succ_opt().unwrap() {
        "tomorrow".into()
    } else {
        date.format("%b %-d").to_string()
    }
}
