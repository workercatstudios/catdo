//! Sheet: a panel that slides in past its rest behind a plate of the primary colour. The bottom
//! sheet is the drawer, with a handle that squashes as the panel lands.
//!
//! Replaces `gpui_kit::component::sheet`. `Sheet` is Kirakira's, with GPUI Component's builder;
//! `SheetSettings` is re-exported. Open it with [`crate::WindowExt::open_sheet`] or
//! [`open_sheet_at`](crate::WindowExt::open_sheet_at) and draw it with
//! [`crate::Root::render_sheet_layer`] (see [`crate::root`]).
//!
//! The panel is GPUI Component's: the same size, title bar, close button, body and footer, on
//! gpui-base's sheet host (focus trap, Escape, backdrop dismissal). The motion is Pop Sheet's, and
//! Pop Drawer's for the bottom sheet:
//!
//! - Open, 0.5 s: the panel slides in from fully off-screen on `cubic-bezier(0.6, 0, 0.2, 1)`,
//!   runs 0.375 rem past its rest at 62 %, comes back 0.125 rem short at 82 % and settles
//!   (ease-in-out). A plate of the primary colour, a copy of the panel lying just under it, sweeps
//!   out ahead of it into the screen, 2.5 rem at 35 % of 0.32 s (ease-out), and the panel catches
//!   up and covers it. A strip of the panel's background past its outer edge fills the gap the
//!   overshoot opens against the window edge.
//! - Close: the panel leaves fast on an ease-in, 0.22 s (the drawer 0.25 s).
//! - The backdrop fades in over 0.3 s (ease-out) and out over 0.22 s (ease-in). The drawer's
//!   fades both ways over 0.5 s on vaul's `cubic-bezier(0.32, 0.72, 0, 1)`.
//! - The drawer's handle, a 100 × 8 px pill above the title, squashes as the panel lands:
//!   (1.3, 0.6) → (0.9, 1.2) → (1.05, 0.95) → 1 over 0.4 s, 0.28 s in. It is an owned shape, so
//!   the squash is exact.
//! - Reduced motion: no slide, plate or squash; the panel fades in (0.2 s) and out (0.15 s) where
//!   it rests. The backdrop still fades.
//!
//! Where it differs from the web version, and why: the panel keeps GPUI Component's square
//! corners, size and title bar (the web sheet rounds its inner edge), and the drawer is GPUI
//! Component's bottom sheet: it has a close button and no drag to dismiss.

use std::rc::Rc;
use std::time::Duration;

use gpui_kit::base::{Sheet as BaseSheet, StyledExt as _, actions::Cancel};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, IconName, Placement, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AnyElement, App, ClickEvent, DefiniteLength, DismissEvent, Edges, EventEmitter, FocusHandle,
    InteractiveElement as _, IntoElement, ParentElement, Pixels, RenderOnce,
    StatefulInteractiveElement as _, StyleRefinement, Styled, Window, anchored, deferred, div,
    point, px, rems,
};

pub use gpui_kit::component::sheet::SheetSettings;

use crate::motion::{Easing, Keyframes, Pose, Track, ms, now};
use crate::root::{self, Presence, Stage, WindowExt as _};

const SLIDE_IN: Duration = ms(500);
const PLATE: Duration = ms(320);
const SHEET_OUT: Duration = ms(220);
const DRAWER_OUT: Duration = ms(250);
const FADE_IN: Duration = ms(200);
const FADE_OUT: Duration = ms(150);
const BACKDROP_IN: Duration = ms(300);
const BACKDROP_OUT: Duration = ms(220);
const DRAWER_BACKDROP: Duration = ms(500);
const HANDLE_DELAY: Duration = ms(280);
const HANDLE: Duration = ms(400);

fn progress(elapsed: Duration, duration: Duration) -> f32 {
    (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
}

fn bezier(x1: f32, y1: f32, x2: f32, y2: f32) -> Easing {
    Easing::cubic_bezier(x1, y1, x2, y2).expect("a valid curve")
}

/// `kk-pop-sheet-in` at `elapsed`: how far the panel is from its rest, into the screen, for a
/// panel `extent` long across its axis and a rem of `rem`. Negative is still off-screen.
pub(crate) fn slide_in(elapsed: Duration, extent: f32, rem: f32) -> f32 {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, -extent, bezier(0.6, 0.0, 0.2, 1.0))
        .at(0.62, 0.375 * rem)
        .at(0.82, -0.125 * rem)
        .at(1.0, 0.0)
        .build()
        .sample(progress(elapsed, SLIDE_IN))
}

/// `kk-pop-sheet-plate` at `elapsed`: how far the plate reaches past the panel, into the screen.
pub(crate) fn plate(elapsed: Duration, rem: f32) -> f32 {
    Track::new(Easing::EaseInOut)
        .at_ease(0.0, 0.0, Easing::EaseOut)
        .at(0.35, 2.5 * rem)
        .at(1.0, 0.0)
        .build()
        .sample(progress(elapsed, PLATE))
}

/// `kk-pop-drawer-handle`: (1.3, 0.6) at 30 % → (0.9, 1.2) at 60 % → (1.05, 0.95) at 80 % → 1,
/// ease-in-out on each segment.
pub(crate) fn handle_track() -> Keyframes<Pose> {
    Track::new(Easing::EaseInOut)
        .at(0.0, Pose::new())
        .at(0.3, Pose::new().scale_xy(1.3, 0.6))
        .at(0.6, Pose::new().scale_xy(0.9, 1.2))
        .at(0.8, Pose::new().scale_xy(1.05, 0.95))
        .at(1.0, Pose::new())
        .build()
}

/// The handle's squash at `elapsed` since the drawer opened (it starts 0.28 s in).
pub(crate) fn handle(elapsed: Duration) -> (f32, f32) {
    let pose = handle_track().sample(progress(elapsed.saturating_sub(HANDLE_DELAY), HANDLE));
    (pose.sx, pose.sy)
}

fn out_duration(placement: Placement) -> Duration {
    if placement == Placement::Bottom {
        DRAWER_OUT
    } else {
        SHEET_OUT
    }
}

fn backdrop_durations(placement: Placement) -> (Duration, Duration) {
    if placement == Placement::Bottom {
        (DRAWER_BACKDROP, DRAWER_BACKDROP)
    } else {
        (BACKDROP_IN, BACKDROP_OUT)
    }
}

/// How long a closed sheet stays mounted: until both the panel and the backdrop are gone.
pub(crate) fn exit_duration(placement: Placement, cx: &App) -> Duration {
    let panel = if cx.reduce_motion() {
        FADE_OUT
    } else {
        out_duration(placement)
    };
    panel.max(backdrop_durations(placement).1)
}

/// The panel's offset into the screen, its opacity, the plate's reach and the backdrop's opacity
/// at `stage`, and whether anything still moves.
struct Frame {
    offset: f32,
    opacity: f32,
    plate: f32,
    backdrop: f32,
    running: bool,
}

fn frame(stage: Stage, placement: Placement, extent: f32, rem: f32, reduced: bool) -> Frame {
    let drawer = placement == Placement::Bottom;
    let vaul = bezier(0.32, 0.72, 0.0, 1.0);
    let (backdrop_in, backdrop_out) = backdrop_durations(placement);
    match stage {
        Stage::Open(t) => {
            let backdrop_ease = if drawer { vaul } else { Easing::EaseOut };
            let backdrop = backdrop_ease.sample(progress(t, backdrop_in));
            if reduced {
                Frame {
                    offset: 0.0,
                    opacity: Easing::EaseOut.sample(progress(t, FADE_IN)),
                    plate: 0.0,
                    backdrop,
                    running: t < backdrop_in.max(FADE_IN),
                }
            } else {
                let handle_end = if drawer {
                    HANDLE_DELAY + HANDLE
                } else {
                    Duration::ZERO
                };
                Frame {
                    offset: slide_in(t, extent, rem),
                    opacity: 1.0,
                    plate: plate(t, rem),
                    backdrop,
                    running: t < SLIDE_IN.max(backdrop_in).max(handle_end),
                }
            }
        }
        Stage::Closing(t) => {
            let backdrop_ease = if drawer { vaul } else { Easing::EaseIn };
            let backdrop = 1.0 - backdrop_ease.sample(progress(t, backdrop_out));
            let running = t < backdrop_out.max(out_duration(placement));
            if reduced {
                Frame {
                    offset: 0.0,
                    opacity: 1.0 - Easing::EaseIn.sample(progress(t, FADE_OUT)),
                    plate: 0.0,
                    backdrop,
                    running,
                }
            } else {
                let e = Easing::EaseIn.sample(progress(t, out_duration(placement)));
                Frame {
                    offset: -extent * e,
                    opacity: 1.0,
                    plate: 0.0,
                    backdrop,
                    running,
                }
            }
        }
    }
}

/// Sheet component that slides in from the side of the window.
#[derive(IntoElement)]
pub struct Sheet {
    focus_handle: FocusHandle,
    placement: Placement,
    size: DefiniteLength,
    resizable: bool,
    on_close: Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>,
    title: Option<AnyElement>,
    footer: Option<AnyElement>,
    style: StyleRefinement,
    children: Vec<AnyElement>,
    overlay: bool,
    overlay_closable: bool,
    key: u64,
    presence: Presence,
}

impl Sheet {
    /// Creates a new Sheet.
    pub fn new(_: &mut Window, cx: &mut App) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            placement: Placement::Right,
            size: DefiniteLength::Absolute(px(350.).into()),
            resizable: true,
            title: None,
            footer: None,
            style: StyleRefinement::default(),
            children: Vec::new(),
            overlay: true,
            overlay_closable: true,
            on_close: Rc::new(|_, _, _| {}),
            key: 0,
            presence: Presence::new(now()),
        }
    }

    /// Sets the title of the sheet.
    pub fn title(mut self, title: impl IntoElement) -> Self {
        self.title = Some(title.into_any_element());
        self
    }

    /// Set the footer of the sheet.
    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    /// Sets the size of the sheet, default is 350px.
    pub fn size(mut self, size: impl Into<DefiniteLength>) -> Self {
        self.size = size.into();
        self
    }

    /// Sets whether the sheet is resizable, default is `true`.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Set whether the sheet should have an overlay, default is `true`.
    pub fn overlay(mut self, overlay: bool) -> Self {
        self.overlay = overlay;
        self
    }

    /// Set whether the sheet should be closable by clicking the overlay, default is `true`.
    pub fn overlay_closable(mut self, overlay_closable: bool) -> Self {
        self.overlay_closable = overlay_closable;
        self
    }

    /// Listen to the close event of the sheet.
    pub fn on_close(
        mut self,
        on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Rc::new(on_close);
        self
    }
}

impl EventEmitter<DismissEvent> for Sheet {}

impl ParentElement for Sheet {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for Sheet {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for Sheet {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let _ = self.resizable;
        let placement = self.placement;
        let drawer = placement == Placement::Bottom;
        let reduced = cx.reduce_motion();
        let stage = self.presence.stage(now());
        let open = matches!(stage, Stage::Open(_));
        let insets = gpui_kit::component::window_paddings(window);
        let size = window.viewport_size()
            - gpui_kit::size(insets.left + insets.right, insets.top + insets.bottom);
        let top = cx.theme().sheet.margin_top;
        let rem = window.rem_size();
        let base_size = window.text_style().font_size;

        let mut paddings = Edges::all(px(16.));
        if let Some(pl) = self.style.padding.left {
            paddings.left = pl.to_pixels(base_size, rem);
        }
        if let Some(pr) = self.style.padding.right {
            paddings.right = pr.to_pixels(base_size, rem);
        }

        let extent: Pixels = if placement.is_horizontal() {
            root::length_px(self.size, size.width, window)
        } else {
            let available = match placement {
                Placement::Bottom => size.height,
                _ => size.height - top,
            };
            root::length_px(self.size, available, window)
        };
        let f = frame(stage, placement, f32::from(extent), f32::from(rem), reduced);
        if f.running {
            window.request_animation_frame();
        }
        let offset = px(f.offset);

        let theme = cx.theme();
        let (background, border, primary, muted) = (
            theme.tokens.background,
            theme.border,
            theme.primary,
            theme.muted,
        );
        let overlay_color = if self.overlay {
            theme.overlay.opacity(f.backdrop)
        } else {
            gpui_kit::hsla(0., 0., 0., 0.)
        };

        // Where the panel's edge sits: `offset` in from the window edge.
        let place = |this: gpui_kit::Stateful<gpui_kit::Div>, offset: Pixels| match placement {
            Placement::Top => this.top(top + offset).left_0().right_0(),
            Placement::Right => this.top(top).right(offset).bottom_0(),
            Placement::Bottom => this.bottom(offset).left_0().right_0(),
            Placement::Left => this.top(top).left(offset).bottom_0(),
        };
        let sized = |this: gpui_kit::Stateful<gpui_kit::Div>| {
            if placement.is_horizontal() {
                this.w(self.size)
            } else {
                this.h(self.size)
            }
        };

        // The plate: a copy of the panel's box in the primary colour, reaching past it.
        let plate = (f.plate > 0.0).then(|| {
            place(
                sized(div().id("kk-sheet-plate").absolute().bg(primary)),
                offset + px(f.plate),
            )
        });

        // Fills the window edge while the panel runs past its rest.
        let filler = {
            let strip = rems(1.);
            let filler = div().absolute().bg(background);
            match placement {
                Placement::Top => filler.left_0().right_0().h(strip).top(-strip),
                Placement::Right => filler.top_0().bottom_0().w(strip).right(-strip),
                Placement::Bottom => filler.left_0().right_0().h(strip).bottom(-strip),
                Placement::Left => filler.top_0().bottom_0().w(strip).left(-strip),
            }
        };

        let handle = drawer.then(|| {
            let (sx, sy) = match stage {
                Stage::Open(t) if !reduced => handle(t),
                _ => (1.0, 1.0),
            };
            let (w, h) = (px(100.), px(8.));
            div()
                .flex_none()
                .mx_auto()
                .mt_4()
                .w(w)
                .h(h)
                .relative()
                .child(
                    div()
                        .absolute()
                        .left((w - w * sx) / 2.)
                        .top((h - h * sy) / 2.)
                        .w(w * sx)
                        .h(h * sy)
                        .rounded_full()
                        .bg(muted),
                )
        });

        let surface = place(
            sized(
                v_flex()
                    .id("sheet-content")
                    .absolute()
                    .when(open, |this| this.occlude())
                    .bg(background)
                    .border_color(border)
                    .shadow_xl()
                    .opacity(f.opacity)
                    .refine_style(&self.style),
            ),
            offset,
        )
        .map(|this| match placement {
            Placement::Top => this.border_b_1(),
            Placement::Right => this.border_l_1(),
            Placement::Bottom => this.border_t_1(),
            Placement::Left => this.border_r_1(),
        })
        .child(filler)
        .children(handle)
        .child(
            // TitleBar
            h_flex()
                .justify_between()
                .pl_4()
                .pr_3()
                .py_2()
                .w_full()
                .font_semibold()
                .child(self.title.unwrap_or(div().into_any_element()))
                .child(
                    Button::new("close")
                        .small()
                        .ghost()
                        .icon(IconName::Close)
                        .on_click(|_, window, cx| {
                            window.dispatch_action(Box::new(Cancel), cx);
                        }),
                ),
        )
        .child(
            div().flex_1().overflow_hidden().child(
                // Body
                v_flex()
                    .id("kk-sheet-body")
                    .size_full()
                    .overflow_y_scroll()
                    .pl(paddings.left)
                    .pr(paddings.right)
                    .children(self.children),
            ),
        )
        .when_some(self.footer, |this, footer| {
            this.child(
                h_flex()
                    .justify_between()
                    .px_4()
                    .py_3()
                    .w_full()
                    .child(footer),
            )
        })
        // Leaving, the panel takes no input: a cover over its controls swallows a second click,
        // so a footer action can't fire twice nor the click reach what lies below.
        .when(!open, |this| {
            this.child(div().absolute().inset_0().occlude())
        });

        if !open {
            // Leaving: drawn where it was, inert, until the exit has played.
            return anchored()
                .position(point(px(0.), px(0.)))
                .child(
                    div()
                        .absolute()
                        .top(insets.top)
                        .left(insets.left)
                        .w(size.width)
                        .h(size.height)
                        .child(div().absolute().inset_0().bg(overlay_color))
                        .children(plate)
                        .child(surface),
                )
                .into_any_element();
        }

        let overlay = div()
            .occlude()
            .w(size.width)
            .h(size.height)
            .bg(overlay_color);
        let on_close = self.on_close;
        BaseSheet::new(cx)
            .top(insets.top)
            .left(insets.left)
            .w(size.width)
            .h(size.height)
            .focus_handle(self.focus_handle)
            .overlay_interactive(self.overlay)
            .overlay_closable(self.overlay && self.overlay_closable)
            .request_close(|window, cx| window.close_sheet(cx))
            .on_close(move |event, window, cx| on_close(event, window, cx))
            .overlay(div().relative().child(overlay).children(plate))
            .surface(surface)
            .into_any_element()
    }
}

/// Draws the open sheet and any still leaving.
pub(crate) fn render_layer(window: &mut Window, cx: &mut App) -> Option<AnyElement> {
    let layers = root::layers(window, cx);
    let now = now();
    let entries = layers.update(cx, |layers, _| {
        layers
            .sheets
            .iter()
            .map(|sheet| {
                (
                    sheet.key,
                    sheet.builder.clone(),
                    sheet.focus_handle.clone(),
                    sheet.placement,
                    sheet.presence,
                )
            })
            .collect::<Vec<_>>()
    });
    let live = entries
        .into_iter()
        .filter(|entry| !entry.4.gone(now, exit_duration(entry.3, cx)))
        .collect::<Vec<_>>();
    let keys = live.iter().map(|entry| entry.0).collect::<Vec<_>>();
    layers.update(cx, |layers, _| {
        layers.sheets.retain(|sheet| keys.contains(&sheet.key));
        if layers.open_sheet().is_none() {
            layers.sheet_size = None;
        }
    });
    if live.is_empty() {
        return None;
    }
    let mut open_size = None;
    let sheets = live
        .into_iter()
        .map(|(key, builder, focus_handle, placement, presence)| {
            let mut sheet = builder(Sheet::new(window, cx), window, cx);
            sheet.focus_handle = focus_handle;
            sheet.placement = placement;
            sheet.key = key;
            sheet.presence = presence;
            if presence.is_open() {
                open_size = Some(sheet.size);
            }
            div()
                .id(("kk-sheet-layer", key as usize))
                .relative()
                .child(sheet)
        })
        .collect::<Vec<_>>();
    layers.update(cx, |layers, _| layers.sheet_size = open_size);
    Some(
        deferred(div().relative().children(sheets))
            .with_priority(5)
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn slide_overshoots_and_settles() {
        let (extent, rem) = (350.0, 16.0);
        assert!(close(slide_in(Duration::ZERO, extent, rem), -350.0));
        assert!(
            close(slide_in(ms(310), extent, rem), 6.0),
            "0.375 rem past rest at 62 %"
        );
        assert!(
            close(slide_in(ms(410), extent, rem), -2.0),
            "0.125 rem short at 82 %"
        );
        assert!(close(slide_in(ms(500), extent, rem), 0.0));
    }

    #[test]
    fn plate_peaks_at_35_percent() {
        assert!(close(plate(Duration::ZERO, 16.0), 0.0));
        assert!(close(plate(ms(112), 16.0), 40.0));
        assert!(close(plate(ms(320), 16.0), 0.0));
    }

    #[test]
    fn handle_squashes_after_landing() {
        assert_eq!(handle(ms(280)), (1.0, 1.0));
        let (sx, sy) = handle(ms(280 + 120));
        assert!(close(sx, 1.3) && close(sy, 0.6));
        let (sx, sy) = handle(ms(280 + 240));
        assert!(close(sx, 0.9) && close(sy, 1.2));
        assert_eq!(handle(ms(680)), (1.0, 1.0));
    }

    #[test]
    fn handle_matches_the_web_keyframes() {
        crate::parity::assert_pose_track(
            "pop-drawer",
            "kk-pop-drawer-handle",
            &handle_track(),
            &[],
        );
    }

    #[test]
    fn fades_match_the_web_keyframes() {
        // `kk-pop-sheet-in`, `-plate`, `-out` and the drawer's are written with `calc(var(...))`
        // per side, which the parity parser can't read; the slide and plate tests above check
        // their numbers instead.
        let fade_in = Track::new(Easing::EaseOut)
            .at(0.0, 0.0_f32)
            .at(1.0, 1.0)
            .build();
        let fade_out = Track::new(Easing::EaseIn)
            .at(0.0, 1.0_f32)
            .at(1.0, 0.0)
            .build();
        for (component, prefix) in [
            ("pop-sheet", "kk-pop-sheet"),
            ("pop-drawer", "kk-pop-drawer"),
        ] {
            let (fade_in_name, fade_out_name) =
                (format!("{prefix}-fade-in"), format!("{prefix}-fade-out"));
            crate::parity::assert_number_track(component, &fade_in_name, "opacity", &fade_in);
            crate::parity::assert_number_track(component, &fade_out_name, "opacity", &fade_out);
        }
    }

    #[test]
    fn drawer_waits_for_its_backdrop() {
        assert_eq!(out_duration(Placement::Bottom), ms(250));
        assert_eq!(backdrop_durations(Placement::Bottom).1, ms(500));
        assert_eq!(backdrop_durations(Placement::Right).1, ms(220));
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
                .children(crate::Root::render_sheet_layer(window, cx))
        }
    }

    /// A second click on a closing sheet's footer lands on nothing.
    #[gpui_kit::test]
    fn a_leaving_sheet_takes_no_clicks(cx: &mut gpui_kit::TestAppContext) {
        use std::cell::Cell;
        use std::time::Instant;
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
            window.open_sheet(cx, move |sheet, _, _| {
                let clicks = clicks.clone();
                sheet.footer(
                    div()
                        .id("act")
                        .debug_selector(|| "act".into())
                        .size(px(80.))
                        .on_click(move |_, _, _| clicks.set(clicks.get() + 1)),
                )
            });
        });
        crate::motion::freeze_time(Some(start + ms(1000)));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        let act = cx
            .debug_bounds("act")
            .expect("the footer is drawn")
            .center();
        cx.simulate_click(act, gpui_kit::Modifiers::none());
        assert_eq!(clicks.get(), 1);

        // Closed, and just into its exit, so the footer has barely moved.
        cx.update(|window, cx| window.close_sheet(cx));
        crate::motion::freeze_time(Some(start + ms(1020)));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        cx.simulate_click(act, gpui_kit::Modifiers::none());
        crate::motion::freeze_time(None);
        assert_eq!(clicks.get(), 1, "the leaving sheet ran its action again");
    }
}
