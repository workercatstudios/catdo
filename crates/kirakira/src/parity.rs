//! Keyframe parity with the web components (tests only).
//!
//! The web components in `apps/kirakira` are the source of truth for every keyframe. These helpers
//! read a component's CSS `@keyframes` straight out of its `.tsx` file and check that a ported
//! [`Track`](crate::motion::Track) lands on the same values at every keyframe, so a port can't
//! drift from the web source:
//!
//! ```ignore
//! #[test]
//! fn rise_matches_the_web_keyframes() {
//!     crate::parity::assert_pose_track("bounce-text", "kk-bounce-text-rise", &rise_track(), &[]);
//! }
//! ```
//!
//! Outside the monorepo the web sources aren't there, and the checks pass with a note.

use std::path::PathBuf;

use crate::motion::{Keyframes, Pose};

/// One CSS keyframe: its offset and the properties it sets. `None` means not set here.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CssFrame {
    pub offset: f32,
    /// Translation in px.
    pub x: Option<f32>,
    pub y: Option<f32>,
    /// Translation as a fraction of the element's own size (`translateY(100%)` is 1.0).
    pub xp: Option<f32>,
    pub yp: Option<f32>,
    pub sx: Option<f32>,
    pub sy: Option<f32>,
    /// Degrees.
    pub rotate: Option<f32>,
    pub opacity: Option<f32>,
}

/// The web source of `component` (`"bounce-text"`), if the monorepo is here.
pub fn source(component: &str) -> Option<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/kirakira/src/registry/kirakira/ui")
        .join(format!("{component}.tsx"));
    std::fs::read_to_string(path).ok()
}

/// The frames of `@keyframes name` in `css`, one per offset, in order.
///
/// # Panics
///
/// When the rule isn't there, or a value can't be read: the test should say so.
pub fn keyframes(css: &str, name: &str) -> Vec<CssFrame> {
    let head = format!("@keyframes {name}");
    let start = css
        .match_indices(&head)
        .find(|(index, _)| {
            let rest = &css[index + head.len()..];
            rest.trim_start().starts_with('{')
        })
        .map(|(index, _)| index)
        .unwrap_or_else(|| panic!("no @keyframes {name}"));
    let body_start = start + css[start..].find('{').expect("rule body") + 1;
    let mut depth = 1;
    let mut end = body_start;
    for (index, c) in css[body_start..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = body_start + index;
                    break;
                }
            }
            _ => {}
        }
    }
    let body = &css[body_start..end];

    let mut frames: Vec<CssFrame> = Vec::new();
    let mut rest = body;
    while let Some(open) = rest.find('{') {
        let selectors = rest[..open].trim();
        let close = rest[open..].find('}').expect("keyframe block closes") + open;
        let declarations = &rest[open + 1..close];
        rest = &rest[close + 1..];
        for selector in selectors.split(',') {
            let offset = match selector.trim() {
                "from" => 0.0,
                "to" => 1.0,
                percent => {
                    percent
                        .trim_end_matches('%')
                        .trim()
                        .parse::<f32>()
                        .unwrap_or_else(|_| panic!("keyframe selector {percent:?}"))
                        / 100.0
                }
            };
            let frame = match frames.iter_mut().find(|frame| frame.offset == offset) {
                Some(frame) => frame,
                None => {
                    frames.push(CssFrame {
                        offset,
                        ..Default::default()
                    });
                    frames.last_mut().expect("just pushed")
                }
            };
            for declaration in declarations.split(';') {
                if let Some((property, value)) = declaration.split_once(':') {
                    apply(frame, property.trim(), value.trim());
                }
            }
        }
    }
    frames.sort_by(|a, b| a.offset.total_cmp(&b.offset));
    frames
}

fn number(value: &str) -> f32 {
    let value = value.trim();
    let digits = value.trim_end_matches(|c: char| c.is_ascii_alphabetic() || c == '%');
    digits
        .parse()
        .unwrap_or_else(|_| panic!("CSS number {value:?}"))
}

/// A length: px, or a percentage as a fraction.
enum Length {
    Px(f32),
    Fraction(f32),
}

fn length(value: &str) -> Length {
    let value = value.trim();
    if value.ends_with('%') {
        Length::Fraction(number(value) / 100.0)
    } else {
        Length::Px(number(value))
    }
}

fn set_x(frame: &mut CssFrame, value: &str) {
    match length(value) {
        Length::Px(x) => {
            frame.x = Some(x);
            frame.xp.get_or_insert(0.0);
        }
        Length::Fraction(xp) => {
            frame.xp = Some(xp);
            frame.x.get_or_insert(0.0);
        }
    }
}

fn set_y(frame: &mut CssFrame, value: &str) {
    match length(value) {
        Length::Px(y) => {
            frame.y = Some(y);
            frame.yp.get_or_insert(0.0);
        }
        Length::Fraction(yp) => {
            frame.yp = Some(yp);
            frame.y.get_or_insert(0.0);
        }
    }
}

fn set_scale(frame: &mut CssFrame, args: &str) {
    let parts: Vec<&str> = args
        .split([',', ' '])
        .filter(|part| !part.is_empty())
        .collect();
    let sx = number(parts[0]);
    let sy = parts.get(1).map_or(sx, |sy| number(sy));
    frame.sx = Some(sx);
    frame.sy = Some(sy);
}

fn apply(frame: &mut CssFrame, property: &str, value: &str) {
    match property {
        "opacity" => frame.opacity = Some(number(value)),
        "scale" if value == "none" => set_scale(frame, "1"),
        "scale" => set_scale(frame, value),
        "rotate" if value == "none" => frame.rotate = Some(0.0),
        "rotate" => frame.rotate = Some(number(value)),
        "translate" if value == "none" => {
            set_x(frame, "0");
            set_y(frame, "0");
        }
        "translate" => {
            let mut parts = value.split_whitespace();
            set_x(frame, parts.next().unwrap_or("0"));
            set_y(frame, parts.next().unwrap_or("0"));
        }
        "transform" => {
            // `transform` resets every function it doesn't list. The individual `scale` and
            // `rotate` properties are separate from it, so a value one of them already set in
            // this frame stands (`scale: 0.6; transform: translateY(4%)` scales by 0.6).
            frame.x = Some(0.0);
            frame.y = Some(0.0);
            frame.xp = Some(0.0);
            frame.yp = Some(0.0);
            frame.sx.get_or_insert(1.0);
            frame.sy.get_or_insert(1.0);
            frame.rotate.get_or_insert(0.0);
            if value == "none" {
                return;
            }
            let mut rest = value;
            while let Some(open) = rest.find('(') {
                let function = rest[..open].trim();
                let close = rest[open..].find(')').expect("function closes") + open;
                let args = &rest[open + 1..close];
                rest = &rest[close + 1..];
                match function {
                    "translateX" => set_x(frame, args),
                    "translateY" => set_y(frame, args),
                    "translate" => {
                        let mut parts = args.split(',');
                        set_x(frame, parts.next().unwrap_or("0"));
                        set_y(frame, parts.next().unwrap_or("0"));
                    }
                    "scale" => set_scale(frame, args),
                    "scaleX" => frame.sx = Some(number(args)),
                    "scaleY" => frame.sy = Some(number(args)),
                    "rotate" => frame.rotate = Some(number(args)),
                    other => {
                        panic!("transform function {other} isn't supported by the parity check")
                    }
                }
            }
        }
        // Timing functions, colours and the rest aren't keyframed values a Pose holds.
        _ => {}
    }
}

/// Checks `track` against `@keyframes name` in `component`'s web source: at every keyframe offset,
/// every transform property the CSS sets must match within 1e-3. Offsets in `skip` aren't checked
/// (for a frame the port deliberately differs on; say why at the call).
pub fn assert_pose_track(component: &str, name: &str, track: &Keyframes<Pose>, skip: &[f32]) {
    let Some(css) = source(component) else {
        eprintln!("parity: {component}.tsx not found; skipping {name}");
        return;
    };
    let frames = keyframes(&css, name);
    assert!(!frames.is_empty(), "{name} has no frames");
    for frame in frames.iter().filter(|frame| !skip.contains(&frame.offset)) {
        let pose = track.sample(frame.offset);
        let checks = [
            ("x", frame.x, pose.x),
            ("y", frame.y, pose.y),
            ("xp", frame.xp, pose.xp),
            ("yp", frame.yp, pose.yp),
            ("sx", frame.sx, pose.sx),
            ("sy", frame.sy, pose.sy),
            ("rotate", frame.rotate, pose.rotate),
        ];
        for (property, expected, actual) in checks {
            if let Some(expected) = expected {
                assert!(
                    (expected - actual).abs() < 1e-3,
                    "{name} at {}%: {property} is {actual}, the web has {expected}",
                    frame.offset * 100.0
                );
            }
        }
    }
}

/// Checks the opacity a pose track carries against `@keyframes name`, at every keyframe that sets
/// one.
pub fn assert_pose_opacity(component: &str, name: &str, track: &Keyframes<Pose>) {
    let Some(css) = source(component) else {
        eprintln!("parity: {component}.tsx not found; skipping {name}");
        return;
    };
    let mut checked = 0;
    for frame in keyframes(&css, name) {
        if let Some(expected) = frame.opacity {
            let actual = track.sample(frame.offset).opacity;
            assert!(
                (expected - actual).abs() < 1e-3,
                "{name} at {}%: opacity is {actual}, the web has {expected}",
                frame.offset * 100.0
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "{name} never sets opacity");
}

/// Checks a one-number track (opacity, or a single scale) against one property of
/// `@keyframes name`: `"opacity"`, `"sx"`, `"sy"`, `"rotate"`, `"x"`, `"y"`, `"xp"` or `"yp"`.
pub fn assert_number_track(component: &str, name: &str, property: &str, track: &Keyframes<f32>) {
    let Some(css) = source(component) else {
        eprintln!("parity: {component}.tsx not found; skipping {name}");
        return;
    };
    let frames = keyframes(&css, name);
    let mut checked = 0;
    for frame in &frames {
        let expected = match property {
            "opacity" => frame.opacity,
            "sx" => frame.sx,
            "sy" => frame.sy,
            "rotate" => frame.rotate,
            "x" => frame.x,
            "y" => frame.y,
            "xp" => frame.xp,
            "yp" => frame.yp,
            other => panic!("unknown property {other}"),
        };
        if let Some(expected) = expected {
            let actual = track.sample(frame.offset);
            assert!(
                (expected - actual).abs() < 1e-3,
                "{name} at {}%: {property} is {actual}, the web has {expected}",
                frame.offset * 100.0
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "{name} never sets {property}");
}

#[cfg(test)]
mod tests {
    use super::*;

    const CSS: &str = r#"
@keyframes kk-demo {
  from { opacity: 0; }
  20%, to { opacity: 1; }
  from { transform: translateY(-120%) scale(0.9, 1.15); animation-timing-function: ease-in; }
  45% { transform: translateY(0) scale(1.18, 0.82); }
  to { transform: none; }
}
@keyframes kk-press {
  to { scale: 0.85; }
}
"#;

    #[test]
    fn reads_frames_and_merges_repeated_selectors() {
        let frames = keyframes(CSS, "kk-demo");
        let offsets: Vec<f32> = frames.iter().map(|frame| frame.offset).collect();
        assert_eq!(offsets, [0.0, 0.2, 0.45, 1.0]);
        let first = &frames[0];
        assert_eq!(first.opacity, Some(0.0));
        assert_eq!(first.yp, Some(-1.2));
        assert_eq!(first.sx, Some(0.9));
        assert_eq!(first.sy, Some(1.15));
        assert_eq!(frames[1].opacity, Some(1.0));
        assert_eq!(frames[1].sx, None);
        assert_eq!(frames[3].sx, Some(1.0));
        assert_eq!(frames[3].opacity, Some(1.0));
    }

    #[test]
    fn transform_keeps_an_individual_scale() {
        let css = "@keyframes kk-pop { 0% { scale: 0.6; transform: translateY(4%); } 50% { scale: 1.04 1.06; transform: none; } }";
        let frames = keyframes(css, "kk-pop");
        assert_eq!((frames[0].sx, frames[0].yp), (Some(0.6), Some(0.04)));
        assert_eq!((frames[1].sx, frames[1].sy), (Some(1.04), Some(1.06)));
    }

    #[test]
    fn reads_individual_transform_properties() {
        let frames = keyframes(CSS, "kk-press");
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].sx, Some(0.85));
        assert_eq!(frames[0].sy, Some(0.85));
    }
}
