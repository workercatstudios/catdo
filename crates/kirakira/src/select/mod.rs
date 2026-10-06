//! Select: a list that squash-pops out of its trigger, rows that drop in one by one, a check that
//! pops, and a chevron that flips past half a turn.
//!
//! Replaces `gpui_kit::component::select`, rebuilt from GPUI Component's source: the same
//! `Select`, `SelectState` and `SelectEvent`, with the same builders, delegates (`SelectDelegate`,
//! `SearchableVec`, `SelectGroup`, `SelectItem`), search, keyboard navigation, groups, disabled
//! rows, sizes, clear button and empty state. The other items of that module (`Caret`,
//! `SelectListItem`) are re-exported, so `use kirakira::select::*` is a drop-in for
//! `use gpui_kit::component::select::*`. It runs on GPUI Component's `List` and gpui-base's
//! `Select` root, as GPUI Component's does; only the rows are drawn by Kirakira.
//!
//! The motion is Pop Select's:
//!
//! - The list pops from the middle of its trigger-facing edge: scale (0.95, 0.6) → (1.02, 1.05) at
//!   55 % → (0.995, 0.98) at 80 % → 1 in 0.3 s, ease-in-out, opaque by 30 %. Closing shrinks it
//!   to (0.96, 0.9) and fades it out in 0.15 s on Kirakira's `in` curve; it stays painted, without
//!   input, until then.
//! - Rows and group labels drop 0.375rem into place, overshooting 1/16 rem at 60 %, in 0.26 s
//!   (ease-out), fading in by 60 %. Each starts 40 ms after the list opens plus 25 ms per step: a
//!   row's step is its group's place in the list plus its own place in the group (a group's label
//!   first), capped at nine, as on the web.
//! - The selected row's check pops 0 → 1.3 → 0.9 → 1 in 0.3 s (ease-in-out), 200 ms plus 25 ms
//!   per step in, once its row has landed.
//! - The chevron flips past half a turn and settles, 0 → 198° → 174° → 180° in 0.36 s
//!   (ease-in-out); closing turns it back in 0.15 s (ease-out), from wherever it was.
//! - Reduced motion: the list fades in over 0.15 s and out over 0.1 s; nothing scales, drops or
//!   turns.
//!
//! Where it differs from the web version, and why:
//!
//! - GPUI can't scale a subtree on two axes; [`motion::transform`](crate::motion::transform)
//!   scales evenly, by the rem size, which would make the squash their geometric mean (0.75 at
//!   the start, where the web is 0.95 × 0.6). So the panel's plate (its background, ring and
//!   shadow) is drawn as an owned shape at the exact two-axis scale, and the list inside scales
//!   evenly with the plate's width and is clipped to the plate. The silhouette matches the web;
//!   the rows aren't squashed vertically, the plate unfolds over them instead. Pixel sizes (the
//!   ring, the shadow, the corner radius) don't scale.
//! - A row is clipped to its own slot in GPUI Component's list, so while it starts its drop,
//!   nearly transparent, the top of a row that's still above its place is cut off.
//! - The list opens below its trigger (above when there's no room) at the trigger's width, like
//!   the web's `alignItemWithTrigger={false}`, not over the selected row like Base UI's
//!   default.
//! - The list keeps GPUI Component's look (its check is the foreground colour and 12 px, a row is
//!   the theme's radius) and its English placeholder: GPUI Component's translations are private
//!   to it.
//! - A searchable select clears its query as it closes, as GPUI Component's does, so a filtered
//!   list shows every row again while it fades out. The web select has no search.
//! - Pop Select isn't composition-safe: it's a control, driven by its own state.

mod adapter;
mod motion;
mod panel;
mod state;

pub use gpui_kit::component::select::{
    Caret, SearchableVec, SelectDelegate, SelectGroup, SelectItem, SelectListItem,
};
pub use state::{Select, SelectEvent, SelectState};
