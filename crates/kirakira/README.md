# Kirakira UI for GPUI (vendored)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://shieldcn.dev/badge/License-MIT-grey.svg?variant=outline&amp;size=sm&amp;logo=false&amp;mode=dark">
  <img alt="License: MIT" src="https://shieldcn.dev/badge/License-MIT-grey.svg?variant=outline&amp;size=sm&amp;logo=false&amp;mode=light">
</picture>

[Kirakira UI](https://kk.workercat.com) for GPUI Kit, the design system CatDo's desktop app is
drawn with: the Kirakira theme (warm paper, cocoa ink, a pink primary, a navy dark mode), the base
kit of drop-in replacements for GPUI Component's controls, and the motion pieces CatDo uses.

CatDo owns this copy, shadcn-style. It comes from `crates/kirakira-rs` in the workercat monorepo
at commit `66a88d9a8f32106abf2429ce01bee87cedabc4e0`. Module paths are unchanged, so a sync is a
copy of the kept files.

## Trimmed for CatDo

- Kept: the theme, the motion core, the whole base kit, and `bounce_text`, `burst`, `hop`,
  `idle_cat`, `reveal` and `sparkles` from the motion kit.
- Left out: the gallery, render and glyph-check examples, the scripts, and the other characters
  and motion pieces.
- `lib.rs` only drops those modules and marks `character` and `draw` `allow(dead_code)`, as some
  of their helpers served the pieces left out.
- The crate's tests don't run here (`test = false`): most check keyframes against the Kirakira
  web sources in the monorepo (`parity.rs`), and they run there. `cargo clippy --all-targets`
  still type-checks them, so `parity.rs` stays.

MIT licensed; see [LICENSE](LICENSE).
