//! Root and WindowExt: open Kirakira's dialogs, sheets and notifications, with their exits.
//!
//! Replaces `gpui_kit::component::{Root, WindowExt}` for the overlay layers. GPUI Component keeps
//! the open dialogs, the sheet and the notification list inside its `Root` view and draws them
//! with its own motion, and it drops a dialog or sheet the moment it closes, so nothing can play
//! an exit. Kirakira keeps its own per-window layer state instead, opens into it through the same
//! [`WindowExt`] methods, and draws it from the same three layer functions:
//!
//! ```ignore
//! - use gpui_kit::component::{Root, WindowExt};
//! - use gpui_kit::component::dialog::Dialog;
//! + use kirakira::{Root, WindowExt};
//! + use kirakira::dialog::Dialog;
//!
//! // Unchanged: the window's root view and your view's overlay layers.
//! cx.new(|cx| Root::new(view, window, cx));
//! div()
//!     .children(Root::render_sheet_layer(window, cx))
//!     .children(Root::render_dialog_layer(window, cx))
//!     .children(Root::render_notification_layer(window, cx))
//! ```
//!
//! [`Root`] is not a view of its own: [`Root::new`], [`Root::update`] and [`Root::read`] hand back
//! GPUI Component's `Root`, which stays the window's first view, so everything else in GPUI
//! Component keeps working. The layer functions draw GPUI Component's layers too, so a stock
//! dialog opened through `gpui_kit::component::WindowExt` still shows.
//!
//! A closed dialog or sheet stays in the layer, drawn but inert, until its exit has played, then
//! it is dropped. The notification layer also paints [tooltips](crate::tooltip) through their
//! fade, since GPUI drops a tooltip the moment the pointer leaves. Every enter and exit is a pure function of [`motion::now`](crate::motion::now)
//! since the open or close, so a screenshot can pin any frame.
//!
//! Differences from GPUI Component: Kirakira's dialogs and sheets share the window's default text
//! selection scope (GPUI Component's `Root` only scopes selection to its own modals), and the
//! layer functions defer their painting, so they draw on top wherever the view renders them.

use std::any::TypeId;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::component::Root as BaseRoot;
use gpui_kit::component::input::AnyInputState;
use gpui_kit::{
    AnyElement, AnyView, App, AppContext as _, Context, DefiniteLength, ElementId, Entity,
    FocusHandle, Global, IntoElement, ParentElement as _, Pixels, Styled as _, WeakFocusHandle,
    Window, WindowId, anchored, deferred, div, point, px,
};

use crate::dialog::{AlertDialog, Dialog};
use crate::notification::{Notification, NotificationList};
use crate::sheet::Sheet;
use gpui_kit::component::Placement;

/// When an overlay opened and, once it has, when it closed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Presence {
    pub opened: Instant,
    pub closed: Option<Instant>,
}

/// Where an overlay is in its life at a given moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    /// Open, this long ago.
    Open(Duration),
    /// Closed this long ago, still playing its exit.
    Closing(Duration),
}

impl Presence {
    pub fn new(opened: Instant) -> Self {
        Self {
            opened,
            closed: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.closed.is_none()
    }

    pub fn stage(&self, now: Instant) -> Stage {
        match self.closed {
            Some(closed) => Stage::Closing(now.saturating_duration_since(closed)),
            None => Stage::Open(now.saturating_duration_since(self.opened)),
        }
    }

    /// Whether the exit, `exit` long, has finished by `now`.
    pub fn gone(&self, now: Instant, exit: Duration) -> bool {
        self.closed
            .is_some_and(|closed| now.saturating_duration_since(closed) >= exit)
    }
}

pub(crate) type DialogBuilder = Rc<dyn Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static>;
pub(crate) type SheetBuilder = Rc<dyn Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static>;

pub(crate) struct ActiveDialog {
    /// Unique per open, so a dialog's element state never leaks into the next one.
    pub key: u64,
    pub focus_handle: FocusHandle,
    pub previous_focused_handle: Option<WeakFocusHandle>,
    pub builder: DialogBuilder,
    pub presence: Presence,
    /// Its place among the open dialogs when it opened. Each layer sits 16 px lower.
    pub layer_ix: usize,
    /// The last press on an alert dialog's backdrop, which shakes the panel.
    pub nudged: Option<Instant>,
}

pub(crate) struct ActiveSheet {
    pub key: u64,
    pub focus_handle: FocusHandle,
    pub previous_focused_handle: Option<WeakFocusHandle>,
    pub placement: Placement,
    pub builder: SheetBuilder,
    pub presence: Presence,
}

/// The overlays of one window.
pub(crate) struct Layers {
    pub dialogs: Vec<ActiveDialog>,
    /// The open sheet last, after any that are still leaving.
    pub sheets: Vec<ActiveSheet>,
    pub notification: Entity<NotificationList>,
    /// The size the open sheet last rendered at, so notifications can make room for it.
    pub sheet_size: Option<DefiniteLength>,
    next_key: u64,
    /// Focus to restore once a confirmed dialog has left, so a dialog opened meanwhile keeps the
    /// chain.
    pending_focus_restore: Option<WeakFocusHandle>,
    /// The tooltips shown, and the ones fading after GPUI dropped them.
    pub tooltips: crate::tooltip::TooltipLayer,
}

impl Layers {
    fn next_key(&mut self) -> u64 {
        self.next_key += 1;
        self.next_key
    }

    pub fn open_dialogs(&self) -> impl Iterator<Item = &ActiveDialog> {
        self.dialogs
            .iter()
            .filter(|dialog| dialog.presence.is_open())
    }

    pub fn open_sheet(&self) -> Option<&ActiveSheet> {
        self.sheets
            .iter()
            .rev()
            .find(|sheet| sheet.presence.is_open())
    }

    /// Marks the top open dialog closed and returns the focus it should hand back.
    fn close_top_dialog(&mut self, now: Instant) -> Option<FocusHandle> {
        let dialog = self
            .dialogs
            .iter_mut()
            .rev()
            .find(|dialog| dialog.presence.is_open())?;
        dialog.presence.closed = Some(now);
        dialog
            .previous_focused_handle
            .as_ref()
            .and_then(WeakFocusHandle::upgrade)
    }
}

#[derive(Default)]
struct Overlays(HashMap<WindowId, Entity<Layers>>);

impl Global for Overlays {}

/// The overlay layers of `window`, created on first use.
pub(crate) fn layers(window: &mut Window, cx: &mut App) -> Entity<Layers> {
    let id = window.window_handle().window_id();
    if !cx.has_global::<Overlays>() {
        install_overlays(cx);
    }
    if let Some(layers) = cx.global::<Overlays>().0.get(&id) {
        return layers.clone();
    }
    let notification = cx.new(|cx| NotificationList::new(window, cx));
    let layers = cx.new(|_| Layers {
        dialogs: Vec::new(),
        sheets: Vec::new(),
        notification,
        sheet_size: None,
        next_key: 0,
        pending_focus_restore: None,
        tooltips: Default::default(),
    });
    cx.global_mut::<Overlays>().0.insert(id, layers.clone());
    layers
}

/// Creates the overlay registry, with the hooks that drop a window's layers when it closes and
/// all of them when the app quits: the layers hold the overlays' builders and whatever entities
/// they capture.
fn install_overlays(cx: &mut App) {
    cx.set_global(Overlays::default());
    cx.on_window_closed(|cx, id| {
        if let Some(overlays) = cx.try_global::<Overlays>()
            && overlays.0.contains_key(&id)
        {
            cx.global_mut::<Overlays>().0.remove(&id);
        }
    })
    .detach();
    cx.on_app_quit(|cx| {
        if cx.has_global::<Overlays>() {
            cx.global_mut::<Overlays>().0.clear();
        }
        async {}
    })
    .detach();
}

/// Records a press on the backdrop of the alert dialog `key`, which shakes it.
pub(crate) fn nudge_dialog(key: u64, window: &mut Window, cx: &mut App) {
    let layers = layers(window, cx);
    let now = crate::motion::now();
    layers.update(cx, |layers, _| {
        if let Some(dialog) = layers.dialogs.iter_mut().find(|dialog| dialog.key == key) {
            dialog.nudged = Some(now);
        }
    });
    window.refresh();
}

/// Closes the top dialog. `deferred` (a confirm) hands focus back once the exit has played, so the
/// key that confirmed can't land on the control behind it.
pub(crate) fn close_dialog(deferred: bool, window: &mut Window, cx: &mut App) {
    let layers = layers(window, cx);
    let now = crate::motion::now();
    let (handle, open_after) = layers.update(cx, |layers, _| {
        let handle = layers.close_top_dialog(now);
        (handle, layers.open_dialogs().count())
    });
    if let Some(handle) = handle {
        if deferred {
            layers.update(cx, |layers, _| {
                layers.pending_focus_restore = Some(handle.downgrade());
            });
            let exit = crate::dialog::exit_duration(cx);
            window
                .spawn(cx, async move |cx| {
                    cx.background_executor().timer(exit).await;
                    let _ = cx.update(|window, cx| {
                        let open_now = layers.update(cx, |layers, _| {
                            layers.pending_focus_restore = None;
                            layers.open_dialogs().count()
                        });
                        // A dialog opened meanwhile owns focus now.
                        if open_now == open_after {
                            window.focus(&handle, cx);
                        }
                    });
                })
                .detach();
        } else {
            window.focus(&handle, cx);
        }
    }
    gpui_kit::base::TextSelection::clear(window, cx);
    window.refresh();
}

/// The window root, with Kirakira's overlay layers.
///
/// A stand-in for `gpui_kit::component::Root` with the same associated functions: [`Root::new`]
/// builds GPUI Component's `Root`, which stays the window's first view, and the `render_*_layer`
/// functions draw Kirakira's dialogs, sheets and notifications along with GPUI Component's own.
pub struct Root;

impl Root {
    /// Creates GPUI Component's `Root` view: `cx.new(|cx| Root::new(view, window, cx))`.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        view: impl Into<AnyView>,
        window: &mut Window,
        cx: &mut Context<BaseRoot>,
    ) -> BaseRoot {
        BaseRoot::new(view, window, cx)
    }

    /// Updates the window's GPUI Component `Root`.
    pub fn update<F, R>(window: &mut Window, cx: &mut App, f: F) -> R
    where
        F: FnOnce(&mut BaseRoot, &mut Window, &mut Context<BaseRoot>) -> R,
    {
        BaseRoot::update(window, cx, f)
    }

    /// Reads the window's GPUI Component `Root`.
    pub fn read<'a>(window: &'a Window, cx: &'a App) -> &'a BaseRoot {
        BaseRoot::read(window, cx)
    }

    /// Renders the dialog layer: Kirakira's dialogs, opening and closing, over GPUI Component's.
    pub fn render_dialog_layer(
        window: &mut Window,
        cx: &mut App,
    ) -> Option<impl IntoElement + use<>> {
        let base = BaseRoot::render_dialog_layer(window, cx).map(IntoElement::into_any_element);
        let ours = crate::dialog::render_layer(window, cx);
        join(base, ours)
    }

    /// Renders the sheet layer: Kirakira's sheet sliding in or out, with GPUI Component's.
    pub fn render_sheet_layer(
        window: &mut Window,
        cx: &mut App,
    ) -> Option<impl IntoElement + use<>> {
        let base = BaseRoot::render_sheet_layer(window, cx).map(IntoElement::into_any_element);
        let ours = crate::sheet::render_layer(window, cx);
        join(base, ours)
    }

    /// Renders the notification layer: Kirakira's toasts, with GPUI Component's. It also paints
    /// [Kirakira's tooltips](crate::tooltip) while they fade out, after GPUI has dropped them.
    pub fn render_notification_layer(
        window: &mut Window,
        cx: &mut App,
    ) -> Option<impl IntoElement + use<>> {
        let base =
            BaseRoot::render_notification_layer(window, cx).map(IntoElement::into_any_element);
        let tooltips = crate::tooltip::render_layer(window, cx);
        let ours = div()
            .child(render_notification_layer(window, cx))
            .children(tooltips)
            .into_any_element();
        join(base, Some(ours))
    }
}

fn join(base: Option<AnyElement>, ours: Option<AnyElement>) -> Option<AnyElement> {
    match (base, ours) {
        (None, None) => None,
        (base, ours) => Some(div().children(base).children(ours).into_any_element()),
    }
}

/// Notifications sit over everything else and step aside for an open sheet, like GPUI
/// Component's.
fn render_notification_layer(window: &mut Window, cx: &mut App) -> AnyElement {
    let layers = layers(window, cx);
    let (placement, size, list) = {
        let layers = layers.read(cx);
        (
            layers.open_sheet().map(|sheet| sheet.placement),
            layers.sheet_size,
            layers.notification.clone(),
        )
    };
    let viewport = window.viewport_size();
    let size = size.filter(|_| placement.is_some());
    deferred(
        anchored().position(point(px(0.), px(0.))).child(
            div().relative().w(viewport.width).h(viewport.height).child(
                div()
                    .absolute()
                    .inset_0()
                    .when_some_sheet(placement, size)
                    .child(list),
            ),
        ),
    )
    .with_priority(40)
    .into_any_element()
}

trait SheetMargin {
    fn when_some_sheet(self, placement: Option<Placement>, size: Option<DefiniteLength>) -> Self;
}

impl SheetMargin for gpui_kit::Div {
    fn when_some_sheet(self, placement: Option<Placement>, size: Option<DefiniteLength>) -> Self {
        let Some(size) = size else { return self };
        match placement {
            Some(Placement::Top) => self.mt(size),
            Some(Placement::Right) => self.mr(size),
            Some(Placement::Bottom) => self.mb(size),
            Some(Placement::Left) => self.ml(size),
            None => self,
        }
    }
}

/// Opens Kirakira's overlays on a window: the same methods as `gpui_kit::component::WindowExt`.
///
/// Import this one instead of GPUI Component's (importing both makes the calls ambiguous). The
/// methods that have nothing to do with overlays forward to GPUI Component.
pub trait WindowExt: Sized {
    /// Opens a Sheet at right placement.
    fn open_sheet<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static;

    /// Opens a Sheet at the given placement. The bottom sheet is the drawer, with its handle.
    fn open_sheet_at<F>(&mut self, placement: Placement, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static;

    /// Return true, if there is an active Sheet.
    fn has_active_sheet(&mut self, cx: &mut App) -> bool;

    /// Closes the active Sheet. It slides out before it unmounts.
    fn close_sheet(&mut self, cx: &mut App);

    /// Opens a Dialog.
    fn open_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static;

    /// Opens an AlertDialog.
    fn open_alert_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(AlertDialog, &mut Window, &mut App) -> AlertDialog + 'static;

    /// Return true, if there is an active Dialog.
    fn has_active_dialog(&mut self, cx: &mut App) -> bool;

    /// Closes the last active Dialog. It squashes away before it unmounts.
    fn close_dialog(&mut self, cx: &mut App);

    /// Closes all active Dialogs.
    fn close_all_dialogs(&mut self, cx: &mut App);

    /// Pushes a notification to the notification list.
    fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App);

    /// Removes all notifications whose id matches `T`, including ones registered with
    /// either `Notification::id` or `Notification::id1` (any key).
    fn remove_notification<T: Sized + 'static>(&mut self, cx: &mut App);

    /// Removes a single notification matching the given type `T` and `key` (paired with
    /// `Notification::id1`).
    fn remove_notification1<T: Sized + 'static>(&mut self, key: impl Into<ElementId>, cx: &mut App);

    /// Clears all notifications.
    fn clear_notifications(&mut self, cx: &mut App);

    /// Returns the mounted notifications, leaving ones included.
    fn notifications(&mut self, cx: &mut App) -> Rc<Vec<Entity<Notification>>>;

    /// Return the currently focused input state. Forwards to GPUI Component.
    fn focused_input(&mut self, cx: &mut App) -> Option<AnyInputState>;

    /// Returns true if there is a focused Input entity. Forwards to GPUI Component.
    fn has_focused_input(&mut self, cx: &mut App) -> bool;

    /// Returns the merged selected text across registered selectable regions.
    #[deprecated(note = "use gpui_base::TextSelection::selected_text instead")]
    fn selected_text(&mut self, cx: &mut App) -> String;

    /// Returns true if any registered region has an active text selection.
    #[deprecated(note = "use gpui_base::TextSelection::has_selection instead")]
    fn has_text_selection(&mut self, cx: &mut App) -> bool;

    /// Clears the window text selection.
    #[deprecated(note = "use gpui_base::TextSelection::clear instead")]
    fn clear_text_selection(&mut self, cx: &mut App);

    /// Ends the in-progress window-level text selection drag (if any).
    #[deprecated(note = "use gpui_base::TextSelection::end instead")]
    fn end_text_selection(&mut self, cx: &mut App);
}

impl WindowExt for Window {
    fn open_sheet<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static,
    {
        self.open_sheet_at(Placement::Right, cx, build)
    }

    fn open_sheet_at<F>(&mut self, placement: Placement, cx: &mut App, build: F)
    where
        F: Fn(Sheet, &mut Window, &mut App) -> Sheet + 'static,
    {
        let layers = layers(self, cx);
        let focused = self.focused(cx).map(|handle| handle.downgrade());
        let focus_handle = cx.focus_handle();
        focus_handle.focus(self, cx);
        let now = crate::motion::now();
        layers.update(cx, |layers, _| {
            // A new sheet replaces the open one, which slides out, and inherits where focus goes
            // back to.
            let mut previous = None;
            for sheet in layers.sheets.iter_mut().filter(|s| s.presence.is_open()) {
                sheet.presence.closed = Some(now);
                previous = sheet.previous_focused_handle.clone();
            }
            let key = layers.next_key();
            layers.sheets.push(ActiveSheet {
                key,
                focus_handle,
                previous_focused_handle: previous.or(focused),
                placement,
                builder: Rc::new(build),
                presence: Presence::new(now),
            });
        });
        gpui_kit::base::TextSelection::clear(self, cx);
        self.refresh();
    }

    fn has_active_sheet(&mut self, cx: &mut App) -> bool {
        layers(self, cx).read(cx).open_sheet().is_some()
    }

    fn close_sheet(&mut self, cx: &mut App) {
        let layers = layers(self, cx);
        let now = crate::motion::now();
        let handle = layers.update(cx, |layers, _| {
            let sheet = layers
                .sheets
                .iter_mut()
                .rev()
                .find(|sheet| sheet.presence.is_open())?;
            sheet.presence.closed = Some(now);
            sheet
                .previous_focused_handle
                .as_ref()
                .and_then(WeakFocusHandle::upgrade)
        });
        if let Some(handle) = handle {
            self.focus(&handle, cx);
        }
        gpui_kit::base::TextSelection::clear(self, cx);
        self.refresh();
    }

    fn open_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(Dialog, &mut Window, &mut App) -> Dialog + 'static,
    {
        let layers = layers(self, cx);
        let focused = self.focused(cx).map(|handle| handle.downgrade());
        let focus_handle = cx.focus_handle();
        focus_handle.focus(self, cx);
        let now = crate::motion::now();
        layers.update(cx, |layers, _| {
            let previous = layers.pending_focus_restore.take().or(focused);
            let key = layers.next_key();
            let layer_ix = layers.open_dialogs().count();
            layers.dialogs.push(ActiveDialog {
                key,
                focus_handle,
                previous_focused_handle: previous,
                builder: Rc::new(build),
                presence: Presence::new(now),
                layer_ix,
                nudged: None,
            });
        });
        gpui_kit::base::TextSelection::clear(self, cx);
        self.refresh();
    }

    fn open_alert_dialog<F>(&mut self, cx: &mut App, build: F)
    where
        F: Fn(AlertDialog, &mut Window, &mut App) -> AlertDialog + 'static,
    {
        self.open_dialog(cx, move |_, window, cx| {
            build(AlertDialog::new(cx), window, cx).build_surface(window, cx)
        })
    }

    fn has_active_dialog(&mut self, cx: &mut App) -> bool {
        layers(self, cx).read(cx).open_dialogs().next().is_some()
    }

    fn close_dialog(&mut self, cx: &mut App) {
        close_dialog(false, self, cx);
    }

    fn close_all_dialogs(&mut self, cx: &mut App) {
        let layers = layers(self, cx);
        let now = crate::motion::now();
        let handle = layers.update(cx, |layers, _| {
            let first = layers
                .open_dialogs()
                .next()
                .and_then(|dialog| dialog.previous_focused_handle.clone());
            for dialog in layers.dialogs.iter_mut() {
                if dialog.presence.is_open() {
                    dialog.presence.closed = Some(now);
                }
            }
            first.and_then(|handle| handle.upgrade())
        });
        if let Some(handle) = handle {
            self.focus(&handle, cx);
        }
        gpui_kit::base::TextSelection::clear(self, cx);
        self.refresh();
    }

    fn push_notification(&mut self, note: impl Into<Notification>, cx: &mut App) {
        let note = note.into();
        let list = layers(self, cx).read(cx).notification.clone();
        list.update(cx, |list, cx| list.push(note, self, cx));
        self.refresh();
    }

    fn remove_notification<T: Sized + 'static>(&mut self, cx: &mut App) {
        let list = layers(self, cx).read(cx).notification.clone();
        list.update(cx, |list, cx| {
            list.close_by_type(TypeId::of::<T>(), self, cx)
        });
        self.refresh();
    }

    fn remove_notification1<T: Sized + 'static>(
        &mut self,
        key: impl Into<ElementId>,
        cx: &mut App,
    ) {
        let key = key.into();
        let list = layers(self, cx).read(cx).notification.clone();
        list.update(cx, |list, cx| {
            list.close((TypeId::of::<T>(), key), self, cx)
        });
        self.refresh();
    }

    fn clear_notifications(&mut self, cx: &mut App) {
        let list = layers(self, cx).read(cx).notification.clone();
        list.update(cx, |list, cx| list.clear(self, cx));
        self.refresh();
    }

    fn notifications(&mut self, cx: &mut App) -> Rc<Vec<Entity<Notification>>> {
        let list = layers(self, cx).read(cx).notification.clone();
        Rc::new(list.read(cx).notifications())
    }

    fn focused_input(&mut self, cx: &mut App) -> Option<AnyInputState> {
        gpui_kit::component::WindowExt::focused_input(self, cx)
    }

    fn has_focused_input(&mut self, cx: &mut App) -> bool {
        gpui_kit::component::WindowExt::has_focused_input(self, cx)
    }

    fn selected_text(&mut self, cx: &mut App) -> String {
        gpui_kit::base::TextSelection::selected_text(self, cx)
    }

    fn has_text_selection(&mut self, cx: &mut App) -> bool {
        gpui_kit::base::TextSelection::has_selection(self, cx)
    }

    fn clear_text_selection(&mut self, cx: &mut App) {
        gpui_kit::base::TextSelection::clear(self, cx);
    }

    fn end_text_selection(&mut self, cx: &mut App) {
        gpui_kit::base::TextSelection::end(self, cx);
    }
}

/// Pixels of a sheet's [`DefiniteLength`] against the window.
pub(crate) fn length_px(length: DefiniteLength, base: Pixels, window: &Window) -> Pixels {
    length.to_pixels(base.into(), window.rem_size())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_stages() {
        let start = Instant::now();
        let mut presence = Presence::new(start);
        let at = |ms| start + Duration::from_millis(ms);
        assert_eq!(
            presence.stage(at(100)),
            Stage::Open(Duration::from_millis(100))
        );
        assert!(!presence.gone(at(10_000), Duration::from_millis(150)));
        presence.closed = Some(at(500));
        assert_eq!(
            presence.stage(at(560)),
            Stage::Closing(Duration::from_millis(60))
        );
        assert!(!presence.gone(at(649), Duration::from_millis(150)));
        assert!(presence.gone(at(650), Duration::from_millis(150)));
    }

    struct Blank;

    impl gpui_kit::Render for Blank {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    fn overlay_windows(cx: &mut gpui_kit::TestAppContext) -> usize {
        cx.read(|cx| {
            cx.try_global::<Overlays>()
                .map_or(0, |overlays| overlays.0.len())
        })
    }

    #[gpui_kit::test]
    fn closing_a_window_drops_its_layers(cx: &mut gpui_kit::TestAppContext) {
        let first = cx.add_window(|_, _| Blank);
        let second = cx.add_window(|_, _| Blank);
        let first_layers = cx
            .update_window(first.into(), |_, window, cx| layers(window, cx).downgrade())
            .unwrap();
        cx.update_window(second.into(), |_, window, cx| {
            layers(window, cx);
        })
        .unwrap();
        assert_eq!(overlay_windows(cx), 2);

        cx.update_window(first.into(), |_, window, _| window.remove_window())
            .unwrap();
        cx.run_until_parked();
        assert_eq!(overlay_windows(cx), 1);
        assert!(
            first_layers.upgrade().is_none(),
            "the closed window's layers leaked"
        );

        cx.update_window(second.into(), |_, window, _| window.remove_window())
            .unwrap();
        cx.run_until_parked();
        assert_eq!(overlay_windows(cx), 0);
    }
}
