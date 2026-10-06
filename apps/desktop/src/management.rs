use crate::app::{CatDo, CreateKind};
use crate::theme::{Accent, accent, project_color_index};
use gpui_kit::component::{ActiveTheme, Icon, IconName, Sizable, StyledExt};
use gpui_kit::{prelude::*, *};
use kirakira::button::{Button, ButtonVariants};
use kirakira::scroll::ScrollableElement;
use uuid::Uuid;

impl CatDo {
    fn space_row(
        &self,
        id: Uuid,
        name: String,
        archived: bool,
        project: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let p = cx.theme().color_tokens();
        div()
            .h_flex()
            .items_center()
            .gap_3()
            .px_4()
            .py_3()
            .when(!project, |el| el.bg(cx.theme().group_box))
            .when(project, |el| el.pl_8().border_t_1().border_color(p.border))
            .child(
                Icon::new(if project {
                    IconName::Folder
                } else {
                    IconName::LayoutDashboard
                })
                .size_4()
                .text_color(if project {
                    accent(Accent::Project(project_color_index(id)), cx)
                } else {
                    p.muted_foreground
                }),
            )
            .child(
                div()
                    .v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap_1()
                    .child(
                        div()
                            .font_weight(if project {
                                FontWeight::MEDIUM
                            } else {
                                FontWeight::BOLD
                            })
                            .child(name),
                    )
                    .when(archived, |el| {
                        el.child(
                            div()
                                .text_xs()
                                .text_color(p.muted_foreground)
                                .child("Archived"),
                        )
                    }),
            )
            .child(
                Button::new(SharedString::from(format!("rename-{id}")))
                    .small()
                    .ghost()
                    .label("Rename")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.start_named(CreateKind::Rename(id), window, cx)
                    })),
            )
            .child(
                Button::new(SharedString::from(format!("archive-{id}")))
                    .small()
                    .ghost()
                    .label(if archived { "Restore" } else { "Archive" })
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if this.change(
                            if archived { "Restored" } else { "Archived" },
                            |data| data.archive(id, !archived),
                            cx,
                        ) {
                            this.repair_sync_navigation();
                            this.refresh_workspace_select(window, cx);
                        }
                    })),
            )
    }

    pub fn render_management(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.theme().color_tokens();
        let update = self.render_update(cx).into_any_element();
        div()
            .id("manage-spaces")
            .size_full()
            .overflow_y_scrollbar()
            .bg(p.muted)
            .p_8()
            .child(
                div()
                    .v_flex()
                    .w_full()
                    .max_w(px(920.))
                    .mx_auto()
                    .gap_8()
                    .child(
                        div()
                            .v_flex()
                            .gap_1()
                            .child(self.render_heading("Settings".into(), None, false, cx))
                            .child(
                                div()
                                    .pl(rems(3.5))
                                    .text_sm()
                                    .font_medium()
                                    .text_color(p.muted_foreground)
                                    .child("Manage appearance, sync, and workspaces."),
                            ),
                    )
                    .child(
                        settings_section(
                            "Appearance",
                            "Choose how CatDo looks on this device.",
                            cx,
                        )
                        .child(
                            settings_panel(cx).child(
                                div()
                                    .h_flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_6()
                                    .child(
                                        div()
                                            .v_flex()
                                            .gap_1()
                                            .flex_1()
                                            .child(div().font_semibold().child("Color theme"))
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(p.muted_foreground)
                                                    .child(
                                                        "System follows your device’s appearance.",
                                                    ),
                                            ),
                                    )
                                    .child(self.render_theme_control(cx)),
                            ),
                        ),
                    )
                    .child(
                        settings_section(
                            "Account & sync",
                            "Keep your tasks available across devices.",
                            cx,
                        )
                        .child(settings_panel(cx).child(self.render_sync(cx))),
                    )
                    .child(
                        settings_section(
                            "Workspaces & projects",
                            "Archive spaces you no longer use. Your tasks and history stay safe.",
                            cx,
                        )
                        .children(self.data.workspaces.iter().map(
                            |workspace| {
                                div()
                                    .v_flex()
                                    .rounded(cx.theme().radius_lg)
                                    .overflow_hidden()
                                    .border_1()
                                    .border_color(p.border)
                                    .bg(cx.theme().popover)
                                    .child(self.space_row(
                                        workspace.id,
                                        workspace.name.clone(),
                                        workspace.archived,
                                        false,
                                        cx,
                                    ))
                                    .children(
                                        self.data
                                            .projects
                                            .iter()
                                            .filter(|project| project.workspace_id == workspace.id)
                                            .map(|project| {
                                                self.space_row(
                                                    project.id,
                                                    project.name.clone(),
                                                    project.archived,
                                                    true,
                                                    cx,
                                                )
                                            }),
                                    )
                            },
                        )),
                    )
                    .child(
                        settings_section("About CatDo", "App version and updates.", cx).child(
                            settings_panel(cx).child(
                                div()
                                    .h_flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_6()
                                    .child(
                                        div()
                                            .v_flex()
                                            .gap_1()
                                            .child(div().font_semibold().child("CatDo"))
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(p.muted_foreground)
                                                    .child(format!(
                                                        "Version {}",
                                                        env!("CARGO_PKG_VERSION")
                                                    )),
                                            ),
                                    )
                                    .child(update),
                            ),
                        ),
                    ),
            )
    }
}

fn settings_section(title: &'static str, description: &'static str, cx: &App) -> Div {
    div().v_flex().gap_3().child(
        div()
            .v_flex()
            .gap_1()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::EXTRA_BOLD)
                    .child(title),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(description),
            ),
    )
}

fn settings_panel(cx: &App) -> Div {
    let p = cx.theme().color_tokens();
    div()
        .v_flex()
        .p_5()
        .gap_4()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(p.border)
        .bg(cx.theme().popover)
}
