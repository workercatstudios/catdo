const bytes = new TextEncoder();

async function key(secret: string) {
  return crypto.subtle.importKey(
    "raw",
    bytes.encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign", "verify"],
  );
}

function payload(owner: string, ticket: string) {
  return bytes.encode(`catdo-sync-events-v1:${owner}:${ticket}`);
}

export async function signEventTicket(
  secret: string,
  owner: string,
  ticket: string,
) {
  const signed = await crypto.subtle.sign(
    "HMAC",
    await key(secret),
    payload(owner, ticket),
  );
  return [...new Uint8Array(signed)]
    .map((byte) => byte.toString(16).padStart(2, "0"))
    .join("");
}

export async function verifyEventTicket(
  secret: string,
  owner: string,
  ticket: string,
  signature: string,
) {
  if (!/^[0-9a-f]{64}$/.test(signature)) return false;
  const signatureBytes = Uint8Array.from(signature.match(/.{2}/g)!, (hex) =>
    parseInt(hex, 16),
  );
  return crypto.subtle.verify(
    "HMAC",
    await key(secret),
    signatureBytes,
    payload(owner, ticket),
  );
}
