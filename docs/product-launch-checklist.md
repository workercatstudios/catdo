# CatDo web product launch checklist

Updated September 26, 2026. This checklist covers what visitors and users need to understand, trust, find, and use the product. The separate [engineering readiness report](web-readiness.md) covers service operation.

## Sources

- [Google SEO Starter Guide](https://developers.google.com/search/docs/fundamentals/seo-starter-guide), [page titles](https://developers.google.com/search/docs/appearance/title-link), and [site-name markup](https://developers.google.com/search/docs/appearance/site-names): understandable content, descriptive titles, crawlable links, and consistent site identity.
- [ICO privacy-information checklist](https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/individual-rights/the-right-to-be-informed/what-privacy-information-should-we-provide/): a useful inventory of policy topics; it does not establish which jurisdictions apply to CatDo.
- [ANPD cookies guidance](https://www.gov.br/anpd/pt-br/assuntos/noticias-periodo-eleitoral/anpd-lanca-guia-orientativo-201ccookies-e-protecao-de-dados-pessoais201d): transparent explanations of collection, purposes, and user choices.
- [W3C accessibility checks](https://www.w3.org/WAI/test-evaluate/easy-checks/): titles, headings, keyboard navigation, contrast, language, and zoom. Automated checks are only part of accessibility evaluation.

## Privacy, terms, and trust

- [x] Add a dedicated `/privacy` information page with readable, expanded sections and provider links. Redirect the old `/help/privacy` URL permanently.
- [x] Explain device storage, account sign-in, cloud sync, diagnostics, export, and the limits of deletion. Remove the unsupported claim that Cloudflare's configured logs are necessarily “short-lived.”
- [x] Explain that sign-out retains local data, cloud sync is not end-to-end encrypted, and Android notification opt-in does not control crash reporting.
- [x] Link privacy and support from the public footer, web sign-in, and app Settings.
- [x] Publish the shared [WorkerCat privacy policy](https://workercat.com/privacy) with verified CatDo and Blossom inventories.
- [x] Publish shared [WorkerCat terms](https://workercat.com/terms), preserving the immutable accepted version and separate source licences.
- [ ] Operator legal identity, country, intended markets and signed-out private contact: explicitly skipped by Kaf; no invented details. Authenticated private request forms are implemented.
- [ ] Complete the privacy policy's legal grounds, rights process, retention periods, and cross-border disclosures using actual provider settings and contracts.
- [x] Record Kaf's choices: WorkerCat-wide policies, personal/workplace use, ages 13+, retention until deletion request, automatic Android crash reporting, and explicit versioned terms acceptance.
- [x] Add 13+ eligibility declarations and explicit, server-recorded terms acceptance to the product account systems; preserve local access and export.
- [x] Link the central terms from sign-in, Settings and the footer. Native clients can review and accept using the same CatDo account on the web.
- [x] Implement a private request queue, authenticated operator handling and documented cloud-data erasure. Keep CatDo task deletion separate from removal of the Clerk identity or Blossom account.
- [ ] Audit deployed cookies and provider diagnostics before making a broader no-tracking claim or deciding whether consent controls are needed. No optional web analytics or decorative cookie banner was added.

The `/privacy` page supplements the canonical WorkerCat policy. Publishing factual notices does not establish full legal compliance: the skipped legal identity/markets and provider-specific retention/transfer details remain unresolved. No invented operator, email address or legal jurisdiction is published.

## Discovery and SEO

- [x] Preserve server-rendered public content, page-specific descriptions, canonical URLs, Open Graph images, and the existing Search Console verification tag.
- [x] Give help pages descriptive search titles while retaining their editorial headings.
- [x] Add `WebSite` structured data alongside the existing `SoftwareApplication` data. Describe only real features and current free pricing; do not fabricate reviews, ratings, or a rich-result guarantee.
- [x] Add explicit Twitter title, description, image, and image alternative text.
- [x] Add privacy/support to the sitemap and remove the old redirected privacy URL. Verify every listed URL returns an indexable 200 response.
- [x] Keep task-app URLs out of the sitemap and marked `noindex`. Allow crawlers to read that directive; robots exclusion is not access control.
- [x] Use one configured canonical origin for metadata and the robots sitemap reference.
- [x] Correct the homepage platform label to include Android, which already has a released APK.
- [ ] Confirm Search Console ownership/indexing and submit the updated sitemap after deployment. The verification tag alone is not proof that the property is verified or indexed.
- [ ] Inspect deployed social previews and field performance after release. A local build cannot establish search rankings or real-user Core Web Vitals.
- [ ] Recheck alternate hostnames, HTTP-to-HTTPS redirects, and preview-domain indexing at the host configuration level.

## Visitor experience and support

- [x] Add `/support` with guides, missing-task/sync help, releases, public bug reports, and private security reporting.
- [x] Warn that GitHub support issues are public, without misrepresenting security reporting as a channel for ordinary privacy requests.
- [x] Keep pricing, downloads, platform boundaries, offline behavior, and source licence discoverable.
- [x] Preserve keyboard navigation, responsive layout, real screenshot alternative text, and reduced-motion behavior.
- [x] Check the new pages at narrow/mobile and desktop widths; run accessibility checks and verify heading structure, footer links, and app Settings links.
- [x] Add `/privacy-requests` for private authenticated requests and replies, available without accepting updated terms. A signed-out contact route remains unresolved.
- [ ] Complete manual assistive-technology checks before claiming broad accessibility conformance.

## Confirmed choices and remaining facts

See the [decision record](legal/decisions.md) for all six choices and the account/acceptance contract. Canonical policies cover the WorkerCat website, CatDo and Blossom. Blossom has a separate Clerk account system; acceptance is recorded per product identity, against the same immutable document version.

The user skipped legal operator details and markets. These remain unresolved, as do provider-specific legal/retention arrangements and a private contact path for people unable to sign in. These are not silently inferred from deployment credentials.

## Verification

Run `pnpm check`, `pnpm test`, `pnpm build`, and `pnpm test:e2e`. Browser coverage includes canonical/social metadata, structured data, privacy redirects, sitemap destinations, accessibility, mobile layout, account controls and existing offline workflows. Server tests cover current-version acceptance, authentication boundaries, private requests and sync enforcement. Deployment and live HTTP checks are performed separately; no Search Console access or indexing outcome is implied.
