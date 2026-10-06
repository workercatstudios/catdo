//! Dialog and AlertDialog: the panel pops in past full size and squashes away on close.
//!
//! Replaces `gpui_kit::component::dialog`. `Dialog`, `AlertDialog` and `DialogButtonProps` are
//! Kirakira's, with GPUI Component's builders; every other item of that module is re-exported, so
//! `use kirakira::dialog::*` is a drop-in. Open them with [`crate::WindowExt`] and draw them with
//! [`crate::Root::render_dialog_layer`] (see [`crate::root`]); `Dialog::trigger` and
//! `AlertDialog::trigger` open through Kirakira too.
//!
//! The panel is GPUI Component's: the same width, top offset, padding, title, content, footer and
//! close button, on gpui-base's modal host (focus trap, Escape, Enter, backdrop dismissal). The
//! motion is Pop Dialog's:
//!
//! - Open, 0.4 s: scale 0.6 → (1.04, 1.06) at 50 % → (0.98, 0.99) at 75 % → 1, ease-out into the
//!   overshoot and ease-in-out after; a 4 % rise that lands at 50 %; opacity done by 30 %, so the
//!   whole bounce is seen. The backdrop fades in over 0.2 s.
//! - Close, 0.15 s on ease-in: a squash to (0.94, 0.9) and a fade; the backdrop fades out with
//!   it. The dialog stays mounted, inert, until the exit has played.
//! - The close button squashes to (1.2, 0.8) while it is held (0.08 s ease-out) and springs back
//!   (0.35 s, Kirakira's spring curve). The footer's OK and Cancel are Kirakira [`Button`]s.
//! - An alert dialog doesn't close from its backdrop, so pressing the backdrop shakes the panel:
//!   0.5 rem left, 0.25 rem right, 0.125 rem left, rest, in 0.4 s. Its icon pops in 0.15 s after
//!   the panel: 0 → (1.2, 1.25) → (0.9, 0.95) → 1 in 0.5 s, opacity done by 20 %.
//! - Reduced motion: the panel fades in (0.15 s) and out (0.1 s) where it rests, the icon is
//!   there at once, the close button doesn't squash, and a backdrop press on an alert dialog
//!   flashes a 0.375 rem ring around the panel instead of shaking it.
//!
//! Where it differs from the web version, and why:
//!
//! - GPUI scales the panel evenly ([`motion::transform`](crate::motion::transform)), so each
//!   two-axis scale is its geometric mean: (1.04, 1.06) is 1.05, (0.94, 0.9) is 0.92. The close
//!   button's X is an SVG, so its squash is exact; the round hover plate behind it stays put.
//! - The panel keeps GPUI Component's place, a tenth of the window down, rather than the web's
//!   centre.
//! - The footer buttons use Kirakira Button's press (sink to 0.95, pop back) rather than Pop Alert
//!   Dialog's own (1.05, 0.9) squash, so they match every other button in the kit.

use std::rc::Rc;
use std::sync::LazyLock;
use std::time::{Duration, Instant};

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::{ActiveTheme as _, TITLE_BAR_HEIGHT, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    Action, AnyElement, App, BoxShadow, ClickEvent, DefiniteLength, Edges, FocusHandle, Hsla,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement, Pixels, Rems, RenderOnce,
    SharedString, StatefulInteractiveElement as _, StyleRefinement, Styled, Transformation, Window,
    WindowControlArea, anchored, deferred, div, hsla, point, px, rems, size,
};

pub use gpui_kit::component::dialog::{
    Cancel, Confirm, DialogAction, DialogClose, DialogContent, DialogDescription, DialogFooter,
    DialogFooterButton, DialogHeader, DialogTitle,
};

use crate::button::{Button, ButtonVariant, ButtonVariants as _, Pressed};
use crate::icons;
use crate::motion::{Easing, Keyframes, Pose, Track, ms, now, transform};
use crate::root::{self, Presence, Stage, WindowExt as _};
use crate::theme::ActiveKira as _;

/// How long a dialog takes to pop in.
pub static ANIMATION_DURATION: LazyLock<Duration> = LazyLock::new(|| ms(400));

const POP_IN: Duration = ms(400);
const POP_OUT: Duration = ms(150);
const FADE_IN: Duration = ms(150);
const FADE_OUT: Duration = ms(100);
const BACKDROP_IN: Duration = ms(200);
const BACKDROP_OUT: Duration = ms(150);
const NUDGE: Duration = ms(400);
const MEDIA_DELAY: Duration = ms(150);
const MEDIA: Duration = ms(500);

fn progress(elapsed: Duration, duration: Duration) -> f32 {
    if duration.is_zero() {
        return 1.0;
    }
    (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}

/// `kk-pop-dialog-in`'s opacity: 0 → 1 by 30 %, on the 0 % keyframe's ease-out.
pub(crate) fn pop_in_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.0_f32, Easing::EaseOut)
        .at(0.3, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// `kk-pop-dialog-in`'s scale on one axis: 0.6 → `overshoot` (50 %) → `settle` (75 %) → 1.
fn pop_in_axis(overshoot: f32, settle: f32) -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.6_f32, Easing::EaseOut)
        .at(0.5, overshoot)
        .at(0.75, settle)
        .at(1.0, 1.0)
        .build()
}

pub(crate) fn pop_in_sx() -> Keyframes<f32> {
    pop_in_axis(1.04, 0.98)
}

pub(crate) fn pop_in_sy() -> Keyframes<f32> {
    pop_in_axis(1.06, 0.99)
}

/// `kk-pop-dialog-in`'s rise, as a fraction of the panel's height: 4 % → 0 by 50 %.
pub(crate) fn pop_in_rise() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.04_f32, Easing::EaseOut)
        .at(0.5, 0.0)
        .at(1.0, 0.0)
        .build()
}

/// `kk-pop-dialog-in` at `elapsed`: the panel's two-axis scale, rise and opacity.
///
/// Each property eases between the keyframes that set it: the 0 % keyframe's ease-out runs to
/// the next keyframe of that property (opacity's 30 %, scale's and the rise's 50 %), ease-in-out
/// after. The rise is a `transform` inside the `scale`, so it is scaled too.
pub(crate) fn pop_in(elapsed: Duration) -> Pose {
    let p = progress(elapsed, POP_IN);
    let sy = pop_in_sy().sample(p);
    Pose::new()
        .scale_xy(pop_in_sx().sample(p), sy)
        .yp(pop_in_rise().sample(p) * sy)
        .opacity(pop_in_opacity().sample(p))
}

/// `kk-pop-dialog-out`: from rest to a (0.94, 0.9) squash and nothing, ease-in.
pub(crate) fn pop_out_track() -> Keyframes<Pose> {
    Track::new(Easing::EaseIn)
        .at(0.0, Pose::new())
        .at(1.0, Pose::new().scale_xy(0.94, 0.9).opacity(0.0))
        .build()
}

pub(crate) fn pop_out(elapsed: Duration) -> Pose {
    pop_out_track().sample(progress(elapsed, POP_OUT))
}

/// `kk-pop-alert-dialog-nudge`, in rems: 0 → −0.5 (20 %) → 0.25 (45 %) → −0.125 (70 %) → 0,
/// ease-in-out on each segment.
pub(crate) fn nudge_track() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.0_f32)
        .at(0.2, -0.5)
        .at(0.45, 0.25)
        .at(0.7, -0.125)
        .at(1.0, 0.0)
        .build()
}

pub(crate) fn nudge(elapsed: Duration) -> f32 {
    nudge_track().sample(progress(elapsed, NUDGE))
}

/// The media pop at `elapsed` since the panel opened (it starts 0.15 s late).
pub(crate) fn media_pop(elapsed: Duration) -> Pose {
    let p = progress(elapsed.saturating_sub(MEDIA_DELAY), MEDIA);
    let opacity = Track::new(Easing::EaseInOut)
        .at(0.0, 0.0_f32)
        .at(0.2, 1.0)
        .at(1.0, 1.0)
        .build()
        .sample(p);
    let mut pose = media_scale().sample(p);
    // A zero rem size lays nothing out; start just above it, invisible anyway.
    pose.sx = pose.sx.max(0.01);
    pose.sy = pose.sy.max(0.01);
    pose.opacity(opacity)
}

/// `kk-pop-alert-dialog-media`'s scale: 0 → (1.2, 1.25) at 50 % → (0.9, 0.95) at 75 % → 1.
/// Opacity is keyed apart, at 20 %, where scale isn't.
pub(crate) fn media_scale() -> Keyframes<Pose> {
    Track::new(Easing::EaseInOut)
        .at(0.0, Pose::new().scale(0.0))
        .at(0.5, Pose::new().scale_xy(1.2, 1.25))
        .at(0.75, Pose::new().scale_xy(0.9, 0.95))
        .at(1.0, Pose::new())
        .build()
}

/// How long a closed dialog stays mounted for its exit.
pub(crate) fn exit_duration(cx: &App) -> Duration {
    if cx.reduce_motion() {
        FADE_OUT
    } else {
        POP_OUT
    }
}

/// The panel's pose and the backdrop's opacity at `stage`, and whether motion is still running.
fn sample(stage: Stage, reduced: bool) -> (Pose, f32, bool) {
    match stage {
        Stage::Open(t) => {
            let backdrop = Easing::EaseOut.sample(progress(t, BACKDROP_IN));
            if reduced {
                let o = Easing::EaseOut.sample(progress(t, FADE_IN));
                (Pose::new().opacity(o), backdrop, t < BACKDROP_IN)
            } else {
                (pop_in(t), backdrop, t < POP_IN)
            }
        }
        Stage::Closing(t) => {
            let backdrop = 1.0 - Easing::EaseIn.sample(progress(t, BACKDROP_OUT));
            let pose = if reduced {
                Pose::new().opacity(1.0 - Easing::EaseIn.sample(progress(t, FADE_OUT)))
            } else {
                pop_out(t)
            };
            (pose, backdrop, t < BACKDROP_OUT.max(POP_OUT))
        }
    }
}

/// Dialog button props.
#[derive(Clone)]
pub struct DialogButtonProps {
    ok_text: Option<SharedString>,
    ok_variant: ButtonVariant,
    cancel_text: Option<SharedString>,
    cancel_variant: ButtonVariant,
    show_cancel: bool,
    on_ok: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static>,
    on_cancel: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static>,
    on_close: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
}

impl Default for DialogButtonProps {
    fn default() -> Self {
        Self {
            ok_text: None,
            ok_variant: ButtonVariant::Primary,
            cancel_text: None,
            cancel_variant: ButtonVariant::default(),
            show_cancel: false,
            on_ok: Rc::new(|_, _, _| true),
            on_cancel: Rc::new(|_, _, _| true),
            on_close: Rc::new(|_, _, _| {}),
        }
    }
}

impl DialogButtonProps {
    /// Sets the text of the OK button. Default is `OK`.
    pub fn ok_text(mut self, ok_text: impl Into<SharedString>) -> Self {
        self.ok_text = Some(ok_text.into());
        self
    }

    /// Sets the variant of the OK button. Default is `ButtonVariant::Primary`.
    pub fn ok_variant(mut self, ok_variant: ButtonVariant) -> Self {
        self.ok_variant = ok_variant;
        self
    }

    /// Sets the text of the Cancel button. Default is `Cancel`.
    pub fn cancel_text(mut self, cancel_text: impl Into<SharedString>) -> Self {
        self.cancel_text = Some(cancel_text.into());
        self
    }

    /// Sets the variant of the Cancel button. Default is `ButtonVariant::default()`.
    pub fn cancel_variant(mut self, cancel_variant: ButtonVariant) -> Self {
        self.cancel_variant = cancel_variant;
        self
    }

    /// Sets whether to show the Cancel button. Default is `false`.
    pub fn show_cancel(mut self, show_cancel: bool) -> Self {
        self.show_cancel = show_cancel;
        self
    }

    /// Sets the callback for when the dialog is has been confirmed.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will
    /// not be closed.
    pub fn on_ok(
        mut self,
        on_ok: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.on_ok = Rc::new(on_ok);
        self
    }

    /// Sets the callback for when the dialog is has been canceled.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will
    /// not be closed.
    pub fn on_cancel(
        mut self,
        on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.on_cancel = Rc::new(on_cancel);
        self
    }

    fn render_ok(&self) -> AnyElement {
        DialogButton {
            anchor_key: "dialog-ok-anchor",
            button: Button::new("ok")
                .label(self.ok_text.clone().unwrap_or_else(|| "OK".into()))
                .with_variant(self.ok_variant),
            action: Rc::new(Confirm { secondary: false }),
        }
        .into_any_element()
    }

    fn render_cancel(&self) -> AnyElement {
        DialogButton {
            anchor_key: "dialog-cancel-anchor",
            button: Button::new("cancel")
                .label(self.cancel_text.clone().unwrap_or_else(|| "Cancel".into()))
                .with_variant(self.cancel_variant),
            action: Rc::new(Cancel),
        }
        .into_any_element()
    }
}

/// A focus node of a dialog control's own, so the control dispatches its action on the dialog it
/// sits in rather than on whatever holds focus when it is clicked (as GPUI Component's does).
#[derive(Clone)]
struct DispatchAnchor {
    handle: FocusHandle,
}

impl DispatchAnchor {
    fn new(key: &'static str, window: &mut Window, cx: &mut App) -> Self {
        let handle = window
            .use_keyed_state(key, cx, |_, cx| cx.focus_handle())
            .read(cx)
            .clone();
        Self { handle }
    }

    fn element(&self) -> impl IntoElement {
        div().absolute().size_0().track_focus(&self.handle)
    }

    fn dispatch(&self, action: &dyn Action, window: &mut Window, cx: &mut App) {
        if self.handle.is_focused(window) || self.handle.contains(&self.handle, window) {
            self.handle.dispatch_action(action, window, cx);
        } else {
            window.dispatch_action(action.boxed_clone(), cx);
        }
    }
}

/// A default dialog button: activating it dispatches `action` on the dialog it sits in.
#[derive(IntoElement)]
struct DialogButton {
    anchor_key: &'static str,
    button: Button,
    action: Rc<dyn Action>,
}

impl RenderOnce for DialogButton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let anchor = DispatchAnchor::new(self.anchor_key, window, cx);
        let action = self.action;
        self.button
            .child(anchor.element())
            .on_click(move |_, window, cx| anchor.dispatch(&*action, window, cx))
    }
}

/// The dialog's round close button: its X squashes while held and springs back.
#[derive(IntoElement)]
pub(crate) struct SquashClose;

impl RenderOnce for SquashClose {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let pressed = window.use_keyed_state("kk-close-pressed", cx, |_, _| Pressed(false));
        let held = pressed.read(cx).0;
        let (duration, easing) = if held {
            (ms(80), Easing::EaseOut)
        } else {
            (ms(350), cx.curves().spring)
        };
        // On Kirakira's clock, so a screenshot can pin any frame of it.
        let squash = crate::state_motion::glide(
            "kk-close-squash",
            if held { 1.0_f32 } else { 0.0 },
            duration,
            easing,
            window,
            cx,
        );
        let squash = if cx.reduce_motion() { 0.0 } else { squash };
        let theme = cx.theme();
        let (fg, hover_bg) = (theme.foreground, theme.accent);
        let (down, up, up_out) = (pressed.clone(), pressed.clone(), pressed);
        let set = |state: &gpui_kit::Entity<Pressed>, value: bool, cx: &mut App| {
            state.update(cx, |pressed, cx| {
                pressed.0 = value;
                cx.notify();
            });
        };
        gpui_kit::base::DialogClose::new().trigger(move |button| {
            button
                .flex()
                .items_center()
                .justify_center()
                .size_6()
                .rounded_full()
                .opacity(0.7)
                .cursor_pointer()
                .hover(move |style| style.bg(hover_bg).opacity(1.0))
                .on_mouse_down(MouseButton::Left, move |_, _, cx| set(&down, true, cx))
                .on_mouse_up(MouseButton::Left, move |_, _, cx| set(&up, false, cx))
                .on_mouse_up_out(MouseButton::Left, move |_, _, cx| set(&up_out, false, cx))
                .child(icons::icon(icons::X).text_color(fg).with_transformation(
                    Transformation::scale(size(1.0 + 0.2 * squash, 1.0 - 0.2 * squash)),
                ))
        })
    }
}

enum BaseDialogRoot {
    Dialog(gpui_kit::base::Dialog),
    AlertDialog(gpui_kit::base::AlertDialog),
}

macro_rules! map_base_root {
    ($self:expr, $method:ident($($arg:expr),* $(,)?)) => {
        match $self {
            BaseDialogRoot::Dialog(root) => BaseDialogRoot::Dialog(root.$method($($arg),*)),
            BaseDialogRoot::AlertDialog(root) => {
                BaseDialogRoot::AlertDialog(root.$method($($arg),*))
            }
        }
    };
}

impl BaseDialogRoot {
    fn new(alert: bool, cx: &mut App) -> Self {
        if alert {
            Self::AlertDialog(gpui_kit::base::AlertDialog::new(cx))
        } else {
            Self::Dialog(gpui_kit::base::Dialog::new(cx))
        }
    }
    fn layer(self, index: usize, topmost: bool) -> Self {
        map_base_root!(self, layer(index, topmost))
    }
    fn focus_handle(self, focus: FocusHandle) -> Self {
        map_base_root!(self, focus_handle(focus))
    }
    fn close_on_escape(self, value: bool) -> Self {
        map_base_root!(self, close_on_escape(value))
    }
    fn close_on_backdrop_press(self, value: bool) -> Self {
        match self {
            Self::Dialog(root) => Self::Dialog(root.close_on_backdrop_press(value)),
            Self::AlertDialog(root) => Self::AlertDialog(root),
        }
    }
    fn dismiss_below_y(self, value: Pixels) -> Self {
        map_base_root!(self, dismiss_below_y(value))
    }
    fn backdrop(self, element: impl IntoElement) -> Self {
        map_base_root!(self, backdrop(element))
    }
    fn popup(self, element: impl IntoElement) -> Self {
        map_base_root!(self, popup(element))
    }
    fn on_ok(self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static) -> Self {
        map_base_root!(self, on_ok(handler))
    }
    fn on_cancel(
        self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        map_base_root!(self, on_cancel(handler))
    }
    fn on_close(self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self {
        map_base_root!(self, on_close(handler))
    }
    fn request_close(self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        map_base_root!(self, request_close(handler))
    }
}

impl IntoElement for BaseDialogRoot {
    type Element = <gpui_kit::base::Dialog as IntoElement>::Element;
    fn into_element(self) -> Self::Element {
        match self {
            Self::Dialog(root) => root.into_element(),
            Self::AlertDialog(root) => root.into_element(),
        }
    }
}

type ContentBuilderFn = Rc<dyn Fn(DialogContent, &mut Window, &mut App) -> DialogContent + 'static>;

#[derive(Clone)]
pub(crate) struct DialogProps {
    width: Pixels,
    max_width: Option<Pixels>,
    margin_top: Option<Pixels>,
    close_button: bool,
    overlay: bool,
    overlay_closable: bool,
    overlay_visible: bool,
    keyboard: bool,
}

impl Default for DialogProps {
    fn default() -> Self {
        Self {
            margin_top: None,
            width: px(448.),
            max_width: None,
            overlay: true,
            keyboard: true,
            overlay_visible: false,
            close_button: true,
            overlay_closable: true,
        }
    }
}

/// An alert dialog's icon, title and description, laid out when it renders so its icon can pop.
struct AlertParts {
    icon: Option<AnyElement>,
    title: Option<AnyElement>,
    description: Option<AnyElement>,
}

/// A modal to display content in a dialog box. Pops in, squashes out.
#[derive(IntoElement)]
pub struct Dialog {
    alert: bool,
    style: StyleRefinement,
    children: Vec<AnyElement>,
    trigger: Option<AnyElement>,
    title: Option<AnyElement>,
    alert_parts: Option<AlertParts>,
    footer: Option<AnyElement>,
    content_builder: Option<ContentBuilderFn>,
    props: DialogProps,
    button_props: DialogButtonProps,

    // Set by the layer when it renders an open or closing dialog.
    focus_handle: FocusHandle,
    layer_ix: usize,
    topmost: bool,
    key: u64,
    presence: Presence,
    nudged: Option<Instant>,
}

impl Dialog {
    /// Create a new dialog.
    pub fn new(cx: &mut App) -> Self {
        Self {
            alert: false,
            focus_handle: cx.focus_handle(),
            style: StyleRefinement::default(),
            trigger: None,
            title: None,
            alert_parts: None,
            footer: None,
            content_builder: None,
            props: DialogProps::default(),
            children: Vec::new(),
            layer_ix: 0,
            topmost: true,
            key: 0,
            presence: Presence::new(now()),
            nudged: None,
            button_props: DialogButtonProps::default(),
        }
    }

    /// Sets the trigger element for the dialog.
    ///
    /// When a trigger is set, the dialog will render as a trigger that opens the dialog when
    /// clicked.
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    /// Sets the content of the dialog.
    pub fn content<F>(mut self, builder: F) -> Self
    where
        F: Fn(DialogContent, &mut Window, &mut App) -> DialogContent + 'static,
    {
        self.content_builder = Some(Rc::new(builder));
        self
    }

    /// Sets the title of the dialog.
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = Some(title.into_any_element());
        self
    }

    /// Sets the footer of the dialog, usually for action buttons.
    ///
    /// When you set the footer, the `button_props` will be ignored.
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Set the button props of the dialog.
    pub fn button_props(mut self, button_props: DialogButtonProps) -> Self {
        self.button_props = button_props;
        self
    }

    fn alert(mut self) -> Self {
        self.alert = true;
        self.props.overlay_closable = false;
        self
    }

    /// Sets the callback for when the dialog is closed.
    ///
    /// Called after [`Self::on_ok`] or [`Self::on_cancel`] callback.
    pub fn on_close(
        mut self,
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.button_props.on_close = Rc::new(on_close);
        self
    }

    /// Sets the callback for when the dialog is has been confirmed.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will
    /// not be closed.
    pub fn on_ok(
        mut self,
        on_ok: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.button_props = self.button_props.on_ok(on_ok);
        self
    }

    /// Sets the callback for when the dialog is has been canceled.
    ///
    /// The callback should return `true` to close the dialog, if return `false` the dialog will
    /// not be closed.
    pub fn on_cancel(
        mut self,
        on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.button_props = self.button_props.on_cancel(on_cancel);
        self
    }

    /// Sets the false to hide close icon, default: true
    pub fn close_button(mut self, close_button: bool) -> Self {
        self.props.close_button = close_button;
        self
    }

    /// Set the top offset of the dialog, defaults to None, will use the 1/10 of the viewport
    /// height.
    pub fn margin_top(mut self, margin_top: impl Into<Pixels>) -> Self {
        self.props.margin_top = Some(margin_top.into());
        self
    }

    /// Sets the width of the dialog, defaults to 448px.
    pub fn w(mut self, width: impl Into<Pixels>) -> Self {
        self.props.width = width.into();
        self
    }

    /// Sets the width of the dialog, defaults to 448px.
    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.props.width = width.into();
        self
    }

    /// Set the maximum width of the dialog, defaults to `None`.
    pub fn max_w(mut self, max_width: impl Into<Pixels>) -> Self {
        self.props.max_width = Some(max_width.into());
        self
    }

    /// Set the overlay of the dialog, defaults to `true`.
    pub fn overlay(mut self, overlay: bool) -> Self {
        self.props.overlay = overlay;
        self
    }

    /// Set the overlay closable of the dialog, defaults to `true`.
    pub fn overlay_closable(mut self, overlay_closable: bool) -> Self {
        self.props.overlay_closable = overlay_closable;
        self
    }

    /// Set whether to support keyboard esc to close the dialog, defaults to `true`.
    pub fn keyboard(mut self, keyboard: bool) -> Self {
        self.props.keyboard = keyboard;
        self
    }

    fn with_props(mut self, props: DialogProps) -> Self {
        self.props = props;
        self
    }
}

impl ParentElement for Dialog {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Dialog {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// A length in rems, so it scales with the panel while it pops.
fn as_rems(length: Pixels, rem: Pixels) -> Rems {
    rems(length / rem)
}

impl Dialog {
    fn render_trigger(self, trigger: AnyElement) -> AnyElement {
        let content_builder = self.content_builder.clone();
        let style = self.style.clone();
        let props = self.props.clone();
        let button_props = self.button_props.clone();
        let alert = self.alert;
        let open = move |window: &mut Window, cx: &mut App| {
            let content_builder = content_builder.clone();
            let style = style.clone();
            let props = props.clone();
            let button_props = button_props.clone();
            window.open_dialog(cx, move |dialog, _, _| {
                dialog
                    .when(alert, Dialog::alert)
                    .refine_style(&style)
                    .button_props(button_props.clone())
                    .with_props(props.clone())
                    .when_some(content_builder.clone(), |this, builder| {
                        this.content(move |content, window, cx| builder(content, window, cx))
                    })
            });
        };
        if alert {
            gpui_kit::base::AlertDialogTrigger::new(trigger)
                .on_open(open)
                .into_any_element()
        } else {
            gpui_kit::base::DialogTrigger::new(trigger)
                .on_open(open)
                .into_any_element()
        }
    }

    fn render_header(&mut self, stage: Stage, reduced: bool) -> Option<AnyElement> {
        let parts = self.alert_parts.take()?;
        if parts.icon.is_none() && parts.title.is_none() && parts.description.is_none() {
            return None;
        }
        let key = self.key;
        let icon = parts.icon.map(|icon| {
            let pose = match stage {
                Stage::Open(t) if !reduced => media_pop(t),
                _ => Pose::new(),
            };
            transform(("kk-alert-media", key as usize), pose, icon)
        });
        Some(
            DialogHeader::new()
                .child(
                    h_flex().gap_2().items_start().children(icon).child(
                        v_flex()
                            .flex_1()
                            .min_w_0()
                            .gap_1()
                            .when_some(parts.title, |this, title| {
                                this.child(DialogTitle::new().child(title))
                            })
                            .when_some(parts.description, |this, desc| {
                                this.child(DialogDescription::new().child(desc))
                            }),
                    ),
                )
                .into_any_element(),
        )
    }
}

impl RenderOnce for Dialog {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if let Some(trigger) = self.trigger.take() {
            return self.render_trigger(trigger);
        }

        let reduced = cx.reduce_motion();
        let now = now();
        let stage = self.presence.stage(now);
        let open = matches!(stage, Stage::Open(_));
        let (mut pose, backdrop_alpha, running) = sample(stage, reduced);

        // A press on an alert dialog's backdrop: shake, or under reduced motion flash a ring.
        let rem = window.rem_size();
        let mut ring = None;
        if let Some(nudged) = self.nudged.filter(|_| open) {
            let t = now.saturating_duration_since(nudged);
            if t < NUDGE {
                window.request_animation_frame();
                if reduced {
                    ring = Some(1.0 - Easing::EaseOut.sample(progress(t, NUDGE)));
                } else {
                    // The shake's `transform` replaces the rise's while it runs, as in CSS.
                    pose.yp = 0.0;
                    pose.x = nudge(t) * f32::from(rem);
                }
            }
        }
        // An alert dialog's icon pops until `MEDIA_DELAY + MEDIA`, after the panel has landed.
        let icon_popping = !reduced
            && self
                .alert_parts
                .as_ref()
                .is_some_and(|parts| parts.icon.is_some())
            && matches!(stage, Stage::Open(t) if t < MEDIA_DELAY + MEDIA);
        if running || icon_popping {
            window.request_animation_frame();
        }

        let key = self.key;
        let layer_ix = self.layer_ix;
        let on_close = self.button_props.on_close.clone();
        let on_ok = self.button_props.on_ok.clone();
        let on_cancel = self.button_props.on_cancel.clone();

        let window_paddings = gpui_kit::component::window_paddings(window);
        let view_size = window.viewport_size()
            - gpui_kit::size(
                window_paddings.left + window_paddings.right,
                window_paddings.top + window_paddings.bottom,
            );
        let margin = cx.theme().spacing_tokens().lg;
        let y = self.props.margin_top.unwrap_or(view_size.height / 10.) + px(layer_ix as f32 * 16.);
        let width = self
            .props
            .width
            .min((view_size.width - margin * 2.).max(px(0.)));
        let x = (view_size.width - width) / 2.;
        let max_height = (view_size.height - y - margin).max(px(0.));

        // GPUI Component's 16 px paddings, held in rems so they scale with the panel.
        let base_size = window.text_style().font_size;
        let one: DefiniteLength = rems(1.).into();
        let mut paddings = Edges::all(one);
        let style_padding = |value: Option<DefiniteLength>| {
            value.map(|value| match value {
                DefiniteLength::Absolute(_) => as_rems(value.to_pixels(base_size, rem), rem).into(),
                value => value,
            })
        };
        paddings.left = style_padding(self.style.padding.left).unwrap_or(one);
        paddings.right = style_padding(self.style.padding.right).unwrap_or(one);
        paddings.top = style_padding(self.style.padding.top).unwrap_or(one);
        paddings.bottom = style_padding(self.style.padding.bottom).unwrap_or(one);
        let top_px = paddings.top.to_pixels(base_size.to_pixels(rem).into(), rem);
        let right_px = paddings
            .right
            .to_pixels(base_size.to_pixels(rem).into(), rem);
        let gap = as_rems(top_px.max(px(8.)), rem);
        let close_top = as_rems((top_px - px(10.)).max(px(8.)), rem);
        let close_right = as_rems((right_px - px(10.)).max(px(8.)), rem);

        let alpha = pose.alpha();
        // GPUI paints a shadow under a translucent panel without group compositing, so the ink
        // follows the fade more steeply and never shows through as a slab.
        let ink = alpha * alpha;
        let mut shadow = vec![
            BoxShadow {
                color: hsla(0., 0., 0., 0.1 * ink),
                offset: point(px(0.), px(20.)),
                blur_radius: px(25.),
                spread_radius: px(-5.),
                inset: false,
            },
            BoxShadow {
                color: hsla(0., 0., 0., 0.1 * ink),
                offset: point(px(0.), px(8.)),
                blur_radius: px(10.),
                spread_radius: px(-6.),
                inset: false,
            },
        ];
        if let Some(fade) = ring {
            shadow.push(BoxShadow {
                color: cx.theme().ring.opacity(0.6 * fade),
                offset: point(px(0.), px(0.)),
                blur_radius: px(0.),
                spread_radius: rem * 0.375 * fade,
                inset: false,
            });
        }

        let header = self.render_header(stage, reduced);
        let popup = v_flex()
            .id(("kk-dialog-panel", key as usize))
            .w_full()
            .bg(cx.theme().tokens.background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius_lg)
            .min_h_24()
            .pt(paddings.top)
            .pb(paddings.bottom)
            .gap(gap)
            .refine_style(&self.style)
            .px_0()
            .max_h(max_height)
            .shadow(shadow)
            .when(open, |this| this.occlude())
            .child(
                v_flex()
                    .flex_1()
                    .overflow_hidden()
                    .gap_y_2()
                    .when_some(header, |this, header| {
                        this.child(div().pl(paddings.left).pr(paddings.right).child(header))
                    })
                    .when_some(self.title, |this, title| {
                        this.child(
                            DialogTitle::new()
                                .pl(paddings.left)
                                .pr(paddings.right)
                                .child(title),
                        )
                    })
                    .when_some(self.content_builder, |this, builder| {
                        this.child(builder(
                            DialogContent::new()
                                .gap(paddings.bottom)
                                .pl(paddings.left)
                                .pr(paddings.right),
                            window,
                            cx,
                        ))
                    })
                    .when(!self.children.is_empty(), |this| {
                        this.child(
                            div().flex_1().overflow_hidden().child(
                                v_flex()
                                    .id("kk-dialog-body")
                                    .size_full()
                                    .overflow_y_scroll()
                                    .pl(paddings.left)
                                    .pr(paddings.right)
                                    .children(self.children),
                            ),
                        )
                    }),
            )
            .when_some(self.footer, |this, footer| {
                this.child(div().pl(paddings.left).pr(paddings.right).child(footer))
            })
            .when(self.props.close_button, |this| {
                this.child(
                    div()
                        .absolute()
                        .top(close_top)
                        .right(close_right)
                        .child(SquashClose),
                )
            })
            // Leaving, the panel takes no input: a cover over its controls swallows a second
            // click, so a footer action can't fire twice nor the click reach a dialog below.
            .when(!open, |this| {
                this.child(div().absolute().inset_0().occlude())
            });

        let panel = div()
            .absolute()
            .left(x)
            .top(y)
            .w(width)
            .when_some(self.props.max_width, |this, w| this.max_w(w))
            .flex()
            .flex_col()
            .child(transform(("kk-dialog-pop", key as usize), pose, popup));

        let overlay = self.props.overlay_visible.then(|| {
            div()
                .absolute()
                .size_full()
                .bg(overlay_color(cx).opacity(backdrop_alpha))
        });

        if !open {
            // Leaving: drawn where it was, inert, until the exit has played.
            return deferred(
                anchored()
                    .position(point(window_paddings.left, window_paddings.top))
                    .child(
                        div()
                            .w(view_size.width)
                            .h(view_size.height)
                            .children(overlay)
                            .child(panel),
                    ),
            )
            .with_priority(10 + layer_ix)
            .into_any_element();
        }

        let alert = self.alert;
        let backdrop = div()
            .absolute()
            .size_full()
            .window_control_area(WindowControlArea::Drag)
            .when_some(overlay, |this, overlay| this.child(overlay))
            .when(alert, |this| {
                this.on_mouse_down(MouseButton::Left, move |event, window, cx| {
                    if event.position.y >= TITLE_BAR_HEIGHT {
                        root::nudge_dialog(key, window, cx);
                    }
                })
            });

        anchored()
            .position(point(window_paddings.left, window_paddings.top))
            .snap_to_window()
            .child(
                div()
                    .id(("kk-dialog", key as usize))
                    .occlude()
                    .w(view_size.width)
                    .h(view_size.height)
                    .child(
                        BaseDialogRoot::new(alert, cx)
                            .layer(layer_ix, self.topmost)
                            .focus_handle(self.focus_handle.clone())
                            .close_on_escape(self.props.keyboard)
                            .close_on_backdrop_press(self.props.overlay_closable)
                            .dismiss_below_y(TITLE_BAR_HEIGHT)
                            .when(self.props.overlay, |this| this.backdrop(backdrop))
                            .on_ok(move |event, window, cx| on_ok(event, window, cx))
                            .on_cancel(move |event, window, cx| on_cancel(event, window, cx))
                            .on_close(move |event, window, cx| on_close(event, window, cx))
                            .request_close(move |deferred, window, cx| {
                                root::close_dialog(deferred, window, cx);
                            })
                            .popup(panel),
                    ),
            )
            .into_any_element()
    }
}

fn overlay_color(cx: &App) -> Hsla {
    cx.theme().overlay
}

/// Draws the open and closing dialogs of the window.
pub(crate) fn render_layer(window: &mut Window, cx: &mut App) -> Option<AnyElement> {
    let layers = root::layers(window, cx);
    let now = now();
    let exit = exit_duration(cx);
    let entries = layers.update(cx, |layers, _| {
        layers
            .dialogs
            .retain(|dialog| !dialog.presence.gone(now, exit));
        layers
            .dialogs
            .iter()
            .map(|dialog| {
                (
                    dialog.key,
                    dialog.builder.clone(),
                    dialog.focus_handle.clone(),
                    dialog.presence,
                    dialog.layer_ix,
                    dialog.nudged,
                )
            })
            .collect::<Vec<_>>()
    });
    if entries.is_empty() {
        return None;
    }
    let open_count = entries.iter().filter(|entry| entry.3.is_open()).count();
    let mut open_ix = 0;
    let mut dialogs = entries
        .into_iter()
        .map(|(key, builder, focus_handle, presence, layer_ix, nudged)| {
            let mut dialog = builder(Dialog::new(cx), window, cx);
            dialog.focus_handle = focus_handle;
            dialog.key = key;
            dialog.presence = presence;
            dialog.layer_ix = layer_ix;
            dialog.nudged = nudged;
            if presence.is_open() {
                open_ix += 1;
                dialog.topmost = open_ix == open_count;
            }
            dialog
        })
        .collect::<Vec<_>>();

    // One backdrop: the top open dialog's, or while none is open, the last leaving one's.
    let shown = dialogs
        .iter()
        .rposition(|dialog| dialog.presence.is_open() && dialog.props.overlay)
        .or_else(|| {
            (open_count == 0)
                .then(|| dialogs.iter().rposition(|dialog| dialog.props.overlay))
                .flatten()
        });
    if let Some(ix) = shown {
        dialogs[ix].props.overlay_visible = true;
    }
    Some(
        div()
            .children(dialogs.into_iter().map(|dialog| {
                div()
                    .id(("kk-dialog-layer", dialog.key as usize))
                    .child(dialog)
            }))
            .into_any_element(),
    )
}

/// AlertDialog is a modal dialog that interrupts the user with important content and expects a
/// response. It pops like [`Dialog`], its icon pops in after the panel, and pressing its backdrop
/// shakes it instead of closing it.
#[derive(IntoElement)]
pub struct AlertDialog {
    base: Dialog,
    trigger: Option<AnyElement>,
    icon: Option<AnyElement>,
    title: Option<AnyElement>,
    description: Option<AnyElement>,
    button_props: DialogButtonProps,
    children: Vec<AnyElement>,
}

impl AlertDialog {
    /// Create a new AlertDialog.
    ///
    /// By default, the dialog is not overlay closable with a OK button.
    pub fn new(cx: &mut App) -> Self {
        Self {
            base: Dialog::new(cx).alert().close_button(false),
            trigger: None,
            icon: None,
            title: None,
            description: None,
            button_props: DialogButtonProps::default(),
            children: Vec::new(),
        }
    }

    /// Set to use confirm dialog, with OK and Cancel buttons.
    pub fn confirm(mut self) -> Self {
        self.button_props.show_cancel = true;
        self
    }

    /// Sets the trigger element for the alert dialog.
    ///
    /// The `title`, `description`, `icon`, and `button_props` will be ignored when used together
    /// with `.trigger()`; define the content with `.content()`.
    pub fn trigger(mut self, trigger: impl IntoElement) -> Self {
        self.trigger = Some(trigger.into_any_element());
        self
    }

    /// Sets the content builder for declarative API.
    pub fn content<F>(mut self, builder: F) -> Self
    where
        F: Fn(DialogContent, &mut Window, &mut App) -> DialogContent + 'static,
    {
        self.base = self.base.content(builder);
        self
    }

    /// Sets the footer for declarative API.
    ///
    /// If not set, a default footer with OK and optional Cancel button will be used.
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.base = self.base.footer(footer);
        self
    }

    #[track_caller]
    fn debug_assert_no_trigger(&self) {
        debug_assert!(
            self.trigger.is_none() && self.base.content_builder.is_none(),
            "Cannot set this property when trigger is used. Use content() to define dialog content instead."
        );
    }

    /// Sets the icon of the alert dialog, default is None. It pops in after the panel.
    #[track_caller]
    pub fn icon(mut self, icon: impl IntoElement) -> Self {
        self.debug_assert_no_trigger();
        self.icon = Some(icon.into_any_element());
        self
    }

    /// Sets the title of the alert dialog.
    #[track_caller]
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.debug_assert_no_trigger();
        self.title = Some(title.into_any_element());
        self
    }

    /// Sets the description of the alert dialog.
    #[track_caller]
    pub fn description(mut self, description: impl IntoElement) -> Self {
        self.debug_assert_no_trigger();
        self.description = Some(description.into_any_element());
        self
    }

    /// Set the button props of the alert dialog.
    #[track_caller]
    pub fn button_props(mut self, button_props: DialogButtonProps) -> Self {
        self.debug_assert_no_trigger();
        self.button_props = button_props;
        self
    }

    /// Sets the width of the alert dialog.
    pub fn width(mut self, width: impl Into<Pixels>) -> Self {
        self.base = self.base.width(width);
        self
    }

    /// Show cancel button. Default is false.
    pub fn show_cancel(mut self, show_cancel: bool) -> Self {
        self.button_props = self.button_props.show_cancel(show_cancel);
        self
    }

    /// Alert dialogs never close from a backdrop press.
    #[deprecated(note = "AlertDialog backdrop dismissal is disabled by design")]
    pub fn overlay_closable(self, _: bool) -> Self {
        self
    }

    /// Set the close button of the alert dialog, defaults to `false`.
    pub fn close_button(mut self, close_button: bool) -> Self {
        self.base = self.base.close_button(close_button);
        self
    }

    /// Set whether to support keyboard esc to close the dialog, defaults to `true`.
    pub fn keyboard(mut self, keyboard: bool) -> Self {
        self.base = self.base.keyboard(keyboard);
        self
    }

    /// Sets the callback for when the alert dialog is closed.
    pub fn on_close(
        mut self,
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.base = self.base.on_close(on_close);
        self
    }

    /// Sets the callback for when the OK/action button is clicked.
    ///
    /// The callback should return `true` to close the dialog.
    pub fn on_ok(
        mut self,
        on_ok: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.button_props = self.button_props.on_ok(on_ok);
        self
    }

    /// Sets the callback for when the alert dialog has been canceled.
    ///
    /// The callback should return `true` to close the dialog.
    pub fn on_cancel(
        mut self,
        on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.button_props = self.button_props.on_cancel(on_cancel);
        self
    }

    /// Builds the dialog surface: header parts, children and the default footer.
    pub(crate) fn build_surface(self, _: &mut Window, _: &mut App) -> Dialog {
        let mut button_props = self.button_props.clone();
        button_props.on_close = self.base.button_props.on_close.clone();
        let has_footer = self.base.footer.is_some();
        let mut dialog = self.base.button_props(button_props.clone());
        dialog.alert_parts = Some(AlertParts {
            icon: self.icon,
            title: self.title,
            description: self.description,
        });
        dialog.children.extend(self.children);
        if !has_footer {
            dialog = dialog.footer(
                DialogFooter::new()
                    .when(button_props.show_cancel, |this| {
                        this.child(button_props.render_cancel())
                    })
                    .child(button_props.render_ok()),
            );
        }
        dialog
    }
}

impl Styled for AlertDialog {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.base.style
    }
}

impl ParentElement for AlertDialog {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for AlertDialog {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        if let Some(trigger) = self.trigger.take() {
            let mut base = self.base;
            let mut button_props = self.button_props;
            button_props.on_close = base.button_props.on_close.clone();
            base.button_props = button_props;
            base.render_trigger(trigger)
        } else {
            self.build_surface(window, cx).into_any_element()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn pop_in_keyframes() {
        let start = pop_in(Duration::ZERO);
        assert!(close(start.sx, 0.6) && close(start.opacity, 0.0));
        assert!(
            close(start.yp, 0.04 * 0.6),
            "the rise is scaled with the panel"
        );
        let peak = pop_in(ms(200));
        assert!(close(peak.uniform_scale(), (1.04_f32 * 1.06).sqrt()));
        assert_eq!(pop_in(ms(400)), Pose::new());
    }

    #[test]
    fn pop_in_matches_the_web_keyframes() {
        for (component, name) in [
            ("pop-dialog", "kk-pop-dialog-in"),
            ("pop-alert-dialog", "kk-pop-alert-dialog-in"),
            ("pop-command", "kk-pop-command-dialog-in"),
        ] {
            crate::parity::assert_number_track(component, name, "opacity", &pop_in_opacity());
            crate::parity::assert_number_track(component, name, "sx", &pop_in_sx());
            crate::parity::assert_number_track(component, name, "sy", &pop_in_sy());
            crate::parity::assert_number_track(component, name, "yp", &pop_in_rise());
        }
    }

    #[test]
    fn pop_out_matches_the_web_keyframes() {
        for (component, name) in [
            ("pop-dialog", "kk-pop-dialog-out"),
            ("pop-alert-dialog", "kk-pop-alert-dialog-out"),
            ("pop-command", "kk-pop-command-dialog-out"),
        ] {
            crate::parity::assert_pose_track(component, name, &pop_out_track(), &[]);
            let opacity = Track::new(Easing::EaseIn)
                .at(0.0, 1.0_f32)
                .at(1.0, 0.0)
                .build();
            crate::parity::assert_number_track(component, name, "opacity", &opacity);
        }
    }

    #[test]
    fn nudge_matches_the_web_keyframes() {
        // The CSS is in rems and the parser reads the bare number, like the track's rems.
        crate::parity::assert_number_track(
            "pop-alert-dialog",
            "kk-pop-alert-dialog-nudge",
            "x",
            &nudge_track(),
        );
    }

    #[test]
    fn media_matches_the_web_keyframes() {
        crate::parity::assert_pose_track(
            "pop-alert-dialog",
            "kk-pop-alert-dialog-media",
            &media_scale(),
            &[],
        );
        assert!(close(media_pop(ms(150)).opacity, 0.0));
        assert!(close(media_pop(ms(250)).opacity, 1.0), "opaque by 20 %");
        assert_eq!(media_pop(ms(650)), Pose::new());
    }

    #[test]
    fn fades_match_the_web_keyframes() {
        let fade_in = Track::new(Easing::EaseOut)
            .at(0.0, 0.0_f32)
            .at(1.0, 1.0)
            .build();
        let fade_out = Track::new(Easing::EaseIn)
            .at(0.0, 1.0_f32)
            .at(1.0, 0.0)
            .build();
        for (component, prefix) in [
            ("pop-dialog", "kk-pop-dialog"),
            ("pop-alert-dialog", "kk-pop-alert-dialog"),
            ("pop-command", "kk-pop-command"),
        ] {
            let (fade_in_name, fade_out_name) =
                (format!("{prefix}-fade-in"), format!("{prefix}-fade-out"));
            crate::parity::assert_number_track(component, &fade_in_name, "opacity", &fade_in);
            crate::parity::assert_number_track(component, &fade_out_name, "opacity", &fade_out);
        }
    }

    struct Host;

    impl gpui_kit::Render for Host {
        fn render(
            &mut self,
            window: &mut Window,
            cx: &mut gpui_kit::Context<Self>,
        ) -> impl IntoElement {
            div()
                .size_full()
                .children(crate::Root::render_dialog_layer(window, cx))
        }
    }

    /// A second click on a closing dialog's footer lands on nothing: it neither runs the action
    /// again nor reaches what lies under the panel.
    #[gpui_kit::test]
    fn a_leaving_dialog_takes_no_clicks(cx: &mut gpui_kit::TestAppContext) {
        use std::cell::Cell;
        cx.update(|cx| {
            gpui_kit::component::init(cx);
            crate::init(cx);
        });
        let start = Instant::now();
        crate::motion::freeze_time(Some(start));
        let (_, cx) = cx.add_window_view(|_, _| Host);
        let clicks = Rc::new(Cell::new(0));
        cx.update(|window, cx| {
            let clicks = clicks.clone();
            window.open_dialog(cx, move |dialog, _, _| {
                let clicks = clicks.clone();
                dialog.footer(
                    div()
                        .id("act")
                        .debug_selector(|| "act".into())
                        .size(px(80.))
                        .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
                )
            });
        });
        // Opened, and landed.
        crate::motion::freeze_time(Some(start + ms(1000)));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let act = cx
            .debug_bounds("act")
            .expect("the footer is drawn")
            .center();
        cx.simulate_click(act, gpui_kit::Modifiers::none());
        assert_eq!(clicks.get(), 1);

        // Closed, and half way through its exit.
        cx.update(|window, cx| window.close_dialog(cx));
        crate::motion::freeze_time(Some(start + ms(1000) + POP_OUT / 2));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_click(act, gpui_kit::Modifiers::none());
        crate::motion::freeze_time(None);
        assert_eq!(clicks.get(), 1, "the leaving dialog ran its action again");
    }
}
