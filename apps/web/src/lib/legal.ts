export const legalVersion = "2026-09-26";
export const termsUrl = `https://workercat.com/legal/${legalVersion}/terms`;
export const privacyUrl = `https://workercat.com/legal/${legalVersion}/privacy`;
export type GetToken = () => Promise<string | null>;
export type LegalStatus = {
  version: string;
  termsUrl: string;
  privacyUrl: string;
  minimumAge: number;
  accepted: boolean;
  acceptedAt?: string;
};

export async function accountRequest<T>(
  getToken: GetToken,
  path: string,
  body?: unknown,
): Promise<T> {
  const token = await getToken();
  if (!token) throw new Error("Sign in again to continue.");
  const response = await fetch(path, {
    method: body === undefined ? "GET" : "POST",
    headers: {
      Authorization: `Bearer ${token}`,
      ...(body === undefined ? {} : { "Content-Type": "application/json" }),
    },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    signal: AbortSignal.timeout(15000),
  });
  if (!response.ok) {
    if (response.status === 401) throw new Error("Sign in again to continue.");
    if (response.status === 409)
      throw new Error(
        "The terms have changed. Reload this page to review them.",
      );
    throw new Error("Couldn’t complete your request. Please try again.");
  }
  return response.json() as Promise<T>;
}

export function downloadJson(value: unknown, filename: string) {
  const url = URL.createObjectURL(
    new Blob([JSON.stringify(value, null, 2)], { type: "application/json" }),
  );
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
