//! Input, Textarea and OtpInput: a focus underline, an invalid shake and characters that pop in.
//!
//! Replaces `gpui_kit::component::input`. Every other item of that module (`InputState`,
//! `TextareaState`, `OtpState`, `InputEvent`, the actions, `NumberInput`, `InputGroup`, ...) is
//! re-exported unchanged.
//!
//! - [`Input`] and [`Textarea`] wrap GPUI Component's, so text editing, IME, selection, menus and
//!   accessibility are exactly GPUI Component's. On focus a primary line 0.125 em thick draws in
//!   along the bottom of the text area from the left (0.25 s, snap curve) and leaves towards the
//!   right on blur (0.15 s, Kirakira's in-curve). [`Input::invalid`] turns the line and border
//!   red and shakes the field once: -A, +A/2, -A/4, +A/8, 0 in 0.3 s (A = 0.375 em).
//! - [`OtpInput`] is rebuilt on gpui-base's `OtpInput` with GPUI Component's layout and sizes.
//!   Each character pops into its slot as it's typed: 0.3 → (1.2, 1.25) → (0.92, 0.96) → 1 in
//!   0.3 s, rising from 0.3 em below, opaque by 30 %. [`OtpInput::invalid`] shakes the slots.
//!   The caret keeps GPUI Component's 1 s blink (on half, off half, as the web caret).
//!
//! Under reduced motion the underline appears and goes at once, nothing shakes, characters appear
//! in place and the caret stays lit.
//!
//! Differences: the line is a child quad over the field rather than a background gradient, so it
//! sits at a fixed inset from the field's edges (GPUI Component's own horizontal padding rather
//! than the web's 0.75 rem). A popping character is a [vector glyph](crate::vector) while it
//! moves, so its uneven squash is exact; at rest it is GPUI's own text. The web OTP joins its
//! slots into one bordered strip; this keeps GPUI Component's separate boxes.

use std::time::Duration;

use gpui_kit::base::OtpInput as BaseOtpInput;
use gpui_kit::component::input::{Input as BaseInput, Textarea as BaseTextarea};
use gpui_kit::component::native_menu::NativeMenu;
use gpui_kit::component::{
    ActiveTheme as _, Colorize as _, Disableable, FocusableExt, Icon, IconName, RoleOverride,
    Selectable, Sizable, Size, ThemeStyled as _, h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, ClipboardItem, DefiniteLength, ElementId, Entity, Focusable as _, Hsla,
    InteractiveElement as _, IntoElement, MouseButton, ParentElement as _, RenderOnce,
    SharedString, StyleRefinement, Styled, TextStyleRefinement, Window, div, point, px, relative,
    rems, size,
};

#[allow(unused_imports)]
pub use gpui_kit::component::input::*;

use crate::glide::glide;
use crate::motion::{Easing, Keyframes, Pose, Pulse, Track, ms, split_layout, transform};
use crate::theme::ActiveKira as _;
use crate::vector::{TextSetting, Vector, transformation};

const LINE_IN: Duration = ms(250);
const LINE_OUT: Duration = ms(150);
const SHAKE: Duration = ms(300);
const POP: Duration = ms(300);

/// `kk-pop-input-shake` (and the textarea's and OTP slot's), in em.
fn shake_track() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.0)
        .at(0.2, -0.375)
        .at(0.45, 0.1875)
        .at(0.7, -0.09)
        .at(0.88, 0.045)
        .at(1.0, 0.0)
        .build()
}

/// `kk-pop-input-otp-pop`'s transform: translation in em.
fn char_pose() -> Keyframes<Pose> {
    Track::new(Easing::EaseInOut)
        .at(0.0, Pose::new().scale(0.3).y(0.3))
        .at(0.55, Pose::new().scale_xy(1.2, 1.25).y(-0.05))
        .at(0.8, Pose::new().scale_xy(0.92, 0.96))
        .at(1.0, Pose::new())
        .build()
}

/// `kk-pop-input-otp-pop`'s opacity, done by 30 %.
fn char_opacity() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, 0.0)
        .at(0.3, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// The shake's offset in em, `elapsed` after the field turned invalid.
pub(crate) fn shake_em(elapsed: Option<Duration>) -> f32 {
    match elapsed {
        Some(elapsed) if elapsed < SHAKE => {
            shake_track().sample(elapsed.as_secs_f32() / SHAKE.as_secs_f32())
        }
        _ => 0.0,
    }
}

/// A character's pose (translation in em) `elapsed` after it was typed, or `None` at rest.
pub(crate) fn char_frame(elapsed: Option<Duration>) -> Option<Pose> {
    let elapsed = elapsed.filter(|elapsed| *elapsed < POP)?;
    let t = elapsed.as_secs_f32() / POP.as_secs_f32();
    Some(char_pose().sample(t).opacity(char_opacity().sample(t)))
}

/// The input's text size, in rems, as GPUI Component's `input_text_size` sets it.
fn text_rems(size: Size) -> f32 {
    match size {
        Size::XSmall => 0.75,
        Size::Large => 1.0,
        _ => 0.875,
    }
}

struct Seen(bool);

/// The underline and shake around a GPUI Component field.
#[allow(clippy::too_many_arguments)]
fn frame(
    id: ElementId,
    field: AnyElement,
    focused: bool,
    invalid: bool,
    size: Size,
    mut outer: StyleRefinement,
    window: &mut Window,
    cx: &mut App,
) -> impl IntoElement {
    let seen = window.use_keyed_state((id.clone(), "kk-invalid"), cx, |_, _| Seen(invalid));
    if seen.read(cx).0 != invalid {
        seen.update(cx, |seen, _| seen.0 = invalid);
        if invalid {
            Pulse::new((id.clone(), "kk-shake"), window, cx).fire(cx);
        }
    }
    let shake = Pulse::new((id.clone(), "kk-shake"), window, cx);
    shake.animate(SHAKE, window);
    let line = glide(
        (id.clone(), "kk-line"),
        if focused { 1.0 } else { 0.0 },
        if focused { LINE_IN } else { LINE_OUT },
        if focused {
            cx.curves().snap
        } else {
            cx.curves().r#in
        },
        window,
        cx,
    );
    let em = rems(text_rems(size)).to_pixels(window.rem_size());
    let dx = shake_em(shake.elapsed()) * f32::from(em);
    let theme = cx.theme();
    let color = if invalid { theme.danger } else { theme.primary };
    // GPUI Component's field fills its parent's width; so does the frame, unless sized.
    if outer.size.width.is_none() {
        outer.size.width = Some(relative(1.).into());
    }
    // Inside the 1 px border: GPUI Component's horizontal padding, 0.25 rem off the bottom.
    let inset = px(1.) + size.input_px();
    let bottom = px(1.) + rems(0.25).to_pixels(window.rem_size());

    transform(
        id,
        Pose::new().x(dx),
        div()
            .relative()
            .w_full()
            .when(outer.size.height.is_some(), |this| this.h_full())
            .child(field)
            .when(line > 0.0, |this| {
                this.child(
                    h_flex()
                        .absolute()
                        .left(inset)
                        .right(inset)
                        .bottom(bottom)
                        .h(em * 0.125)
                        // Grows from the left on focus, leaves towards the right on blur.
                        .when(focused, |this| this.justify_start())
                        .when(!focused, |this| this.justify_end())
                        .child(div().h_full().w(relative(line)).bg(color)),
                )
            }),
    )
    .outer_style(outer)
}

/// A single-line text input bound to an [`InputState`].
#[derive(IntoElement)]
pub struct Input {
    inner: BaseInput,
    state: Entity<InputState>,
    id: Option<ElementId>,
    size: Size,
    height: Option<DefiniteLength>,
    disabled: bool,
    invalid: bool,
}

impl Input {
    pub fn new(state: &Entity<InputState>) -> Self {
        Self {
            inner: BaseInput::new(state),
            state: state.clone(),
            id: None,
            size: Size::default(),
            height: None,
            disabled: false,
            invalid: false,
        }
    }

    fn map(mut self, f: impl FnOnce(BaseInput) -> BaseInput) -> Self {
        self.inner = f(self.inner);
        self
    }

    pub fn id(mut self, id: impl Into<ElementId>) -> Self {
        let id = id.into();
        self.id = Some(id.clone());
        self.map(|inner| inner.id(id))
    }

    pub fn accessibility_id(self, id: impl Into<SharedString>) -> Self {
        self.map(|inner| inner.accessibility_id(id))
    }

    pub fn aria_label(self, label: impl Into<SharedString>) -> Self {
        self.map(|inner| inner.aria_label(label))
    }

    pub fn prefix(self, prefix: impl IntoElement) -> Self {
        self.map(|inner| inner.prefix(prefix))
    }

    pub fn suffix(self, suffix: impl IntoElement) -> Self {
        self.map(|inner| inner.suffix(suffix))
    }

    /// Full height (multi-line only).
    pub fn h_full(mut self) -> Self {
        self.height = Some(relative(1.));
        self.map(|inner| inner.h_full())
    }

    /// The height (multi-line only).
    pub fn h(mut self, height: impl Into<DefiniteLength>) -> Self {
        let height = height.into();
        self.height = Some(height);
        self.map(|inner| inner.h(height))
    }

    pub fn appearance(self, appearance: bool) -> Self {
        self.map(|inner| inner.appearance(appearance))
    }

    pub fn bordered(self, bordered: bool) -> Self {
        self.map(|inner| inner.bordered(bordered))
    }

    pub fn focus_bordered(self, bordered: bool) -> Self {
        self.map(|inner| inner.focus_bordered(bordered))
    }

    pub fn cleanable(self, cleanable: bool) -> Self {
        self.map(|inner| inner.cleanable(cleanable))
    }

    pub fn mask_toggle(self) -> Self {
        self.map(|inner| inner.mask_toggle())
    }

    pub fn content_type(self, content_type: InputContentType) -> Self {
        self.map(|inner| inner.content_type(content_type))
    }

    pub fn role(self, role: impl Into<RoleOverride>) -> Self {
        self.map(|inner| inner.role(role))
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.map(|inner| inner.disabled(disabled))
    }

    pub fn readonly(self, readonly: bool) -> Self {
        self.map(|inner| inner.readonly(readonly))
    }

    pub fn tab_index(self, index: isize) -> Self {
        self.map(|inner| inner.tab_index(index))
    }

    pub fn context_menu(
        self,
        f: impl Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu + 'static,
    ) -> Self {
        self.map(|inner| inner.context_menu(f))
    }

    pub fn on_paste(
        self,
        handler: impl Fn(&ClipboardItem, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.map(|inner| inner.on_paste(handler))
    }

    /// Mark the value invalid: the line and border turn red, and the field shakes once when it
    /// becomes invalid. Like the web's `aria-invalid`. Not in GPUI Component.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }
}

impl Sizable for Input {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        let size = self.size;
        self.map(|inner| inner.with_size(size))
    }
}

impl Selectable for Input {
    fn selected(self, selected: bool) -> Self {
        self.map(|inner| inner.selected(selected))
    }

    fn is_selected(&self) -> bool {
        self.inner.is_selected()
    }
}

impl FocusableExt for Input {
    fn focus_ring(self, enabled: bool) -> Self {
        self.map(|inner| inner.focus_ring(enabled))
    }

    fn is_focus_ring_enabled(&self) -> bool {
        self.inner.is_focus_ring_enabled()
    }
}

impl Styled for Input {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}

impl RenderOnce for Input {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self
            .id
            .clone()
            .unwrap_or_else(|| ("kk-input", self.state.entity_id()).into());
        let focused = !self.disabled && self.state.focus_handle(cx).contains_focused(window, cx);
        let mut outer = split_layout(self.inner.style());
        if let Some(height) = self.height {
            outer.size.height = Some(height.into());
        }
        let invalid = self.invalid;
        let danger = cx.theme().danger;
        let inner = self.inner.when(invalid, |this| this.border_color(danger));
        frame(
            id,
            inner.into_any_element(),
            focused,
            invalid,
            self.size,
            outer,
            window,
            cx,
        )
    }
}

/// A multi-line text field bound to a [`TextareaState`].
#[derive(IntoElement)]
pub struct Textarea {
    inner: BaseTextarea,
    state: Entity<TextareaState>,
    height: Option<DefiniteLength>,
    disabled: bool,
    invalid: bool,
}

impl Textarea {
    pub fn new(state: &Entity<TextareaState>) -> Self {
        Self {
            inner: BaseTextarea::new(state),
            state: state.clone(),
            height: None,
            disabled: false,
            invalid: false,
        }
    }

    fn map(mut self, f: impl FnOnce(BaseTextarea) -> BaseTextarea) -> Self {
        self.inner = f(self.inner);
        self
    }

    pub fn h(mut self, height: impl Into<DefiniteLength>) -> Self {
        let height = height.into();
        self.height = Some(height);
        self.map(|inner| inner.h(height))
    }

    pub fn appearance(self, appearance: bool) -> Self {
        self.map(|inner| inner.appearance(appearance))
    }

    pub fn bordered(self, bordered: bool) -> Self {
        self.map(|inner| inner.bordered(bordered))
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self.map(|inner| inner.disabled(disabled))
    }

    pub fn readonly(self, readonly: bool) -> Self {
        self.map(|inner| inner.readonly(readonly))
    }

    pub fn tab_index(self, index: isize) -> Self {
        self.map(|inner| inner.tab_index(index))
    }

    pub fn role(self, role: impl Into<RoleOverride>) -> Self {
        self.map(|inner| inner.role(role))
    }

    pub fn accessibility_id(self, id: impl Into<SharedString>) -> Self {
        self.map(|inner| inner.accessibility_id(id))
    }

    pub fn aria_label(self, label: impl Into<SharedString>) -> Self {
        self.map(|inner| inner.aria_label(label))
    }

    pub fn context_menu(
        self,
        f: impl Fn(NativeMenu, &mut Window, &mut App) -> NativeMenu + 'static,
    ) -> Self {
        self.map(|inner| inner.context_menu(f))
    }

    pub fn on_paste(
        self,
        handler: impl Fn(&ClipboardItem, &mut Window, &mut App) -> bool + 'static,
    ) -> Self {
        self.map(|inner| inner.on_paste(handler))
    }

    /// Mark the value invalid: red line and border, and one shake. Not in GPUI Component.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }
}

impl Styled for Textarea {
    fn style(&mut self) -> &mut StyleRefinement {
        self.inner.style()
    }
}

impl RenderOnce for Textarea {
    fn render(mut self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id: ElementId = ("kk-textarea", self.state.entity_id()).into();
        let focused = !self.disabled && self.state.focus_handle(cx).contains_focused(window, cx);
        let mut outer = split_layout(self.inner.style());
        if let Some(height) = self.height {
            outer.size.height = Some(height.into());
        }
        let invalid = self.invalid;
        let danger = cx.theme().danger;
        let inner = self.inner.when(invalid, |this| this.border_color(danger));
        frame(
            id,
            inner.into_any_element(),
            focused,
            invalid,
            Size::Medium,
            outer,
            window,
            cx,
        )
    }
}

/// A one-time password input bound to an [`OtpState`].
#[derive(IntoElement)]
pub struct OtpInput {
    state: Entity<OtpState>,
    number_of_groups: usize,
    size: Size,
    focus_ring_enabled: bool,
    disabled: bool,
    invalid: bool,
}

impl OtpInput {
    pub fn new(state: &Entity<OtpState>) -> Self {
        Self {
            state: state.clone(),
            number_of_groups: 2,
            size: Size::Medium,
            focus_ring_enabled: true,
            disabled: false,
            invalid: false,
        }
    }

    /// The number of groups the slots are split into.
    pub fn groups(mut self, n: usize) -> Self {
        self.number_of_groups = n;
        self
    }

    /// Mark the code wrong: red borders and one shake. Not in GPUI Component.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    fn resolved_groups(length: usize, requested: usize) -> usize {
        requested.max(1).min(length.max(1))
    }
}

impl Disableable for OtpInput {
    fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl FocusableExt for OtpInput {
    fn focus_ring(mut self, enabled: bool) -> Self {
        self.focus_ring_enabled = enabled;
        self
    }

    fn is_focus_ring_enabled(&self) -> bool {
        self.focus_ring_enabled
    }
}

impl Sizable for OtpInput {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

/// The characters each slot last showed, to arm a pop when one changes.
struct Slots(Vec<Option<char>>);

impl RenderOnce for OtpInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        // GPUI Component's OtpInput registers the focused input with the window's Root while it
        // renders; build it for that and drop it, so `window.focused_input` keeps working.
        {
            let stock = gpui_kit::component::input::OtpInput::new(&self.state)
                .with_size(self.size)
                .disabled(self.disabled);
            drop(RenderOnce::render(stock, window, cx).into_any_element());
        }

        let id: ElementId = ("kk-otp", self.state.entity_id()).into();
        let state = self.state.read(cx);
        let reduced = cx.reduce_motion();
        let blink_show = state.cursor_visible(cx) || reduced;
        let is_focused = state.focus_handle(cx).is_focused(window);
        let masked = state.is_masked();
        let length = state.len();
        let chars: Vec<Option<char>> = (0..length)
            .map(|ix| state.value().chars().nth(ix))
            .collect();
        let cursor_ix = state.value().chars().count().min(length.saturating_sub(1));

        let text_size = match self.size {
            Size::XSmall => px(14.),
            Size::Small => px(14.),
            Size::Medium => px(16.),
            Size::Large => px(18.),
            Size::Size(v) => v * 0.5,
        };

        // Characters typed after the first render pop in; a code already there shows at rest.
        let seen =
            window.use_keyed_state((id.clone(), "kk-slots"), cx, |_, _| Slots(chars.clone()));
        let changed: Vec<usize> = (0..length)
            .filter(|&ix| seen.read(cx).0.get(ix).copied().flatten() != chars[ix])
            .collect();
        if !changed.is_empty() {
            seen.update(cx, |seen, _| seen.0 = chars.clone());
            for &ix in &changed {
                if chars[ix].is_some() {
                    Pulse::new(crate::motion::child_id(&id, ix), window, cx).fire(cx);
                }
            }
        }
        let poses: Vec<Option<Pose>> = (0..length)
            .map(|ix| {
                let pulse = Pulse::new(crate::motion::child_id(&id, ix), window, cx);
                pulse.animate(POP, window);
                char_frame(pulse.elapsed())
            })
            .collect();

        let seen_invalid =
            window.use_keyed_state((id.clone(), "kk-invalid"), cx, |_, _| Seen(self.invalid));
        if seen_invalid.read(cx).0 != self.invalid {
            seen_invalid.update(cx, |seen, _| seen.0 = self.invalid);
            if self.invalid {
                Pulse::new((id.clone(), "kk-shake"), window, cx).fire(cx);
            }
        }
        let shake = Pulse::new((id.clone(), "kk-shake"), window, cx);
        shake.animate(SHAKE, window);
        let dx = shake_em(shake.elapsed()) * f32::from(text_size);

        let theme = cx.theme();
        let (bg, fg): (Hsla, Hsla) = if self.disabled {
            (
                theme.input.mix_oklab(theme.transparent, 0.8),
                theme.muted_foreground,
            )
        } else {
            (theme.input_background(), theme.foreground)
        };
        let border = if self.invalid {
            theme.danger
        } else {
            theme.input
        };
        let ring = theme.ring;
        let caret = theme.caret;
        let mask_color = if self.disabled {
            theme.muted_foreground
        } else {
            theme.secondary_foreground
        };
        let radius = theme.radius;
        let text = TextSetting::new(
            &TextStyleRefinement {
                font_size: Some(text_size.into()),
                color: Some(fg),
                ..Default::default()
            },
            window,
        );

        let number_of_groups = Self::resolved_groups(length, self.number_of_groups);
        let group_items_count = length.div_ceil(number_of_groups).max(1);
        let mut groups: Vec<Vec<AnyElement>> = (0..number_of_groups).map(|_| Vec::new()).collect();
        let mut group_ix = 0;
        for ix in 0..length {
            if ix % group_items_count == 0 && ix != 0 {
                group_ix += 1;
            }
            let is_input_focused = ix == cursor_ix && is_focused;
            let focus_visible = is_input_focused && !self.disabled && self.focus_ring_enabled;
            let content: Option<AnyElement> = chars[ix].map(|c| {
                let glyph = c.to_string();
                match (poses[ix], masked) {
                    // Moving: a vector glyph, squashed and raised exactly.
                    (Some(pose), false) => {
                        let line_box = text.line_box(&glyph, window);
                        let pose = Pose {
                            y: pose.y * f32::from(text_size),
                            ..pose
                        };
                        Vector::new(line_box.size)
                            .layer(text.layer(&glyph, line_box))
                            .pose(pose)
                            .into_any_element()
                    }
                    (pose, true) => {
                        let icon = Icon::new(IconName::Asterisk)
                            .text_color(mask_color)
                            .with_size(text_size);
                        match pose {
                            Some(pose) => {
                                let pose = Pose {
                                    y: pose.y * f32::from(text_size),
                                    ..pose
                                };
                                let side = size(text_size, text_size);
                                div()
                                    .opacity(pose.alpha())
                                    .child(icon.transform(transformation(
                                        &pose,
                                        point(0.5, 0.5),
                                        side,
                                    )))
                                    .into_any_element()
                            }
                            None => icon.into_any_element(),
                        }
                    }
                    (None, false) => glyph.into_any_element(),
                }
            });
            let state_entity = self.state.clone();
            groups[group_ix].push(
                h_flex()
                    .id(ix)
                    .border_1()
                    .border_color(border)
                    .bg(bg)
                    .text_color(fg)
                    .when(self.disabled, |this| this.opacity(0.5))
                    .when(focus_visible, |this| this.border_color(ring))
                    .items_center()
                    .justify_center()
                    .rounded(radius)
                    .text_size(text_size)
                    .map(|this| match self.size {
                        Size::XSmall | Size::Small => this.w_6().h_6(),
                        Size::Medium => this.w_8().h_8(),
                        Size::Large => this.w_11().h_11(),
                        Size::Size(px) => this.w(px).h(px),
                    })
                    .when(focus_visible, |this| this.focus_ring_style(window, cx))
                    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                        state_entity.read(cx).focus_handle(cx).focus(window, cx)
                    })
                    .children(content)
                    .when(
                        content_is_empty(&chars, ix) && is_input_focused && blink_show,
                        |this| this.child(div().h_4().w_0().border_l_3().border_color(caret)),
                    )
                    .into_any_element(),
            );
        }

        BaseOtpInput::new(&self.state)
            .disabled(self.disabled)
            .child(
                v_flex().id(id.clone()).items_center().child(transform(
                    (id, "kk-shake-pose"),
                    Pose::new().x(dx),
                    h_flex().items_center().gap_5().children(
                        groups
                            .into_iter()
                            .map(|inputs| h_flex().items_center().gap_1().children(inputs)),
                    ),
                )),
            )
    }
}

fn content_is_empty(chars: &[Option<char>], ix: usize) -> bool {
    chars.get(ix).copied().flatten().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_match_the_web() {
        use crate::parity::{assert_number_track, assert_pose_track};
        let shake = shake_track();
        assert_number_track("pop-input", "kk-pop-input-shake", "x", &shake);
        assert_number_track("pop-textarea", "kk-pop-textarea-shake", "x", &shake);
        assert_number_track("pop-input-otp", "kk-pop-input-otp-shake", "x", &shake);
        assert_pose_track("pop-input-otp", "kk-pop-input-otp-pop", &char_pose(), &[]);
        assert_number_track(
            "pop-input-otp",
            "kk-pop-input-otp-pop",
            "opacity",
            &char_opacity(),
        );
    }

    #[test]
    fn a_character_pops_then_rests() {
        let first = char_frame(Some(ms(0))).unwrap();
        assert_eq!((first.sx, first.opacity), (0.3, 0.0));
        let peak = char_frame(Some(ms(165))).unwrap();
        assert!((peak.sx - 1.2).abs() < 1e-4 && (peak.sy - 1.25).abs() < 1e-4);
        assert_eq!(char_frame(Some(ms(300))), None);
        assert_eq!(char_frame(None), None);
    }

    #[test]
    fn the_shake_halves_each_swing() {
        assert!((shake_em(Some(ms(60))) + 0.375).abs() < 1e-4);
        assert_eq!(shake_em(None), 0.0);
        assert_eq!(shake_em(Some(ms(400))), 0.0);
    }

    #[test]
    fn invalid_group_counts_are_safely_clamped() {
        assert_eq!(OtpInput::resolved_groups(6, 0), 1);
        assert_eq!(OtpInput::resolved_groups(6, 20), 6);
        assert_eq!(OtpInput::resolved_groups(5, 2), 2);
    }
}
