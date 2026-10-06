//! Notification: a toast that drops in from a pin and swings to rest, and slides away on close.
//!
//! Replaces `gpui_kit::component::notification`. `Notification` and `NotificationList` are
//! Kirakira's, with GPUI Component's builders; `NotificationType`, `NotificationDelivery` and
//! `NotificationSettings` are re-exported. Push one with
//! [`crate::WindowExt::push_notification`] and draw the list with
//! [`crate::Root::render_notification_layer`] (see [`crate::root`]).
//!
//! The toast is GPUI Component's: its icon, title, message, action, hover close button, width and
//! stacking (gpui-base's `ToastStack` and `ToastManager`: newest in front, the stack fans out on
//! hover, five seconds before it hides). The motion is Pop Toast's:
//!
//! - In, 0.55 s, ease-in-out on every segment: it drops from 1.25 rem up to 0.2 rem past its rest
//!   at 40 % and settles at 62 %, fading in by 20 %. It hangs from a pin at its top centre and
//!   swings to rest, each swing half the last: 6° → −3° (40 %) → 1.5° (62 %) → −0.5° (82 %) → 0.
//! - Out, 0.2 s on Kirakira's ease-in curve: it slides off sideways, 105 % of its width, and
//!   fades.
//! - Reduced motion: it fades in (0.2 s) and out (0.15 s).
//!
//! Where it differs from the web version, and why:
//!
//! - GPUI can't rotate text or boxes, so while a plain toast (title, message, type icon; no
//!   action, content, custom icon or style) swings, it is drawn as [vector layers](crate::vector)
//!   turned exactly on its pin: card, border, shadow, icon and text. The real toast sits under it,
//!   hidden, and takes over at rest, so the final frame is GPUI's own. Vector text is a touch
//!   softer while it moves. A toast with other content, or a line too long for one line, can't
//!   be drawn that way: it sways instead, moving the way its centre moves when it turns about the
//!   pin (`h / 2 · sin θ` across), without the tilt.
//! - It leaves to the right, one of the web's swipe directions, or to the left from a left-hand
//!   stack, and there is no swipe gesture (GPUI Component has none either).
//! - System delivery goes through GPUI Component's notification list, which owns the platform's
//!   single response handler. Clicking the system notification runs `on_click` but doesn't close
//!   the in-app toast.

use std::any::TypeId;
use std::borrow::Cow;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use gpui_kit::base::{
    ElementExt as _, StyledExt as _, Toast as BaseToast, ToastManager, ToastMotion, ToastOptions,
    ToastStack, ToastStackState, ToastTransitionStatus,
};
use gpui_kit::component::button::{Button as BaseButton, ButtonVariants as _};
use gpui_kit::component::notification::Notification as BaseNotification;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, Sizable, v_flex};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::{
    Anchor, AnyElement, App, AppContext as _, BoxShadow, ClickEvent, Context, DismissEvent,
    ElementId, Entity, EventEmitter, FocusHandle, InteractiveElement as _, IntoElement,
    ParentElement as _, Pixels, Render, SharedString, Size, StatefulInteractiveElement as _,
    StyleRefinement, Styled, Subscription, Window, div, hsla, point, px,
};

pub use gpui_kit::component::notification::{
    NotificationDelivery, NotificationSettings, NotificationType,
};

use crate::icons;
use crate::motion::{Easing, Keyframes, Pose, Track, ms, now, transform};
use crate::vector::{Layer, TextSetting, Vector, svg_family};

const IN: Duration = ms(550);
const OUT: Duration = ms(200);
const FADE_IN: Duration = ms(200);
const FADE_OUT: Duration = ms(150);
const ADVANCE_INTERVAL: Duration = ms(50);

fn progress(elapsed: Duration, duration: Duration) -> f32 {
    (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}

/// `kk-pop-toast-in`'s drop, in rems: −1.25 → 0.2 past rest at 40 % → rest at 62 %.
pub(crate) fn drop_track() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, -1.25_f32)
        .at(0.4, 0.2)
        .at(0.62, 0.0)
        .at(1.0, 0.0)
        .build()
}

/// `kk-pop-toast-in`'s swing, in degrees: 6 → −3 (40 %) → 1.5 (62 %) → −0.5 (82 %) → 0.
pub(crate) fn swing_track() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 6.0_f32)
        .at(0.4, -3.0)
        .at(0.62, 1.5)
        .at(0.82, -0.5)
        .at(1.0, 0.0)
        .build()
}

/// `kk-pop-toast-in`'s opacity: 0 → 1 by 20 %.
pub(crate) fn fade_track() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.0_f32)
        .at(0.2, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// `kk-pop-toast-in` at `elapsed`: the drop in rems, the swing in degrees and the opacity.
pub(crate) fn drop_in(elapsed: Duration) -> (f32, f32, f32) {
    let p = progress(elapsed, IN);
    (
        drop_track().sample(p),
        swing_track().sample(p),
        fade_track().sample(p),
    )
}

/// Where the centre of a box `height` tall moves when it turns `degrees` about its top centre.
pub(crate) fn sway(degrees: f32, height: f32) -> (f32, f32) {
    let (sin, cos) = degrees.to_radians().sin_cos();
    let arm = height / 2.0;
    (-arm * sin, arm * (cos - 1.0))
}

/// shadcn's `shadow-lg` at `strength` of its ink, as GPUI Component draws its toasts.
fn toast_shadow(strength: f32) -> Vec<BoxShadow> {
    let ink = hsla(0., 0., 0., 0.1 * strength.clamp(0., 1.));
    vec![
        BoxShadow {
            color: ink,
            offset: point(px(0.), px(10.)),
            blur_radius: px(7.5),
            spread_radius: px(-3.),
            inset: false,
        },
        BoxShadow {
            color: ink,
            offset: point(px(0.), px(4.)),
            blur_radius: px(3.),
            spread_radius: px(-4.),
            inset: false,
        },
    ]
}

struct DismissRequest;

#[derive(Debug, PartialEq, Clone, Hash, Eq)]
pub(crate) enum NotificationId {
    Id(TypeId),
    IdAndElementId(TypeId, ElementId),
}

impl From<TypeId> for NotificationId {
    fn from(type_id: TypeId) -> Self {
        Self::Id(type_id)
    }
}

impl From<(TypeId, ElementId)> for NotificationId {
    fn from((type_id, id): (TypeId, ElementId)) -> Self {
        Self::IdAndElementId(type_id, id)
    }
}

/// The same id on a GPUI Component notification, for system delivery, and its removal.
#[derive(Clone)]
struct SystemId {
    apply: Rc<dyn Fn(BaseNotification) -> BaseNotification>,
    retract: Rc<dyn Fn(&mut Window, &mut App)>,
}

impl SystemId {
    fn of<T: 'static>() -> Self {
        Self {
            apply: Rc::new(|note| note.id::<T>()),
            retract: Rc::new(|window, cx| {
                gpui_kit::component::WindowExt::remove_notification::<T>(window, cx)
            }),
        }
    }

    fn of1<T: 'static>(key: ElementId) -> Self {
        let retract_key = key.clone();
        Self {
            apply: Rc::new(move |note| note.id1::<T>(key.clone())),
            retract: Rc::new(move |window, cx| {
                gpui_kit::component::WindowExt::remove_notification1::<T>(
                    window,
                    retract_key.clone(),
                    cx,
                )
            }),
        }
    }
}

struct DefaultIdType;

type ActionBuilder =
    Rc<dyn Fn(&mut Notification, &mut Window, &mut Context<Notification>) -> AnyElement>;
type ContentBuilder =
    Rc<dyn Fn(&mut Notification, &mut Window, &mut Context<Notification>) -> AnyElement>;

/// A notification element.
pub struct Notification {
    /// Pushing a notification with the same id replaces the previous one.
    id: NotificationId,
    system_id: SystemId,
    style: StyleRefinement,
    type_: Option<NotificationType>,
    title: Option<SharedString>,
    message: Option<SharedString>,
    icon: Option<Icon>,
    placement: Option<Anchor>,
    delivery: Option<NotificationDelivery>,
    autohide: bool,
    action_builder: Option<ActionBuilder>,
    content_builder: Option<ContentBuilder>,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    on_close: Option<Rc<dyn Fn(&mut Window, &mut App)>>,
    transition_status: ToastTransitionStatus,
    opened: Instant,
    closing: Option<Instant>,
    /// The toast's size as last laid out, for the swing.
    size: Rc<Cell<Size<Pixels>>>,
}

impl From<String> for Notification {
    fn from(s: String) -> Self {
        Self::new().message(s)
    }
}

impl From<SharedString> for Notification {
    fn from(s: SharedString) -> Self {
        Self::new().message(s)
    }
}

impl From<&str> for Notification {
    fn from(s: &str) -> Self {
        Self::new().message(s)
    }
}

impl<'a> From<Cow<'a, str>> for Notification {
    fn from(s: Cow<'a, str>) -> Self {
        Self::new().message(s)
    }
}

impl<T> From<(NotificationType, T)> for Notification
where
    T: Into<SharedString>,
{
    fn from((type_, content): (NotificationType, T)) -> Self {
        Self::new().message(content).with_type(type_)
    }
}

impl Default for Notification {
    fn default() -> Self {
        Self::new()
    }
}

impl Notification {
    /// Create a new notification with a unique id.
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let key: ElementId = SharedString::from(format!(
            "kk-notification-{}",
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
        .into();
        Self {
            id: (TypeId::of::<DefaultIdType>(), key.clone()).into(),
            system_id: SystemId::of1::<DefaultIdType>(key),
            style: StyleRefinement::default(),
            title: None,
            message: None,
            type_: None,
            icon: None,
            placement: None,
            delivery: None,
            autohide: true,
            action_builder: None,
            content_builder: None,
            on_click: None,
            on_close: None,
            transition_status: ToastTransitionStatus::Starting,
            opened: now(),
            closing: None,
            size: Rc::new(Cell::new(Size::default())),
        }
    }

    /// Set the message of the notification, default is None.
    pub fn message(mut self, message: impl Into<SharedString>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Create an info notification with the given message.
    pub fn info(message: impl Into<SharedString>) -> Self {
        Self::new()
            .message(message)
            .with_type(NotificationType::Info)
    }

    /// Create a success notification with the given message.
    pub fn success(message: impl Into<SharedString>) -> Self {
        Self::new()
            .message(message)
            .with_type(NotificationType::Success)
    }

    /// Create a warning notification with the given message.
    pub fn warning(message: impl Into<SharedString>) -> Self {
        Self::new()
            .message(message)
            .with_type(NotificationType::Warning)
    }

    /// Create an error notification with the given message.
    pub fn error(message: impl Into<SharedString>) -> Self {
        Self::new()
            .message(message)
            .with_type(NotificationType::Error)
    }

    /// Set the type for unique identification of the notification.
    pub fn id<T: Sized + 'static>(mut self) -> Self {
        self.id = TypeId::of::<T>().into();
        self.system_id = SystemId::of::<T>();
        self
    }

    /// Set the type and id of the notification, used to uniquely identify the notification.
    pub fn id1<T: Sized + 'static>(mut self, key: impl Into<ElementId>) -> Self {
        let key = key.into();
        self.id = (TypeId::of::<T>(), key.clone()).into();
        self.system_id = SystemId::of1::<T>(key);
        self
    }

    /// Set the title of the notification, default is None.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the icon of the notification. Without one, the type's icon is used.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the type of the notification.
    pub fn with_type(mut self, type_: NotificationType) -> Self {
        self.type_ = Some(type_);
        self
    }

    /// Set the placement of the notification, overriding [`NotificationSettings::placement`].
    pub fn placement(mut self, placement: Anchor) -> Self {
        self.placement = Some(placement);
        self
    }

    /// Set where this notification is delivered, overriding
    /// [`NotificationSettings::delivery`]. See GPUI Component's `Notification::delivery` for the
    /// platform requirements of system delivery.
    pub fn delivery(mut self, delivery: NotificationDelivery) -> Self {
        self.delivery = Some(delivery);
        self
    }

    /// Deliver this notification only to the OS notification center.
    pub fn system(self) -> Self {
        self.delivery(NotificationDelivery::System)
    }

    /// Deliver this notification as an in-app toast and to the OS notification center.
    pub fn in_app_and_system(self) -> Self {
        self.delivery(NotificationDelivery::InAppAndSystem)
    }

    /// Set the auto hide of the notification, default is true.
    pub fn autohide(mut self, autohide: bool) -> Self {
        self.autohide = autohide;
        self
    }

    /// Set the click callback of the notification.
    pub fn on_click(
        mut self,
        on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(on_click));
        self
    }

    /// Set the close callback of the notification, called however it closes.
    pub fn on_close(mut self, on_close: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Rc::new(on_close));
        self
    }

    /// Set the action button of the notification: GPUI Component's `Button` or Kirakira's.
    ///
    /// When an action is set, the notification will not autohide.
    pub fn action<F, B>(mut self, action: F) -> Self
    where
        F: Fn(&mut Self, &mut Window, &mut Context<Self>) -> B + 'static,
        B: Sizable + Styled + IntoElement,
    {
        self.action_builder = Some(Rc::new(move |this, window, cx| {
            action(this, window, cx).small().mr_3p5().into_any_element()
        }));
        self.autohide = false;
        self
    }

    /// Dismiss the notification.
    pub fn dismiss(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DismissRequest);
    }

    /// Set the content of the notification.
    pub fn content(
        mut self,
        content: impl Fn(&mut Self, &mut Window, &mut Context<Self>) -> AnyElement + 'static,
    ) -> Self {
        self.content_builder = Some(Rc::new(content));
        self
    }

    fn begin_close(&mut self, cx: &mut Context<Self>) {
        if self.transition_status != ToastTransitionStatus::Ending {
            self.transition_status = ToastTransitionStatus::Ending;
            self.closing = Some(now());
            cx.notify();
        }
    }

    fn complete_enter(&mut self, cx: &mut Context<Self>) {
        if self.transition_status == ToastTransitionStatus::Starting {
            self.transition_status = ToastTransitionStatus::Present;
            cx.notify();
        }
    }

    fn complete_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.emit(DismissEvent);
        if let Some(on_close) = self.on_close.clone() {
            on_close(window, cx);
        }
    }

    fn type_icon(type_: NotificationType, cx: &App) -> Icon {
        let theme = cx.theme();
        match type_ {
            NotificationType::Info => Icon::new(IconName::Info).text_color(theme.info),
            NotificationType::Success => Icon::new(IconName::CircleCheck).text_color(theme.success),
            NotificationType::Warning => {
                Icon::new(IconName::TriangleAlert).text_color(theme.warning)
            }
            NotificationType::Error => Icon::new(IconName::CircleX).text_color(theme.danger),
        }
    }

    /// The toast's pose at `now` and whether it is still moving.
    fn pose_at(&self, now: Instant, placement: Anchor, rem: f32, reduced: bool) -> (Pose, bool) {
        match self.closing {
            Some(closing) => {
                let t = now.saturating_duration_since(closing);
                if reduced {
                    let e = Easing::EaseIn.sample(progress(t, FADE_OUT));
                    return (Pose::new().opacity(1.0 - e), t < FADE_OUT);
                }
                let e = crate::theme::Curves::default()
                    .r#in
                    .sample(progress(t, OUT));
                let side = match placement {
                    Anchor::TopLeft | Anchor::BottomLeft | Anchor::LeftCenter => -1.0,
                    _ => 1.0,
                };
                (Pose::new().xp(1.05 * side * e).opacity(1.0 - e), t < OUT)
            }
            None => {
                let t = now.saturating_duration_since(self.opened);
                if reduced {
                    let o = Easing::EaseOut.sample(progress(t, FADE_IN));
                    return (Pose::new().opacity(o), t < FADE_IN);
                }
                let (drop, swing, opacity) = drop_in(t);
                let (dx, dy) = sway(swing, f32::from(self.size.get().height));
                (Pose::new().at(dx, drop * rem + dy).opacity(opacity), t < IN)
            }
        }
    }
}

/// Room around the vector toast for its shadow.
const SHADOW_ROOM: f32 = 24.0;

/// Lucide's circle-x, for the error type.
const CIRCLE_X: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20z"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/></svg>"#;

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn open_svg(w: f32, h: f32) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}">"#
    )
}

fn rect(w: f32, h: f32, (x, y, rw, rh): (f32, f32, f32, f32), r: f32, blur: f32) -> String {
    let r = r.min(rw / 2.0).min(rh / 2.0).max(0.0);
    let (filter, use_filter) = if blur > 0.0 {
        (
            format!(
                r#"<defs><filter id="b" filterUnits="userSpaceOnUse" x="0" y="0" width="{w}" height="{h}"><feGaussianBlur stdDeviation="{}"/></filter></defs>"#,
                blur / 2.0
            ),
            r#" filter="url(#b)""#,
        )
    } else {
        (String::new(), "")
    };
    format!(
        r#"{}{filter}<rect x="{x}" y="{y}" width="{rw}" height="{rh}" rx="{r}" ry="{r}" fill="black"{use_filter}/></svg>"#,
        open_svg(w, h)
    )
}

impl Notification {
    /// The toast drawn as vector layers turned on its pin, while it swings in. Only a plain
    /// toast (a title, a message, a type icon, no action, content or custom style) can be drawn
    /// so; anything else sways instead.
    fn swing_vector(
        &self,
        has_icon: bool,
        rem: f32,
        window: &Window,
        cx: &App,
    ) -> Option<AnyElement> {
        if cx.reduce_motion()
            || self.closing.is_some()
            || now().saturating_duration_since(self.opened) >= IN
            || self.content_builder.is_some()
            || self.action_builder.is_some()
            || (self.icon.is_some() && self.type_.is_none())
            || self.style != StyleRefinement::default()
        {
            return None;
        }
        let size = self.size.get();
        let (w, h) = (f32::from(size.width), f32::from(size.height));
        if w <= 0.0 || h <= 0.0 {
            return None;
        }
        let (drop, swing, opacity) = drop_in(now().saturating_duration_since(self.opened));
        let pose = Pose::new().y(drop * rem).rotate(swing).opacity(opacity);
        let theme = cx.theme();
        let room = SHADOW_ROOM;
        let (bw, bh) = (w + room * 2.0, h + room * 2.0);
        let radius = f32::from(theme.radius_lg);
        let alpha = pose.alpha();
        let ink = 0.1 * alpha * alpha;

        // The text, set as GPUI sets it: from the inside of the 1 px border and the padding.
        let left = room + 1.0 + rem + if has_icon { 1.5 * rem } else { 0.0 };
        let available = w - 2.0 - 2.0 * rem - if has_icon { 1.5 * rem } else { 0.0 };
        let mut top = room + 1.0 + 0.875 * rem;
        let mut text = Vec::new();
        for (line, weight) in [
            (self.title.clone(), gpui_kit::FontWeight::SEMIBOLD),
            (self.message.clone(), gpui_kit::FontWeight::NORMAL),
        ] {
            let Some(line) = line else { continue };
            let refinement = gpui_kit::TextStyleRefinement {
                font_size: Some(gpui_kit::rems(0.875).into()),
                font_weight: Some(weight),
                ..Default::default()
            };
            let setting = TextSetting::new(&refinement, window);
            let line_box = setting.line_box(&line, window);
            if f32::from(line_box.size.width) > available {
                // It wraps in GPUI; a vector line wouldn't.
                return None;
            }
            let markup = format!(
                r#"{}<text x="{left}" y="{}" font-family="{}" font-weight="{}" font-size="{}" fill="black">{}</text></svg>"#,
                open_svg(bw, bh),
                top + f32::from(line_box.baseline),
                escape(&svg_family(&setting.face.family)),
                setting.face.weight.0.round() as u32,
                f32::from(setting.face.size),
                escape(&line),
            );
            text.push(Layer::svg(markup, setting.color));
            top += f32::from(setting.line_height);
        }

        let mut layers = vec![
            // shadcn's `shadow-lg`, as GPUI Component draws it.
            Layer::svg(
                rect(
                    bw,
                    bh,
                    (room + 3.0, room + 13.0, w - 6.0, h - 6.0),
                    radius - 3.0,
                    7.5,
                ),
                hsla(0., 0., 0., ink),
            ),
            Layer::svg(
                rect(
                    bw,
                    bh,
                    (room + 4.0, room + 8.0, w - 8.0, h - 8.0),
                    radius - 4.0,
                    3.0,
                ),
                hsla(0., 0., 0., ink),
            ),
            Layer::svg(rect(bw, bh, (room, room, w, h), radius, 0.0), theme.border),
            Layer::svg(
                rect(
                    bw,
                    bh,
                    (room + 1.0, room + 1.0, w - 2.0, h - 2.0),
                    radius - 1.0,
                    0.0,
                ),
                theme.tokens.popover,
            ),
        ];
        if let Some(type_) = self.type_ {
            let (icon, color) = match type_ {
                NotificationType::Info => (icons::INFO, theme.info),
                NotificationType::Success => (icons::CIRCLE_CHECK, theme.success),
                NotificationType::Warning => (icons::TRIANGLE_ALERT, theme.warning),
                NotificationType::Error => (CIRCLE_X, theme.danger),
            };
            let (x, y, side) = (room + 1.0 + rem, room + 1.0 + 18.0, rem);
            let nested = icon.replacen(
                "<svg ",
                &format!(r#"<svg x="{x}" y="{y}" width="{side}" height="{side}" "#),
                1,
            );
            layers.push(Layer::svg(
                format!("{}{nested}</svg>", open_svg(bw, bh)),
                color,
            ));
        }
        layers.extend(text);

        Some(
            div()
                .absolute()
                .left(px(-room))
                .top(px(-room))
                .child(
                    Vector::new(gpui_kit::size(px(bw), px(bh)))
                        .layers(layers)
                        .pose(pose)
                        .origin(0.5, room / bh),
                )
                .into_any_element(),
        )
    }
}

impl EventEmitter<DismissEvent> for Notification {}
impl EventEmitter<DismissRequest> for Notification {}
impl FluentBuilder for Notification {}

impl Styled for Notification {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl Render for Notification {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = self
            .content_builder
            .clone()
            .map(|builder| builder(self, window, cx));
        let action = self
            .action_builder
            .clone()
            .map(|builder| builder(self, window, cx));
        let icon = match self.type_ {
            None => self.icon.clone(),
            Some(type_) => Some(Self::type_icon(type_, cx)),
        };
        let has_icon = icon.is_some();
        let placement = self.placement.unwrap_or(cx.theme().notification.placement);
        let rem = f32::from(window.rem_size());
        let (pose, running) = self.pose_at(now(), placement, rem, cx.reduce_motion());
        if running {
            window.request_animation_frame();
        }
        let alpha = pose.alpha();
        let measured = self.size.clone();
        let swing = self.swing_vector(has_icon, rem, window, cx);

        let toast = BaseToast::new("notification")
            .transition_status(self.transition_status)
            .h_flex()
            .group("")
            .occlude()
            .relative()
            .w_full()
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tokens.popover)
            .rounded(cx.theme().radius_lg)
            // A translucent card shows its own shadow through; the ink follows the fade steeply.
            .shadow(toast_shadow(alpha * alpha))
            .py_3p5()
            .px_4()
            .gap_3()
            .refine_style(&self.style)
            .when_some(icon, |this, icon| {
                this.child(div().absolute().top(px(18.)).left_4().child(icon))
            })
            .child(
                v_flex()
                    .flex_1()
                    .overflow_hidden()
                    .when(has_icon, |this| this.pl_6())
                    .when_some(self.title.clone(), |this, title| {
                        this.child(div().text_sm().font_semibold().child(title))
                    })
                    .when_some(self.message.clone(), |this, message| {
                        this.child(div().text_sm().child(message))
                    })
                    .when_some(content, |this, content| this.child(content)),
            )
            .when_some(action, |this, action| this.child(action))
            .child(
                div()
                    .absolute()
                    .top_1()
                    .right_1()
                    .invisible()
                    .group_hover("", |this| this.visible())
                    .child(
                        BaseButton::new("close")
                            .icon(IconName::Close)
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|this, _, window, cx| {
                                cx.stop_propagation();
                                this.dismiss(window, cx);
                            })),
                    ),
            )
            .when_some(self.on_click.clone(), |this, on_click| {
                this.on_click(cx.listener(move |view, event, window, cx| {
                    view.dismiss(window, cx);
                    on_click(event, window, cx);
                }))
            })
            .on_aux_click(cx.listener(move |view, event: &ClickEvent, window, cx| {
                if event.is_middle_click() {
                    view.dismiss(window, cx);
                }
            }));
        let id = ("kk-toast", cx.entity_id());
        match swing {
            // Swinging: the vector toast turns on its pin over the real one, held in place and
            // hidden, so the stack measures it and the final frame is GPUI's own.
            Some(swing) => div()
                .relative()
                .w_full()
                .child(transform(id, Pose::new().opacity(0.0), toast))
                .child(swing)
                .on_prepaint(move |bounds, _, _| measured.set(bounds.size))
                .into_any_element(),
            None => transform(
                id,
                pose,
                div()
                    .w_full()
                    .child(toast)
                    .on_prepaint(move |bounds, _, _| measured.set(bounds.size)),
            )
            .into_any_element(),
        }
    }
}

/// Per-placement stack state, created lazily for placements in use.
struct AnchorStack {
    state: ToastStackState,
    focus_handle: FocusHandle,
}

/// A list of notifications.
pub struct NotificationList {
    notifications: ToastManager<NotificationId, Entity<Notification>>,
    stacks: Vec<(Anchor, AnchorStack)>,
    focus_handle: FocusHandle,
    is_advancing: bool,
    _subscriptions: HashMap<NotificationId, Subscription>,
}

impl NotificationList {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            // The pop-in takes 0.55 s; a toast counts as present once it has played.
            notifications: ToastManager::new(ToastMotion {
                duration: IN,
                exit_duration: OUT,
                ..ToastMotion::sonner()
            }),
            stacks: Vec::new(),
            focus_handle: cx.focus_handle().tab_stop(true),
            is_advancing: false,
            _subscriptions: HashMap::new(),
        }
    }

    /// Tick the toast lifecycle until the last notification is unmounted.
    ///
    /// The ticks come from a real timer, but the lifecycle reads [`now`], the clock the toasts'
    /// poses are drawn on, so a pinned clock (screenshots) holds both at the same moment.
    fn start_advancing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.is_advancing {
            return;
        }
        self.is_advancing = true;
        cx.spawn_in(window, async move |view, cx| {
            loop {
                cx.background_executor().timer(ADVANCE_INTERVAL).await;
                let running = view.update_in(cx, |view, window, cx| {
                    view.advance(window, cx);
                    view.is_advancing = !view.notifications.is_empty();
                    view.is_advancing
                });
                if !matches!(running, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
    }

    fn is_expanded(&self) -> bool {
        self.stacks
            .iter()
            .any(|(_, stack)| stack.state.is_expanded())
    }

    fn grouped(
        &self,
        cx: &App,
    ) -> Vec<(
        Anchor,
        ElementId,
        Vec<(NotificationId, Entity<Notification>)>,
    )> {
        let settings = &cx.theme().notification;
        let (max_items, placement) = (settings.max_items, settings.placement);
        let mut groups: Vec<(Anchor, Vec<(NotificationId, Entity<Notification>)>)> = Vec::new();
        for (id, item, _) in self.notifications.visible(max_items) {
            let anchor = item.read(cx).placement.unwrap_or(placement);
            match groups.iter_mut().find(|(a, _)| *a == anchor) {
                Some((_, items)) => items.push((id.clone(), item.clone())),
                None => groups.push((anchor, vec![(id.clone(), item.clone())])),
            }
        }
        groups
            .into_iter()
            .map(|(anchor, items)| (anchor, stack_id(anchor), items))
            .collect()
    }

    pub fn push(
        &mut self,
        notification: impl Into<Notification>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut notification = notification.into();
        let delivery = notification
            .delivery
            .unwrap_or(cx.theme().notification.delivery);
        if delivery.includes_system() {
            push_system(&notification, window, cx);
        }
        if !delivery.includes_in_app() {
            return;
        }
        notification.opened = now();
        let id = notification.id.clone();
        let autohide = notification.autohide;
        let system = delivery.includes_system();
        let retract = notification.system_id.retract.clone();
        let window_handle = window.window_handle();
        let notification = cx.new(|_| notification);
        let dismiss_id = id.clone();
        self._subscriptions.insert(
            id.clone(),
            cx.subscribe(&notification, move |view, _, _: &DismissRequest, cx| {
                if view.notifications.dismiss(&dismiss_id, now()) {
                    if let Some(note) = view.notifications.get(&dismiss_id) {
                        note.update(cx, |note, cx| note.begin_close(cx));
                    }
                    if system {
                        let retract = retract.clone();
                        cx.defer(move |cx| {
                            let _ = window_handle.update(cx, |_, window, cx| retract(window, cx));
                        });
                    }
                }
            }),
        );
        self.notifications.push(
            id,
            notification,
            ToastOptions {
                timeout: autohide.then_some(Duration::from_secs(5)),
            },
            now(),
        );
        self.start_advancing(window, cx);
        cx.notify();
    }

    fn advance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let changes = self.notifications.advance(now(), self.is_expanded());
        for id in changes.presented {
            if let Some(note) = self.notifications.get(&id) {
                note.update(cx, |note, cx| note.complete_enter(cx));
            }
        }
        for id in changes.ending {
            if let Some(note) = self.notifications.get(&id) {
                note.update(cx, |note, cx| note.begin_close(cx));
            }
        }
        for (id, note) in changes.removed {
            self._subscriptions.remove(&id);
            note.update(cx, |note, cx| note.complete_close(window, cx));
        }
        if changes.changed {
            cx.notify();
        }
    }

    /// Starts closing the notification `id`, and retracts its system notification.
    pub(crate) fn close(
        &mut self,
        id: impl Into<NotificationId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id: NotificationId = id.into();
        if let Some(note) = self.notifications.get(&id).cloned() {
            let retract = note.read(cx).system_id.retract.clone();
            retract(window, cx);
            if self.notifications.dismiss(&id, now()) {
                note.update(cx, |note, cx| note.begin_close(cx));
            }
        }
        cx.notify();
    }

    /// Closes every notification whose id matches `type_id`, keyed or not.
    pub(crate) fn close_by_type(
        &mut self,
        type_id: TypeId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let matched: Vec<_> = self
            .notifications
            .iter()
            .filter(|(id, _, _)| match id {
                NotificationId::Id(t) | NotificationId::IdAndElementId(t, _) => *t == type_id,
            })
            .map(|(id, _, _)| id.clone())
            .collect();
        for id in matched {
            self.close(id, window, cx);
        }
        cx.notify();
    }

    pub fn clear(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let ids: Vec<_> = self
            .notifications
            .iter()
            .map(|(id, _, _)| id.clone())
            .collect();
        for id in ids {
            self.close(id, window, cx);
        }
        cx.notify();
    }

    pub fn notifications(&self) -> Vec<Entity<Notification>> {
        self.notifications
            .iter()
            .map(|(_, value, _)| value.clone())
            .collect()
    }
}

/// Posts the system half through GPUI Component, which owns the platform's response handler.
fn push_system(notification: &Notification, window: &mut Window, cx: &mut App) {
    if notification.title.is_none() && notification.message.is_none() {
        return;
    }
    let note = (notification.system_id.apply)(BaseNotification::new())
        .when_some(notification.title.clone(), |this, title| this.title(title))
        .when_some(notification.message.clone(), |this, message| {
            this.message(message)
        })
        .when_some(notification.on_click.clone(), |this, on_click| {
            this.on_click(move |event, window, cx| on_click(event, window, cx))
        })
        .delivery(NotificationDelivery::System);
    gpui_kit::component::WindowExt::push_notification(window, note, cx);
}

/// The element id a placement's stack renders under, keyed by the placement so a stack keeps its
/// state when another one comes or goes.
fn stack_id(anchor: Anchor) -> ElementId {
    let ix = match anchor {
        Anchor::TopLeft => 0,
        Anchor::TopCenter => 1,
        Anchor::TopRight => 2,
        Anchor::BottomLeft => 3,
        Anchor::BottomCenter => 4,
        Anchor::BottomRight => 5,
        Anchor::LeftCenter => 6,
        Anchor::RightCenter => 7,
    };
    ("kk-notification-list", ix as usize).into()
}

impl Render for NotificationList {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = window.viewport_size();
        let settings = &cx.theme().notification;
        let (placement, margins, width) =
            (settings.placement, settings.margins.clone(), settings.width);
        let groups = self.grouped(cx);
        self.stacks
            .retain(|(anchor, _)| groups.iter().any(|(a, _, _)| a == anchor));
        let default_focus_handle = self.focus_handle.clone();
        let stacks = groups
            .into_iter()
            .map(|(anchor, stack_id, items)| {
                let stack_ix = self
                    .stacks
                    .iter()
                    .position(|(a, _)| *a == anchor)
                    .unwrap_or_else(|| {
                        self.stacks.push((
                            anchor,
                            AnchorStack {
                                state: ToastStackState::default(),
                                focus_handle: if anchor == placement {
                                    default_focus_handle.clone()
                                } else {
                                    cx.focus_handle().tab_stop(true)
                                },
                            },
                        ));
                        self.stacks.len() - 1
                    });
                let stack = &self.stacks[stack_ix].1;
                items
                    .into_iter()
                    .fold(
                        ToastStack::new(stack_id, stack.state.clone()),
                        |stack, (id, item)| stack.item(format!("{id:?}"), item),
                    )
                    .placement(anchor)
                    .focus_handle(stack.focus_handle.clone())
                    .v_flex()
                    .w(width)
                    .max_h(size.height)
                    .absolute()
                    .map(|this| match anchor {
                        Anchor::TopLeft => this.top(margins.top).left(margins.left),
                        Anchor::TopRight => this.top(margins.top).right(margins.right),
                        Anchor::TopCenter => this.top(margins.top).left_0().right_0().mx_auto(),
                        Anchor::BottomLeft => this.bottom(margins.bottom).left(margins.left),
                        Anchor::BottomRight => this.bottom(margins.bottom).right(margins.right),
                        Anchor::BottomCenter => {
                            this.bottom(margins.bottom).left_0().right_0().mx_auto()
                        }
                        Anchor::LeftCenter => this.left(margins.left).top_0().bottom_0().my_auto(),
                        Anchor::RightCenter => {
                            this.right(margins.right).top_0().bottom_0().my_auto()
                        }
                    })
            })
            .collect::<Vec<_>>();
        div().size_full().children(stacks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn drop_and_swing_keyframes() {
        let (drop, swing, opacity) = drop_in(Duration::ZERO);
        assert!(close(drop, -1.25) && close(swing, 6.0) && close(opacity, 0.0));
        let (drop, swing, _) = drop_in(ms(220));
        assert!(close(drop, 0.2) && close(swing, -3.0));
        let (drop, swing, _) = drop_in(ms(341));
        assert!(close(drop, 0.0) && close(swing, 1.5));
        assert!(close(drop_in(ms(451)).1, -0.5));
        assert_eq!(drop_in(ms(550)), (0.0, 0.0, 1.0));
        assert!(close(drop_in(ms(110)).2, 1.0), "opaque by 20 %");
    }

    #[test]
    fn drop_in_matches_the_web_keyframes() {
        // The drop is in rems, and the parser reads the bare number.
        crate::parity::assert_number_track("pop-toast", "kk-pop-toast-in", "y", &drop_track());
        crate::parity::assert_number_track(
            "pop-toast",
            "kk-pop-toast-in",
            "rotate",
            &swing_track(),
        );
        crate::parity::assert_number_track(
            "pop-toast",
            "kk-pop-toast-in",
            "opacity",
            &fade_track(),
        );
        // `kk-pop-toast-out` slides by `var(--radix-toast-swipe-end-x)` and the swipe direction's
        // custom property, which the parser can't read; `exit_slides_and_fades` checks it.
        let fade = Track::new(Easing::EaseOut)
            .at(0.0, 0.0_f32)
            .at(1.0, 1.0)
            .build();
        crate::parity::assert_number_track("pop-toast", "kk-pop-toast-fade", "opacity", &fade);
    }

    #[test]
    fn sway_follows_the_centre() {
        assert_eq!(sway(0.0, 80.0), (0.0, 0.0));
        let (dx, dy) = sway(6.0, 80.0);
        // A clockwise turn about the top swings the centre left and a hair up.
        assert!(dx < -4.0 && dx > -4.3 && dy < 0.0 && dy > -0.3);
    }

    #[test]
    fn exit_slides_and_fades() {
        let mut note = Notification::new();
        let closing = now();
        note.closing = Some(closing);
        let (start, running) = note.pose_at(closing, Anchor::TopRight, 16.0, false);
        assert_eq!(start, Pose::new());
        assert!(running);
        let (end, running) = note.pose_at(closing + OUT, Anchor::TopRight, 16.0, false);
        assert!(close(end.xp, 1.05) && close(end.opacity, 0.0) && !running);
        let (left, _) = note.pose_at(closing + OUT, Anchor::BottomLeft, 16.0, false);
        assert!(close(left.xp, -1.05));
    }

    #[test]
    fn builder_keeps_settings() {
        let note = Notification::new()
            .title("title")
            .message("message")
            .with_type(NotificationType::Success)
            .placement(Anchor::BottomLeft)
            .autohide(false);
        assert_eq!(note.title, Some("title".into()));
        assert_eq!(note.placement, Some(Anchor::BottomLeft));
        assert!(!note.autohide);
        assert_ne!(Notification::new().id, Notification::new().id);
        assert_eq!(
            Notification::new().system().delivery,
            Some(NotificationDelivery::System)
        );
    }
}
