# CatDo

CatDo is a personal task manager inspired by Todoist, built around simplicity and a clean, professional interface. It is developed for daily individual use, with source available under the PolyForm Noncommercial License 1.0.0.

Address: `catdo.workercat.com`.

Implementation status: the Fedora desktop, landing page, web app, shared WorkerCat sign-in, and Cloudflare sync are built. Android remains a later milestone. See [the README](../README.md) for running it and [implementation notes](implementation.md) for behavior and limits.

## Product and brand

- Make capturing, organizing, scheduling, and completing tasks easy.
- Help someone develop a routine without requiring a formal productivity system.
- Follow WorkerCat's restrained neutral palette, plain language, and warm personality.
- Keep task lists compact and quiet. Use the cat sparingly in branding and empty states.
- Reveal secondary controls in the task editor rather than crowding each row.

## Platforms

- Landing page and web app.
- Native desktop app using Rust and GPUI: Linux first, with Fedora as the initial daily-use environment. Windows later; macOS is not planned.
- Native Android app.
- Cloudflare backend shared by the clients.
- Clerk for authentication across web, desktop, and Android.

Implemented: React and Vite for the web; Cloudflare Workers with SQLite-backed Durable Objects for per-account storage. Kotlin and Jetpack Compose are the intended Android stack.

## Authentication and ownership

CatDo shares WorkerCat accounts through Clerk. CatDo's backend owns workspaces, projects, tasks, and their access rules. Personal and work workspaces belong to the same user account; they do not require separate logins.

The desktop uses Clerk public-client device authorization with credentials in Secret Service. Desktop task management works without sign-in. Android's native integration remains to be implemented.

## Organization

The hierarchy is **workspaces → projects → tasks**. Each workspace has its own Inbox, Today, Upcoming, Calendar, and projects.

- Workspaces separate personal and work tasks; they do not imply team collaboration.
- Tasks without a project belong to the active workspace's Inbox.
- Switching workspaces changes the visible tasks, search scope, and counts.
- Tasks can move between projects and workspaces.
- Today and Calendar initially show the active workspace. A combined view can be considered after daily use.
- Tasks support subtasks.

## Scheduling and due dates

Both dates are optional and have distinct meanings:

- **Scheduled date:** when the user plans to work on the task.
- **Due date:** when the task must be finished.

A task can be scheduled for Tuesday and due Friday. Adding a task to Today sets its scheduled date without creating a deadline.

## Calendar

Calendar is a launch feature, with a month view and an agenda for the selected day.

- Show scheduled tasks and clearly distinguished deadlines.
- Scope the view to the active workspace.
- Dragging a task to another day changes its scheduled date.
- Deadline changes must be explicit.

Dates are date-only. Coincident scheduled and due dates appear together; missed deadlines have a distinct overdue style.

## Recurrence

Recurring tasks are a launch feature. Support daily, selected weekdays, weekly, monthly, and every N days, weeks, or months.

- **Fixed schedule:** for example, every Monday, independent of completion time.
- **After completion:** for example, three days after the previous completion.
- Completing an occurrence preserves its history and advances to the next occurrence.

Implemented recurrence behavior, including month boundaries, missed dates, reminders, and undo, is documented in the implementation notes.

## First-release scope

- Workspaces and projects.
- Task capture and editing, subtasks, completion, and undo.
- Inbox, Today, Upcoming, and Calendar.
- Scheduled dates, due dates, and recurring tasks.
- Reminders and search.
- Offline task creation, editing, and completion, with reliable cross-device sync.

Labels, custom filters, boards, shared projects, and assignments are deferred. Pricing and public-launch commitments are undecided.

## Delivery direction

Start with the Fedora desktop experience. Design and prototype the workspace switcher, sidebar, Today list, calendar, and task editor to validate navigation, focus, keyboard interactions, and date controls.

Validate offline storage and sync with a complete task workflow before expanding to the web and Android clients. Keep shared API contracts and behavior consistent while allowing native interactions on each platform. The landing page can begin as a concise product introduction.

Success for the initial version means people choose to use CatDo every day and trusts it to retain tasks and dates across devices.
