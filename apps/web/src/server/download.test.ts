import { afterEach, expect, it, vi } from "vitest";
import { androidAssetUrl, androidDownload } from "./download";
afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

it("selects the versioned Android APK from the latest release", () => {
  expect(
    androidAssetUrl({
      assets: [
        {
          name: "catdo-0.2.5-android.apk.sha256",
          browser_download_url:
            "https://github.com/workercatstudios/catdo/releases/download/v0.2.5/catdo-0.2.5-android.apk.sha256",
        },
        {
          name: "catdo-0.2.5-android.apk",
          browser_download_url:
            "https://github.com/workercatstudios/catdo/releases/download/v0.2.5/catdo-0.2.5-android.apk",
        },
      ],
    }),
  ).toBe(
    "https://github.com/workercatstudios/catdo/releases/download/v0.2.5/catdo-0.2.5-android.apk",
  );
});

it("cancels a stalled release lookup and returns a retryable failure", async () => {
  // AbortSignal.timeout uses runtime timers, so supply a controllable signal.
  const controller = new AbortController();
  const timeout = vi
    .spyOn(AbortSignal, "timeout")
    .mockReturnValue(controller.signal);
  vi.stubGlobal(
    "fetch",
    vi.fn(
      (_url, init: RequestInit) =>
        new Promise((_, reject) => {
          init.signal!.addEventListener("abort", () =>
            reject(new DOMException("Aborted", "AbortError")),
          );
        }),
    ),
  );
  const pending = androidDownload(
    new Request("https://catdo.example/download/android", { method: "HEAD" }),
  );
  controller.abort();
  const response = await pending;
  expect(timeout).toHaveBeenCalledWith(5000);
  expect(response.status).toBe(503);
  expect(response.headers.get("Retry-After")).toBe("30");
  expect(response.headers.get("Cache-Control")).toBe("no-store");
  expect(await response.text()).toBe("");
  timeout.mockRestore();
});

it("rejects absent and untrusted APK URLs", () => {
  expect(androidAssetUrl({ assets: [] })).toBeUndefined();
  expect(
    androidAssetUrl({
      assets: [
        {
          name: "catdo-0.2.5-android.apk",
          browser_download_url: "https://example.com/catdo-0.2.5-android.apk",
        },
      ],
    }),
  ).toBeUndefined();
});
