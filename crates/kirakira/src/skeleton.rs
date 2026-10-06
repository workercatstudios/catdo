//! Skeleton: a placeholder with a slanted glint that sweeps across it.
//!
//! Replaces `gpui_kit::component::skeleton`, rebuilt with GPUI Component's size and colours, so
//! `use kirakira::skeleton::*` is a drop-in.
//!
//! A 110° glint (transparent at 30 %, the glint colour at 50 %, transparent at 70 % of its band)
//! crosses the placeholder in the first 0.9 s of a 1.6 s loop, on `ease-in-out`, then rests for
//! 0.7 s, so the sweeps read as glints rather than a pulse. The glint is white at 70 % in light
//! themes and 9 % in dark ones. It replaces GPUI Component's opacity pulse. Under reduced motion
//! the placeholder is plain and still.
//!
//! Composition-safe: the loop runs on a [`Clock`], so inside a
//! [`Timeline`](crate::timeline::Timeline) it follows the timeline's time. Every skeleton under the
//! same parent shares one clock, so their glints cross in step.
//!
//! How it's drawn: GPUI gradients have two stops and no transforms, so the glint is two gradient
//! quads with the placeholder's own bounds and corner radii (so it stays inside rounded and round
//! placeholders): one ramps the glint in, the next paints the placeholder colour back over it.
//! Their angle and stops are worked out per frame so the band has the web version's 110° slant
//! and sweep. `secondary()` gets an opaque colour (the half-strength skeleton over the theme
//! background) for that second quad to restore. A solid `.bg(...)` of your own is what it restores
//! instead; give it an opaque colour, since a see-through one shows the glint's tail through the
//! restoring quad. Over a gradient there is nothing to restore with, so a skeleton with a gradient
//! background doesn't glint.

use gpui_kit::base::StyledExt as _;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::{
    AbsoluteLength, App, BorderStyle, Bounds, Corners, Edges, Fill, Hsla, IntoElement,
    ParentElement as _, Pixels, RenderOnce, StyleRefinement, Styled, Window, canvas, div, hsla,
    linear_color_stop, linear_gradient, quad,
};

use crate::motion::{Clock, Easing, IterationCount, Keyframes, Timing, Track, Trigger, ms};

/// A skeleton loading placeholder element.
#[derive(IntoElement)]
pub struct Skeleton {
    style: StyleRefinement,
    secondary: bool,
}

impl Default for Skeleton {
    fn default() -> Self {
        Self::new()
    }
}

impl Skeleton {
    /// Create a new Skeleton element.
    pub fn new() -> Self {
        Self {
            style: StyleRefinement::default(),
            secondary: false,
        }
    }

    /// Set use secondary color.
    pub fn secondary(mut self) -> Self {
        self.secondary = true;
        self
    }
}

impl Styled for Skeleton {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

/// `kk-pop-skeleton-glint`: the glint's offset as a fraction of the placeholder's width.
pub(crate) fn sweep_track() -> Keyframes<f32> {
    Track::new(Easing::EaseInOut)
        .at(0.0, -1.0)
        .at(0.56, 1.0)
        .at(1.0, 1.0)
        .build()
}

/// The CSS gradient direction of the glint, 110°: right and a little down.
const GLINT_DIR: (f32, f32) = (0.939_692_6, 0.342_020_14);

/// How to draw a CSS `linear-gradient(110deg, ...)` band with GPUI's two-stop gradients on a
/// `width × height` box whose band is shifted right by `shift × width`.
///
/// Returns the GPUI gradient angle in degrees and a function from a CSS stop position (`0..=1`
/// along the CSS gradient line of the shifted box) to the GPUI stop position. GPUI measures its
/// gradient along a direction squashed to the box's aspect ratio and normalised by the box's
/// width, so both need converting.
pub(crate) fn glint_geometry(width: f32, height: f32, shift: f32) -> (f32, impl Fn(f32) -> f32) {
    let (ux, uy) = GLINT_DIR;
    let width = width.max(1.0);
    let height = height.max(1.0);
    // The CSS gradient line's length for this box.
    let line = width * ux + height * uy;
    let angle = 90.0 + (uy / ux * width / height).atan().to_degrees();
    let stop = move |s: f32| ((s - 0.5) * line + shift * width * ux + width * 0.5) / width;
    (angle, stop)
}

fn corners(style: &StyleRefinement, rem: Pixels) -> Corners<Pixels> {
    let r = &style.corner_radii;
    let px = |length: Option<AbsoluteLength>| length.unwrap_or_default().to_pixels(rem);
    Corners {
        top_left: px(r.top_left),
        top_right: px(r.top_right),
        bottom_right: px(r.bottom_right),
        bottom_left: px(r.bottom_left),
    }
}

/// The solid colour a skeleton styled with `style` is filled with, over a default fill of `base`;
/// `None` when its background isn't a solid colour.
fn restore_color(style: &StyleRefinement, base: Hsla) -> Option<Hsla> {
    match &style.background {
        None => Some(base),
        Some(Fill::Color(background)) => background.as_solid(),
    }
}

fn paint_glint(
    bounds: Bounds<Pixels>,
    radii: Corners<Pixels>,
    shift: f32,
    base: Hsla,
    glint: Hsla,
    window: &mut Window,
) {
    let radii = radii.clamp_radii_for_quad_size(bounds.size);
    let (angle, stop) = glint_geometry(
        bounds.size.width.as_f32(),
        bounds.size.height.as_f32(),
        shift,
    );
    let paint = |from, to, window: &mut Window| {
        window.paint_quad(quad(
            bounds,
            radii,
            linear_gradient(angle, from, to),
            Edges::default(),
            gpui_kit::transparent_black(),
            BorderStyle::default(),
        ));
    };
    paint(
        linear_color_stop(glint.alpha(0.0), stop(0.3)),
        linear_color_stop(glint, stop(0.5)),
        window,
    );
    paint(
        linear_color_stop(base.alpha(0.0), stop(0.5)),
        linear_color_stop(base, stop(0.7)),
        window,
    );
}

impl RenderOnce for Skeleton {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let clock = Clock::new("kk-skeleton", Trigger::Mount, window, cx);
        let theme = cx.theme();
        let base = if self.secondary {
            theme.background.blend(theme.skeleton.opacity(0.5))
        } else {
            theme.skeleton
        };
        let glint = if theme.is_dark() {
            hsla(0., 0., 1., 0.09)
        } else {
            hsla(0., 0., 1., 0.7)
        };
        let shift = (!clock.reduced()).then(|| {
            clock.animate(None, window);
            let timing = Timing::new(ms(1600)).iterations(IterationCount::Infinite);
            clock.sample(&sweep_track(), &timing)
        });
        let radii = corners(&self.style, window.rem_size());
        // The colour the glint's tail restores: the background actually drawn.
        let restore = restore_color(&self.style, base);
        let shift = shift.zip(restore);

        div()
            // Before the caller's style, so `.absolute()` still wins.
            .relative()
            .w_full()
            .h_4()
            .bg(base)
            .refine_style(&self.style)
            .when_some(shift, |this, (shift, restore)| {
                this.child(
                    canvas(
                        |_, _, _| {},
                        move |bounds, _, window, _| {
                            paint_glint(bounds, radii, shift, restore, glint, window)
                        },
                    )
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full(),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyframes_match_the_web() {
        crate::parity::assert_number_track(
            "pop-skeleton",
            "kk-pop-skeleton-glint",
            "xp",
            &sweep_track(),
        );
    }

    #[test]
    fn sweep_crosses_in_the_first_56_percent_then_rests() {
        let sweep = sweep_track();
        assert_eq!(sweep.sample(0.0), -1.0);
        assert_eq!(sweep.sample(0.56), 1.0);
        assert_eq!(sweep.sample(0.8), 1.0);
    }

    #[test]
    fn glint_band_is_centred_when_unshifted() {
        let (angle, stop) = glint_geometry(200.0, 20.0, 0.0);
        // The CSS 50 % mark lands on GPUI's centre.
        assert!((stop(0.5) - 0.5).abs() < 1e-5);
        // A wide box needs a steeper GPUI angle to keep the 110° slant.
        assert!(angle > 110.0 && angle < 180.0);
        // Shifted a whole width right, the band starts past the right edge.
        let (_, shifted) = glint_geometry(200.0, 20.0, 1.0);
        assert!(shifted(0.3) > 1.0);
        let (_, back) = glint_geometry(200.0, 20.0, -1.0);
        assert!(back(0.7) < 0.0);
    }

    #[test]
    fn the_glint_restores_the_drawn_background() {
        let base = hsla(0.5, 0.2, 0.8, 1.0);
        let red = hsla(0.0, 1.0, 0.5, 1.0);
        assert_eq!(restore_color(&StyleRefinement::default(), base), Some(base));
        let solid = Skeleton::new().bg(red).style;
        assert_eq!(restore_color(&solid, base), Some(red));
        let gradient = Skeleton::new()
            .bg(linear_gradient(
                90.,
                linear_color_stop(red, 0.),
                linear_color_stop(base, 1.),
            ))
            .style;
        assert_eq!(restore_color(&gradient, base), None);
    }

    #[test]
    fn square_box_keeps_the_css_angle() {
        let (angle, _) = glint_geometry(40.0, 40.0, 0.0);
        assert!((angle - 110.0).abs() < 1e-3);
    }
}
