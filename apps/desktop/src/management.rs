use crate::app::{CatDo, CreateKind};
use gpui_kit::component::{
    ActiveTheme, Icon, IconName, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    group_box::{GroupBox, GroupBoxVariants},
    scroll::ScrollableElement,
};
use gpui_kit::{prelude::*, *};
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
            .py_2()
            .when(project, |el| el.pl_3().border_t_1().border_color(p.border))
            .child(
                Icon::new(if project {
                    IconName::Folder
                } else {
                    IconName::LayoutDashboard
                })
                .size_4()
                .text_color(p.muted_foreground),
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
                                FontWeight::NORMAL
                            } else {
                                FontWeight::SEMIBOLD
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
        div().id("manage-spaces").size_full().overflow_y_scrollbar().p_8()
            .child(div().v_flex().max_w(px(800.)).mx_auto().gap_5()
                .child(div().v_flex().gap_2().mb_2()
                    .child(div().text_3xl().font_weight(FontWeight::SEMIBOLD).child("Workspaces & projects"))
                    .child(div().text_sm().text_color(p.muted_foreground)
                        .child("Archives keep your tasks and history. Restore them whenever you need.")))
                .children(self.data.workspaces.iter().map(|workspace| {
                    GroupBox::new().id(SharedString::from(format!("workspace-{}", workspace.id)))
                        .normal()
                        .content_style(div().gap_1().p_0().style().clone())
                        .child(self.space_row(workspace.id, workspace.name.clone(), workspace.archived, false, cx))
                        .children(self.data.projects.iter().filter(|p| p.workspace_id == workspace.id).map(|project| {
                            self.space_row(project.id, project.name.clone(), project.archived, true, cx)
                        }))
                })))
    }
}
