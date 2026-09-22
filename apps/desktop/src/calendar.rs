use chrono::{Datelike, Days, Local, Months, NaiveDate};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{
    IconName, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    input::Input,
};
use gpui_kit::{prelude::*, *};
use uuid::Uuid;

use crate::app::CatDo;

#[derive(Clone)]
struct DragTask {
    id: Uuid,
    title: String,
}

impl Render for DragTask {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.theme().color_tokens();
        div()
            .px_3()
            .py_2()
            .bg(p.accent)
            .text_color(p.primary)
            .border_1()
            .border_color(p.border)
            .rounded_md()
            .child(self.title.clone())
    }
}

impl CatDo {
    fn month_step(&mut self, forward: bool, cx: &mut Context<Self>) {
        self.month = if forward {
            self.month.checked_add_months(Months::new(1))
        } else {
            self.month.checked_sub_months(Months::new(1))
        }
        .unwrap_or(self.month);
        cx.notify();
    }

    fn calendar_day(&self, date: NaiveDate, cx: &Context<Self>) -> impl IntoElement {
        let p = cx.theme().color_tokens();
        let today = Local::now().date_naive();
        let tasks = self
            .data
            .tasks
            .iter()
            .filter(|t| {
                t.workspace_id == self.workspace_id && self.data.visible_task(t) && t.on_day(date)
            })
            .cloned()
            .collect::<Vec<_>>();
        let total = tasks.len();
        div()
            .id(SharedString::from(format!("day-{date}")))
            .v_flex()
            .flex_1()
            .min_w_0()
            .h(px(88.))
            .p_2()
            .gap_1()
            .border_b_1()
            .border_r_1()
            .border_color(p.border)
            .cursor_pointer()
            .bg(if date == self.selected_day {
                p.muted
            } else {
                p.background
            })
            .drag_over::<DragTask>(move |el, _, _, _| el.bg(p.accent))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected_day = date;
                cx.notify();
            }))
            .on_drop(cx.listener(move |this, drag: &DragTask, _, cx| {
                if !this.commit_editor(cx) {
                    return;
                }
                this.change(
                    "Task rescheduled · deadline unchanged",
                    |data| {
                        let mut task = data
                            .tasks
                            .iter()
                            .find(|t| t.id == drag.id)
                            .cloned()
                            .ok_or_else(|| anyhow::anyhow!("Task no longer exists."))?;
                        task.scheduled = Some(date);
                        if let Some(rule) = &mut task.recurrence {
                            rule.month_day = date.day();
                        }
                        data.save_task(task)
                    },
                    cx,
                );
                this.selected_day = date;
            }))
            .child(
                div()
                    .h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(24.))
                            .h(px(24.))
                            .rounded_full()
                            .text_xs()
                            .text_color(if date.month() != self.month.month() {
                                p.muted_foreground
                            } else {
                                p.foreground
                            })
                            .when(date == today, |el| {
                                el.bg(p.primary)
                                    .text_color(p.background)
                                    .font_weight(FontWeight::BOLD)
                            })
                            .child(date.day().to_string()),
                    )
                    .when(total > 2, |el| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(p.muted_foreground)
                                .child(format!("+{}", total - 2)),
                        )
                    }),
            )
            .children(tasks.into_iter().take(2).map(|task| {
                let id = task.id;
                let due = task.due == Some(date);
                let scheduled = task.scheduled == Some(date);
                let label = if due && scheduled {
                    format!("• ◆ {}", task.title)
                } else if due {
                    format!("◆ {}", task.title)
                } else {
                    task.title.clone()
                };
                div()
                    .id(SharedString::from(format!("calendar-{date}-{id}")))
                    .px_1p5()
                    .py_0p5()
                    .rounded_sm()
                    .text_xs()
                    .truncate()
                    .text_color(if due { p.destructive } else { p.primary })
                    .bg(p.muted)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.open_task(id, window, cx);
                    }))
                    .when(scheduled, |el| {
                        el.on_drag(
                            DragTask {
                                id,
                                title: task.title,
                            },
                            |drag, _, _, cx| cx.new(|_| drag.clone()),
                        )
                    })
                    .child(label)
            }))
    }

    pub fn render_calendar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.theme().color_tokens();
        let start = self
            .month
            .checked_sub_days(Days::new(
                self.month.weekday().num_days_from_monday().into(),
            ))
            .unwrap();
        let last = self
            .month
            .checked_add_months(Months::new(1))
            .unwrap()
            .pred_opt()
            .unwrap();
        let week_count =
            (self.month.weekday().num_days_from_monday() + last.day()).div_ceil(7) as u64;
        let agenda = self
            .data
            .tasks
            .iter()
            .filter(|t| {
                t.workspace_id == self.workspace_id
                    && self.data.visible_task(t)
                    && t.on_day(self.selected_day)
            })
            .cloned()
            .collect::<Vec<_>>();
        div()
            .id("calendar-scroll")
            .size_full()
            .overflow_y_scrollbar()
            .px_6()
            .py_6()
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .justify_between()
                    .mb_5()
                    .child(
                        div()
                            .v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_2xl()
                                    .font_weight(FontWeight::BOLD)
                                    .child(self.title()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(p.muted_foreground)
                                    .child("Your plans, with room to move."),
                            ),
                    )
                    .child(
                        div()
                            .h_flex()
                            .gap_1()
                            .child(
                                Button::new("previous-month")
                                    .ghost()
                                    .small()
                                    .icon(IconName::ChevronLeft)
                                    .tooltip("Previous month")
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.month_step(false, cx)),
                                    ),
                            )
                            .child(
                                Button::new("current-month")
                                    .small()
                                    .label("Today")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        let today = Local::now().date_naive();
                                        this.month = today.with_day(1).unwrap();
                                        this.selected_day = today;
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Button::new("next-month")
                                    .ghost()
                                    .small()
                                    .icon(IconName::ChevronRight)
                                    .tooltip("Next month")
                                    .on_click(
                                        cx.listener(|this, _, _, cx| this.month_step(true, cx)),
                                    ),
                            ),
                    ),
            )
            .child(div().h_flex().mb_2().children(
                ["MON", "TUE", "WED", "THU", "FRI", "SAT", "SUN"].map(|day| {
                    div()
                        .flex_1()
                        .px_2()
                        .text_xs()
                        .text_color(p.muted_foreground)
                        .child(day)
                }),
            ))
            .child(
                div()
                    .v_flex()
                    .border_t_1()
                    .border_l_1()
                    .border_color(p.border)
                    .children((0..week_count).map(|week| {
                        div().h_flex().children((0..7).map(|day| {
                            self.calendar_day(
                                start.checked_add_days(Days::new(week * 7 + day)).unwrap(),
                                cx,
                            )
                        }))
                    })),
            )
            .child(
                div()
                    .h_flex()
                    .gap_4()
                    .mt_3()
                    .mb_6()
                    .text_xs()
                    .text_color(p.muted_foreground)
                    .child("• Scheduled")
                    .child(div().text_color(p.destructive).child("◆ Deadline"))
                    .child("Drag scheduled tasks to move your plan."),
            )
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::SEMIBOLD)
                    .mb_3()
                    .child(self.selected_day.format("%A, %B %-d").to_string()),
            )
            .when(agenda.is_empty(), |el| {
                el.child(
                    div()
                        .text_color(p.muted_foreground)
                        .py_3()
                        .child("Nothing planned for this day yet."),
                )
            })
            .children(agenda.into_iter().map(|task| self.task_row(task, cx)))
            .child(
                div()
                    .h_flex()
                    .mt_3()
                    .gap_2()
                    .child(Input::new(&self.quick_add).appearance(false))
                    .child(
                        Button::new("calendar-add")
                            .ghost()
                            .small()
                            .icon(IconName::Plus)
                            .label("Add")
                            .on_click(
                                cx.listener(|this, _, window, cx| this.quick_create(window, cx)),
                            ),
                    ),
            )
    }
}
