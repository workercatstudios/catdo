import { z } from "zod";

export const legalPolicy = {
  version: "2026-09-26",
  termsUrl: "https://workercat.com/legal/2026-09-26/terms",
  privacyUrl: "https://workercat.com/legal/2026-09-26/privacy",
  minimumAge: 13,
} as const;

export const acceptanceSchema = z.strictObject({
  version: z.literal(legalPolicy.version),
  accepted: z.literal(true),
  ageConfirmed: z.literal(true),
});

export const termsRequired = {
  error:
    "Review and accept WorkerCat terms at https://catdo.workercat.com/app before syncing. Your local tasks are safe.",
  code: "terms_required",
  reviewUrl: "https://catdo.workercat.com/app",
  ...legalPolicy,
};
