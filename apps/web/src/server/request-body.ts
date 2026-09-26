export const MAX_SYNC_BYTES = 1_900_000;
const READ_TIMEOUT_MS = 15_000;

export class RequestBodyError extends Error {
  constructor(
    readonly status: number,
    message: string,
  ) {
    super(message);
  }
}

/** Bound bytes and elapsed time even when Content-Length is absent or false. */
export async function readSyncJson(request: Request): Promise<unknown> {
  const mediaType = request.headers
    .get("content-type")
    ?.split(";", 1)[0]
    .trim()
    .toLowerCase();
  if (mediaType !== "application/json")
    throw new RequestBodyError(415, "Expected JSON");
  if (Number(request.headers.get("content-length")) > MAX_SYNC_BYTES)
    throw new RequestBodyError(413, "Changes are too large to sync.");
  const reader = request.body?.getReader();
  if (!reader) throw new RequestBodyError(400, "Missing changes");
  let timer: ReturnType<typeof setTimeout> | undefined;
  const timeout = new Promise<never>((_, reject) => {
    timer = setTimeout(() => {
      reject(new RequestBodyError(408, "Sync upload timed out. Please retry."));
      void reader.cancel().catch(() => {});
    }, READ_TIMEOUT_MS);
  });
  try {
    const chunks: Uint8Array[] = [];
    let length = 0;
    while (true) {
      const { done, value } = await Promise.race([reader.read(), timeout]);
      if (done) break;
      length += value.byteLength;
      if (length > MAX_SYNC_BYTES)
        throw new RequestBodyError(413, "Changes are too large to sync.");
      chunks.push(value);
    }
    const bytes = new Uint8Array(length);
    let offset = 0;
    for (const chunk of chunks) {
      bytes.set(chunk, offset);
      offset += chunk.length;
    }
    return JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
  } catch (error) {
    void reader.cancel().catch(() => {});
    if (error instanceof RequestBodyError) throw error;
    throw new RequestBodyError(400, "Invalid sync request.");
  } finally {
    clearTimeout(timer);
    reader.releaseLock();
  }
}
