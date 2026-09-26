# Private privacy-request operations

CatDo accepts requests from signed-in users at `/privacy-requests` without requiring terms acceptance. Requests are linked to the verified Clerk account; user-supplied account IDs are rejected. The inbox lives in the reserved `privacy:inbox` Durable Object, separate from task snapshots. No request text, identity, or tokens belong in logs or public issues.

An operator must review the private inbox regularly. There are no email notifications. The list API returns at most 100 pending requests and a cursor. One pending request per account and kind is allowed. Duplicate submissions return the original request. Up to 20 completed requests per account are kept for at most 90 days; a Durable Object alarm removes expired completed requests. Pending requests remain until resolved. The global pending limit is 10,000; the API fails visibly rather than discarding excess requests.

## Provisioning

Use a separate random secret of at least 32 characters as the Worker's `PRIVACY_ADMIN_TOKEN`. It must not be a Clerk key, browser token, source-controlled value, or client-exposed environment variable. Keep the operator copy at `~/.config/workercat/catdo-privacy-admin-token`, mode 0600, with its parent directory mode 0700. Provision and rotate through `pnpm exec wrangler secret put PRIVACY_ADMIN_TOKEN`, passing the value through standard input. The operator API rejects browser Origin headers and ordinary Clerk sessions.

The checked-in CLI reads the local secret, uses HTTPS without redirects, and writes results only to a newly created private file. It does not print request contents. Never upload those files to an issue or commit them. Keep exports only long enough to fulfill the request, then remove the local copies.

## Review and respond

```sh
node scripts/privacy-admin.mjs list --out ~/.local/share/workercat/privacy/inbox-1.json
node scripts/privacy-admin.mjs list --cursor-file ~/.local/share/workercat/privacy/inbox-1.json --out ~/.local/share/workercat/privacy/inbox-2.json
```

Inspect the private file locally. Each item includes its request ID, verified account ID, kind, message, and creation time. Review the request's actual scope before taking action. Access and cloud-deletion actions below are restricted to matching, still-pending request kinds. A request marked resolved cannot subsequently authorize an export or erasure.

Write the user-facing outcome or clarification into a private text file, then submit it:

```sh
node scripts/privacy-admin.mjs resolve --id REQUEST_UUID --response-file ~/.local/share/workercat/privacy/response.txt --out ~/.local/share/workercat/privacy/resolution.json
```

The response appears only in that user's signed-in request history. Resolving records an outcome; it does not itself change or delete any account data. For a clarification, explain that the user should submit a new request after reading the response. A repeated resolution preserves the first response.

## Access

Users can export their current cloud tasks directly without accepting terms. For a broader access request:

```sh
node scripts/privacy-admin.mjs export --id ACCESS_REQUEST_UUID --out ~/.local/share/workercat/privacy/account-export.json
```

This exports that requester's cloud task snapshot, versioned acceptance receipts, and retained privacy-request history. It does not export Clerk's identity records, vendor diagnostics, other products, or copies only on the user's devices. Review those separately if they are within the request's scope, and verify the destination before any private delivery. Never put an export into the plain-text response field or a public download location.

## Cloud task erasure

After confirming the requested scope, this explicitly authorized operator action permanently clears the account's cloud task snapshot and sync retry receipts:

```sh
node scripts/privacy-admin.mjs erase-cloud-data --id DELETION_REQUEST_UUID --confirm erase-cloud-data --out ~/.local/share/workercat/privacy/erasure.json
```

The request itself must already be a pending deletion request belonging to the account. The server derives the account from that request. A closure marker blocks all later uploads from old clients and new terms acceptance cannot bypass it. Repeating the erasure is safe. Cloud reads become empty. This operation has no self-service reopening path.

Cloud erasure leaves local device copies, the Clerk identity, versioned acceptance receipts, the closure marker, privacy requests, and any applicable provider backups or security records. Explain these limits in the private response. Full account deletion requires separately processing the identity and any justified record retention, assessing the requested scope and applicable obligations. Do not mark a request fulfilled before completing that work. Removing Clerk identity before responding prevents the user from reading the signed-in response, so arrange any necessary verified private communication first.

Legal acceptance receipts and the closure marker must not be silently removed by the cloud-erasure action. Their longer-term retention requires an operator decision based on the particular request; this tool does not claim all personal data has been erased.
