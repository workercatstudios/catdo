# CatDo UI

CatDo is designed with [Kirakira UI](https://kk.workercat.com), WorkerCat's
pop design system. Kirakira was made for CatDo, so its design is authoritative:
warm paper and cocoa ink, a pink primary, a navy night mode, rounded shapes,
the M PLUS 1 typeface and quick, springy motion. CatDo adds meaning colours on
top. Each view has its own hue for its icon tile, every project gets a stable
colour dot from a hash of its id (eight slots shared by all clients), deadlines
are red when late and amber when due today, scheduled dates read blue, repeats
read teal, and a finished check fills pink. Metadata always pairs its colour
with an icon (flag, calendar, repeat, subtasks) so it still reads without
colour. Green is never an accent. Keep native keyboard, focus, touch and
accessibility behaviour. Text must keep 4.5:1 contrast on its surface.

## Where Kirakira comes from

- Web: Kirakira's shadcn registry components, copied unchanged into
  `apps/web/src/components/ui` (`pop-*.tsx` for the base kit, plus `burst`,
  `sparkles`, `idle-cat`, `bounce-text` and friends). They build on Base UI.
  Keep them byte-for-byte as the registry ships them so an update is a copy; put
  CatDo styling in `apps/web/src/styles`.
- Desktop: Kirakira for GPUI, vendored in `crates/kirakira` (see its README for
  the source commit and what was trimmed). `kirakira::theme::apply` installs the
  theme; controls come from `kirakira::<module>` instead of
  `gpui_kit::component::<module>`.
- Android: Kirakira has no Compose kit, so `apps/android` ports its tokens and
  motion by hand in the `ui` package.

## Colours

Kirakira theme tokens (the `kirakira-theme` registry item):

| Role               | Light     | Dark      |
| ------------------ | --------- | --------- |
| Background (paper) | `#FFFCF8` | `#1F2238` |
| Card / popover     | `#FFFFFF` | `#272B45` |
| Text (cocoa)       | `#4B3832` | `#F6ECE4` |
| Muted surface      | `#F7F0EA` | `#2A2E48` |
| Muted text         | `#7A655D` | `#B3A8B8` |
| Border / input     | `#F0E4DB` | `#383C5C` |
| Primary (pink)     | `#D6336F` | `#FF7AA5` |
| On primary         | `#FFFFFF` | `#1F2238` |
| Accent (blush)     | `#FDE6EE` | `#3B2F4A` |
| Destructive        | `#E5484D` | `#FF6B6F` |

Kirakira hues: pink `#EC5F8F`, yellow `#F7D35C`, orange `#F4A35F`, teal
`#4FB0AA`, sky `#5AA9E6`, lime `#B9CC5A`, lilac `#B79AD1`, navy `#2D3F63`.

CatDo's additions:

| Role                         | Light     | Dark      | Fill behind icons |
| ---------------------------- | --------- | --------- | ----------------- |
| Sidebar                      | `#FBF6F1` | `#1B1E33` |                   |
| Selected row                 | accent    | accent    |                   |
| Overdue deadline             | `#C2333A` | `#FF8A8D` | destructive       |
| Today / due today            | `#A35A12` | `#F7D35C` | yellow            |
| Inbox / scheduled            | `#2F6FB0` | `#8CC4F0` | sky               |
| Upcoming                     | `#7A52A3` | `#C9B0E3` | lilac             |
| Calendar / repeat            | `#2B7A75` | `#7FD0C9` | teal              |
| Completed / finished check   | primary   | primary   | pink              |

Text uses the left two columns; icon tiles and chips use the pastel fill at a
low strength behind them. A selected row is blush with cocoa text and a pink
icon, because pink text on blush falls under 4.5:1.

Project slots p0–p7, in order: pink, sky `#5AA9E6`, teal `#4FB0AA`, orange
`#F4A35F`, lilac `#B79AD1`, yellow (`#E0B531` light, `#F7D35C` dark), coral
(`#E8775F` light, `#FF9B85` dark) and navy (`#2D3F63` light, `#8FA6D6` dark).
Pink is `#EC5F8F` light and `#FF7AA5` dark.

Token definitions live in:

- Web: `apps/web/src/styles/foundation.css`, shared by the application and site.
- Desktop: `crates/kirakira/themes/kirakira.json` for the theme, plus
  `apps/desktop/src/theme.rs` for CatDo's additions.
- Android: `apps/android/app/src/main/java/com/workercat/catdo/ui/Theme.kt`,
  mapped to Compose Material 3 roles.

## Type and shape

M PLUS 1 everywhere: the variable font on the web, and static Latin weights
(400, 500, 700, 800) on desktop and Android, whose renderers can't pick weights
from a variable font. Headings are extra bold (800); task titles are 600;
metadata is 500–700 at 11–13px. Main headings are 28–32px.

Kirakira's radius is 1rem. Controls are pills or near-pills, task rows and
cards round to 18–28px, and dialogs to 28px. Section rules are 2px dotted lines
rather than solid borders. Icon tiles are rounded squares tilted a few degrees.
Decorative Kirakira shapes (dots, circles, pills, sparkles) belong to the public
site's hero and empty states, not to working lists.

## Layout and hierarchy

Desktop and wide web layouts use a 240px sidebar with workspace selection,
search, task views, projects and settings. Phone navigation is a sheet from the
left. Put the primary Add task action in the sidebar (a pink pill) and quick
entry at the end of the task list (a dashed pill that turns solid pink when
focused). Use one main view title and count. Task titles lead each row; project
names and note previews stay close beneath them. Keep deadlines separate from
scheduled dates. Divide overdue and current tasks with group rules rather than
borders on every row. Hide redundant scheduled-today metadata in Today, which
shows the date on a small Kirakira tape badge. Empty states pair the idle cat
with a short line about the next action.

Desktop and wide web task editors use a centered dialog with a writing area and
a compact properties rail. Keep the save footer visible. Phone editors stack
these areas and keep generous touch targets. Keep the labels that distinguish
workspace, project, scheduled date, deadline, repeat and reminder.

Wide calendars pair a month grid with a selected-day agenda card. Limit crowded
day previews and show every task in the agenda. On phones, show task counts in
the month grid and readable task names below it. Settings sections are single
cards with compact rows inside; don't nest cards.

## Icon

The app icon is a cream cat on a navy tile with a pink completion badge, drawn
to Kirakira's character rules. The source is `assets/catdo-icon.svg`; see
`assets/README.md`.

## Motion

Kirakira's motion, kept quick. Buttons sink to 0.95 while pressed and pop
0.95 → 1.05 → 0.98 → 1 on release. A task's check squashes, rebounds
0.85 → 1.08 → 0.97 → 1 and draws its tick, a small pink and yellow burst fires
behind it, and the row slides away before the list reflows. View icon tiles pop
in on Kirakira's spring; rows settle in with a short staggered rise. Dialogs,
sheets and menus use the Kirakira base kit's pop entrances and exits. UI
feedback takes 0.15–0.4s and entrances stay under about 0.5s. Honour the
platform's reduced-motion setting: state changes still show, without movement.
On the web, keyboard-driven sessions stay immediate.

## Reviewing a UI change

Check the actual rendered application in light and dark modes. Include a
populated list, a long title, an empty state, the task editor, settings and the
calendar. For web, review both wide and phone viewports; for Android, use an
emulator or device. Check keyboard focus and touch targets as well as
screenshots. Build and run each surface's existing checks after changing its
UI.

## Visual references

Kirakira UI's docs and style guide at [kk.workercat.com](https://kk.workercat.com)
are the design reference. Product layout references, reviewed without copying
assets:

- [Things — Today and task details](https://culturedcode.com/things/features/):
  task names have priority, with quieter project context below.
- [Todoist — task view](https://www.todoist.com/inspiration/todoist-new-task-view):
  explicit, compact attribute labels and usable narrow layouts.
- [TickTick — list and calendar views](https://www.ticktick.com/features):
  navigation, tasks and details have distinct regions; calendars pair with
  agendas.

Keep existing labels stable. Avoid generic section titles that merely repeat
the page's purpose, and helper copy that only restates the adjacent control.
Counts can sit beside the existing view title instead of introducing a second
heading. Keep field labels that disambiguate values (especially scheduled dates
versus deadlines).
