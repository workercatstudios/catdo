import { expect, it } from "vitest";
import { androidAssetUrl } from "./download";

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
