//! The few icons Kirakira draws itself, inline so a component never depends on the app's assets.
//!
//! Each is a monochrome SVG for [`icon`], which colours it with the text colour like Lucide's
//! `currentColor` icons. Paths follow Lucide's 24×24 grid and 2px rounded strokes.

use gpui_kit::{Styled as _, Svg, svg};

/// An inline SVG icon, `1em` square by default, coloured with the text colour.
pub fn icon(data: &'static str) -> Svg {
    svg().data(data.as_bytes()).size_4().flex_none()
}

macro_rules! stroke {
    ($($path:literal),+ $(,)?) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="black" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">"#,
            $(r#"<path d=""#, $path, r#""/>"#,)+
            "</svg>"
        )
    };
}

macro_rules! fill {
    ($view:literal, $($path:literal),+ $(,)?) => {
        concat!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox=""#, $view, r#"" fill="black">"#,
            $(r#"<path d=""#, $path, r#""/>"#,)+
            "</svg>"
        )
    };
}

pub const X: &str = stroke!("M18 6 6 18", "m6 6 12 12");
pub const CHECK: &str = stroke!("M20 6 9 17l-5-5");
pub const MINUS: &str = stroke!("M5 12h14");
pub const PLUS: &str = stroke!("M5 12h14", "M12 5v14");
pub const CHEVRON_DOWN: &str = stroke!("m6 9 6 6 6-6");
pub const CHEVRON_UP: &str = stroke!("m18 15-6-6-6 6");
pub const CHEVRON_LEFT: &str = stroke!("m15 18-6-6 6-6");
pub const CHEVRON_RIGHT: &str = stroke!("m9 18 6-6-6-6");
pub const CHEVRONS_UP_DOWN: &str = stroke!("m7 15 5 5 5-5", "m7 9 5-5 5 5");
pub const MORE_HORIZONTAL: &str = stroke!("M12 12h.01", "M19 12h.01", "M5 12h.01");
pub const SEARCH: &str = stroke!("m21 21-4.34-4.34", "M11 3a8 8 0 1 0 0 16 8 8 0 0 0 0-16z");
pub const COPY: &str = stroke!(
    "M8 8h12v12H8z",
    "M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"
);
pub const CIRCLE_ALERT: &str = stroke!(
    "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20z",
    "M12 8v4",
    "M12 16h.01"
);
pub const INFO: &str = stroke!(
    "M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20z",
    "M12 16v-4",
    "M12 8h.01"
);
pub const CIRCLE_CHECK: &str =
    stroke!("M12 22a10 10 0 1 0 0-20 10 10 0 0 0 0 20z", "m9 12 2 2 4-4");
pub const TRIANGLE_ALERT: &str = stroke!(
    "m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3",
    "M12 9v4",
    "M12 17h.01"
);
pub const CALENDAR: &str = stroke!(
    "M8 2v4",
    "M16 2v4",
    "M5 4h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2z",
    "M3 10h18"
);
pub const LOADER: &str = stroke!("M21 12a9 9 0 1 1-6.22-8.56");

/// Play and pause, filled, for timeline controls.
pub const PLAY: &str = fill!("0 0 16 16", "M4 2.2v11.6L13.5 8z");
pub const PAUSE: &str = fill!("0 0 16 16", "M3 2h3.5v12H3zM9.5 2H13v12H9.5z");

/// A four-point star whose sides curve in towards the centre: Kirakira's sparkle.
pub const SPARKLE: &str = fill!(
    "0 0 24 24",
    "M12 0Q13.4 10.6 24 12 13.4 13.4 12 24 10.6 13.4 0 12 10.6 10.6 12 0Z"
);

/// A filled heart.
pub const HEART: &str = fill!(
    "0 0 24 24",
    "M12 21.35 10.55 20C5.4 15.36 2 12.28 2 8.5 2 5.42 4.42 3 7.5 3c1.74 0 3.41.81 4.5 2.09C13.09 3.81 14.76 3 16.5 3 19.58 3 22 5.42 22 8.5c0 3.78-3.4 6.86-8.55 11.54z"
);

/// A heart outline.
pub const HEART_OUTLINE: &str = stroke!(
    "M19 14c1.49-1.46 3-3.21 3-5.5A5.5 5.5 0 0 0 16.5 3c-1.76 0-3 .5-4.5 2-1.5-1.5-2.74-2-4.5-2A5.5 5.5 0 0 0 2 8.5c0 2.3 1.5 4.05 3 5.5l7 7z"
);

/// A filled circle, for dots and confetti.
pub const DOT: &str = fill!("0 0 24 24", "M12 2a10 10 0 1 0 0 20 10 10 0 0 0 0-20z");
