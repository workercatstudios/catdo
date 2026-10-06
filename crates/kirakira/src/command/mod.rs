//! Command: a palette whose highlight springs from row to row, whose list eases to the height of
//! its results, and whose empty message pops in.
//!
//! Replaces `gpui_kit::component::command`, rebuilt on GPUI Component's source: the same
//! `Command`, `CommandState`, `CommandItem`, `CommandGroup` and `CommandEntry`, with the same
//! builders, query field, filtering, groups, separators, keybinding hints, keyboard navigation and
//! callbacks. It runs in GPUI Component's `Command` key context, so the keys GPUI Component binds
//! there (Escape, Enter, Up, Down) drive it too. Put it in a [`crate::dialog::Dialog`] for Pop
//! Command's `CommandDialog`: the dialog pops in and squashes out with the same keyframes.
//!
//! The motion is Pop Command's:
//!
//! - The highlight is one plate behind the rows, not each row's own background. When the selected
//!   row changes, by arrow, hover or typing, it springs there: its top and height move over
//!   0.32 s on Kirakira's spring curve, `cubic-bezier(0.34, 1.56, 0.64, 1)`. It appears in place,
//!   fading in over 0.15 s (ease-out) while it grows from 0.9 to full size (0.2 s, spring), and it
//!   shrinks and fades where it is when nothing matches.
//! - The list's height follows its results over 0.2 s on Kirakira's out curve,
//!   `cubic-bezier(0.05, 0.3, 0.1, 1)`.
//! - The empty message pops in: scale 0.85 → (1.05, 1.08) at 55 % → 0.98 at 80 % → 1 in 0.3 s,
//!   ease-in-out, opaque by 40 %.
//! - Reduced motion: the highlight jumps, the list resizes at once and the empty message fades in
//!   (0.15 s). The highlight still fades in and out.
//!
//! Where it differs from the web version, and why:
//!
//! - The empty message scales evenly ([`motion::transform`](crate::motion::transform)), so its
//!   (1.05, 1.08) overshoot is their geometric mean, 1.065.
//! - The placeholder and empty message are GPUI Component's English strings: its translations are
//!   private to it.
//! - The highlight is drawn in the list's own coordinates and follows its scrolling, like the
//!   web's `::before`; while the list scrolls under a moving highlight, it slides from where it
//!   was on screen.

#[allow(clippy::module_inception)]
mod command;
mod item;
mod motion;
mod state;

pub use command::Command;
pub use item::{CommandEntry, CommandGroup, CommandItem};
pub use state::CommandState;
