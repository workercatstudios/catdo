use crate::{
    app::{CatDo, View},
    cloud,
};
use catdo_core::sync::Choice;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::{
    Disableable, Sizable, StyledExt,
    button::{Button, ButtonVariants},
};
use gpui_kit::{prelude::*, *};
use std::time::Duration;

impl CatDo {
    pub fn start_sync(cx: &mut Context<Self>) {
        cx.spawn(async move |entity, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(10))
                    .await;
                if entity.update(cx, |this, cx| this.sync_now(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
    }
    pub fn sign_in(&mut self, cx: &mut Context<Self>) {
        if self.sync_busy {
            return;
        }
        self.sync_busy = true;
        self.sync_status = "Opening sign-in…".into();
        cx.notify();
        let api = cloud::api_url();
        let begin = cx
            .background_executor()
            .spawn(async move { cloud::begin_login(&api) });
        cx.spawn(async move |entity, cx| {
            let result = begin.await;
            let _ = entity.update(cx, |this, cx| match result {
                Ok(login) => {
                    let url = login
                        .code
                        .verification_uri_complete
                        .clone()
                        .unwrap_or_else(|| login.code.verification_uri.clone());
                    this.sync_status = format!(
                        "Sign in with code {} at {}",
                        login.code.user_code, login.code.verification_uri
                    );
                    this.login_url = Some(url.clone());
                    cx.open_url(&url);
                    let finish = cx
                        .background_executor()
                        .spawn(async move { cloud::finish_login(login) });
                    cx.spawn(async move |entity, cx| {
                        let result = finish.await;
                        let _ = entity.update(cx, |this, cx| {
                            this.sync_busy = false;
                            this.login_url = None;
                            match result.and_then(|owner| this.store.bind_account(&owner)) {
                                Ok(()) => {
                                    this.sync_enabled = true;
                                    let _ = this.store.set_preference("sync_enabled", &true);
                                    this.sync_status = "Signed in · ready to sync".into();
                                    this.sync_now(cx);
                                }
                                Err(e) => this.sync_status = e.to_string(),
                            }
                            cx.notify();
                        });
                    })
                    .detach();
                }
                Err(e) => {
                    this.sync_busy = false;
                    this.sync_status = e.to_string();
                    cx.notify();
                }
            });
        })
        .detach();
    }
    pub fn sync_now(&mut self, cx: &mut Context<Self>) {
        if self.sync_busy || !self.sync_enabled || self.editor.is_some() {
            return;
        }
        let state = match self.store.sync_state() {
            Ok(Some(state)) => state,
            _ => return,
        };
        if state.conflict.is_some() {
            self.sync_status = "Changed on two devices · choose which versions to keep".into();
            cx.notify();
            return;
        }
        let pending = match self.store.prepare_sync(&self.data) {
            Ok(p) => p,
            Err(e) => {
                self.sync_status = e.to_string();
                cx.notify();
                return;
            }
        };
        self.sync_busy = true;
        self.sync_status = "Syncing…".into();
        cx.notify();
        let sent = pending.is_some();
        let api = cloud::api_url();
        let work = cx
            .background_executor()
            .spawn(async move { cloud::sync(&api, &state.owner, pending) });
        cx.spawn(async move |entity, cx| {
            let result = work.await;
            let _ = entity.update(cx, |this, cx| {
                this.sync_busy = false;
                if this.editor.is_some() {
                    this.sync_status = "Sync resumes when task details close".into();
                    cx.notify();
                    return;
                }
                match result {
                    Ok((remote, accepted)) => {
                        match this.store.accept_sync(&this.data, remote, sent && accepted) {
                            Ok(Some(data)) => {
                                let changed = this.data != data;
                                this.data = data;
                                if changed {
                                    this.undo.clear();
                                }
                                this.repair_sync_navigation();
                                this.workspace_select_dirty = true;
                                this.sync_status = if this
                                    .store
                                    .sync_state()
                                    .ok()
                                    .flatten()
                                    .is_some_and(|s| s.base.data == this.data)
                                {
                                    "All changes synced"
                                } else {
                                    "Saved on this device · waiting to sync"
                                }
                                .into();
                            }
                            Ok(None) => {
                                this.sync_status =
                                    "Changed on two devices · choose which versions to keep".into()
                            }
                            Err(e) => this.sync_status = e.to_string(),
                        }
                    }
                    Err(e) => this.sync_status = e.to_string(),
                }
                cx.notify();
            });
        })
        .detach();
    }
    pub(crate) fn repair_sync_navigation(&mut self) {
        if !self
            .data
            .workspaces
            .iter()
            .any(|w| w.id == self.workspace_id && !w.archived)
        {
            self.workspace_id = self
                .data
                .workspaces
                .iter()
                .find(|w| !w.archived)
                .unwrap()
                .id;
            self.view = View::Today;
        }
        if let View::Project(id) = self.view
            && !self.data.projects.iter().any(|p| p.id == id && !p.archived)
        {
            self.view = View::Inbox;
        }
    }
    fn resolve_cloud(&mut self, choice: Choice, cx: &mut Context<Self>) {
        if !self.commit_editor(cx) {
            return;
        }
        match self.store.resolve_sync(&self.data, choice) {
            Ok(data) => {
                self.data = data;
                self.undo.clear();
                self.repair_sync_navigation();
                self.workspace_select_dirty = true;
                self.sync_now(cx);
            }
            Err(e) => self.error(e.to_string(), cx),
        }
        cx.notify();
    }
    pub fn render_sync(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let p = cx.theme().color_tokens();
        let conflict = self
            .store
            .sync_state()
            .ok()
            .flatten()
            .is_some_and(|s| s.conflict.is_some());
        let structural = self
            .store
            .sync_state()
            .ok()
            .flatten()
            .and_then(|s| s.conflict.map(|remote| (s.base, remote)))
            .is_some_and(|(base, remote)| {
                [Choice::Local, Choice::Remote].into_iter().any(|choice| {
                    catdo_core::sync::merge(&base.data, &self.data, &remote.data, Some(choice))
                        .is_ok_and(|(d, _)| d.validate().is_err())
                })
            });
        div()
            .v_flex()
            .gap_2()
            .px_2()
            .text_xs()
            .text_color(p.muted_foreground)
            .child(self.sync_status.clone())
            .when_some(self.login_url.clone(), |el, url| {
                el.child(
                    Button::new("open-login")
                        .small()
                        .ghost()
                        .label("Open sign-in page")
                        .on_click(move |_, _, cx| cx.open_url(&url)),
                )
            })
            .when(structural, |el|el.child("These moves cannot be combined. Choosing a list replaces all tasks on this device."))
            .when(conflict, |el| {
                el.child(
                    div()
                        .v_flex()
                        .gap_1()
                        .child(
                            Button::new("keep-local")
                                .small()
                                .label(if structural {"Keep this entire task list"}else{"Keep this device’s versions"})
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.resolve_cloud(Choice::Local, cx)
                                })),
                        )
                        .child(
                            Button::new("keep-remote")
                                .small()
                                .label(if structural {"Use the entire synced list"}else{"Use synced versions"})
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.resolve_cloud(Choice::Remote, cx)
                                })),
                        ),
                )
            })
            .child(
                div()
                    .h_flex()
                    .gap_2()
                    .child(
                        Button::new("sign-in")
                            .small()
                            .ghost()
                            .disabled(self.sync_busy)
                            .label(if self.sync_enabled {
                                "Sign in again"
                            } else {
                                "Sign in to sync"
                            })
                            .on_click(cx.listener(|this, _, _, cx| this.sign_in(cx))),
                    )
                    .when(self.sync_enabled, |el| {
                        el.child(
                            Button::new("sign-out")
                                .small()
                                .ghost()
                                .disabled(self.sync_busy)
                                .label("Sign out")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.sync_enabled = false;
                                    let _ = this.store.set_preference("sync_enabled", &false);
                                    this.sync_status =
                                        "Signed out · tasks remain on this device".into();
                                    this.sync_busy = true;
                                    let api = cloud::api_url();
                                    let work = cx
                                        .background_executor()
                                        .spawn(async move { cloud::sign_out(&api) });
                                    cx.spawn(async move |entity, cx| {
                                        let result = work.await;
                                        let _ = entity.update(cx, |this, cx| {
                                            this.sync_busy = false;
                                            if let Err(e) = result {
                                                this.sync_status = e.to_string();
                                            }
                                            cx.notify();
                                        });
                                    })
                                    .detach();
                                    cx.notify();
                                })),
                        )
                    }),
            )
    }
}
