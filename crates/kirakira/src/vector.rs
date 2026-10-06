//! Vector layers: real transforms in GPUI.
//!
//! GPUI paints quads and glyphs without a transform, but its SVG pipeline takes a full affine
//! matrix: a monochrome SVG is rasterised once and drawn on the GPU scaled, squashed and rotated.
//! GPUI's SVG renderer also loads the system fonts and the app's bundled fonts and draws `<text>`.
//! So anything that is one colour per layer can move exactly the way CSS moves it: a letter that
//! squashes as it lands, a badge that swings on its pin, a pill that tilts.
//!
//! A [`Vector`] is a box of [`Layer`]s, each an SVG painted in one colour, all under one
//! [`Pose`] around one origin. Build layers with [`Layer::text`], [`Layer::rounded_rect`] or any
//! SVG markup with [`Layer::svg`].
//!
//! Text in a vector layer is drawn by resvg rather than the platform text renderer, so it is a
//! touch softer than GPUI's own text. Components therefore draw real text at rest and switch to
//! vector glyphs only while they move: the final frame is GPUI's text, pixel for pixel.

use std::fmt::Write as _;
use std::ops::Range;
use std::sync::Arc;

use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    App, FontStyle, FontWeight, Hsla, IntoElement, ParentElement as _, Pixels, Point, RenderOnce,
    Size, Styled as _, Transformation, Window, div, point, px, radians, size, svg,
};

use crate::motion::Pose;

/// One single-colour SVG layer of a [`Vector`]. Its markup is drawn in `color`, like Lucide's
/// `currentColor` icons: fills and strokes become the layer's colour, their alpha is kept.
#[derive(Clone, Debug)]
pub struct Layer {
    markup: Arc<str>,
    color: Hsla,
}

/// What a text layer is set in.
#[derive(Clone, Debug)]
pub struct TextFace {
    /// The family, as GPUI names it. `.SystemUIFont` maps to the platform UI family.
    pub family: String,
    pub weight: FontWeight,
    pub style: FontStyle,
    pub size: Pixels,
}

/// The SVG `font-family` list for a GPUI family. GPUI's `.SystemUIFont` is a virtual name that
/// only GPUI resolves, so it becomes the platform's UI family with a generic fallback.
pub fn svg_family(family: &str) -> String {
    let system = if cfg!(target_os = "windows") {
        "Segoe UI"
    } else if cfg!(target_os = "macos") {
        "SF Pro Text, Helvetica Neue"
    } else {
        "Noto Sans, DejaVu Sans"
    };
    // The families the platform text system falls back to for Japanese and Chinese, so mixed text
    // picks the same faces in both renderers.
    let cjk = if cfg!(target_os = "windows") {
        "Yu Gothic UI, Meiryo UI, Microsoft YaHei UI, Malgun Gothic"
    } else if cfg!(target_os = "macos") {
        "Hiragino Sans, PingFang SC, Apple SD Gothic Neo"
    } else {
        "Noto Sans CJK JP, Noto Sans CJK SC"
    };
    let family = match family {
        ".SystemUIFont" | "" => system.to_string(),
        name => format!("{}, {system}", quote(name)),
    };
    format!("{family}, {cjk}, sans-serif")
}

fn quote(name: &str) -> String {
    if name.contains([' ', ',']) {
        format!("'{}'", name.replace('\'', ""))
    } else {
        name.to_string()
    }
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

fn open(size: Size<Pixels>) -> String {
    let (w, h) = (f32::from(size.width), f32::from(size.height));
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}">"#
    )
}

impl Layer {
    /// Any SVG markup for a box of `size`, drawn in `color`.
    pub fn svg(markup: impl Into<Arc<str>>, color: impl Into<Hsla>) -> Self {
        Self {
            markup: markup.into(),
            color: color.into(),
        }
    }

    /// `text` centred horizontally in a box of `size`, with its baseline `baseline` from the top.
    /// [`TextSetting`] gives the box and baseline GPUI's own text would have.
    pub fn text(
        text: &str,
        face: &TextFace,
        box_size: Size<Pixels>,
        baseline: Pixels,
        color: impl Into<Hsla>,
    ) -> Self {
        let mut markup = open(box_size);
        let style = if face.style == FontStyle::Italic {
            "italic"
        } else {
            "normal"
        };
        let _ = write!(
            markup,
            r#"<text x="{x}" y="{y}" text-anchor="middle" font-family="{family}" font-weight="{weight}" font-style="{style}" font-size="{size}" fill="black">{text}</text></svg>"#,
            x = f32::from(box_size.width) / 2.0,
            y = f32::from(baseline),
            family = escape(&svg_family(&face.family)),
            weight = face.weight.0.round() as u32,
            size = f32::from(face.size),
            text = escape(text),
        );
        Self::svg(markup, color)
    }

    /// A rounded rectangle filling a box of `size`, inset by `inset` on every side.
    pub fn rounded_rect(
        box_size: Size<Pixels>,
        radius: Pixels,
        inset: Pixels,
        color: impl Into<Hsla>,
    ) -> Self {
        let mut markup = open(box_size);
        let inset = f32::from(inset);
        let w = (f32::from(box_size.width) - inset * 2.0).max(0.0);
        let h = (f32::from(box_size.height) - inset * 2.0).max(0.0);
        let r = f32::from(radius).min(w / 2.0).min(h / 2.0).max(0.0);
        let _ = write!(
            markup,
            r#"<rect x="{inset}" y="{inset}" width="{w}" height="{h}" rx="{r}" ry="{r}" fill="black"/></svg>"#
        );
        Self::svg(markup, color)
    }

    /// The same layer in another colour.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = color.into();
        self
    }
}

/// How text is set where a component renders: the inherited text style refined with the
/// component's own text styles. What [`Layer::text`] needs to match GPUI's text.
#[derive(Clone, Debug)]
pub struct TextSetting {
    pub face: TextFace,
    pub color: Hsla,
    pub line_height: Pixels,
    style: gpui_kit::TextStyle,
}

/// Where GPUI puts a run of text: its line box and the baseline in it, from the top.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineBox {
    pub size: Size<Pixels>,
    pub baseline: Pixels,
}

impl TextSetting {
    /// The setting under `refinement` (usually a component's `style.text`) at this point of the
    /// tree.
    pub fn new(refinement: &gpui_kit::TextStyleRefinement, window: &Window) -> Self {
        use gpui_kit::Refineable as _;
        let mut style = window.text_style();
        style.refine(refinement);
        let rem = window.rem_size();
        let face = TextFace {
            family: style.font_family.to_string(),
            weight: style.font_weight,
            style: style.font_style,
            size: style.font_size.to_pixels(rem),
        };
        // GPUI's text element snaps its line height to device pixels (22.65 px is 22.4 at 1.25×),
        // where `TextStyle::line_height_in_pixels` rounds to whole logical pixels (23).
        let line_height = window.pixel_snap(style.line_height.to_pixels(face.size.into(), rem));
        Self {
            face,
            color: style.color,
            line_height,
            style,
        }
    }

    /// The line box GPUI gives `text` set this way, and its baseline. Shaped like GPUI's own text,
    /// so fallback fonts (CJK, emoji) get their own metrics.
    pub fn line_box(&self, text: &str, window: &Window) -> LineBox {
        let run = self.style.to_run(text.len());
        let shaped =
            window
                .text_system()
                .shape_line(text.to_string().into(), self.face.size, &[run], None);
        // GPUI centres the shaped line in its line box the same way.
        let padding_top = (self.line_height - shaped.ascent - shaped.descent) / 2.;
        LineBox {
            size: size(shaped.width, self.line_height),
            baseline: padding_top + shaped.ascent,
        }
    }

    /// Where GPUI breaks `text` set this way in a box `width` wide: the byte range of each line,
    /// in order. GPUI's text shaper decides, so the lines match its own wrapped text; set each with
    /// [`line_box`](Self::line_box) one line height below the last.
    pub fn wrap(&self, text: &str, width: Pixels, window: &Window) -> Vec<Range<usize>> {
        let run = self.style.to_run(text.len());
        let Ok(lines) = window.text_system().shape_text(
            text.to_string().into(),
            self.face.size,
            &[run],
            Some(width),
            None,
        ) else {
            return std::iter::once(0..text.len()).collect();
        };
        let mut ranges = Vec::new();
        // `shape_text` splits at newlines first; each piece wraps on its own.
        let mut offset = 0;
        for line in &lines {
            let layout = &line.unwrapped_layout;
            let mut start = 0;
            for boundary in &line.wrap_boundaries {
                let next = layout.runs[boundary.run_ix].glyphs[boundary.glyph_ix].index;
                ranges.push(offset + start..offset + next);
                start = next;
            }
            ranges.push(offset + start..offset + line.text.len());
            offset += line.text.len() + 1;
        }
        ranges
    }

    /// `text` as a vector layer filling its [`line_box`](Self::line_box).
    pub fn layer(&self, text: &str, line_box: LineBox) -> Layer {
        Layer::text(
            text,
            &self.face,
            line_box.size,
            line_box.baseline,
            self.color,
        )
    }
}

/// The matrix that applies `pose` around `origin` (fractions of `size`) to a box of `size`,
/// for GPUI's centre-based [`Transformation`].
pub fn transformation(pose: &Pose, origin: Point<f32>, box_size: Size<Pixels>) -> Transformation {
    let w = f32::from(box_size.width);
    let h = f32::from(box_size.height);
    let offset = pose.offset(box_size);
    // GPUI scales and rotates about the box centre; move that about `origin` instead:
    // p' = M (p - o) + o + t = M (p - c) + c + (o - c) - M (o - c) + t.
    let (ox, oy) = ((origin.x - 0.5) * w, (origin.y - 0.5) * h);
    let (sin, cos) = pose.rotate.to_radians().sin_cos();
    let (mx, my) = (pose.sx * ox, pose.sy * oy);
    let rotated = (cos * mx - sin * my, sin * mx + cos * my);
    let shift = point(offset.x + px(ox - rotated.0), offset.y + px(oy - rotated.1));
    Transformation::scale(size(pose.sx, pose.sy))
        .with_rotation(radians(pose.rotate.to_radians()))
        .with_translation(shift)
}

/// A box of single-colour layers under one transform.
///
/// It takes `size` in the layout and paints its layers transformed: the box itself never moves,
/// so the layout around it doesn't either.
#[derive(IntoElement)]
pub struct Vector {
    size: Size<Pixels>,
    layers: Vec<Layer>,
    pose: Pose,
    origin: Point<f32>,
}

impl Vector {
    pub fn new(size: Size<Pixels>) -> Self {
        Self {
            size,
            layers: Vec::new(),
            pose: Pose::IDENTITY,
            origin: point(0.5, 0.5),
        }
    }

    pub fn layer(mut self, layer: Layer) -> Self {
        self.layers.push(layer);
        self
    }

    pub fn layers(mut self, layers: impl IntoIterator<Item = Layer>) -> Self {
        self.layers.extend(layers);
        self
    }

    pub fn pose(mut self, pose: Pose) -> Self {
        self.pose = pose;
        self
    }

    /// The point that stays put while scaling and rotating, as fractions of the box, like
    /// `transform-origin`.
    pub fn origin(mut self, x: f32, y: f32) -> Self {
        self.origin = point(x, y);
        self
    }
}

impl RenderOnce for Vector {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let transformation = transformation(&self.pose, self.origin, self.size);
        let alpha = self.pose.alpha();
        div()
            .relative()
            .flex_none()
            .w(self.size.width)
            .h(self.size.height)
            .children(self.layers.into_iter().map(move |layer| {
                let color = layer.color;
                svg()
                    .data(layer.markup.as_bytes())
                    .absolute()
                    .inset_0()
                    .size_full()
                    .text_color(color.opacity(alpha))
                    .with_transformation(transformation)
            }))
            .when(alpha <= 0.0, |this| this.invisible())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_font_maps_to_a_real_family() {
        let family = svg_family(".SystemUIFont");
        assert!(!family.contains(".SystemUIFont"));
        assert!(family.ends_with("sans-serif"));
        assert!(svg_family("M PLUS 1").starts_with("'M PLUS 1'"));
    }

    #[test]
    fn text_is_escaped() {
        let face = TextFace {
            family: "Inter".into(),
            weight: FontWeight::BOLD,
            style: FontStyle::Normal,
            size: px(20.0),
        };
        let layer = Layer::text(
            "<&>",
            &face,
            size(px(20.), px(30.)),
            px(22.),
            gpui_kit::black(),
        );
        assert!(layer.markup.contains("&lt;&amp;&gt;"));
        assert!(layer.markup.contains(r#"font-weight="700""#));
    }

    #[test]
    fn identity_pose_is_identity_matrix() {
        let t = transformation(&Pose::IDENTITY, point(0.5, 1.0), size(px(10.), px(20.)));
        assert_eq!(t, Transformation::default());
    }

    #[test]
    fn scaling_about_the_bottom_keeps_the_bottom_still() {
        // Scaling by 2 about the bottom centre moves the box centre up by half the height.
        let t = transformation(
            &Pose::new().scale(2.0),
            point(0.5, 1.0),
            size(px(10.), px(20.)),
        );
        let expected =
            Transformation::scale(size(2.0, 2.0)).with_translation(point(px(0.), px(-10.)));
        assert_eq!(t, expected);
    }

    /// Lines break where GPUI breaks them, and together they are the whole text.
    #[gpui_kit::test]
    fn wrapped_lines_cover_the_text(cx: &mut gpui_kit::TestAppContext) {
        let (_, cx) = cx.add_window_view(|_, _| gpui_kit::Empty);
        cx.update(|window, _| {
            let text = "Pop, anime-title motion for shadcn/ui and GPUI.\nTwo paragraphs.";
            let setting = TextSetting::new(&gpui_kit::TextStyleRefinement::default(), window);
            let narrow = setting.wrap(text, px(80.), window);
            assert!(narrow.len() > 3, "{narrow:?}");
            let joined: String = narrow.iter().map(|range| &text[range.clone()]).collect();
            assert_eq!(joined, text.replace('\n', ""));
            let wide = setting.wrap(text, px(10_000.), window);
            assert_eq!(wide.len(), 2, "a newline still breaks: {wide:?}");
        });
    }
}
