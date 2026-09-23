const releaseUrl =
  "https://api.github.com/repos/workercatstudios/catdo/releases/latest";
const apkName = /^catdo-\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?-android\.apk$/;

export function androidAssetUrl(release: unknown): string | undefined {
  if (!release || typeof release !== "object" || !("assets" in release))
    return undefined;
  const assets = release.assets;
  if (!Array.isArray(assets)) return undefined;
  for (const asset of assets) {
    if (
      !asset ||
      typeof asset.name !== "string" ||
      !apkName.test(asset.name) ||
      typeof asset.browser_download_url !== "string"
    )
      continue;
    let url: URL;
    try {
      url = new URL(asset.browser_download_url);
    } catch {
      continue;
    }
    if (
      url.protocol === "https:" &&
      url.hostname === "github.com" &&
      url.pathname.startsWith(
        "/workercatstudios/catdo/releases/download/",
      ) &&
      url.pathname.endsWith(`/${asset.name}`)
    )
      return url.href;
  }
  return undefined;
}

export async function androidDownload(request: Request): Promise<Response> {
  if (request.method !== "GET" && request.method !== "HEAD")
    return new Response("Method not allowed", {
      status: 405,
      headers: { Allow: "GET, HEAD" },
    });
  try {
    const response = await fetch(releaseUrl, {
      headers: {
        Accept: "application/vnd.github+json",
        "User-Agent": "CatDo-download",
      },
      cf: { cacheEverything: true, cacheTtl: 300 },
    } as RequestInit & { cf: { cacheEverything: boolean; cacheTtl: number } });
    if (!response.ok) throw new Error(`GitHub release API: ${response.status}`);
    const url = androidAssetUrl(await response.json());
    if (!url) throw new Error("Latest release has no versioned Android APK");
    return new Response(null, {
      status: 302,
      headers: {
        Location: url,
        "Cache-Control": "public, max-age=60, s-maxage=300",
      },
    });
  } catch (error) {
    console.error("Android download unavailable", error);
    return new Response("Android download temporarily unavailable", {
      status: 503,
      headers: { "Cache-Control": "no-store" },
    });
  }
}
