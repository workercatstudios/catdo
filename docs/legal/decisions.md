# WorkerCat policy decisions

Confirmed by Kaf during the product readiness review and authorised for implementation, publication and deployment across WorkerCat, CatDo and Blossom.

| Choice        | Direction                                                                               |
| ------------- | --------------------------------------------------------------------------------------- |
| 1B            | WorkerCat-wide terms and privacy policy with verified product sections                  |
| 2A            | Personal and workplace use of hosted services; source licences remain separate          |
| 3C, clarified | Ages 13+ only, with higher local eligibility and guardian requirements still applicable |
| 4A            | Keep primary cloud content until deletion/request; no automatic inactivity expiry       |
| 5B            | Keep automatic CatDo Android crash reporting and disclose it                            |
| 6A            | Explicit acceptance of a published, recorded terms version                              |

Operator legal name/type, country, private contact email and intended markets were requested and **skipped by Kaf**. No legal entity, mailbox, address or governing jurisdiction is inferred. The subsequent implementation request explicitly includes the three repositories, establishing the covered product inventory.

## Canonical documents

The WorkerCat site owns the source and immutable version `2026-09-26`:

- [Terms](https://workercat.com/legal/2026-09-26/terms)
- [Privacy policy](https://workercat.com/legal/2026-09-26/privacy)
- Latest aliases: <https://workercat.com/terms> and <https://workercat.com/privacy>.

Source files live in the WorkerCat repository under `apps/site/src/legal/2026-09-26/`. Change a published policy by creating a new version; preserve previous text and receipts. CatDo's `/privacy` supplies additional product details and links to the central policy. The earlier draft files are superseded by these canonical documents.

## Identity and acceptance

CatDo's web, desktop and Android clients use the same CatDo account. Blossom uses a separate Clerk application, plus guest browser identities for collaboration. A common policy version does not imply a universal account or transfer acceptance between unrelated identities.

CatDo records acceptance in the authenticated account's Durable Object. The server records the version, document references, time and separate eligibility declaration; the client cannot choose another identity or acceptance time. Repeat acceptance is idempotent. New versions retain previous records. POST sync requires the current receipt; account lookup, cloud export and private requests remain accessible without accepting. Native clients direct users to CatDo's web app to review terms for that same account.

The sign-in UI asks for 13+ eligibility before offering registration. Terms agreement uses an unchecked explicit control. This is a declaration, not identity-based age verification or evidence of parental permission. Do not collect full birth dates or identity documents without a demonstrated need and retention plan.

Declining does not erase saved work. Users can access and export local tasks, export the cloud snapshot, read policies, sign out and submit private requests. Offline app access still works.

## Private requests and retention

CatDo's `/privacy-requests` accepts authenticated access, correction, deletion and other requests, with private replies. Its queue and operator access are separate from public GitHub issues. See the [privacy operations runbook](../privacy-operations.md) for secure queue access and request fulfilment. Do not resolve a deletion request until the relevant deletion has actually been completed; deleting CatDo data is distinct from deleting an authentication account or another product's data.

Primary content has no inactivity expiry. Blossom's transient Assist artifacts, optional diagnostics and recovery copies retain their existing separate lifetimes. Provider-managed logs, account records and backups need a provider-settings review before promising exact lifetimes. Automatic Android crash reporting remains separate from notification permission.

## Remaining limitations

Publishing factual policies and enforcing the chosen product rules does not resolve the skipped operator identity, markets, or jurisdiction-specific legal requirements. A general private contact route for people unable to sign in is still absent. Provider retention and international-transfer arrangements need review against actual contracts/settings. No legal-compliance certification is claimed.

Starting references include the [FTC children's privacy guidance](https://www.ftc.gov/business-guidance/resources/complying-coppa-frequently-asked-questions) and [ANPD guidance on children and adolescents](https://www.gov.br/anpd/pt-br/assuntos/noticias/anpd-divulga-enunciado-sobre-o-tratamento-de-dados-pessoais-de-criancas-e-adolescentes). The product's 13+ minimum does not settle every country's eligibility or consent requirements.
