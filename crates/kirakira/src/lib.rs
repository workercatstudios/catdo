//! Kirakira UI for GPUI: pop, anime-title motion components on GPUI Kit.
//!
//! A port of [Kirakira UI](https://kk.workercat.com), the shadcn/ui motion registry, to
//! [GPUI Kit](https://gpui-kit.com). Kirakira stands to GPUI Component the way the web kit
//! stands to shadcn: a styled layer over the same headless primitives (`gpui-base` here, Base
//! UI there), with motion added.
//!
//! - **The base kit** replaces GPUI Component's controls one for one. Each module has the same
//!   path and builder API as its GPUI Component counterpart, so switching is an import change:
//!   `use kirakira::button::Button` for `use gpui_kit::component::button::Button`.
//! - **The motion kit** adds text entrances, plate wipes, bursts and sparkles, squishy controls,
//!   loaders and scene pieces.
//! - **[`timeline`]** drives a whole composition from one clock, frame by frame.
//! - **[`theme`]** installs the Kirakira palette into GPUI Component's theme and keeps the
//!   `--kk-*` colours and curves.
//!
//! Call [`init`] after `gpui_kit::init`.

// Trimmed in CatDo: the rim helpers serve characters CatDo does not vendor.
#[allow(dead_code)]
mod character;
mod glide;
pub mod icons;
pub mod motion;
mod overlay;
#[cfg(test)]
mod parity;
pub mod root;
pub mod shapes;
pub mod text;
pub mod theme;
pub mod timeline;
pub mod vector;

mod painted;
mod state_motion;

// The base kit: drop-in replacements for GPUI Component's modules.
pub mod accordion;
pub mod alert;
pub mod avatar;
pub mod badge;
pub mod breadcrumb;
pub mod button;
pub mod calendar;
pub mod card;
pub mod carousel;
pub mod checkbox;
pub mod collapsible;
pub mod command;
pub mod dialog;
pub mod hover_card;
pub mod input;
pub mod label;
pub mod menu;
pub mod navigation_menu;
pub mod notification;
pub mod pagination;
pub mod popover;
pub mod progress;
pub mod radio;
pub mod scroll;
pub mod select;

mod button_group;
mod button_toggle;
pub mod separator;
pub mod sheet;
pub mod skeleton;
pub mod slider;
pub mod spinner;
pub mod switch;
pub mod tab;
pub mod table;
pub mod tooltip;

mod squash;

// The motion kit.
pub mod bounce_text;
pub mod burst;
pub mod hop;
pub mod idle_cat;
pub mod reveal;
pub mod sparkles;

// Trimmed in CatDo: some drawing helpers serve motion pieces CatDo does not vendor.
#[allow(dead_code)]
mod draw;

use gpui_kit::App;
use gpui_kit::component::ThemeMode;

pub use root::{Root, WindowExt};
pub use theme::{ActiveKira, Kira, Palette};

/// Installs the Kirakira tokens and switches GPUI Component to the Kirakira light theme.
///
/// Call it after `gpui_kit::init(cx)`. Use [`init_tokens`] instead to keep your own GPUI Component
/// theme and only add Kirakira's decorative colours and curves.
pub fn init(cx: &mut App) {
    init_tokens(cx);
    theme::apply(ThemeMode::Light, None, cx);
}

/// Installs the Kirakira tokens without touching GPUI Component's theme.
pub fn init_tokens(cx: &mut App) {
    cx.set_global(Kira::default());
}
