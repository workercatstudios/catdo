//! Button: GPUI Component's button with a press and a pop.
//!
//! Replaces `gpui_kit::component::button`. Every other item of that module is re-exported, so
//! `use kirakira::button::*` is a drop-in for `use gpui_kit::component::button::*`.
//!
//! Pressing sinks the button evenly to 0.95 in 0.1 s; letting go pops it back past full size,
//! 0.95 → 1.05 → 0.98 → 1 in 0.3 s. Each click restarts the pop. Under reduced motion nothing
//! moves; the colours still change on hover and press.
//!
//! [`ButtonGroup`] and [`DropdownButton`] are GPUI Component's, ported to take this module's
//! [`Button`], so a group's buttons and a split button's halves press and pop too.
//!
//! [`Button::icon`] takes everything GPUI Component's does: an [`Icon`] or anything that converts
//! into one ([`IconName`](gpui_kit::component::IconName), your own icon types), GPUI Component's
//! [`Spinner`](gpui_kit::component::spinner::Spinner) or a
//! [`ProgressCircle`](gpui_kit::component::progress::ProgressCircle), and Kirakira's own
//! [`Spinner`](crate::spinner::Spinner) too, any variant. GPUI Component's bound,
//! `impl Into<ButtonIcon>`, names a type it keeps private, so the bound here is [`IntoButtonIcon`]
//! instead; calls read the same.
//!
//! Loading: where GPUI Component's button swaps its icon for GPUI Component's spinner, this one
//! turns Kirakira's [`Spinner`](crate::spinner::Spinner) (the same loader icon, on Kirakira's
//! clock, still under reduced motion), turning [`Button::loading_icon`] if one is set.
//! [`Button::loading_spinner`] picks another loader for the slot, such as the hopping dots of the
//! web's loading button, and shows it while loading even on a button without an icon. A loading
//! button stays dimmed and inert, as in GPUI Component.
//!
//! A Kirakira spinner in the slot is drawn by this button, not GPUI Component's (whose slot only
//! takes its own kinds): the spinner, then the label, then the children, with GPUI Component's
//! spacing and icon-button size. The turning icon takes the icon size; the loaders take the
//! button's text size, as the web's loader is sized in the button's `em`, and its text colour,
//! as `text-current`, unless the spinner sets a colour. Both are in rems, so they pop with the
//! button.

use std::rc::Rc;

use gpui_kit::component::button::Button as BaseButton;
use gpui_kit::component::progress::ProgressCircle;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{
    ActiveTheme as _, Disableable, FocusableExt, Icon, Placement, Selectable, Sizable, Size,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, Background, ClickEvent, Corners, Edges, ElementId, InteractiveElement,
    Interactivity, IntoElement, MouseButton, ParentElement, Refineable as _, RenderOnce,
    SharedString, StyleRefinement, Styled, Window, div, px,
};

pub use crate::button_group::{ButtonGroup, DropdownButton};
pub use crate::button_toggle::{Toggle, ToggleGroup};
pub use gpui_kit::component::button::{
    ButtonCustomVariant, ButtonRounded, ButtonVariant, ButtonVariants, ToggleVariant,
    ToggleVariants,
};

use crate::motion::{Easing, Pose, Pulse, Timing, Track, ms, split_layout, transform};
use crate::spinner::{Spinner as KiraSpinner, SpinnerVariant};

/// The pop after a click: `0.95 → 1.05 → 0.98 → 1`, ease-in-out on each segment.
pub(crate) fn pop_track() -> crate::motion::Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.95)
        .at(0.45, 1.05)
        .at(0.75, 0.98)
        .at(1.0, 1.0)
        .build()
}

const POP: std::time::Duration = ms(300);
const SINK: std::time::Duration = ms(100);

/// What [`Button::icon`] shows: an icon, GPUI Component's spinner or progress circle, or
/// Kirakira's spinner.
pub enum ButtonIcon {
    Icon(Icon),
    Spinner(Spinner),
    Progress(ProgressCircle),
    /// Kirakira's [`Spinner`](crate::spinner::Spinner), any variant.
    Kirakira(KiraSpinner),
}

/// Anything [`Button::icon`] takes: anything that converts into an [`Icon`], GPUI Component's
/// [`Spinner`], a [`ProgressCircle`], Kirakira's [`Spinner`](crate::spinner::Spinner), or a
/// [`ButtonIcon`].
///
/// `Marker` only keeps the impls apart (Rust won't allow a blanket impl for `impl Into<Icon>`
/// beside one for `Spinner` when both types live in another crate); it is always inferred.
pub trait IntoButtonIcon<Marker> {
    fn into_button_icon(self) -> ButtonIcon;
}

#[doc(hidden)]
pub struct IconMarker;
#[doc(hidden)]
pub struct SpinnerMarker;
#[doc(hidden)]
pub struct ProgressMarker;
#[doc(hidden)]
pub struct ButtonIconMarker;
#[doc(hidden)]
pub struct KirakiraSpinnerMarker;

impl<T: Into<Icon>> IntoButtonIcon<IconMarker> for T {
    fn into_button_icon(self) -> ButtonIcon {
        ButtonIcon::Icon(self.into())
    }
}

impl IntoButtonIcon<SpinnerMarker> for Spinner {
    fn into_button_icon(self) -> ButtonIcon {
        ButtonIcon::Spinner(self)
    }
}

impl IntoButtonIcon<ProgressMarker> for ProgressCircle {
    fn into_button_icon(self) -> ButtonIcon {
        ButtonIcon::Progress(self)
    }
}

impl IntoButtonIcon<ButtonIconMarker> for ButtonIcon {
    fn into_button_icon(self) -> ButtonIcon {
        self
    }
}

impl IntoButtonIcon<KirakiraSpinnerMarker> for KiraSpinner {
    fn into_button_icon(self) -> ButtonIcon {
        ButtonIcon::Kirakira(self)
    }
}

/// What the icon slot shows: the icon, or while loading the spinner that stands in for it. GPUI
/// Component swaps an icon for its own spinner; this swaps it for Kirakira's.
fn slot(
    icon: Option<ButtonIcon>,
    loading: bool,
    loading_icon: Option<Icon>,
    loading_spinner: Option<KiraSpinner>,
) -> Option<ButtonIcon> {
    if !loading {
        return icon;
    }
    match (loading_spinner, icon) {
        (Some(spinner), _) => Some(ButtonIcon::Kirakira(spinner)),
        (None, Some(ButtonIcon::Icon(_))) => Some(ButtonIcon::Kirakira(
            KiraSpinner::new().when_some(loading_icon, |spinner, icon| spinner.icon(icon)),
        )),
        // Spinners and progress circles already show progress, as in GPUI Component.
        (None, icon) => icon,
    }
}

/// The size a Kirakira spinner takes in a button: the turning icon takes the icon size (GPUI
/// Component's), a loader the button's text size.
fn spinner_size(button: Size, variant: SpinnerVariant) -> Size {
    match (variant, button) {
        (SpinnerVariant::Icon, Size::Size(size)) => Size::Size(size * 0.75),
        (SpinnerVariant::Icon, size) => size,
        // `button_text_size`: `text_xs`, `text_sm`, else `text_base`, which are the icon sizes of
        // `XSmall`, `Small` and `Medium`.
        (_, Size::XSmall) => Size::XSmall,
        (_, Size::Small) => Size::Small,
        (_, _) => Size::Medium,
    }
}

/// A button that sinks while pressed and pops when clicked.
///
/// The same builder as GPUI Component's `Button`: `Button::new("ok").primary().label("OK")`.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    inner: BaseButton,
    disabled: bool,
    selected: bool,
    loading: bool,
    variant: ButtonVariant,
    size: Size,
    focus_ring: bool,
    on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    // Held until render, which decides who draws the icon slot. Boxed: builders move the button
    // by value at every call, and a debug build keeps each copy on the stack.
    icon: Option<Box<ButtonIcon>>,
    label: Option<SharedString>,
    named: bool,
    loading_icon: Option<Box<Icon>>,
    loading_spinner: Option<Box<KiraSpinner>>,
    children: Vec<AnyElement>,
}

impl From<Button> for AnyElement {
    fn from(button: Button) -> Self {
        button.into_any_element()
    }
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        let id = id.into();
        Self {
            inner: BaseButton::new(id.clone()),
            id,
            disabled: false,
            selected: false,
            loading: false,
            variant: ButtonVariant::default(),
            size: Size::Medium,
            focus_ring: true,
            on_click: None,
            icon: None,
            label: None,
            named: false,
            loading_icon: None,
            loading_spinner: None,
            children: Vec::new(),
        }
    }

    fn map(mut self, f: impl FnOnce(BaseButton) -> BaseButton) -> Self {
        self.inner = f(self.inner);
        self
    }

    pub(crate) fn variant(&self) -> ButtonVariant {
        self.variant
    }

    pub(crate) fn button_size(&self) -> Size {
        self.size
    }

    pub(crate) fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Sets styles under the user's own, as GPUI Component's button applies its defaults first
    /// and the instance style last.
    fn style_under(mut self, f: impl FnOnce(&mut StyleRefinement)) -> Self {
        let user = std::mem::take(self.inner.style());
        let mut style = StyleRefinement::default();
        f(&mut style);
        style.refine(&user);
        *self.inner.style() = style;
        self
    }

    /// Joins the button to its neighbours: GPUI Component's private `border_corners` and
    /// `border_edges`. Corners left out are square and edges left out have no border; the others
    /// keep the button's own.
    pub(crate) fn join(self, corners: Corners<bool>, edges: Edges<bool>) -> Self {
        self.style_under(|style| {
            let radii = &mut style.corner_radii;
            for (keep, radius) in [
                (corners.top_left, &mut radii.top_left),
                (corners.top_right, &mut radii.top_right),
                (corners.bottom_left, &mut radii.bottom_left),
                (corners.bottom_right, &mut radii.bottom_right),
            ] {
                if !keep {
                    *radius = Some(px(0.).into());
                }
            }
            let widths = &mut style.border_widths;
            for (keep, width) in [
                (edges.left, &mut widths.left),
                (edges.top, &mut widths.top),
                (edges.right, &mut widths.right),
                (edges.bottom, &mut widths.bottom),
            ] {
                if !keep {
                    *width = Some(px(0.).into());
                }
            }
        })
    }

    /// Joins a hover group of ghost buttons: while any member is hovered, or while `held`, an idle
    /// member shows its hover surface at half strength. GPUI Component's private `hover_group` and
    /// `hover_group_held`, which only its ghost split button uses.
    pub(crate) fn ghost_hover_group(self, group: &'static str, held: bool, cx: &App) -> Self {
        if self.disabled || self.selected || self.loading || !self.variant.is_ghost() {
            return self;
        }
        let accent: Background = cx.theme().tokens.accent.into();
        let hover = if cx.theme().mode.is_dark() {
            accent.opacity(0.5)
        } else {
            accent
        };
        let idle = hover.opacity(0.5);
        self.style_under(|style| {
            if held {
                style.background = Some(idle.into());
            }
        })
        .group_hover(group, move |style| style.bg(idle))
    }

    pub fn role(self, role: impl Into<gpui_kit::base::RoleOverride>) -> Self {
        self.map(|inner| inner.role(role))
    }

    pub fn outline(self) -> Self {
        self.map(|inner| inner.outline())
    }

    pub fn rounded(self, rounded: impl Into<ButtonRounded>) -> Self {
        self.map(|inner| inner.rounded(rounded))
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn accessibility_id(self, id: impl Into<SharedString>) -> Self {
        self.map(|inner| inner.accessibility_id(id))
    }

    pub fn accessibility_label(mut self, label: impl Into<SharedString>) -> Self {
        self.named = true;
        self.map(|inner| inner.accessibility_label(label))
    }

    /// Set the icon of the button. Without a label, the button is an icon button.
    ///
    /// Takes an [`Icon`] or anything that converts into one, GPUI Component's [`Spinner`], a
    /// [`ProgressCircle`] or Kirakira's [`Spinner`](crate::spinner::Spinner).
    pub fn icon<M>(mut self, icon: impl IntoButtonIcon<M>) -> Self {
        self.icon = Some(Box::new(icon.into_button_icon()));
        self
    }

    /// The spinner a loading button shows in its icon slot, even without an icon: one of
    /// Kirakira's loaders, say `Spinner::new().variant(SpinnerVariant::Dots)`. By default a loading
    /// button turns its [`loading_icon`](Self::loading_icon) in place of its icon.
    pub fn loading_spinner(mut self, spinner: KiraSpinner) -> Self {
        self.loading_spinner = Some(Box::new(spinner));
        self
    }

    pub fn tooltip(self, tooltip: impl Into<SharedString>) -> Self {
        self.map(|inner| inner.tooltip(tooltip))
    }

    pub fn tooltip_placement(self, placement: Placement) -> Self {
        self.map(|inner| inner.tooltip_placement(placement))
    }

    pub fn tooltip_with_action(
        self,
        tooltip: impl Into<SharedString>,
        action: &dyn gpui_kit::Action,
        context: Option<&str>,
    ) -> Self {
        self.map(|inner| inner.tooltip_with_action(tooltip, action, context))
    }

    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self.map(|inner| inner.loading(loading))
    }

    pub fn compact(self) -> Self {
        self.map(|inner| inner.compact())
    }

    /// Add a click handler. Pointer, Enter and Space clicks all pop the button.
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    pub fn on_hover(self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.map(|inner| inner.on_hover(handler))
    }

    /// The icon the loading spinner turns. Default is GPUI Component's loader icon.
    pub fn loading_icon(mut self, icon: impl Into<Icon>) -> Self {
        self.loading_icon = Some(Box::new(icon.into()));
        self
    }

    pub fn tab_index(self, tab_index: isize) -> Self {
        self.map(|inner| inner.tab_index(tab_index))
    }

    pub fn tab_stop(self, tab_stop: bool) -> Self {
        self.map(|inner| inner.tab_stop(tab_stop))
    }

    pub fn dropdown_caret(self, dropdown_caret: bool) -> Self {
        self.map(|inner| inner.dropdown_caret(dropdown_caret))
    }

    pub fn toggled(self, toggled: bool) -> Self {
        self.map(|inner| inner.toggled(toggled))
    }

    /// Hands the icon, label and children to GPUI Component's button, or, when the slot holds a
    /// Kirakira spinner, draws it here: spinner, label, children, in GPUI Component's order and
    /// spacing, at GPUI Component's icon-button size when there is nothing else.
    fn fill(mut self) -> Self {
        let icon = slot(
            self.icon.take().map(|icon| *icon),
            self.loading,
            self.loading_icon.take().map(|icon| *icon),
            self.loading_spinner.take().map(|spinner| *spinner),
        );
        let label = self.label.take();
        let children = std::mem::take(&mut self.children);
        let spinner = match icon {
            Some(ButtonIcon::Kirakira(spinner)) => spinner,
            icon => {
                return self.map(|inner| {
                    let inner = match icon {
                        Some(ButtonIcon::Icon(icon)) => inner.icon(icon),
                        Some(ButtonIcon::Spinner(spinner)) => inner.icon(spinner),
                        Some(ButtonIcon::Progress(progress)) => inner.icon(progress),
                        Some(ButtonIcon::Kirakira(_)) | None => inner,
                    };
                    inner
                        .when_some(label, |inner, label| inner.label(label))
                        .children(children)
                });
            }
        };

        let size = self.size;
        let variant = spinner.variant;
        let spinner = spinner
            .inherit_color()
            .with_size(spinner_size(size, variant));
        let icon_only = label.is_none() && children.is_empty();
        // GPUI Component sizes a button as an icon button only when its own slot holds the icon.
        let this = if icon_only && !(self.variant.is_link() || self.variant.is_text()) {
            self.style_under(|style| {
                let base = std::mem::take(style);
                *style = match size {
                    Size::Size(side) => base.size(side),
                    Size::XSmall => base.size_5(),
                    Size::Small => base.size_6(),
                    Size::Medium | Size::Large => base.size_8(),
                }
                .px_0();
            })
        } else {
            self
        };
        // GPUI Component names a button by its label unless told otherwise.
        let name = label.clone().filter(|_| !this.named);
        this.map(|inner| {
            inner
                .when_some(name, |inner, name| inner.accessibility_label(name))
                .child(spinner)
                .when_some(label, |inner, label| {
                    inner.child(
                        div()
                            .min_w_0()
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(label),
                    )
                })
                .children(children)
        })
    }
}

impl Disableable for Button {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.map(|inner| inner.disabled(disabled))
    }
}

impl Selectable for Button {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self.map(|inner| inner.selected(selected))
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl Sizable for Button {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        let size = size.into();
        self.size = size;
        self.map(|inner| inner.with_size(size))
    }
}

impl ButtonVariants for Button {
    fn with_variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self.map(|inner| inner.with_variant(variant))
    }
}

impl FocusableExt for Button {
    fn focus_ring(mut self, enabled: bool) -> Self {
        self.focus_ring = enabled;
        self.map(|inner| inner.focus_ring(enabled))
    }

    fn is_focus_ring_enabled(&self) -> bool {
        self.focus_ring
    }
}

impl Styled for Button {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}

impl ParentElement for Button {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl InteractiveElement for Button {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.inner.interactivity()
    }
}

/// Held-down state shared by the pressable pieces of the kit.
pub(crate) struct Pressed(pub bool);

/// How a pressable control scales: it sinks to `sink` while held, then plays `pop` from the
/// pulse's last fire.
pub(crate) struct Press {
    pub sink: f32,
    pub pop: crate::motion::Keyframes<f32>,
    pub duration: std::time::Duration,
}

impl Press {
    /// The scale this frame. 1 under reduced motion.
    pub fn scale(
        &self,
        id: &ElementId,
        pressed: bool,
        pulse: &Pulse,
        window: &mut Window,
        cx: &mut App,
    ) -> f32 {
        // Kirakira's clock, not GPUI's, so screenshot mode can pin the sink.
        let sunk = crate::state_motion::glide(
            (id.clone(), "kk-sink"),
            if pressed { self.sink } else { 1.0 },
            SINK,
            Easing::EaseOut,
            window,
            cx,
        );
        if cx.reduce_motion() {
            return 1.0;
        }
        if pressed {
            return sunk;
        }
        match pulse.running(self.duration) {
            Some(elapsed) => {
                pulse.animate(self.duration, window);
                self.pop
                    .sample(Timing::new(self.duration).sample(elapsed).directed_progress)
            }
            None => 1.0,
        }
    }
}

impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let this = self.fill();
        let id = this.id.clone();
        let pulse = Pulse::new((id.clone(), "kk-pop"), window, cx);
        let pressed = window.use_keyed_state((id.clone(), "kk-pressed"), cx, |_, _| Pressed(false));
        let interactive = !this.disabled;
        let held = interactive && pressed.read(cx).0;
        let scale = Press {
            sink: 0.95,
            pop: pop_track(),
            duration: POP,
        }
        .scale(&id, held, &pulse, window, cx);

        let mut inner = this.inner;
        let outer = split_layout(inner.style());
        let on_click = this.on_click;
        let fire = pulse.clone();
        let down = pressed.clone();
        let up = pressed.clone();
        let up_out = pressed;

        let inner = inner
            .when(interactive, |this| {
                this.on_mouse_down(MouseButton::Left, move |_, _, cx| {
                    down.update(cx, |pressed, cx| {
                        pressed.0 = true;
                        cx.notify();
                    });
                })
                .on_mouse_up(MouseButton::Left, move |_, _, cx| {
                    up.update(cx, |pressed, cx| {
                        pressed.0 = false;
                        cx.notify();
                    });
                })
                .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
                    up_out.update(cx, |pressed, cx| {
                        pressed.0 = false;
                        cx.notify();
                    });
                })
            })
            .on_click(move |event, window, cx| {
                fire.fire(cx);
                if let Some(on_click) = on_click.as_ref() {
                    on_click(event, window, cx);
                }
            });

        transform(id, Pose::new().scale(scale), inner).outer_style(outer)
    }
}

#[cfg(test)]
mod tests {
    use crate::parity::assert_number_track;
    use gpui_kit::component::{FocusableExt as _, Icon, IconName};

    #[test]
    fn icon_takes_what_gpui_component_takes() {
        use gpui_kit::component::{progress::ProgressCircle, spinner::Spinner};
        let _ = super::Button::new("a").icon(IconName::Plus);
        let _ = super::Button::new("b").icon(Icon::new(IconName::Plus));
        let _ = super::Button::new("c").icon(Spinner::new());
        let _ = super::Button::new("d").icon(ProgressCircle::new("p"));
    }

    #[test]
    fn the_slot_keeps_the_button_small() {
        // Builders move the button by value at every call; debug builds keep each copy on the
        // stack, so what the wrapper adds to GPUI Component's button is boxed.
        let extra = std::mem::size_of::<super::Button>() - std::mem::size_of::<super::BaseButton>();
        assert!(extra <= 256, "the wrapper adds {extra} bytes");
    }

    #[test]
    fn icon_takes_kirakiras_spinner() {
        use crate::spinner::{Spinner, SpinnerVariant};
        let _ = super::Button::new("a").icon(Spinner::new());
        let _ = super::Button::new("b").icon(Spinner::new().variant(SpinnerVariant::Dots));
        let _ = super::Button::new("c")
            .loading(true)
            .loading_spinner(Spinner::new().variant(SpinnerVariant::Bars));
    }

    #[test]
    fn loading_swaps_an_icon_for_kirakiras_spinner() {
        use super::{ButtonIcon, slot};
        use crate::spinner::{Spinner, SpinnerVariant};
        let icon = || Some(ButtonIcon::Icon(Icon::new(IconName::Plus)));
        // At rest the icon shows.
        assert!(matches!(
            slot(icon(), false, None, None),
            Some(ButtonIcon::Icon(_))
        ));
        // Loading, an icon becomes Kirakira's turning spinner, GPUI Component's spinner stays,
        // and a button without an icon shows nothing, as in GPUI Component.
        assert!(matches!(
            slot(icon(), true, None, None),
            Some(ButtonIcon::Kirakira(spinner)) if spinner.variant == SpinnerVariant::Icon
        ));
        assert!(matches!(
            slot(
                Some(ButtonIcon::Spinner(
                    gpui_kit::component::spinner::Spinner::new()
                )),
                true,
                None,
                None
            ),
            Some(ButtonIcon::Spinner(_))
        ));
        assert!(slot(None, true, None, None).is_none());
        // A loading spinner shows while loading, icon or not, and only then.
        let dots = || Some(Spinner::new().variant(SpinnerVariant::Dots));
        assert!(matches!(
            slot(None, true, None, dots()),
            Some(ButtonIcon::Kirakira(spinner)) if spinner.variant == SpinnerVariant::Dots
        ));
        assert!(matches!(
            slot(icon(), true, None, dots()),
            Some(ButtonIcon::Kirakira(spinner)) if spinner.variant == SpinnerVariant::Dots
        ));
        assert!(slot(None, false, None, dots()).is_none());
    }

    #[test]
    fn spinners_take_the_icon_or_the_text_size() {
        use super::spinner_size;
        use crate::spinner::SpinnerVariant::{Dots, Icon};
        use gpui_kit::component::Size;
        use gpui_kit::px;
        assert_eq!(spinner_size(Size::Large, Icon), Size::Large);
        assert_eq!(spinner_size(Size::Size(px(40.)), Icon), Size::Size(px(30.)));
        // Loaders are sized in the button's text: xs, sm, else base.
        assert_eq!(spinner_size(Size::XSmall, Dots), Size::XSmall);
        assert_eq!(spinner_size(Size::Small, Dots), Size::Small);
        assert_eq!(spinner_size(Size::Large, Dots), Size::Medium);
        assert_eq!(spinner_size(Size::Size(px(40.)), Dots), Size::Medium);
    }

    #[test]
    fn focus_ring_is_tracked() {
        let button = super::Button::new("a");
        assert!(button.is_focus_ring_enabled());
        assert!(!button.focus_ring(false).is_focus_ring_enabled());
    }

    #[test]
    fn pop_matches_the_web() {
        assert_number_track("pop-button", "kk-pop-button-a", "sx", &super::pop_track());
    }
}
