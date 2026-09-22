use chrono::Local;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::{
    Icon, IconName, Selectable, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    input::Input,
    select::Select,
};
use gpui_kit::{prelude::*, *};

use crate::app::{CatDo, CreateKind, View};

impl CatDo {
    fn nav_item(
        &self,
        id: impl Into<ElementId>,
        label: String,
        icon: IconName,
        view: View,
        count: usize,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        Button::new(id)
            .ghost()
            .selected(self.view == view)
            .w_full()
            .h(px(38.))
            .accessibility_label(label.clone())
            .child(
                div()
                    .h_flex()
                    .w_full()
                    .items_center()
                    .gap_3()
                    .child(Icon::new(icon).size_4())
                    .child(div().flex_1().text_left().child(label))
                    .when(count > 0, |el| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(count.to_string()),
                        )
                    }),
            )
            .on_click(cx.listener(move |this, _, window, cx| this.navigate(view, window, cx)))
    }

    pub fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.theme().color_tokens();
        let today = Local::now().date_naive();
        let tasks = self
            .data
            .tasks
            .iter()
            .filter(|t| {
                t.workspace_id == self.workspace_id && t.active() && self.data.visible_task(t)
            })
            .collect::<Vec<_>>();
        let inbox = tasks
            .iter()
            .filter(|t| t.project_id.is_none() && t.parent_id.is_none())
            .count();
        let today_count = tasks
            .iter()
            .filter(|t| {
                t.scheduled.is_some_and(|d| d <= today) || t.due.is_some_and(|d| d <= today)
            })
            .count();
        let projects = self
            .data
            .projects
            .iter()
            .filter(|p| p.workspace_id == self.workspace_id && !p.archived)
            .cloned()
            .collect::<Vec<_>>();
        div()
            .v_flex()
            .w(px(236.))
            .min_w(px(236.))
            .h_full()
            .px_3()
            .py_5()
            .bg(p.muted)
            .border_r_1()
            .border_color(p.border)
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .pb_5()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(FontWeight::BOLD)
                            .child("CatDo"),
                    )
                    .child(
                        div()
                            .pt_1()
                            .text_xs()
                            .text_color(p.muted_foreground)
                            .child("by WorkerCat"),
                    ),
            )
            .child(
                div()
                    .h_flex()
                    .gap_1()
                    .pb_4()
                    .child(Select::new(&self.workspace_select).w_full())
                    .child(
                        Button::new("new-workspace")
                            .ghost()
                            .icon(IconName::Plus)
                            .tooltip("New workspace")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.start_named(CreateKind::Workspace, window, cx)
                            })),
                    ),
            )
            .child(
                Input::new(&self.search)
                    .small()
                    .prefix(Icon::new(IconName::Search).size_3())
                    .cleanable(true),
            )
            .child(
                div()
                    .v_flex()
                    .gap_1()
                    .mt_5()
                    .child(self.nav_item(
                        "inbox",
                        "Inbox".into(),
                        IconName::Inbox,
                        View::Inbox,
                        inbox,
                        cx,
                    ))
                    .child(self.nav_item(
                        "today",
                        "Today".into(),
                        IconName::Sun,
                        View::Today,
                        today_count,
                        cx,
                    ))
                    .child(self.nav_item(
                        "upcoming",
                        "Upcoming".into(),
                        IconName::Calendar,
                        View::Upcoming,
                        0,
                        cx,
                    ))
                    .child(self.nav_item(
                        "calendar",
                        "Calendar".into(),
                        IconName::LayoutDashboard,
                        View::Calendar,
                        0,
                        cx,
                    )),
            )
            .child(
                div()
                    .h_flex()
                    .items_center()
                    .justify_between()
                    .mt_8()
                    .px_3()
                    .mb_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(p.muted_foreground)
                            .child("Projects"),
                    )
                    .child(
                        Button::new("new-project")
                            .ghost()
                            .xsmall()
                            .icon(IconName::Plus)
                            .tooltip("New project")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.start_named(CreateKind::Project, window, cx)
                            })),
                    ),
            )
            .child(
                div()
                    .id("projects-scroll")
                    .v_flex()
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scrollbar()
                    .gap_1()
                    .children(projects.iter().map(|project| {
                        self.nav_item(
                            SharedString::from(format!("project-{}", project.id)),
                            project.name.clone(),
                            IconName::Folder,
                            View::Project(project.id),
                            tasks
                                .iter()
                                .filter(|t| {
                                    t.project_id == Some(project.id) && t.parent_id.is_none()
                                })
                                .count(),
                            cx,
                        )
                    }))
                    .when(projects.is_empty(), |el| {
                        el.child(
                            div()
                                .px_3()
                                .py_2()
                                .text_xs()
                                .text_color(p.muted_foreground)
                                .child("A place for each project.\nAdd your first one with +."),
                        )
                    }),
            )
            .child(self.nav_item(
                "manage",
                "Manage spaces".into(),
                IconName::Folder,
                View::Manage,
                0,
                cx,
            ))
            .child(self.render_sync(cx))
            .child(self.nav_item(
                "completed",
                "Completed".into(),
                IconName::CircleCheck,
                View::Completed,
                0,
                cx,
            ))
            .child(self.render_theme_control(cx))
            .child(
                div()
                    .h_flex()
                    .justify_end()
                    .pt_2()
                    .child(self.render_update(cx)),
            )
    }
}
