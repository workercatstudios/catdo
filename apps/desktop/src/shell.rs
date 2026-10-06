use crate::app::{
    CatDo, CloseEditor, CreateKind, Find, NewTask, Quit, SaveTask, TodayView, Undo, View,
};
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::{Sizable, StyledExt, TitleBar};
use gpui_kit::{prelude::*, *};
use kirakira::button::{Button, ButtonVariants};
use kirakira::input::Input;

use crate::motion::{fade_in, pop_in};
use crate::theme::{Accent, accent};

impl Render for CatDo {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.workspace_select_dirty {
            self.workspace_select_dirty = false;
            self.refresh_workspace_select(window, cx);
        }
        let p = cx.theme().color_tokens();
        let searching = !self.search.read(cx).value().is_empty();
        let editor = self
            .editor
            .clone()
            .map(|editor| pop_in("editor-pop", editor, window, cx).into_any_element());
        let name_dialog = self
            .create_kind
            .map(|kind| self.render_name_dialog(kind, window, cx));
        div()
            .id("catdo")
            .key_context("CatDo")
            .track_focus(&self.focus)
            .size_full()
            .v_flex()
            .bg(p.background)
            .text_color(p.foreground)
            .text_sm()
            .on_action(cx.listener(|this, _: &Quit, _, cx| {
                if this.save_before_close(cx) {
                    cx.quit();
                }
            }))
            .on_action(cx.listener(|this, _: &NewTask, window, cx| this.new_task(window, cx)))
            .on_action(cx.listener(|this, _: &Find, window, cx| {
                this.search.read(cx).focus_handle(cx).focus(window, cx)
            }))
            .on_action(cx.listener(|this, _: &Undo, window, cx| this.undo(window, cx)))
            .on_action(cx.listener(|this, _: &SaveTask, window, cx| {
                if this.commit_editor(cx) {
                    this.editor = None;
                    this.resume_sync(cx);
                    this.focus.focus(window, cx);
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &CloseEditor, window, cx| {
                if this.commit_editor(cx) {
                    this.editor = None;
                    this.resume_sync(cx);
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
            // Windows hides its native caption for the transparent title bar.
            .when(
                cfg!(windows) || matches!(window.window_decorations(), Decorations::Client { .. }),
                |el| {
                    el.child(
                        TitleBar::new()
                            .child(div().text_sm().font_semibold().child("CatDo"))
                            .on_close_window(cx.listener(|this, _, window, cx| {
                                if this.save_before_close(cx) {
                                    window.remove_window();
                                }
                            })),
                    )
                },
            )
            .child(
                div()
                    .h_flex()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .child(self.render_sidebar(cx))
                    .child(
                        div()
                            .v_flex()
                            .flex_1()
                            .min_w_0()
                            .h_full()
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
                                        |el| el.child(self.render_task_list(window, cx)),
                                    ),
                            )
                            .when(self.message.is_some() || !self.undo.is_empty(), |el| {
                                el.child(
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
                                        .child(div().flex_1().when_some(
                                            self.message.clone(),
                                            |el, (message, error)| {
                                                el.text_color(if error {
                                                    accent(Accent::Overdue, cx)
                                                } else {
                                                    p.muted_foreground
                                                })
                                                .child(message)
                                            },
                                        ))
                                        .when(!self.undo.is_empty(), |el| {
                                            el.child(
                                                Button::new("undo")
                                                    .ghost()
                                                    .xsmall()
                                                    .label("Undo")
                                                    .tooltip("Ctrl+Z")
                                                    .on_click(cx.listener(
                                                        |this, _, window, cx| this.undo(window, cx),
                                                    )),
                                            )
                                        }),
                                )
                            }),
                    ),
            )
            .when_some(editor, |el, editor| {
                el.child(fade_in(
                    div()
                        .id("editor-modal")
                        .occlude()
                        .absolute()
                        .inset_0()
                        .bg(cx.theme().overlay)
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(editor),
                    "editor-scrim",
                    cx,
                ))
            })
            .children(name_dialog)
    }
}

impl CatDo {
    /// Naming a workspace or project, or renaming one: a small Kirakira-style dialog.
    fn render_name_dialog(
        &self,
        kind: CreateKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let p = cx.theme().color_tokens();
        let panel = div()
            .v_flex()
            .w(rems(24.))
            .p_6()
            .gap_4()
            .rounded(cx.theme().radius_lg)
            .bg(cx.theme().popover)
            .border_1()
            .border_color(p.border)
            .shadow_lg()
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::EXTRA_BOLD)
                    .child(match kind {
                        CreateKind::Workspace => "New workspace",
                        CreateKind::Project => "New project",
                        CreateKind::Rename(_) => "Rename",
                    }),
            )
            .child(Input::new(&self.name_input))
            .child(
                div()
                    .h_flex()
                    .justify_end()
                    .gap_2()
                    .child(Button::new("cancel-name").ghost().label("Cancel").on_click(
                        cx.listener(|this, _, _, cx| {
                            this.create_kind = None;
                            cx.notify();
                        }),
                    ))
                    .child(
                        Button::new("create-name")
                            .primary()
                            .label(if matches!(kind, CreateKind::Rename(_)) {
                                "Save"
                            } else {
                                "Create"
                            })
                            .on_click(
                                cx.listener(|this, _, window, cx| this.create_named(window, cx)),
                            ),
                    ),
            )
            .when_some(
                self.message.as_ref().filter(|(_, error)| *error),
                |el, (text, _)| {
                    el.child(
                        div()
                            .text_xs()
                            .text_color(accent(Accent::Overdue, cx))
                            .child(text.clone()),
                    )
                },
            );
        fade_in(
            div()
                .absolute()
                .inset_0()
                .bg(cx.theme().overlay)
                .flex()
                .items_center()
                .justify_center()
                .child(pop_in("name-dialog-pop", panel, window, cx)),
            "name-dialog-scrim",
            cx,
        )
        .into_any_element()
    }
}
