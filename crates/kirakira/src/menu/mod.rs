//! Menus: dropdown, context menu and the app menu bar, with rows that drop into place.
//!
//! Replaces `gpui_kit::component::menu`: `PopupMenu`, `PopupMenuItem`, `DropdownMenu`,
//! `ContextMenu`, `ContextMenuExt`, `ContextMenuState` and `AppMenuBar`, with the same builders.
//! `use kirakira::menu::*` is a drop-in for `use gpui_kit::component::menu::*`.
//!
//! Every menu pops out of what opened it like [`Popover`](crate::popover): `scale` 0.7 → 1.04 →
//! 0.985 → 1 over 0.3 s from the trigger-facing corner, sliding the last 0.5rem away from it.
//! Then the rows drop 0.4em into place one after another on Kirakira's `out` curve, 0.24 s each,
//! 35 ms apart, starting 50 ms in; rows past the twelfth share the last slot. Check marks pop in
//! (0 → 1.3 → 0.9 → 1 in 0.3 s) as the menu opens. Dismissing a menu shrinks it to 0.9 and
//! fades it in 0.12 s, sliding half way back. A dropdown opens below its trigger, a context menu
//! below and right of the pointer, a submenu from its parent's side, and the menu bar's menus
//! below the bar; sliding along the bar closes one menu while the next pops. Submenus close the
//! same way, back into their parent's side: when the pointer moves to another row, and together
//! with their parent when it closes.
//!
//! Under reduced motion menus fade in over 0.15 s and out over 0.1 s, the rows are there at once
//! and check marks don't move.
//!
//! # How it's built
//!
//! The rows move inside the menu, so wrapping GPUI Component's menu isn't enough: `PopupMenu`
//! is rebuilt from GPUI Component's source, item for item (keyboard navigation, submenus, key
//! binding hints, links, scrolling), with the motion added to the parts it now owns. The
//! surface goes through [`motion::transform`](crate::motion::transform); each row is offset and
//! faded directly, which is exact. Row heights and padding are in rems (1.625rem for GPUI
//! Component's 26px) so the pop scales them; at the default rem size they are the same.
//!
//! The containers are ported too, since they decide when a menu is shown: `DropdownMenu` on
//! gpui-base's `Popover`, `ContextMenu` and `AppMenuBar` as in GPUI Component. Each keeps a
//! dismissed menu painted where it was, without input, until its exit is done; a `PopupMenu`
//! does the same for a submenu it stops showing, one layer above itself.
//!
//! # Differences from the web version
//!
//! - The scale is even and comes from the rem size, so pixel sizes (the ring, the shadow, the
//!   separator line) don't scale.
//! - GPUI Component's menus have no checkbox or radio items that stay open, so the check mark
//!   only pops when a menu opens; it never animates out.
//! - GPUI Component's own widgets that build menus for you (a table's or an input's context menu,
//!   the title bar's menu bar) take GPUI Component's `PopupMenu` and keep its motion.

mod app_menu_bar;
mod context_menu;
mod dropdown_menu;
mod menu_item;
mod popup_menu;

pub use app_menu_bar::AppMenuBar;
pub use context_menu::{ContextMenu, ContextMenuExt, ContextMenuState};
pub use dropdown_menu::{DropdownMenu, DropdownMenuPopover};
pub use popup_menu::{PopupMenu, PopupMenuItem};
