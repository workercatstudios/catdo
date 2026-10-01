# Web service readiness

Reviewed 2026-09-25. Scope: CatDo's website, web app, HTTP API, and account storage. These are repository changes; this review does not deploy them or certify production.

## References

- [Google SRE launch checklist](https://sre.google/sre-book/launch-checklist/): failure behavior, capacity, monitoring, recovery, and releases.
- [Google SRE readiness reviews](https://sre.google/sre-book/evolving-sre-engagement-model/): tailor the checklist to the service.
- [OWASP REST security](https://cheatsheetseries.owasp.org/cheatsheets/REST_Security_Cheat_Sheet.html): authentication, methods, input limits, content types, and errors.
- [OWASP HTTP headers](https://cheatsheetseries.owasp.org/cheatsheets/HTTP_Headers_Cheat_Sheet.html): response security and caching.
- Cloudflare's [static headers](https://developers.cloudflare.com/workers/static-assets/headers/), [Worker rollback](https://developers.cloudflare.com/workers/versions-and-deployments/rollbacks/), and [SQLite recovery](https://developers.cloudflare.com/durable-objects/api/sqlite-storage-api/#pitr-point-in-time-recovery-api) documentation.

## Current state

| Area | State |
| --- | --- |
| Authentication and account isolation | Bearer-only API; Clerk verification; origin restrictions; desktop OAuth client/scope checks; storage identity comes from verified credentials. Regression tested. |
| Request validation | Explicit methods with `Allow`; exact JSON media type; schema and relationship validation; invalid data returns 400 before storage changes. |
| Resource bounds | Uploads have a 1,900,000-byte actual-stream limit and 15-second read deadline. Snapshots remain capped at 900,000 bytes. No traffic limits or capacity targets are set. |
| Atomic storage and retries | Snapshot and receipt commit together. Tests cover interrupted receipt writes, retries after later edits, conflicts, invalid data, and byte limits. Receipts remain unbounded to preserve long-offline retries. |
| Dependency failures | GitHub lookups have a 5-second deadline; web sync has a 20-second deadline. API/download 503s include `Retry-After: 30`. Application error logs omit exception messages and payloads. |
| Response security | Shared headers cover rendered pages, downloads, and handled errors; static assets use `_headers`. Baseline CSP blocks framing, object embeds, and foreign base URLs. Camera/microphone/geolocation are disabled. |
| Caching | API/app/error responses and credential-bearing dynamic requests are `no-store`; service worker revalidates; fingerprinted assets use immutable caching. Offline Cache Storage deliberately keeps the generic app shell. |
| Liveness | `GET /api/health` returns only `{"status":"ok"}`; `HEAD` has no body. No auth, network, or storage access. This does **not** prove Clerk or account storage is healthy. |
| Deployment | Manual deployments run the reusable Checks workflow. Deployments are serialized and followed by bounded, read-only HTTP smoke checks. Failures mark the workflow failed; they do not automatically roll back. |
| Local verification | At review time: TypeScript check, 62 unit tests, production build, 8 HTTP/browser tests including offline data and service-worker upgrades. Production dependency audit reported no known advisories. |
| Monitoring | Worker observability is enabled. No availability target, external checks, or alerts are provisioned. |
| Recovery | SQLite Durable Objects support recovery within 30 days. This is platform capability, not a tested CatDo restore procedure or an independent backup. JSON export exists; web import does not. |
| Privacy/deletion | Sign-out retains local data by design. Cloud deletion and backup retention must respect the shared WorkerCat identity. |
| Hosted testing and capacity | No isolated staging. Browser tests use local fixtures, not live Clerk sign-in or production writes. No production load test has been performed. |

## Verify a build or deployment

```sh
pnpm check
pnpm test
pnpm build
pnpm test:e2e
pnpm audit --prod
```

The browser suite runs the smoke script against the local production preview. To run its read-only checks against a deployed origin:

```sh
pnpm check:web https://catdo.workercat.com
```

It checks liveness, public HTML, a referenced built asset, app-shell headers, public auth configuration, anonymous sync rejection, and the generated service worker. Each request has a 10-second deadline; redirects are not followed. It never signs in, reads tasks, or creates account storage. Production requires anonymous sync to return 401. Local CI can use `--allow-unconfigured-auth` to tolerate a missing Clerk secret (503); do not use that option in production.

Deployment retries these read-only checks three times with five seconds between attempts for propagation. A pass proves these HTTP behaviors, not that the exact new version serves every location or that authenticated sync works.

## Failed-release runbook

1. Inspect the workflow's failing check and Cloudflare Worker errors. Separate rendering/asset failures from Clerk and storage failures. Liveness can remain green during a dependency outage.
2. Exercise sign-in, upload, readback, and operation-ID replay with a disposable hosted account. Never paste tokens or task payloads into logs or issues.
3. Identify the last known good Worker version. Preserve the `catdo` name, `ACCOUNTS` binding, `CatDoAccount` class, and existing migrations.
4. Check storage compatibility. Code rollback does not restore tasks, and Cloudflare restricts rollback across certain binding or Durable Object migration changes.
5. For a compatible rollback, use the installed CLI with the selected version:

```sh
pnpm --filter @catdo/web exec wrangler deployments list
pnpm --filter @catdo/web exec wrangler rollback <known-good-version-id> --message "Reason for rollback"
pnpm check:web https://catdo.workercat.com
```

These are operator instructions; this review did not execute them. A version predating the new endpoint/headers needs its previous checks, rather than the new smoke script unchanged. Fix the failed release before deploying it again. Offline clients may retain their cached shell until they accept an update.

For data recovery, preserve the affected account's current state and local pending edits, then stop its writes. Restoring an object also restores its retry receipts and revision; reconnecting stale clients can replay or merge edits. Rehearse on a disposable hosted account and document how clients rebase before restoring production. Never expose an unauthenticated restore endpoint. Cloudflare PITR cannot be rehearsed solely in the local emulator.
