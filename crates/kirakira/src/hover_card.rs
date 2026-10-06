//! Hover Card: a card that drops out of its trigger when hovered and swings like a hung tag.
//!
//! Replaces `gpui_kit::component::hover_card`, with the same builder and the same
//! `HoverCardState`.
//!
//! Over 0.5 s the card grows from 0.85 and drops the last 0.5rem away from the trigger (both done
//! by 40 %); opacity is done by 25 %. It hangs from the middle of its trigger-facing edge like a
//! tag on a string and swings 6° → −1.92° (40 %) → 0.96° (70 %) → 0: the settle swing, a first
//! rebound of a third, then halving. Closing shrinks it to 0.9 and fades it out in 0.12 s,
//! sliding half way back. Under reduced motion it fades in over 0.15 s and out over 0.1 s where
//! it rests, without the swing.
//!
//! # How it's built
//!
//! Like GPUI Component's, it is a skin over gpui-base's `HoverCard`: the popover surface,
//! `overflow_hidden`, wrapped in [`motion::transform`](crate::motion::transform). The slide hangs
//! on the side the card landed on, measured from where trigger and card were painted. gpui-base
//! drops the card the frame its close delay runs out; the exit builds it once more from the same
//! content builder and paints it where it was for 0.12 s, without input.
//!
//! GPUI can't turn an element subtree, only [vector layers](crate::vector), so how the swing is
//! drawn depends on what the card holds:
//!
//! - **A plain card** — one built with [`HoverCard::title`] and [`HoverCard::description`] and
//!   nothing else: no `content`, no children, the default appearance, and no style beyond its
//!   size — turns exactly. While it swings it is drawn as vector layers on its pin: the surface
//!   (background, ring and the two soft shadows) and each text line, wrapped where GPUI wraps it.
//!   The real card sits under it, hidden, to measure the layout, and takes over at rest, so the
//!   final frame is GPUI's own. Vector text is a touch softer while it moves.
//! - **Any other card** (an avatar, buttons, arbitrary elements) sways instead: it moves the way
//!   its centre moves when it turns about the pin, without the tilt. Its content can't be drawn
//!   as single-colour layers, and turning only the surface would tilt the card under level
//!   content.
//!
//! # Differences from the web version
//!
//! - Cards with arbitrary content sway without tilting (above).
//! - The scale is even and comes from the rem size: rem-sized spacing and text scale, pixel sizes
//!   don't. A plain card's vector rendition scales exactly.

use std::rc::Rc;
use std::time::Duration;

use gpui_kit::base::{ElementExt as _, HoverCard as BaseHoverCard, StyledExt as _};
use gpui_kit::component::{ActiveTheme as _, ThemeStyled as _};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    Anchor, AnyElement, App, Bounds, Context, Div, ElementId, Entity, FontWeight, Hsla,
    InteractiveElement as _, IntoElement, ParentElement, Pixels, RenderOnce, SharedString,
    Stateful, StyleRefinement, Styled, TextStyleRefinement, Window, div, hsla, px, rems, size,
};

pub use gpui_kit::base::HoverCardState;

use crate::motion::{self, Pose};
use crate::overlay::{self, Anchored, Phase, Side, Surface, anchor_alignment};
use crate::vector::{Layer, TextSetting, Vector, svg_family};

type ContentBuilder = Rc<
    dyn Fn(&mut HoverCardState, &mut Window, &mut Context<HoverCardState>) -> AnyElement + 'static,
>;

/// Room around the vector card for its shadow: the softer layer's blur (3 px, three deviations
/// out) below its 4 px drop.
const SHADOW_ROOM: f32 = 16.0;

/// A hover card element that displays content when hovering over a trigger element.
///
/// Like Popover but opened by hovering, with delays for showing and hiding. On iOS and Android,
/// tapping the trigger toggles it.
#[derive(IntoElement)]
pub struct HoverCard {
    id: ElementId,
    style: StyleRefinement,
    anchor: Anchor,
    trigger: Option<Box<dyn FnOnce(&mut Window, &App) -> AnyElement + 'static>>,
    content: Option<ContentBuilder>,
    children: Vec<AnyElement>,
    title: Option<SharedString>,
    description: Option<SharedString>,
    open_delay: Duration,
    close_delay: Duration,
    appearance: bool,
    on_open_change: Option<Rc<dyn Fn(&bool, &mut Window, &mut App)>>,
}

impl HoverCard {
    /// Create a new HoverCard.
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            style: StyleRefinement::default(),
            anchor: Anchor::TopCenter,
            trigger: None,
            content: None,
            children: vec![],
            title: None,
            description: None,
            open_delay: Duration::from_secs_f64(0.6),
            close_delay: Duration::from_secs_f64(0.3),
            appearance: true,
            on_open_change: None,
        }
    }

    /// Set the anchor corner of the hover card, default is [`Anchor::TopCenter`].
    pub fn anchor(mut self, anchor: impl Into<Anchor>) -> Self {
        self.anchor = anchor.into();
        self
    }

    /// Set the trigger element of the hover card.
    pub fn trigger<T>(mut self, trigger: T) -> Self
    where
        T: IntoElement + 'static,
    {
        self.trigger = Some(Box::new(|_, _| trigger.into_any_element()));
        self
    }

    /// Set the content builder of the hover card.
    pub fn content<F, E>(mut self, content: F) -> Self
    where
        F: Fn(&mut HoverCardState, &mut Window, &mut Context<HoverCardState>) -> E + 'static,
        E: IntoElement + 'static,
    {
        self.content = Some(Rc::new(move |state, window, cx| {
            content(state, window, cx).into_any_element()
        }));
        self
    }

    /// A title line, in semibold small text, above the description. Kirakira's own: a card of
    /// only a title and a description swings exactly (see the [module docs](self)). It is 16rem
    /// wide, like the web card, unless styled otherwise.
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Small muted text under the title, wrapped to the card's width. Kirakira's own, like
    /// [`HoverCard::title`].
    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the delay before showing the hover card, default is 600ms.
    pub fn open_delay(mut self, duration: Duration) -> Self {
        self.open_delay = duration;
        self
    }

    /// Set the delay before hiding the hover card, default is 300ms.
    pub fn close_delay(mut self, duration: Duration) -> Self {
        self.close_delay = duration;
        self
    }

    /// Set whether to apply default appearance styles, default is `true`.
    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    /// Set a callback to be called when the open state changes.
    pub fn on_open_change<F>(mut self, callback: F) -> Self
    where
        F: Fn(&bool, &mut Window, &mut App) + 'static,
    {
        self.on_open_change = Some(Rc::new(callback));
        self
    }

    /// GPUI Component's hover card surface: the popover's, clipped.
    fn render_card(anchor: Anchor, appearance: bool, cx: &App) -> Stateful<Div> {
        gpui_kit::base::v_flex()
            .id("content")
            .occlude()
            .tab_group()
            .when(appearance, |this| this.popover_style(cx).p_3())
            .map(|this| match anchor {
                Anchor::BottomLeft | Anchor::BottomCenter | Anchor::BottomRight => this.bottom_1(),
                _ => this.top_1(),
            })
            .overflow_hidden()
    }
}

impl Styled for HoverCard {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl ParentElement for HoverCard {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

/// A plain card's text: its title and description, each with how it is set.
#[derive(Clone)]
struct Plain {
    blocks: Vec<(SharedString, TextStyleRefinement)>,
}

impl Plain {
    fn new(title: Option<SharedString>, description: Option<SharedString>, cx: &App) -> Self {
        let theme = cx.theme();
        let style = |weight: FontWeight, color: Hsla| TextStyleRefinement {
            font_size: Some(rems(0.875).into()),
            font_weight: Some(weight),
            color: Some(color),
            ..Default::default()
        };
        let blocks = [
            title.map(|title| (title, style(FontWeight::SEMIBOLD, theme.popover_foreground))),
            description.map(|text| (text, style(FontWeight::NORMAL, theme.muted_foreground))),
        ];
        Self {
            blocks: blocks.into_iter().flatten().collect(),
        }
    }

    /// The card's text as GPUI elements, each block reporting where it was laid out.
    fn render(&self, layout: &Entity<PlainLayout>) -> Div {
        gpui_kit::base::v_flex()
            .w_full()
            .gap_1()
            .children(self.blocks.iter().enumerate().map(|(ix, (text, style))| {
                let layout = layout.clone();
                let mut block = div().on_prepaint(move |bounds, _, cx| {
                    layout.update(cx, |layout, _| layout.blocks[ix] = Some(bounds))
                });
                *block.text_style() = style.clone();
                block.child(text.clone())
            }))
    }
}

/// Where a plain card and its text blocks rest, measured from the real card.
#[derive(Default)]
struct PlainLayout {
    card: Option<Bounds<Pixels>>,
    blocks: [Option<Bounds<Pixels>>; 2],
}

/// Whether `style` only sizes the card, so its surface still looks as the vector draws it.
fn sizes_only(style: &StyleRefinement) -> bool {
    let mut rest = style.clone();
    rest.size = Default::default();
    rest.min_size = Default::default();
    rest.max_size = Default::default();
    rest == StyleRefinement::default()
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A rounded rectangle's outline as SVG path data.
fn rounded(x: f32, y: f32, w: f32, h: f32, r: f32) -> String {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    format!(
        "M{} {y}h{}a{r} {r} 0 0 1 {r} {r}v{}a{r} {r} 0 0 1 -{r} {r}h-{}a{r} {r} 0 0 1 -{r} -{r}v-{}a{r} {r} 0 0 1 {r} -{r}z",
        x + r,
        w - 2.0 * r,
        h - 2.0 * r,
        w - 2.0 * r,
        h - 2.0 * r,
    )
}

/// One SVG layer of a `bw` × `bh` box: `body`, optionally blurred by `blur` (a gaussian deviation,
/// as GPUI's box shadows take it).
fn layer_svg(bw: f32, bh: f32, body: &str, blur: f32) -> String {
    let (defs, filter) = if blur > 0.0 {
        (
            format!(
                r#"<defs><filter id="b" filterUnits="userSpaceOnUse" x="0" y="0" width="{bw}" height="{bh}"><feGaussianBlur stdDeviation="{blur}"/></filter></defs>"#
            ),
            r#" filter="url(#b)""#,
        )
    } else {
        (String::new(), "")
    };
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {bw} {bh}" width="{bw}" height="{bh}">{defs}<g{filter}>{body}</g></svg>"#
    )
}

/// A plain card drawn as vector layers at `pose`, turned about `origin` (fractions of the card),
/// placed over the real card's wrapper at `wrapper`. `None` until the real card has been measured.
fn swing_vector(
    plain: &Plain,
    layout: &PlainLayout,
    wrapper: Bounds<Pixels>,
    pose: Pose,
    origin: (f32, f32),
    window: &Window,
    cx: &App,
) -> Option<AnyElement> {
    let card = layout.card?;
    let (w, h) = (f32::from(card.size.width), f32::from(card.size.height));
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let theme = cx.theme();
    let room = SHADOW_ROOM;
    let (bw, bh) = (w + 2.0 * room, h + 2.0 * room);
    let r = f32::from(theme.radius);
    // `overlay::pose_shadow`: the ring and shadows follow the cube of the opacity, so a fading
    // card doesn't show them through itself.
    let alpha = pose.alpha();
    let strength = alpha * alpha * alpha;
    let ink = hsla(0., 0., 0., 0.1 * strength);
    let fill = |path: String| format!(r#"<path d="{path}" fill="black" fill-rule="evenodd"/>"#);

    let mut layers = vec![
        // GPUI Component's popover shadow: two soft layers, then the 1 px ring outside the edge.
        Layer::svg(
            layer_svg(
                bw,
                bh,
                &fill(rounded(room + 1.0, room + 5.0, w - 2.0, h - 2.0, r - 1.0)),
                3.0,
            ),
            ink,
        ),
        Layer::svg(
            layer_svg(
                bw,
                bh,
                &fill(rounded(room + 2.0, room + 4.0, w - 4.0, h - 4.0, r - 2.0)),
                2.0,
            ),
            ink,
        ),
        Layer::svg(
            layer_svg(
                bw,
                bh,
                &fill(format!(
                    "{}{}",
                    rounded(room - 1.0, room - 1.0, w + 2.0, h + 2.0, r + 1.0),
                    rounded(room, room, w, h, r)
                )),
                0.0,
            ),
            theme.foreground.alpha(0.1 * strength),
        ),
        Layer::svg(
            layer_svg(bw, bh, &fill(rounded(room, room, w, h, r)), 0.0),
            theme.popover,
        ),
    ];

    // The text, line by line where GPUI breaks it, from where the real blocks were laid out.
    for ((text, style), bounds) in plain.blocks.iter().zip(&layout.blocks) {
        let bounds = (*bounds)?;
        let setting = TextSetting::new(style, window);
        let baseline = f32::from(setting.line_box(text, window).baseline);
        let line_height = f32::from(setting.line_height);
        let x = room + f32::from(bounds.origin.x - card.origin.x);
        let top = room + f32::from(bounds.origin.y - card.origin.y);
        let mut markup = String::new();
        for (ix, range) in setting
            .wrap(text, bounds.size.width, window)
            .into_iter()
            .enumerate()
        {
            let line = text[range].trim_end();
            if line.is_empty() {
                continue;
            }
            markup.push_str(&format!(
                r#"<text x="{x}" y="{}" font-family="{}" font-weight="{}" font-size="{}" fill="black">{}</text>"#,
                top + ix as f32 * line_height + baseline,
                escape(&svg_family(&setting.face.family)),
                setting.face.weight.0.round() as u32,
                f32::from(setting.face.size),
                escape(line),
            ));
        }
        layers.push(Layer::svg(layer_svg(bw, bh, &markup, 0.0), setting.color));
    }

    let origin = ((room + origin.0 * w) / bw, (room + origin.1 * h) / bh);
    Some(
        div()
            .absolute()
            .left(card.origin.x - wrapper.origin.x - px(room))
            .top(card.origin.y - wrapper.origin.y - px(room))
            .child(
                Vector::new(size(px(bw), px(bh)))
                    .layers(layers)
                    .pose(pose)
                    .origin(origin.0, origin.1),
            )
            .into_any_element(),
    )
}

impl RenderOnce for HoverCard {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Some(trigger) = self.trigger else {
            return div().id("empty").into_any_element();
        };

        let anchor = self.anchor;
        let appearance = self.appearance;
        let content = self.content;
        let style = self.style;
        let kk = window.use_keyed_state((self.id.clone(), "kk-hover-card"), cx, |_, _| {
            Anchored::<HoverCardState>::default()
        });
        let has_text = self.title.is_some() || self.description.is_some();
        let plain_text = has_text.then(|| Plain::new(self.title, self.description, cx));
        // Only a card of nothing but its text can be drawn as vector layers.
        let plain = plain_text.clone().filter(|_| {
            content.is_none() && self.children.is_empty() && appearance && sizes_only(&style)
        });
        let plain_layout =
            window.use_keyed_state((self.id.clone(), "kk-hover-card-plain"), cx, |_, _| {
                PlainLayout::default()
            });

        let open = kk
            .read(cx)
            .state
            .as_ref()
            .is_some_and(|state| state.read(cx).is_open());
        let now = motion::now();
        let reduced = cx.reduce_motion();
        if !open {
            kk.update(cx, |kk, _| kk.presence.set(false, now));
        }
        let phase = Surface::hover_card().phase(&kk.read(cx).presence, now, reduced);
        let side = kk.read(cx).side(Side::of_anchor(anchor));
        let origin = side.origin(anchor_alignment(anchor));

        // The card: GPUI Component's surface with the content, the children and a plain card's
        // text, which reports where it rests.
        let build_card =
            {
                let content = content.clone();
                let plain_text = plain_text.clone();
                let plain_layout = plain_layout.clone();
                move |children: Vec<AnyElement>,
                      pose: Pose,
                      state: &mut HoverCardState,
                      window: &mut Window,
                      cx: &mut Context<HoverCardState>| {
                    let measure = plain_layout.clone();
                    Self::render_card(anchor, appearance, cx)
                        .when(plain_text.is_some(), |this| this.w(rems(16.)))
                        .when_some(content.clone(), |this, content| {
                            this.child(content(state, window, cx))
                        })
                        .when_some(plain_text.clone(), |this, plain| {
                            this.child(plain.render(&plain_layout))
                        })
                        .children(children)
                        .refine_style(&style)
                        .when(appearance, |this| {
                            this.when_some(overlay::pose_shadow(pose, cx), |this, shadow| {
                                this.shadow(shadow)
                            })
                        })
                        // A plain card's own box, for its vector rendition.
                        .when(plain_text.is_some(), |this| {
                            this.child(div().absolute().inset_0().on_prepaint(
                                move |bounds, _, cx| {
                                    measure.update(cx, |layout, _| layout.card = Some(bounds))
                                },
                            ))
                        })
                }
            };

        // The exit: the card once more, where it was.
        let (children, ghost) = match (phase, kk.read(cx).state.clone(), kk.read(cx).surface) {
            (Phase::Closing(_), Some(state), Some(bounds)) => {
                window.request_animation_frame();
                let pose = Surface::hover_card().pose(phase, side, reduced, window.rem_size());
                let children = self.children;
                let card = state.update(cx, |state, cx| {
                    build_card(children, pose, state, window, cx)
                });
                let ghost = overlay::ghost(
                    bounds,
                    motion::transform("kk-hover-card-ghost", pose, card).origin(origin.0, origin.1),
                );
                (Vec::new(), Some(ghost))
            }
            _ => (self.children, None),
        };

        let trigger = overlay::record_trigger(&kk, div())
            .child(trigger(window, cx))
            .children(ghost);
        let surface_kk = kk.clone();

        BaseHoverCard::new(self.id)
            .anchor(anchor)
            .open_delay(self.open_delay)
            .close_delay(self.close_delay)
            .trigger(trigger)
            .content(move |state, window, cx| {
                // gpui-base renders this only while open, so this is where an open begins.
                let (mut pose, origin) =
                    overlay::open_pose(&surface_kk, Surface::hover_card(), anchor, window, cx);
                let wrapper = surface_kk.read(cx).surface;
                let swinging = pose.rotate != 0.0;
                let id = "kk-hover-card-surface";

                // A plain card swings as vector layers over the real one, which rests hidden
                // underneath so its layout can be measured.
                if swinging && let Some(plain) = plain.as_ref() {
                    let hidden = Pose::new().opacity(0.0);
                    let card = build_card(children, hidden, state, window, cx);
                    let vector = wrapper.and_then(|wrapper| {
                        swing_vector(
                            plain,
                            plain_layout.read(cx),
                            wrapper,
                            pose,
                            origin,
                            window,
                            cx,
                        )
                    });
                    return div().id("kk-hover-card").child(
                        overlay::surface(&surface_kk, id, hidden, origin, card)
                            .relative()
                            .children(vector),
                    );
                }

                // Any other card can't turn: it sways the way the turn would move it.
                if swinging && let Some(wrapper) = wrapper {
                    let (dx, dy) = overlay::sway(&pose, origin, wrapper.size);
                    pose.x += dx;
                    pose.y += dy;
                }
                let card = build_card(children, pose, state, window, cx);
                // gpui-base listens for hover on what this returns, so return a stateful div.
                div().id("kk-hover-card").child(overlay::surface(
                    &surface_kk,
                    id,
                    pose,
                    origin,
                    card,
                ))
            })
            .when_some(self.on_open_change, |this, callback| {
                this.on_open_change(move |open, window, cx| callback(open, window, cx))
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_sizes_keep_a_card_plain() {
        let mut style = StyleRefinement::default();
        assert!(sizes_only(&style));
        style.size.width = Some(rems(20.).into());
        assert!(sizes_only(&style));
        style.background = Some(gpui_kit::red().into());
        assert!(!sizes_only(&style));
    }

    #[test]
    fn rounded_paths_close() {
        let path = rounded(0.0, 0.0, 100.0, 40.0, 8.0);
        assert!(path.starts_with("M8 0h84"));
        assert!(path.ends_with('z'));
        // A radius larger than the box allows is clamped to half its short side.
        assert!(rounded(0.0, 0.0, 10.0, 10.0, 20.0).starts_with("M5 0h0"));
    }
}
