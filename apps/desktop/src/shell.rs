use crate::app::{CatDo, CloseEditor, CreateKind, Find, NewTask, SaveTask, TodayView, Undo, View};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::{
    IconName, Sizable, StyledExt,
    button::{Button, ButtonVariants},
    input::Input,
};
use gpui_kit::{prelude::*, *};

impl Render for CatDo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.workspace_select_dirty {
            self.workspace_select_dirty = false;
            self.refresh_workspace_select(window, cx);
        }
        let p = cx.theme().color_tokens();
        let workspace = self
            .data
            .workspaces
            .iter()
            .find(|w| w.id == self.workspace_id)
            .map(|w| w.name.clone())
            .unwrap_or_default();
        let searching = !self.search.read(cx).value().is_empty();
        div()
            .id("catdo")
            .key_context("CatDo")
            .track_focus(&self.focus)
            .size_full()
            .h_flex()
            .bg(p.background)
            .text_color(p.foreground)
            .text_sm()
            .on_action(cx.listener(|this, _: &NewTask, window, cx| this.new_task(window, cx)))
            .on_action(cx.listener(|this, _: &Find, window, cx| {
                this.search.read(cx).focus_handle(cx).focus(window, cx)
            }))
            .on_action(cx.listener(|this, _: &Undo, window, cx| this.undo(window, cx)))
            .on_action(cx.listener(|this, _: &SaveTask, window, cx| {
                if this.commit_editor(cx) {
                    this.editor = None;
                    this.focus.focus(window, cx);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &CloseEditor, window, cx| {
                if this.commit_editor(cx) {
                    this.editor = None;
                    this.create_kind = None;
                    this.focus.focus(window, cx);
                    cx.notify();
                }
            }))
            .on_action(
                cx.listener(|this, _: &TodayView, window, cx| {
                    this.navigate(View::Today, window, cx)
                }),
            )
            .child(self.render_sidebar(cx))
            .child(
                div()
                    .v_flex()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .child(
                        div()
                            .h_flex()
                            .h(px(62.))
                            .flex_shrink_0()
                            .items_center()
                            .justify_between()
                            .px_8()
                            .border_b_1()
                            .border_color(p.border)
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(p.muted_foreground)
                                    .child(format!(
                                        "{workspace}   /   {}",
                                        if searching {
                                            "Search".into()
                                        } else {
                                            self.title()
                                        }
                                    )),
                            )
                            .child(
                                Button::new("new-task")
                                    .primary()
                                    .small()
                                    .icon(IconName::Plus)
                                    .label("Add task")
                                    .tooltip("Ctrl+N")
                                    .on_click(
                                        cx.listener(|this, _, window, cx| {
                                            this.new_task(window, cx)
                                        }),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .when(self.view == View::Calendar && !searching, |el| {
                                el.child(self.render_calendar(cx))
                            })
                            .when(self.view == View::Manage && !searching, |el| {
                                el.child(self.render_management(cx))
                            })
                            .when(
                                (self.view != View::Calendar && self.view != View::Manage)
                                    || searching,
                                |el| el.child(self.render_task_list(cx)),
                            ),
                    )
                    .child(
                        div()
                            .h_flex()
                            .min_h(px(38.))
                            .px_6()
                            .py_2()
                            .items_center()
                            .gap_3()
                            .border_t_1()
                            .border_color(p.border)
                            .text_xs()
                            .text_color(p.muted_foreground)
                            .child(
                                div()
                                    .flex_1()
                                    .when_some(self.message.clone(), |el, (message, error)| {
                                        el.text_color(if error {
                                            p.destructive
                                        } else {
                                            p.muted_foreground
                                        })
                                        .child(message)
                                    })
                                    .when(self.message.is_none(), |el| {
                                        el.child("Saved on this device")
                                    }),
                            )
                            .when(!self.undo.is_empty(), |el| {
                                el.child(
                                    Button::new("undo")
                                        .ghost()
                                        .xsmall()
                                        .label("Undo")
                                        .tooltip("Ctrl+Z")
                                        .on_click(
                                            cx.listener(|this, _, window, cx| {
                                                this.undo(window, cx)
                                            }),
                                        ),
                                )
                            }),
                    ),
            )
            .when_some(self.editor.clone(), |el, editor| el.child(editor))
            .when_some(self.create_kind, |el, kind| {
                el.child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(gpui_kit::black().opacity(0.25))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            div()
                                .v_flex()
                                .w(px(360.))
                                .p_6()
                                .gap_4()
                                .rounded_lg()
                                .bg(p.background)
                                .border_1()
                                .border_color(p.border)
                                .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child(
                                    match kind {
                                        CreateKind::Workspace => "New workspace",
                                        CreateKind::Project => "New project",
                                        CreateKind::Rename(_) => "Rename",
                                    },
                                ))
                                .child(Input::new(&self.name_input))
                                .child(
                                    div()
                                        .h_flex()
                                        .justify_end()
                                        .gap_2()
                                        .child(
                                            Button::new("cancel-name")
                                                .ghost()
                                                .label("Cancel")
                                                .on_click(cx.listener(|this, _, _, cx| {
                                                    this.create_kind = None;
                                                    cx.notify();
                                                })),
                                        )
                                        .child(
                                            Button::new("create-name")
                                                .primary()
                                                .label(if matches!(kind, CreateKind::Rename(_)) {
                                                    "Save"
                                                } else {
                                                    "Create"
                                                })
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.create_named(window, cx)
                                                })),
                                        ),
                                )
                                .when_some(
                                    self.message.as_ref().filter(|(_, error)| *error),
                                    |el, (text, _)| {
                                        el.child(
                                            div()
                                                .text_xs()
                                                .text_color(p.destructive)
                                                .child(text.clone()),
                                        )
                                    },
                                ),
                        ),
                )
            })
    }
}
