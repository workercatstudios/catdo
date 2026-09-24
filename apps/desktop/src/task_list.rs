use catdo_core::Task;
use chrono::{Local, NaiveDate};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{
    Icon, IconName, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    checkbox::Checkbox,
    input::Input,
    list::ListItem,
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
        let scheduled = task.scheduled.filter(|date| match self.view {
            View::Today => *date != today,
            View::Calendar => *date != self.selected_day,
            _ => true,
        });
        let agenda = self.view == View::Calendar;
        let notes = task
            .notes
            .lines()
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
        div()
            .id(SharedString::from(format!("task-{id}")))
            .h_flex()
            .items_start()
            .gap_2()
            .min_h(px(52.))
            .child(
                Checkbox::new(SharedString::from(format!("complete-{id}")))
                    .large()
                    .mt_3()
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
                ListItem::new(SharedString::from(format!("open-{id}")))
                    .role(gpui_kit::accesskit::Role::Button)
                    .aria_label(format!("Open {}", task.title))
                    .rounded(px(6.))
                    .py_2p5()
                    .px_1()
                    .text_size(px(15.))
                    .flex_1()
                    .min_w_0()
                    .cursor_pointer()
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.open_task(id, window, cx)),
                    )
                    .child(
                        div()
                            .h_flex()
                            .items_start()
                            .gap_4()
                            .w_full()
                            .when(agenda, |el| el.flex_col().gap_1())
                            .child(
                                div()
                                    .v_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .when(agenda, |el| el.w_full().flex_none())
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_color(if task.active() {
                                                p.foreground
                                            } else {
                                                p.muted_foreground
                                            })
                                            .child(task.title.clone()),
                                    )
                                    .when(!notes.is_empty() || project.is_some(), |el| {
                                        el.child(
                                            div()
                                                .h_flex()
                                                .w_full()
                                                .min_w_0()
                                                .overflow_hidden()
                                                .gap_3()
                                                .text_xs()
                                                .text_color(p.muted_foreground)
                                                .when_some(project, |el, project| {
                                                    el.child(
                                                        div()
                                                            .flex_shrink_0()
                                                            .max_w(px(140.))
                                                            .truncate()
                                                            .child(project),
                                                    )
                                                })
                                                .when(!notes.is_empty(), |el| {
                                                    el.child(
                                                        div()
                                                            .flex_1()
                                                            .min_w_0()
                                                            .truncate()
                                                            .child(notes),
                                                    )
                                                }),
                                        )
                                    })
                                    .when(children > 0 || task.recurrence.is_some(), |el| {
                                        el.child(
                                            div()
                                                .h_flex()
                                                .gap_3()
                                                .text_xs()
                                                .text_color(p.muted_foreground)
                                                .when(children > 0, |el| {
                                                    el.child(format!(
                                                        "{completed_children}/{children} subtasks"
                                                    ))
                                                })
                                                .when_some(task.recurrence.clone(), |el, rule| {
                                                    el.child(format!("↻ {}", rule.label()))
                                                }),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .v_flex()
                                    .items_end()
                                    .when(agenda, |el| el.items_start())
                                    .gap_1()
                                    .max_w(px(155.))
                                    .text_xs()
                                    .text_color(p.muted_foreground)
                                    .when_some(task.due, |el, due| {
                                        el.child(
                                            div()
                                                .h_flex()
                                                .items_center()
                                                .gap_1()
                                                .text_color(if due < today && task.active() {
                                                    p.destructive
                                                } else {
                                                    p.muted_foreground
                                                })
                                                .child(Icon::new(IconName::Calendar).size_3())
                                                .child(friendly_date(due, today)),
                                        )
                                    })
                                    .when_some(scheduled, |el, date| {
                                        el.child(
                                            div()
                                                .h_flex()
                                                .gap_1()
                                                .items_center()
                                                .child(Icon::new(IconName::Calendar).size_3())
                                                .child(friendly_date(date, today)),
                                        )
                                    }),
                            ),
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
        let icon = if searching {
            IconName::Search
        } else {
            match self.view {
                View::Today => IconName::Sun,
                View::Inbox => IconName::Inbox,
                View::Upcoming | View::Calendar => IconName::Calendar,
                View::Completed => IconName::CircleCheck,
                _ => IconName::Folder,
            }
        };
        div()
            .id("task-list-scroll")
            .size_full()
            .overflow_y_scrollbar()
            .px_10()
            .py_10()
            .child(
                div()
                    .v_flex()
                    .max_w(px(800.))
                    .mx_auto()
                    .child(
                        div()
                            .h_flex()
                            .items_center()
                            .gap_3()
                            .child(Icon::new(icon).size_6().text_color(
                                if self.view == View::Today && !searching && !cx.theme().is_dark() {
                                    rgb(0xB1843D).into()
                                } else {
                                    p.primary
                                },
                            ))
                            .child(div().text_3xl().font_semibold().child(title))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(p.muted_foreground)
                                    .child(tasks.len().to_string()),
                            ),
                    )
                    .when(self.view == View::Today && !searching, |el| {
                        el.child(
                            div()
                                .pl_9()
                                .mt_2()
                                .text_xs()
                                .text_color(p.muted_foreground)
                                .child(today.format("%A, %B %-d").to_string()),
                        )
                    })
                    .when(searching, |el| {
                        el.child(
                            div()
                                .pl_9()
                                .mt_2()
                                .text_xs()
                                .text_color(p.muted_foreground)
                                .child(format!("Matching “{query}”")),
                        )
                    })
                    .child(div().h_8())
                    .when(!overdue.is_empty(), |el| {
                        el.child(
                            div()
                                .v_flex()
                                .mb_6()
                                .child(
                                    div()
                                        .h_flex()
                                        .items_center()
                                        .gap_2()
                                        .pb_2()
                                        .border_b_1()
                                        .border_color(p.border)
                                        .text_xs()
                                        .child(
                                            div()
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(p.destructive)
                                                .child("Overdue"),
                                        )
                                        .child(
                                            div()
                                                .text_color(p.muted_foreground)
                                                .child(overdue.len().to_string()),
                                        ),
                                )
                                .children(overdue.into_iter().map(|task| self.task_row(task, cx))),
                        )
                    })
                    .when(
                        self.view == View::Today && !rest.is_empty() && !searching,
                        |el| {
                            el.child(
                                div()
                                    .h_flex()
                                    .gap_2()
                                    .pb_2()
                                    .border_b_1()
                                    .border_color(p.border)
                                    .text_xs()
                                    .child(div().font_weight(FontWeight::SEMIBOLD).child("Today"))
                                    .child(
                                        div()
                                            .text_color(p.muted_foreground)
                                            .child(rest.len().to_string()),
                                    ),
                            )
                        },
                    )
                    .children(rest.into_iter().map(|task| self.task_row(task, cx)))
                    .when(self.view != View::Completed && !searching, |el| {
                        el.child(
                            div()
                                .h_flex()
                                .items_center()
                                .gap_2()
                                .mt_3()
                                .child(Input::new(&self.quick_add).appearance(false).prefix(
                                    Icon::new(IconName::Plus).size_4().text_color(p.primary),
                                ))
                                .when(!self.quick_add.read(cx).value().trim().is_empty(), |el| {
                                    el.child(
                                        Button::new("quick-add")
                                            .ghost()
                                            .small()
                                            .label("Add")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.quick_create(window, cx)
                                            })),
                                    )
                                }),
                        )
                    })
                    .when(tasks.is_empty() && self.view != View::Completed, |el| {
                        el.child(
                            div()
                                .v_flex()
                                .items_center()
                                .gap_2()
                                .py_16()
                                .child(
                                    Icon::new(if searching {
                                        IconName::Search
                                    } else {
                                        IconName::CircleCheck
                                    })
                                    .size_8()
                                    .text_color(p.muted_foreground),
                                )
                                .child(if searching {
                                    "No matching tasks"
                                } else {
                                    "All clear for now"
                                })
                                .child(div().text_sm().text_color(p.muted_foreground).child(
                                    if searching {
                                        "Try another word, or switch workspaces."
                                    } else {
                                        "Add a task when you're ready."
                                    },
                                )),
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
                    .child("Recurring completions"),
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
