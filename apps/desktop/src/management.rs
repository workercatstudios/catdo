use crate::app::{CatDo, CreateKind};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::{
    Sizable, StyledExt,
    button::{Button, ButtonVariants},
    scroll::ScrollableElement,
};
use gpui_kit::{prelude::*, *};
impl CatDo {
    pub fn render_management(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.theme().color_tokens();
        let mut rows = Vec::new();
        for workspace in &self.data.workspaces {
            rows.push((
                workspace.id,
                workspace.name.clone(),
                workspace.archived,
                false,
            ));
            for project in self
                .data
                .projects
                .iter()
                .filter(|p| p.workspace_id == workspace.id)
            {
                rows.push((project.id, project.name.clone(), project.archived, true));
            }
        }
        div()
            .id("manage-spaces")
            .v_flex()
            .size_full()
            .overflow_y_scrollbar()
            .p_8()
            .gap_4()
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Workspaces & projects"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(p.muted_foreground)
                    .child("Archives keep your tasks and history. Restore them whenever you need."),
            )
            .children(rows.into_iter().map(|(id, name, archived, project)| {
                div()
                    .h_flex()
                    .items_center()
                    .gap_3()
                    .py_2()
                    .border_b_1()
                    .border_color(p.border)
                    .when(project, |el| el.pl_6())
                    .child(div().flex_1().child(format!(
                        "{name}{}",
                        if archived { " · archived" } else { "" }
                    )))
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
            }))
    }
}
