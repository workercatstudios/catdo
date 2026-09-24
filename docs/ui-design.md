# CatDo UI

Desktop, web, and Android share a neutral foundation with sage for actions,
focus, and selection. Use platform components with these tokens; keep native
keyboard, focus, touch, and accessibility behavior.

## Colors

| Role                     | Light     | Dark      |
| ------------------------ | --------- | --------- |
| Canvas                   | `#FFFFFF` | `#1C201D` |
| Sidebar / low surface    | `#F7F7F5` | `#181B19` |
| Secondary surface        | `#F7F7F5` | `#242925` |
| Text                     | `#242824` | `#EDF0EA` |
| Secondary text           | `#696F66` | `#A2AAA0` |
| Border                   | `#E8EAE5` | `#333B34` |
| Primary                  | `#3E624E` | `#ACCCB1` |
| On primary               | `#FFFFFF` | `#1C201D` |
| Selection                | `#E9EFE9` | `#2C3E30` |
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

Use the existing CatDo artwork. No decorative gradients, glass, or large shadows
are needed for the task workspace.

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
