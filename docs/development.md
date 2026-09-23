# Development guide

[← Back to CatDo](../README.md)

## Fedora desktop

```sh
cargo run -p catdo-desktop
```

Tasks are stored in `~/.local/share/catdo/catdo.sqlite3`, subject to XDG overrides. Desktop use does not require sign-in. **Sign in to sync** opens the browser to authorize CatDo Desktop with your WorkerCat account. OAuth credentials live in the desktop Secret Service keyring, not the task database.

Use a separate data directory for development or another account:

```sh
cargo run -p catdo-desktop -- --data-dir /tmp/my-catdo
```

A database becomes bound to its first signed-in account to prevent accidental uploads to someone else's account. Signing out keeps local tasks. Sync runs about every ten seconds and pauses while task details are open.

The sidebar’s **Appearance** toggle follows the OS by default, including changes while CatDo is running. Select **Light** or **Dark** to override it; **System** resumes automatic updates. This device preference is saved immediately and restored at startup. All desktop screens and controls use GPUI Kit’s shared theme colors.

The app supports Wayland and X11; visual checks have used Fedora through XWayland. If the Wayland graphics path has trouble:

```sh
env -u WAYLAND_DISPLAY cargo run -p catdo-desktop
```

GPUI Kit 0.6.6 supplies the matched framework, components, assets, and test tools through one dependency. On Linux, the Cargo configuration uses the font backend’s supported runtime Fontconfig loading; the regular Fedora `fontconfig` package is required, but its development headers are not.

Build an optimized binary with `cargo build --locked --release -p catdo-desktop`, then run `target/release/catdo`. `bash scripts/install-desktop.sh` builds and installs to the user's application launcher without root access or changing task data.

| Shortcut | Action                                                       |
| -------- | ------------------------------------------------------------ |
| Ctrl+N   | New task                                                     |
| Ctrl+F   | Search the current workspace                                 |
| Ctrl+S   | Save the task editor                                         |
| Ctrl+Z   | Undo the last task action; text fields retain their own undo |
| Ctrl+1   | Today                                                        |
| Escape   | Save and close details / dismiss the creation dialog         |

Generate sample tasks in a new directory (the generator refuses to overwrite an existing database):

```sh
cargo run -p catdo-core --example demo -- /tmp/catdo-demo
cargo run -p catdo-desktop -- --data-dir /tmp/catdo-demo
```

## Web and API development

Use the Node version in `.node-version`, pnpm from `package.json`, and Rust from `rust-toolchain.toml`. Both dependency lockfiles are checked in.

```sh
pnpm install --frozen-lockfile
cp apps/web/.dev.vars.example apps/web/.dev.vars
```

Fill `.dev.vars` with your own Clerk development instance configuration. Keep secrets out of source control. Create a public OAuth client for the desktop with device authorization and `openid profile email offline_access` scopes. Development users and task storage are separate from production.

```sh
pnpm dev
```

Open `http://127.0.0.1:5173/app`. TanStack Start runs public pages, the task app, and the API in one Cloudflare Vite development server. For the production build and service worker, run `pnpm build` followed by `pnpm --filter @catdo/web preview` (port 4173). Set `APP_ORIGIN` in local `.dev.vars` to the origin you use for sign-in.

To connect the native client to local development:

```sh
CATDO_API_URL=http://127.0.0.1:5173 cargo run -p catdo-desktop -- --data-dir /tmp/catdo-dev
```

Wrangler emulates a SQLite Durable Object per account locally. No production task data is used. Clerk development users are distinct from production users.

## Deployment

Production is a Cloudflare Worker named `catdo`, with TanStack Start server rendering, static assets, and SQLite-backed Durable Objects. Public configuration and the custom domain are in `apps/web/wrangler.jsonc`. Each account has independent task storage.

For your own deployment, update the domain and public Clerk configuration in `apps/web/wrangler.jsonc`, authenticate the installed Wrangler CLI, and set your production Clerk secret through its hidden prompt:

```sh
pnpm exec wrangler secret put CLERK_SECRET_KEY --config apps/web/wrangler.jsonc
pnpm run deploy
```

Use `pnpm run deploy`, since `pnpm deploy` is a different built-in command. Do not put a development secret into production.

The `Deploy site` GitHub Actions workflow deploys the current `main` commit after the `Checks` workflow succeeds. It builds with the installed project tooling, uploads a version of the existing `catdo` Worker, and promotes that version to all traffic. Set repository variable `CLOUDFLARE_ACCOUNT_ID` and repository secret `CLOUDFLARE_API_TOKEN`; scope the token to the CatDo Worker with Workers Editor access and rotate it before expiration. The existing `catdo.workercat.com` custom domain is managed in Cloudflare, so the Wrangler configuration omits routes. Adding or changing a custom domain requires Workers Routes Write for that zone. The existing `CLERK_SECRET_KEY` remains a Worker secret in Cloudflare and is not stored in GitHub.

## Checks

```sh
pnpm check
pnpm test
pnpm build
pnpm test:e2e
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

A development-only integration example uses the real desktop OAuth/keyring/storage/sync code. Authorize its printed code with a disposable Clerk development user:

```sh
CATDO_API_URL=http://127.0.0.1:5173 cargo run -p catdo-desktop --example sync_smoke
```

It checks native task upload, offline edit and database reopen, retry idempotency, recurrence dates, and completion history. It creates a clearly named test task in that development user's local Wrangler account.

## Structure and current limits

- `crates/catdo-core`: Rust domain rules, recurrence, SQLite storage, sync, and tests.
- `apps/desktop`: GPUI screens, browser authorization, keyring credentials, reminders.
- `apps/web`: TanStack Start public site, shadcn task app, API, IndexedDB storage, and offline shell.
- `packages/domain`: TypeScript domain rules and sync contract.
- `apps/web/src/server`: authenticated HTTP API and per-account Durable Object storage.

Sync currently sends complete snapshots and caps an account's serialized task data at 900 KB. Conflicting edits to the same record require a choice; changes to different records merge automatically. Web Settings includes JSON export. See [implementation notes](implementation.md) for the protocol and limits.

Reminders require the desktop app to be running and a working notification service; web push and scheduled Android delivery are not implemented. Android is registered with Firebase Cloud Messaging and can receive alerts after notification opt-in, but no server sender is configured yet. Dates are date-only, while reminders have local times. Recurrence shows the current occurrence, rather than an infinite calendar preview. Task and subtask completion are independent. Browser caches and desktop data remain on the device after sign-out; avoid offline access on a shared computer. Broader Android device testing, production load testing, and a full accessibility pass remain outstanding. See the [Android app guide](../apps/android/README.md) for its build and current feature boundaries.

## CI and releases

**Checks** runs on pull requests, pushes to `main`, and manual dispatch. It checks
TypeScript, domain/API tests, the production web build, Rust formatting, clippy,
Rust/GPUI tests, and release tooling. It needs no Clerk or Cloudflare credentials.

**Release** runs when a `v*` tag is pushed. It validates the tag against
`workspace.package.version` in `Cargo.toml`, runs the same checks, builds the
Linux x86-64 desktop and signed Android APK on Ubuntu 24.04, smoke-tests the packages, and publishes an
AppImage, a `.tar.gz`, an `.apk`, and SHA-256 checksums to GitHub Releases. Android signing uses the repository secrets described in the [Android app guide](../apps/android/README.md). Only the publish
job receives `contents: write`. No cloud deployment runs as part of a release.

To release:

1. Update the workspace version in `Cargo.toml` and refresh `Cargo.lock` with
   `cargo check --workspace`. Commit both files and push.
2. Tag that commit with the matching version, for example `git tag v0.1.0`.
3. Push that tag with `git push origin v0.1.0`.

Versions such as `0.2.0-rc.1` produce GitHub prereleases. The tag must exactly
match the Cargo version with a `v` prefix. Use a new version for each published
release; the workflow does not overwrite existing release assets.

For a rehearsal, dispatch **Release** from the Actions tab on a branch. It runs
the checks and uploads downloadable build artifacts without publishing a
release or creating a tag. A rerun of an existing published tag deliberately
fails at publication rather than replacing a download.

Build packages locally after building the optimized desktop binary:

```sh
bash scripts/package-linux.sh
python3 scripts/check-release.py --archive dist/catdo-0.1.0-linux-x86_64.tar.gz
bash scripts/package-appimage.sh
```

AppImage packaging downloads a pinned, checksum-verified linuxdeploy tool into
ignored `.local/release-tools`; `patchelf` must be installed on the build host.
The AppImage runtime is pinned and checksum-verified as well. The release archive includes the desktop icon,
launcher, installer, license, and notices. Packaging tests install into a
throwaway directory, never the developer's task database.

## Web migration and browser checks

The Worker name `catdo`, `ACCOUNTS` binding, exported `CatDoAccount` class, and `v1` SQLite migration are unchanged. Do not rename these or add a replacement migration for the web rewrite: existing production accounts must retain their storage. `/api/config`, `/api/me`, and `/api/sync` keep the native HTTP contract.

Public pages render on the server. The app authenticates on the client so the cached shell can open existing account-scoped IndexedDB data without an auth round trip. The service worker caches only the generic app shell and static assets, never API responses. A waiting worker asks for a reload and never reloads an open task automatically. Existing `catdo-shell-*` caches are retired when the new worker activates; the `catdo` IndexedDB database is not changed.

`pnpm test:e2e` runs Playwright against the production preview, with disposable local accounts and offline browser data. Install its browser once with `pnpm exec playwright install chromium`; on Linux CI use `--with-deps`. An installed Chrome can be selected with `CHROME_PATH=/path/to/chrome`. Tests do not require production credentials.

Public page content and release links live in `apps/web/src/site/content.ts`. Keep the version and asset URLs there current when publishing a desktop release. shadcn components are source-owned in `src/components/ui` and configured in `components.json`; theme tokens and reduced-motion styles are shared by the site and app.
