#[cfg(not(windows))]
use std::process::Command;
use std::time::Duration;

use chrono::Utc;
use gpui_kit::*;

use crate::app::CatDo;

impl CatDo {
    pub fn start_reminders(cx: &mut Context<Self>) {
        cx.spawn(async move |entity, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(15))
                    .await;
                if entity
                    .update(cx, |this, cx| {
                        this.deliver_reminders(cx);
                        // Refresh date-sensitive views when the local day changes.
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    fn deliver_reminders(&mut self, cx: &mut Context<Self>) {
        let ready = self
            .data
            .tasks
            .iter()
            .filter(|task| {
                task.active()
                    && self.data.visible_task(task)
                    && !self.reminders_in_flight.contains(&task.id)
                    && task
                        .reminder
                        .as_ref()
                        .is_some_and(|r| !r.delivered && r.at <= Utc::now())
            })
            .cloned()
            .collect::<Vec<_>>();
        for task in ready {
            self.reminders_in_flight.insert(task.id);
            let title = task.title.clone();
            let workspace = self
                .data
                .workspaces
                .iter()
                .find(|w| w.id == task.workspace_id)
                .map_or("CatDo".into(), |w| w.name.clone());
            #[cfg(not(windows))]
            let delivery = cx.background_executor().spawn(async move {
                Command::new("notify-send")
                    .args([
                        "--app-name=CatDo",
                        "--",
                        &title,
                        &workspace
                            .replace('&', "&amp;")
                            .replace('<', "&lt;")
                            .replace('>', "&gt;"),
                    ])
                    .status()
                    .map(|s| s.success())
            });
            // GPUI posts a Windows toast and only logs delivery failures.
            #[cfg(windows)]
            let delivery = {
                cx.show_system_notification(SystemNotification {
                    tag: format!("reminder-{}", task.id).into(),
                    title: title.into(),
                    body: workspace.into(),
                    actions: Vec::new(),
                });
                Task::ready(std::io::Result::Ok(true))
            };
            cx.spawn(async move |entity, cx| {
                let result = delivery.await;
                let _ = entity.update(cx, |this, cx| {
                    this.reminders_in_flight.remove(&task.id);
                    if matches!(result, Ok(true)) {
                        let mut next = this.data.clone();
                        if let Some(current) = next.tasks.iter_mut().find(|t| t.id == task.id) {
                            // Do not mark a newly edited reminder as delivered.
                            if current.reminder == task.reminder {
                                if let Some(reminder) = &mut current.reminder { reminder.delivered = true; }
                                if let Err(error) = this.store.save(&next) { this.error(error.to_string(), cx); }
                                else { this.data = next; this.sync_now(cx); }
                            }
                        }
                    } else {
                        // Avoid retrying an unavailable notification service every tick.
                        this.reminders_in_flight.insert(task.id);
                        this.error("Desktop notification delivery failed. Check that notify-send is installed and desktop notifications are enabled; restart CatDo to retry.".into(), cx);
                    }
                });
            }).detach();
        }
    }
}
