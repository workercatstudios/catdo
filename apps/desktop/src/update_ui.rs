use crate::{
    app::CatDo,
    updates::{self, ReadyUpdate, Update},
};
use gpui_kit::assets::IconName;
use gpui_kit::component::{
    Disableable, Sizable,
    button::{Button, ButtonVariants},
};
use gpui_kit::{Context, IntoElement, Styled};
use std::time::Duration;

#[derive(Default)]
pub enum UpdateState {
    #[default]
    Idle,
    Checking,
    Available(Update),
    Downloading,
    Ready(ReadyUpdate),
    Failed {
        retry: Option<Update>,
        message: String,
    },
}

impl CatDo {
    pub(crate) fn start_updates(&mut self, cx: &mut Context<Self>) {
        self.check_updates(cx);
        cx.spawn(async move |entity, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_secs(6 * 60 * 60))
                    .await;
                if entity
                    .update(cx, |this, cx| {
                        if matches!(
                            this.update_state,
                            UpdateState::Idle | UpdateState::Failed { retry: None, .. }
                        ) {
                            this.check_updates(cx);
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    fn check_updates(&mut self, cx: &mut Context<Self>) {
        self.update_state = UpdateState::Checking;
        let check = cx.background_executor().spawn(async { updates::check() });
        cx.spawn(async move |entity, cx| {
            let result = check.await;
            let _ = entity.update(cx, |this, cx| {
                this.update_state = match result {
                    Ok(Some(update)) => UpdateState::Available(update),
                    Ok(None) => UpdateState::Idle,
                    Err(error) => UpdateState::Failed {
                        retry: None,
                        message: format!("Could not check for updates: {error}"),
                    },
                };
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn download_update(&mut self, update: Update, cx: &mut Context<Self>) {
        self.update_state = UpdateState::Downloading;
        let retry = update.clone();
        let download = cx
            .background_executor()
            .spawn(async move { updates::download(update) });
        cx.spawn(async move |entity, cx| {
            let result = download.await;
            let _ = entity.update(cx, |this, cx| {
                this.update_state = match result {
                    Ok(ready) => UpdateState::Ready(ready),
                    Err(error) => {
                        let message = format!("Could not download the update: {error}");
                        this.error(message.clone(), cx);
                        UpdateState::Failed {
                            retry: Some(retry),
                            message,
                        }
                    }
                };
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn click_update(&mut self, cx: &mut Context<Self>) {
        match &self.update_state {
            UpdateState::Checking | UpdateState::Downloading => (),
            UpdateState::Available(update)
            | UpdateState::Failed {
                retry: Some(update),
                ..
            } => {
                self.download_update(update.clone(), cx);
            }
            UpdateState::Ready(_) => {
                if !self.save_before_close(cx) {
                    return;
                }
                if let UpdateState::Ready(ready) = &self.update_state {
                    let update = ready.update.clone();
                    if let Err(error) = ready.install_and_restart() {
                        let message = format!("Could not install the update: {error:#}");
                        self.error(message.clone(), cx);
                        self.update_state = UpdateState::Failed {
                            retry: Some(update),
                            message,
                        };
                    }
                }
            }
            _ => self.check_updates(cx),
        }
    }

    pub(crate) fn render_update(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (icon, tooltip, busy) = match &self.update_state {
            UpdateState::Idle => (
                IconName::CircleCheck,
                format!("CatDo {} · Check for updates", env!("CARGO_PKG_VERSION")),
                false,
            ),
            UpdateState::Checking => (IconName::LoaderCircle, "Checking for updates…".into(), true),
            UpdateState::Available(update) => (
                IconName::Download,
                format!("Download CatDo {}", update.version),
                false,
            ),
            UpdateState::Downloading => {
                (IconName::LoaderCircle, "Downloading update…".into(), true)
            }
            UpdateState::Ready(ready) => (
                IconName::ArrowUpFromLine,
                format!("Install CatDo {} and restart", ready.update.version),
                false,
            ),
            UpdateState::Failed { message, .. } => (
                IconName::RefreshCw,
                format!("{message} · Click to retry"),
                false,
            ),
        };
        Button::new("app-update")
            .ghost()
            .small()
            .rounded_full()
            .icon(icon)
            .accessibility_label(tooltip.clone())
            .tooltip(tooltip)
            .loading(busy)
            .disabled(busy)
            .on_click(cx.listener(|this, _, _, cx| this.click_update(cx)))
    }
}
