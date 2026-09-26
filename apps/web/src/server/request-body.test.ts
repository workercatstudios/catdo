import { afterEach, expect, it, vi } from "vitest";
import { MAX_SYNC_BYTES, readSyncJson } from "./request-body";

function request(body: BodyInit | null, headers: Record<string, string> = {}) {
  return new Request("https://catdo.example/api/sync", {
    method: "POST",
    headers: { "Content-Type": "application/json", ...headers },
    body,
    duplex: "half",
  } as RequestInit);
}
afterEach(() => vi.useRealTimers());

it("accepts JSON media types with casing and charset parameters", async () => {
  await expect(
    readSyncJson(
      request('{"title":"猫"}', {
        "Content-Type": "Application/JSON; charset=utf-8",
      }),
    ),
  ).resolves.toEqual({ title: "猫" });
});

it.each(["application/jsonp", "application/json-malicious", "text/plain"])(
  "rejects %s instead of matching a prefix",
  async (type) => {
    await expect(
      readSyncJson(request("{}", { "Content-Type": type })),
    ).rejects.toMatchObject({ status: 415 });
  },
);

it.each([null, "", "{bad json", new Uint8Array([0x22, 0xff, 0x22])])(
  "rejects missing, malformed, or non-UTF-8 JSON",
  async (body) => {
    await expect(readSyncJson(request(body))).rejects.toMatchObject({
      status: 400,
    });
  },
);

it("rejects oversized declared lengths before reading the body", async () => {
  await expect(
    readSyncJson(
      request("{}", {
        "Content-Length": String(MAX_SYNC_BYTES + 1),
      }),
    ),
  ).rejects.toMatchObject({ status: 413 });
});

it.each([undefined, "2"])(
  "bounds actual streamed bytes with Content-Length %s",
  async (length) => {
    const cancel = vi.fn();
    const body = new ReadableStream({
      start(controller) {
        controller.enqueue(new Uint8Array(MAX_SYNC_BYTES));
        controller.enqueue(new Uint8Array(1));
      },
      cancel,
    });
    await expect(
      readSyncJson(request(body, length ? { "Content-Length": length } : {})),
    ).rejects.toMatchObject({ status: 413 });
    expect(cancel).toHaveBeenCalledOnce();
  },
);

it("accepts the byte limit and counts multibyte text as bytes", async () => {
  await expect(
    readSyncJson(request('"' + "a".repeat(MAX_SYNC_BYTES - 2) + '"')),
  ).resolves.toHaveLength(MAX_SYNC_BYTES - 2);
  await expect(
    readSyncJson(request('"' + "猫".repeat(640_000) + '"')),
  ).rejects.toMatchObject({ status: 413 });
});

it("cancels stalled uploads and returns a retryable timeout", async () => {
  vi.useFakeTimers();
  const cancel = vi.fn();
  const result = readSyncJson(request(new ReadableStream({ cancel })));
  const assertion = expect(result).rejects.toMatchObject({ status: 408 });
  await vi.advanceTimersByTimeAsync(15_000);
  await assertion;
  expect(cancel).toHaveBeenCalledOnce();
  expect(vi.getTimerCount()).toBe(0);
});

it("handles an interrupted upload as a client error", async () => {
  const body = new ReadableStream({
    start(controller) {
      controller.error(new Error("connection lost"));
    },
  });
  await expect(readSyncJson(request(body))).rejects.toMatchObject({
    status: 400,
  });
});
