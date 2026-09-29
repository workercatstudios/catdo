# CatDo UI

Desktop, web, and Android share a warm neutral foundation. Actions, focus, and
selection use ink (near-black in light mode, near-white in dark mode) rather
than a hue. Colour is reserved for meaning: each view has its own hue for its
icon (amber Today, blue Inbox, violet Upcoming, teal Calendar, green Completed),
every project gets a stable colour dot from a hash of its id (eight slots shared
by all clients), deadlines are red when late and amber when due today, scheduled
dates read blue, repeats read teal, and a finished check fills green. Metadata
always pairs its colour with an icon (flag, calendar, repeat, subtasks) so it
still reads without colour. Use platform components with these tokens; keep
native keyboard, focus, touch, and accessibility behavior. Text on a tint must
keep 4.5:1 contrast; the light-mode hues above are chosen for that.

## Colors

| Role                     | Light     | Dark      |
| ------------------------ | --------- | --------- |
| Canvas                   | `#FFFFFF` | `#1B1B1A` |
| Sidebar / low surface    | `#F6F6F4` | `#171716` |
| Secondary surface        | `#F6F6F4` | `#232322` |
| Text                     | `#232323` | `#EDEDEA` |
| Secondary text           | `#6B6B67` | `#A3A39F` |
| Border                   | `#E7E7E3` | `#333331` |
| Primary (ink)            | `#262626` | `#EDEDEA` |
| On primary               | `#FFFFFF` | `#1B1B1A` |
| Selection                | `#ECECE8` | `#2C2C2A` |
| Today accent             | `#946A2A` | `#D9A860` |
| Inbox / scheduled        | `#3B6FB6` | `#8FB4E8` |
| Upcoming                 | `#6B5BB5` | `#B3A6E8` |
| Calendar / repeat        | `#25736A` | `#86CDBF` |
| Done                     | `#3E7A4F` | `#9ACB9F` |
| Error / overdue deadline | `#AD3F3C` | `#E7988B` |

Token definitions live in:

- Desktop: `apps/desktop/src/theme.rs`, applied to GPUI Kit's theme so its
  sidebar, lists, buttons, inputs, and menus inherit the palette.
- Web: `apps/web/src/styles/foundation.css`, shared by the application and site.
- Android: `apps/android/app/src/main/java/com/workercat/catdo/ui/Theme.kt`, mapped
  to Compose Material 3 color, typography, and shape roles.

## Layout and hierarchy

Use 28–32px main headings, 14–16px task titles, and 12–13px supporting metadata.
Keep ordinary controls near an 8px corner radius and grouped panels near 12px.
Use spacing and subtle boundaries to separate sections. Reserve stronger color
for actions, selected destinations, and deadlines that need attention.

Desktop and wide web layouts use a 240px sidebar with workspace selection,
search, task views, projects, and settings. Phone navigation adapts to a drawer.
Put the primary Add task action in the desktop sidebar and quick entry at the end
of the task list. Use one main view title and count. Task titles lead each row;
project names and note previews stay close beneath them. Keep deadlines separate
from scheduled dates. Divide overdue and current tasks with group rules rather
than adding borders to every row. Hide redundant scheduled-today metadata in
Today. Empty states explain the next action without adding visual noise.

Desktop and wide web task editors use a centered dialog with a writing area and
a compact properties rail. Keep the save footer visible. Phone editors stack
these areas and retain generous touch targets. Preserve labels that distinguish
workspace, project, scheduled date, deadline, repeat, and reminder.

Wide calendars pair a month grid with a selected-day agenda. Limit crowded day
previews and expose all tasks in the agenda. On phones, show task counts in the
month grid and readable task names below it. Settings use compact rows and
section rules without nested cards.

Use the existing CatDo artwork. No decorative gradients or glass. Dialogs may
carry a soft shadow; rows and panels stay flat.

## Motion

Motion is quick and purposeful. Views and rows settle in with a short fade and
lift (about 250ms, staggered a little per row); checks fill and spring when a
task completes, and the row leaves before the list reflows; dialogs rise over a
fading scrim; hover and press states transition in roughly 150ms. Keep every
entrance under 400ms, and honour the platform's reduced-motion setting by
keeping only the fades. On the web, keyboard-driven sessions stay immediate.

## Reviewing a UI change

Check the actual rendered application in light and dark modes. Include a populated
list, a long title, an empty state, the task editor, settings, and the calendar.
For web, review both wide and phone viewports; for Android, use an emulator or
device. Check keyboard focus and touch targets in addition to screenshots. Build
and run each surface's existing checks after changing its UI.

## Visual references

Reviewed official product examples as layout references, without copying assets:

- [Things — Today and task details](https://culturedcode.com/things/features/):
  task names have priority, with quieter project context below.
- [Todoist — task view](https://www.todoist.com/inspiration/todoist-new-task-view):
  explicit, compact attribute labels and usable narrow layouts.
- [TickTick — list and calendar views](https://www.ticktick.com/features):
  navigation, tasks, and details have distinct regions; calendars pair with agendas.

Keep existing labels stable. Avoid adding generic section titles that merely
repeat the page's purpose, or helper copy that only restates the adjacent control.
Counts can sit beside the existing view title instead of introducing a second
heading. Keep field labels that disambiguate values (especially scheduled dates
versus deadlines).
