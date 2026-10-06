use catdo_core::Task;
use chrono::{Local, NaiveDate};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::{Icon, IconName, Sizable, StyledExt, list::ListItem};
use gpui_kit::{prelude::*, *};
use kirakira::ActiveKira as _;
use kirakira::bounce_text::BounceText;
use kirakira::burst::Burst;
use kirakira::button::{Button, ButtonVariants};
use kirakira::idle_cat::{IdleCat, Mood};
use kirakira::input::Input;
use kirakira::motion::Trigger;
use kirakira::reveal::{Reveal, RevealEffect};
use kirakira::scroll::ScrollableElement;
use kirakira::sparkles::Sparkles;

use crate::app::{CatDo, View};
use crate::motion::{LEAVE, check_pop, leave_progress, settle, settle_after};
use crate::theme::{Accent, accent, chip, fill, project_color_index};
use gpui_kit::assets::IconName as Lucide;

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

    /// The tasks the list shows: the visible ones, plus any just completed here that are still
    /// leaving, drawn as they were, with how far through [`LEAVE`] they are.
    pub(crate) fn listed_tasks(&self, cx: &App) -> Vec<(Task, Option<f32>)> {
        let today = Local::now().date_naive();
        let leaving = |id: &uuid::Uuid| {
            self.leaving
                .get(id)
                .is_some_and(|leaving| leaving.view == self.view)
        };
        let mut tasks = self
            .visible_tasks(cx)
            .into_iter()
            .filter(|task| !leaving(&task.id))
            .map(|task| (task, None))
            .chain(
                self.leaving
                    .values()
                    .filter(|leaving| leaving.view == self.view)
                    .map(|leaving| {
                        let t = leaving.started.elapsed().as_secs_f32() / LEAVE.as_secs_f32();
                        (leaving.task.clone(), Some(t.min(1.0)))
                    }),
            )
            .collect::<Vec<_>>();
        tasks.sort_by_key(|(t, _)| {
            (
                !t.overdue(today),
                t.scheduled.or(t.due).unwrap_or(NaiveDate::MAX),
                t.created_at,
            )
        });
        tasks
    }

    pub fn task_row(&self, task: Task, cx: &Context<Self>) -> impl IntoElement {
        self.task_row_at(task, 0, None, cx)
    }

    /// A round check that fills with the pink primary once the task is done. Completing it pops
    /// the check (Kirakira Checkbox's rebound) and fires a pink burst behind it.
    fn check(&self, task: &Task, done: bool, cx: &Context<Self>) -> impl IntoElement {
        let p = cx.theme().color_tokens();
        let id = task.id;
        let bursts = self.bursts.get(&id).copied().unwrap_or(0);
        let palette = cx.kira();
        let circle = div()
            .size(px(20.))
            .rounded_full()
            .border(px(1.5))
            .border_color(if done {
                accent(Accent::Done, cx)
            } else {
                p.muted_foreground.opacity(0.6)
            })
            .bg(if done {
                accent(Accent::Done, cx)
            } else {
                transparent_black()
            })
            .flex()
            .items_center()
            .justify_center()
            .text_color(p.primary_foreground)
            .when(done, |el| el.child(Icon::new(IconName::Check).size_3()));
        let circle = if done && bursts > 0 && !cx.reduce_motion() {
            circle
                .with_animation(
                    SharedString::from(format!("check-pop-{id}-{bursts}")),
                    Animation::new(std::time::Duration::from_millis(340)),
                    |el, t| el.size(px(20. * check_pop().sample(t))),
                )
                .into_any_element()
        } else {
            circle.into_any_element()
        };
        Burst::new(SharedString::from(format!("burst-{id}")))
            .fire(bursts)
            .colors([palette.pink, p.primary, palette.yellow, palette.pink])
            .text_size(px(12.))
            .size(0.72)
            .mt_2()
            .flex_shrink_0()
            .child(
                Button::new(SharedString::from(format!("complete-{id}")))
                    .ghost()
                    .w(px(28.))
                    .h(px(28.))
                    .p_0()
                    .rounded_full()
                    .accessibility_label(format!(
                        "{} {}",
                        if done { "Reopen" } else { "Complete" },
                        task.title
                    ))
                    .tooltip(if done { "Reopen task" } else { "Complete task" })
                    .child(
                        div()
                            .size(px(22.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(circle),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.leaving.contains_key(&id) {
                            this.complete_task(id, cx)
                        }
                    })),
            )
    }

    fn task_row_at(
        &self,
        task: Task,
        index: usize,
        leaving: Option<f32>,
        cx: &Context<Self>,
    ) -> AnyElement {
        let p = cx.theme().color_tokens();
        let done = leaving.is_some() || !task.active();
        // A leaving row keeps the same elements (so the check's burst keeps its state) and fades
        // its contents once the check has popped.
        let gone = leaving.map_or(0., |t| leave_progress(t, &cx.curves().r#in));
        let today = Local::now().date_naive();
        let id = task.id;
        let project = task
            .project_id
            .and_then(|id| self.data.projects.iter().find(|p| p.id == id))
            .map(|p| {
                (
                    p.name.clone(),
                    accent(Accent::Project(project_color_index(p.id)), cx),
                )
            });
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
        let check = self.check(&task, done, cx);
        let row = div()
            .id(SharedString::from(format!("task-{id}")))
            .h_flex()
            .items_start()
            .gap_2()
            .px_2()
            .py_1p5()
            .rounded(cx.theme().radius)
            .min_h(px(60.))
            .hover(|s| s.bg(cx.theme().list_hover))
            .child(
                div()
                    .h_flex()
                    .items_start()
                    .gap_2()
                    .flex_1()
                    .min_w_0()
                    .when(gone > 0., |el| {
                        el.opacity(1. - gone).relative().left(px(24. * gone))
                    })
                    .child(check)
                    .child(
                        ListItem::new(SharedString::from(format!("open-{id}")))
                            .role(gpui_kit::accesskit::Role::Button)
                            .aria_label(format!("Open {}", task.title))
                            .rounded(cx.theme().radius)
                            .py_2()
                            .px_1()
                            .text_size(px(16.))
                            .flex_1()
                            .min_w_0()
                            .cursor_pointer()
                            .on_click(cx.listener(move |this, _, window, cx| {
                                this.open_task(id, window, cx)
                            }))
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
                                                    .text_color(if done {
                                                        p.muted_foreground
                                                    } else {
                                                        p.foreground
                                                    })
                                                    .when(done, |el| el.line_through())
                                                    .font_medium()
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
                                                        .when_some(project, |el, (name, hue)| {
                                                            el.child(
                                                                div()
                                                                    .h_flex()
                                                                    .items_center()
                                                                    .gap_1p5()
                                                                    .flex_shrink_0()
                                                                    .max_w(px(160.))
                                                                    .child(
                                                                        div()
                                                                            .size(px(7.))
                                                                            .flex_shrink_0()
                                                                            .rounded_full()
                                                                            .bg(hue),
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .truncate()
                                                                            .child(name),
                                                                    ),
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
                                            .when(
                                                children > 0 || task.recurrence.is_some(),
                                                |el| {
                                                    el.child(
                                                        div()
                                                            .h_flex()
                                                            .gap_3()
                                                            .text_xs()
                                                            .text_color(p.muted_foreground)
                                                            .when(children > 0, |el| {
                                                                el.child(
                                                        div()
                                                            .h_flex()
                                                            .items_center()
                                                            .gap_1()
                                                            .child(
                                                                Icon::new(Lucide::ListTree)
                                                                    .size_3(),
                                                            )
                                                            .child(format!(
                                                                "{completed_children}/{children}"
                                                            )),
                                                    )
                                                            })
                                                            .when_some(
                                                                task.recurrence.clone(),
                                                                |el, rule| {
                                                                    el.child(
                                                                        div()
                                                                            .h_flex()
                                                                            .items_center()
                                                                            .gap_1()
                                                                            .text_color(accent(
                                                                                Accent::Repeat,
                                                                                cx,
                                                                            ))
                                                                            .child(
                                                                                Icon::new(
                                                                                    Lucide::Repeat,
                                                                                )
                                                                                .size_3(),
                                                                            )
                                                                            .child(rule.label()),
                                                                    )
                                                                },
                                                            ),
                                                    )
                                                },
                                            ),
                                    )
                                    .child(
                                        div()
                                            .v_flex()
                                            .items_end()
                                            .when(agenda, |el| el.items_start())
                                            .gap_1()
                                            .max_w(px(220.))
                                            .flex_shrink_0()
                                            .text_sm()
                                            .text_color(p.muted_foreground)
                                            .when_some(task.due, |el, due| {
                                                let late = due < today && task.active();
                                                let soon = due == today && task.active();
                                                let (tone, wash) = if late {
                                                    (
                                                        accent(Accent::Overdue, cx),
                                                        fill(Accent::Overdue, cx),
                                                    )
                                                } else if soon {
                                                    (
                                                        accent(Accent::Today, cx),
                                                        fill(Accent::Today, cx),
                                                    )
                                                } else {
                                                    (p.muted_foreground, p.muted)
                                                };
                                                el.child(
                                                    div()
                                                        .h_flex()
                                                        .items_center()
                                                        .gap_1()
                                                        .px_2()
                                                        .py_0p5()
                                                        .rounded_full()
                                                        .text_xs()
                                                        .bg(wash)
                                                        .text_color(tone)
                                                        .child(Icon::new(Lucide::Flag).size_3())
                                                        .font_medium()
                                                        .child(if task.overdue(today) {
                                                            format!(
                                                                "Overdue · {}",
                                                                friendly_date(due, today)
                                                            )
                                                        } else {
                                                            format!(
                                                                "Due {}",
                                                                friendly_date(due, today)
                                                            )
                                                        }),
                                                )
                                            })
                                            .when_some(scheduled, |el, date| {
                                                el.child(
                                                    div()
                                                        .h_flex()
                                                        .gap_1()
                                                        .items_center()
                                                        .px_2()
                                                        .py_0p5()
                                                        .rounded_full()
                                                        .text_xs()
                                                        .font_medium()
                                                        .bg(chip(Accent::Scheduled, cx))
                                                        .text_color(accent(Accent::Scheduled, cx))
                                                        .child(
                                                            Icon::new(Lucide::CalendarDays)
                                                                .size_3(),
                                                        )
                                                        .child(format!(
                                                            "Planned {}",
                                                            friendly_date(date, today)
                                                        )),
                                                )
                                            }),
                                    ),
                            ),
                    ),
            );
        settle_after(row, SharedString::from(format!("settle-{id}")), index, cx).into_any_element()
    }

    /// The view's heading: its icon on a Kirakira pastel tile that pops in, a title whose letters
    /// rise, and the task count. Both entrances replay when the view changes.
    pub(crate) fn render_heading(
        &self,
        title: String,
        count: Option<usize>,
        searching: bool,
        cx: &App,
    ) -> Div {
        let p = cx.theme().color_tokens();
        let (meaning, icon) = if searching {
            (None, Lucide::Search)
        } else {
            match self.view {
                View::Today => (Some(Accent::Today), Lucide::Sun),
                View::Inbox => (Some(Accent::Inbox), Lucide::Inbox),
                View::Upcoming => (Some(Accent::Upcoming), Lucide::CalendarDays),
                View::Calendar => (Some(Accent::Calendar), Lucide::Calendar),
                View::Completed => (Some(Accent::Done), Lucide::CircleCheck),
                View::Project(id) => (
                    Some(Accent::Project(project_color_index(id))),
                    Lucide::Folder,
                ),
                View::Manage => (None, Lucide::Settings),
            }
        };
        let (tone, wash) = meaning.map_or((p.primary, p.accent), |meaning| {
            (accent(meaning, cx), fill(meaning, cx))
        });
        let key = if searching {
            "search".to_string()
        } else if self.view == View::Calendar {
            format!("calendar-{}", self.month)
        } else {
            format!("{:?}", self.view)
        };
        let letters = title.chars().count().max(1) as u64;
        div()
            .h_flex()
            .items_center()
            .gap_3()
            .child(
                Reveal::new(SharedString::from(format!("heading-icon-{key}")))
                    .effect(RevealEffect::Pop)
                    .trigger(Trigger::Mount)
                    .delay(0)
                    .duration(450)
                    .child(
                        div()
                            .size(rems(2.75))
                            .rounded(cx.theme().radius)
                            .bg(wash)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(icon).size_5().text_color(tone)),
                    ),
            )
            .child(
                BounceText::new(SharedString::from(format!("heading-{key}")), title)
                    .trigger(Trigger::Mount)
                    .delay(40)
                    .stagger((240 / letters).clamp(12, 32))
                    .duration(450)
                    .text_3xl()
                    .font_weight(FontWeight::EXTRA_BOLD),
            )
            .when_some(count, |el, count| {
                el.child(
                    div()
                        .px_2p5()
                        .py_0p5()
                        .rounded_full()
                        .bg(p.muted)
                        .text_xs()
                        .font_semibold()
                        .text_color(p.muted_foreground)
                        .child(format!(
                            "{} {}",
                            count,
                            if count == 1 { "task" } else { "tasks" }
                        )),
                )
            })
    }

    pub fn render_task_list(
        &self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let p = cx.theme().color_tokens();
        if !self.leaving.is_empty() {
            window.request_animation_frame();
        }
        let today = Local::now().date_naive();
        let tasks = self.listed_tasks(cx);
        let query = self.search.read(cx).value().to_string();
        let searching = !query.is_empty();
        let (overdue, rest): (Vec<_>, Vec<_>) =
            tasks.iter().cloned().partition(|(t, _)| t.overdue(today));
        let overdue_len = overdue.len();
        let title = if searching {
            "Search results".into()
        } else {
            self.title()
        };
        let count = tasks
            .iter()
            .filter(|(_, leaving)| leaving.is_none())
            .count();
        let group = |label: &'static str, icon: Lucide, tone: Hsla, count: usize| {
            div()
                .h_flex()
                .items_center()
                .gap_2()
                .pb_2()
                .mb_1()
                .border_b_1()
                .border_color(p.border)
                .text_xs()
                .child(Icon::new(icon).size_3p5().text_color(tone))
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .text_color(tone)
                        .child(label),
                )
                .child(
                    div()
                        .text_color(p.muted_foreground)
                        .child(count.to_string()),
                )
        };
        div()
            .id("task-list-scroll")
            .size_full()
            .overflow_y_scrollbar()
            .p_8()
            .child(
                div()
                    .v_flex()
                    .w_full()
                    .max_w(px(1120.))
                    .mx_auto()
                    .py_4()
                    .child(self.render_heading(title, Some(count), searching, cx))
                    .when(self.view == View::Today && !searching, |el| {
                        el.child(
                            div()
                                .pl(rems(3.5))
                                .mt_1()
                                .text_sm()
                                .font_medium()
                                .text_color(p.muted_foreground)
                                .child(today.format("%A, %B %-d").to_string()),
                        )
                    })
                    .when(searching, |el| {
                        el.child(
                            div()
                                .pl(rems(3.5))
                                .mt_1()
                                .text_sm()
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
                                .child(group(
                                    "Overdue",
                                    Lucide::CircleAlert,
                                    accent(Accent::Overdue, cx),
                                    overdue.len(),
                                ))
                                .children(overdue.into_iter().enumerate().map(
                                    |(i, (task, leaving))| self.task_row_at(task, i, leaving, cx),
                                )),
                        )
                    })
                    .when(
                        self.view == View::Today && !rest.is_empty() && !searching,
                        |el| {
                            el.child(group(
                                "Today",
                                Lucide::Sun,
                                accent(Accent::Today, cx),
                                rest.len(),
                            ))
                        },
                    )
                    .children(rest.into_iter().enumerate().map(|(i, (task, leaving))| {
                        self.task_row_at(task, i + overdue_len, leaving, cx)
                    }))
                    .when(self.view != View::Completed && !searching, |el| {
                        el.child(
                            div()
                                .id("quick-add-row")
                                .h_flex()
                                .items_center()
                                .gap_2()
                                .mt_4()
                                .pl_3()
                                .pr_1p5()
                                .py_1p5()
                                .rounded(cx.theme().radius)
                                .border_1()
                                .border_color(transparent_black())
                                .hover(|s| s.bg(cx.theme().list_hover))
                                .when(!self.quick_add.read(cx).value().is_empty(), |el| {
                                    el.bg(cx.theme().popover).border_color(p.border)
                                })
                                .child(Input::new(&self.quick_add).appearance(false).prefix(
                                    Icon::new(IconName::Plus).size_4().text_color(p.primary),
                                ))
                                .child(
                                    Button::new("quick-add")
                                        .primary()
                                        .small()
                                        .label("Add task")
                                        .tooltip("Enter to add · Ctrl+N for task details")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            if this.quick_add.read(cx).value().trim().is_empty() {
                                                this.new_task(window, cx);
                                            } else {
                                                this.quick_create(window, cx);
                                            }
                                        })),
                                ),
                        )
                    })
                    .when(tasks.is_empty() && self.view != View::Completed, |el| {
                        el.child(self.render_empty(searching, cx))
                    })
                    .when(self.view == View::Completed && !searching, |el| {
                        el.child(self.render_history(cx))
                    }),
            )
    }

    /// Kirakira's Idle Cat stands in for CatDo's cat: asleep when the list is clear, awake and
    /// looking around when a search finds nothing.
    fn render_empty(&self, searching: bool, cx: &App) -> impl IntoElement {
        let p = cx.theme().color_tokens();
        let cat = IdleCat::new(if searching {
            "empty-cat-search"
        } else {
            "empty-cat"
        })
        .size(rems(8.5))
        .mood(if searching { Mood::Idle } else { Mood::Sleepy })
        .label(if searching {
            "A cat looking around"
        } else {
            "A cat napping"
        });
        settle(
            div()
                .v_flex()
                .items_center()
                .gap_2()
                .py_12()
                .child(if searching {
                    cat.into_any_element()
                } else {
                    Sparkles::new("empty-sparkles")
                        .count(5)
                        .duration(900)
                        .stagger(500)
                        .child(cat)
                        .into_any_element()
                })
                .child(
                    div()
                        .mt_3()
                        .text_lg()
                        .font_weight(FontWeight::EXTRA_BOLD)
                        .child(if searching {
                            "No matching tasks"
                        } else {
                            "A little breathing room."
                        }),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(p.muted_foreground)
                        .child(if searching {
                            "Try another word, or switch workspaces."
                        } else {
                            "Add something to do, or enjoy the clear space."
                        }),
                ),
            "empty-state",
            cx,
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
            .gap_1()
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_2()
                    .pb_2()
                    .border_b_1()
                    .border_color(p.border)
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(accent(Accent::Repeat, cx))
                    .child(Icon::new(Lucide::Repeat).size_3p5())
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
                    .h_flex()
                    .items_start()
                    .gap_3()
                    .px_2()
                    .py_2p5()
                    .rounded(cx.theme().radius)
                    .hover(|s| s.bg(cx.theme().list_hover))
                    .child(
                        div()
                            .mt_0p5()
                            .size(px(18.))
                            .flex_shrink_0()
                            .rounded_full()
                            .bg(accent(Accent::Done, cx))
                            .text_color(p.primary_foreground)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(Icon::new(IconName::Check).size_3()),
                    )
                    .child(
                        div()
                            .v_flex()
                            .gap_0p5()
                            .child(div().font_medium().child(h.task.title.clone()))
                            .child(
                                div().text_xs().text_color(p.muted_foreground).child(
                                    h.completed_at
                                        .with_timezone(&Local)
                                        .format("%b %-d, %Y · %H:%M")
                                        .to_string(),
                                ),
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
