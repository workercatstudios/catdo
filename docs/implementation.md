# Implementation notes

## Boundaries

`catdo-core` has no UI dependency. GPUI Kit owns presentation, drafts, navigation, and notifications. IDs are UUIDs, dates serialize as ISO dates, and completion times use UTC. The Rust and TypeScript clients share the same JSON contract and recurrence behavior.

The desktop has one UI dependency, `gpui-kit = "=0.6.6"`, which supplies the matching framework, components, assets, and testing APIs. All imports use the Kit facade. Cargo.lock pins its dependency graph. The Linux font backend uses its supported runtime Fontconfig loading mode, configured in `.cargo/config.toml`.

## Appearance

The desktop uses GPUI Kit's shared color tokens for custom task/calendar surfaces and its standard components for inputs, textareas, date pickers, selectors, checkboxes, buttons, navigation buttons, and the appearance toggle. There is no separate hard-coded light/dark palette or per-switch color override.

Appearance is a local SQLite preference, separate from synced task data. **System** is the default. Startup resolves the current window appearance; an active window-appearance observer then follows the OS, including Linux XDG portal updates. **Light** and **Dark** overrides ignore later OS changes. Choosing **System** immediately resolves the current OS appearance again. The three-way Kit button group saves its choice before applying it and reports storage failures.

The release was visually checked against Fedora’s dark preference. An isolated D-Bus portal test sent live dark → light → dark notifications to the running application and verified that System followed them; a restored Light override remained light through both notifications. These checks used window-only captures without changing the user’s desktop settings.

## Storage and mutation

Each workspace, project, task, and completion is a separate versioned JSON record in SQLite. Domain validation enforces workspace ownership and prevents cyclic task ancestry. A save updates only changed records and removes deleted records inside a transaction. SQLite uses WAL and full synchronous writes.

A monotonic revision rejects stale saves from another app instance. A failed save leaves the in-memory state unchanged and displays an error. Unknown newer database schema versions are rejected. UI preferences live separately from task data.

Undo restores a full command snapshot, including recurrence advancement and completion history. It is limited to 30 commands and deliberately stays local to the current session.

## Dates and recurrence

- A scheduled date is a plan; a due date is a deadline. No ordering constraint is imposed between them.
- Today includes tasks scheduled on or before today and deadlines on or before today. Only missed deadlines receive the overdue style.
- Calendar shows both dates. When they coincide, a task has one entry marked with both meanings.
- Dragging a scheduled entry only changes its scheduled date and the monthly recurrence anchor. Due-only entries cannot be dragged as if they were scheduled work.
- A fixed recurring task advances to the first occurrence after both the current occurrence and the completion day. Missed dates do not produce a backlog of duplicates.
- A completion-relative task advances from the actual completion day.
- Monthly fixed schedules retain their original day: January 31 → February 28 → March 31. Completion-relative months use the completion day.
- Scheduled and due dates move together when a recurring task completes, preserving their offset. The scheduled date is the recurrence anchor when both dates exist.
- Completion stores an immutable copy of the completed occurrence. The task ID remains stable for the next occurrence.
- Subtasks currently have independent completion state. Moving a parent moves its subtree; deleting a parent deletes its subtree, with undo available.

## Reminders

Reminder entry uses the system's local timezone and stores an absolute UTC instant. Ambiguous/nonexistent local times are rejected. While open, the app checks reminders every 15 seconds and sends native notifications off the UI thread. Successful delivery is persisted; failed delivery is reported and suppressed until restart.

Recurring reminders preserve wall-clock time in the current system timezone across daylight-saving changes. A spring gap advances to the next valid minute; an ambiguous fall-back time uses its earlier occurrence. Reminder delivery is currently desktop-only and follows the system timezone. Explicit IANA timezone preferences, background delivery, and cross-device notification deduplication remain future work.

## Validation

The GPUI test platform verifies keyboard creation/save, invalid draft handling, workspace isolation, completion/undo, date controls, recurrence selection, and persistence after reopen. Kit UI tests click the real appearance controls, check returning to System, and verify restoring a saved override. Core tests cover month-end and leap-year recurrence, missed occurrences, relative schedules, cyclic ancestry rejection, atomic validation, and stale-write protection.

The native app has been launched on Fedora through XWayland and window-only captures have been inspected. Native Wayland startup also leaves a running process, but the available X11 inspection path cannot verify its window. Assistive-technology support and native Wayland interaction still need a dedicated platform pass before a broader release.

## Authentication

CatDo shares WorkerCat accounts through Clerk. The hosted service uses `https://clerk.workercat.com`; each development or self-hosted deployment supplies its own instance configuration. The desktop has a public OAuth client configured separately for each instance.

Web uses Clerk session tokens. Desktop uses public-client device authorization, `openid profile email offline_access` scopes, and rotating refresh credentials in the OS keyring. No client secret is embedded in desktop code. API identity comes exclusively from verified Clerk tokens. Session tokens require a CatDo origin; OAuth tokens require the configured CatDo Desktop client ID and openid scope. OAuth access JWTs do not carry browser `azp`, so they are verified separately from browser session tokens. See [Clerk's OAuth verification documentation](https://clerk.com/docs/guides/configure/auth-strategies/oauth/verify-oauth-tokens).

The API is bearer-only, rejects foreign browser origins, and does not accept a user ID from request parameters. A local native database is bound to its first Clerk user. Another account needs a separate data directory. Web IndexedDB is partitioned by Clerk user ID. Signing out removes credentials/access to sync but keeps local task data; these are personal-device caches, not encrypted multi-user vaults.

## Cloudflare storage

The `catdo` Worker serves the web assets and `/api` routes. Each verified Clerk user maps to a separate SQLite-backed Durable Object. A DO serializes mutations and stores the current snapshot plus small operation receipts.

`GET /api/config` exposes only public Clerk configuration. `GET /api/me` returns the verified identity. `GET /api/sync` returns `{revision, data}`. `POST /api/sync` accepts `{id, base, data}` and returns the latest snapshot, or HTTP 409 with the snapshot requiring conflict resolution. Actual request bytes are bounded, snapshots are limited to 900 KB, and domain validation runs before storage. This is a personal-use full-snapshot protocol; pagination and larger account support are future work.

## Offline sync and conflicts

Native SQLite and web IndexedDB persist local changes before networking. The first upload freezes an operation ID, baseline, and intended data on disk. Interrupted requests retry the same ID. The server commits the new snapshot and receipt atomically. Repeated IDs return the latest snapshot without repeating completion or creating duplicate history.

The three-way merge compares each record against the baseline. Changes to different records combine; unchanged local records follow remote changes, including deletion. Conflicting edits or edit-versus-delete of the same record stop sync and ask which versions to keep. Recurring completion and its history are resolved together so retries and concurrent completion do not duplicate occurrences.

Acknowledgement rebases against the uploaded data, preserving edits made while the request was in flight. If combining parent/workspace moves would break relationships, the UI explicitly offers a whole-list choice. Web Manage includes JSON export before resolving. Conflict state survives application restart. Desktop stale-window revision checks apply to sync metadata as well as normal task writes. Browser Web Locks serialize account writes and upload preparation; BroadcastChannel refreshes other tabs.

Sync polls about every ten seconds while a client is open. The desktop pauses applying sync while a task editor is open. Web editors reject a save when their original task changed or was deleted during editing. Desktop undo is cleared when remote task data changes; web undo checks for overlapping edits before reverting.

The web service worker caches the application shell and public assets only. API responses, Clerk traffic, and credentials are excluded. Offline reload offers **Open saved tasks** for the last signed-in account, then **Reconnect** to restore live auth and sync. The first visit and sign-in require connectivity. Browser storage can still be cleared by the user or browser; export is available in Manage.

## Milestone validation

Automated tests cover Rust recurrence/storage/GPUI behavior, durable queued uploads and restart, deletion propagation, stale-window rejection, recurrence conflict history, TypeScript domain parity, server transactions/idempotency/account isolation, and API authorization boundaries.

The development integration example performs real Clerk device authorization through the shared WorkerCat development instance, uses Secret Service credentials and SQLite, uploads a recurring task, edits it without network calls, reopens its database, retries an acknowledged operation, and verifies dates/history. Production smoke checks cover deployed assets, shared public auth configuration, and rejection of anonymous sync; a production user session still requires the user's own sign-in.

## Remaining platform work

Native Android source is now in [`apps/android`](../apps/android), with local tasks, the shared sync protocol, and signed APKs on GitHub Releases. Physical-device testing, reminders, and Play Store distribution remain. Windows follows Linux; macOS is out of scope. Before a broader public release, validate native Wayland and accessibility, account recovery/export/import, background reminders, and larger-data performance through daily use.
