import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { seed, openOffline } from "./fixtures";

test("public HTML, metadata, links, status codes and mobile layout", async ({
  page,
  request,
}) => {
  const routes = [
    "/",
    "/help/getting-started",
    "/help/dates-and-repeats",
    "/help/sync-and-offline",
    "/help/keyboard-shortcuts",
    "/privacy",
    "/support",
  ];
  const titles = new Set<string>();
  for (const route of routes) {
    const response = await request.get(route);
    expect(response.status()).toBe(200);
    const html = await response.text();
    expect(html).toContain("<h1");
    expect(html).toContain('rel="canonical"');
    expect(html).toContain('property="og:image"');
    expect(html).toContain(`href="https://catdo.workercat.com${route}"`);
    expect(html).toContain('name="twitter:title"');
    expect(html).toContain('name="twitter:image"');
    const title = html.match(/<title>(.*?)<\/title>/)?.[1];
    expect(title).toBeTruthy();
    expect(titles.has(title!)).toBe(false);
    titles.add(title!);
  }
  for (const [route, location] of [
    ["/features", "/#features"],
    ["/download", "/#download"],
    ["/help", "/#help"],
    ["/changelog", "https://github.com/workercatstudios/catdo/releases"],
    ["/help/privacy", "/privacy"],
  ]) {
    const response = await request.get(route, { maxRedirects: 0 });
    expect(response.status()).toBe(301);
    expect(response.headers().location).toBe(location);
  }
  expect((await request.get("/missing-page")).status()).toBe(404);
  expect((await request.get("/help/missing-guide")).status()).toBe(404);
  const app = await request.get("/app");
  expect(app.headers()["cache-control"]).toBe("no-store");
  expect(app.headers()["x-robots-tag"]).toContain("noindex");
  expect(await app.text()).toContain("noindex");
  expect((await request.get("/api/config")).headers()["cache-control"]).toBe(
    "no-store",
  );
  expect([401, 503]).toContain((await request.get("/api/sync")).status());
  const sitemap = await (await request.get("/sitemap.xml")).text();
  expect(sitemap).not.toContain("/app");
  expect(sitemap).toContain("/privacy");
  expect(sitemap).toContain("/support");
  expect(sitemap).not.toContain("/help/privacy");
  expect(sitemap).not.toContain("/features");
  expect(sitemap.match(/<loc>/g)).toHaveLength(7);
  expect((await request.get("/social.png")).status()).toBe(200);
  for (const width of [1440, 390, 320]) {
    await page.setViewportSize({ width, height: 844 });
    for (const route of ["/", "/privacy", "/support"]) {
      await page.goto(route);
      expect(
        await page.evaluate(
          () => document.documentElement.scrollWidth <= innerWidth,
        ),
        `${route} at ${width}px`,
      ).toBe(true);
    }
  }
  await page.goto("/");
  await expect(
    page.getByRole("img", { name: /CatDo’s Today view/ }),
  ).toBeVisible();
  expect(
    await page
      .locator('.screenshot-layer[data-active="true"] img')
      .evaluate(
        (img: HTMLImageElement) => img.complete && img.naturalWidth > 0,
      ),
  ).toBe(true);
  expect(await page.locator("body").innerText()).not.toContain("—");
  await page.getByText("Is CatDo free?", { exact: true }).click();
  await expect(
    page.getByText("Yes. All current features are free to use.", {
      exact: false,
    }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Your calendar", exact: true })
    .click();
  await expect(
    page.getByRole("img", { name: /CatDo’s calendar/ }),
  ).toBeVisible();
  await expect(
    page.getByText("See what’s coming. Make room for it.", { exact: true }),
  ).toBeVisible();
  const projectButton = page.getByRole("button", {
    name: "Your projects",
    exact: true,
  });
  await projectButton.focus();
  await page.keyboard.press("Enter");
  await expect(projectButton).toHaveAttribute("aria-pressed", "true");
  expect(
    await page
      .locator('.screenshot-layer[data-active="true"]')
      .evaluate((el) => getComputedStyle(el).transitionDuration),
  ).toBe("0s");
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.getByRole("button", { name: "Your day", exact: true }).click();
  expect(
    await page
      .locator('.screenshot-layer[data-active="true"]')
      .evaluate((el) => getComputedStyle(el).transform),
  ).toBe("none");
  await page.getByRole("button", { name: "Open menu", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByRole("link", { name: "Download", exact: true })
    .click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page).toHaveURL(/#download$/);
  const axe = await new AxeBuilder({ page }).analyze();
  expect(axe.violations).toEqual([]);
});

test("privacy and support are discoverable, readable and accessible", async ({
  page,
  request,
}) => {
  await page.goto("/");
  const footer = page.getByRole("navigation", { name: "Footer", exact: true });
  await expect(
    footer.getByRole("link", { name: "Privacy & your data", exact: true }),
  ).toHaveAttribute("href", "/privacy");
  await expect(
    footer.getByRole("link", { name: "Support", exact: true }),
  ).toHaveAttribute("href", "/support");
  await expect(
    footer.getByRole("link", { name: "Terms", exact: true }),
  ).toHaveAttribute("href", "https://workercat.com/legal/2026-09-26/terms");
  await expect(
    footer.getByRole("link", { name: "Privacy policy", exact: true }),
  ).toHaveAttribute("href", "https://workercat.com/legal/2026-09-26/privacy");
  const requests = await request.get("/privacy-requests");
  expect(requests.headers()["cache-control"]).toBe("no-store");
  expect(requests.headers()["x-robots-tag"]).toContain("noindex");
  expect(await requests.text()).toContain("noindex");
  const data = await page
    .locator('script[type="application/ld+json"]')
    .first()
    .textContent();
  const graph = JSON.parse(data!)["@graph"];
  expect(
    graph.find((item: { "@type": string }) => item["@type"] === "WebSite"),
  ).toMatchObject({ name: "CatDo", url: "https://catdo.workercat.com/" });
  expect(
    graph.find(
      (item: { "@type": string }) => item["@type"] === "SoftwareApplication",
    ),
  ).toMatchObject({ offers: { price: "0" } });

  for (const path of ["/privacy", "/support"]) {
    await page.goto(path);
    await expect(page.locator("h1")).toHaveCount(1);
    await expect(page.locator("main h2").first()).toBeVisible();
    expect(await page.locator("main details").count()).toBe(0);
    expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  }
  await page.goto("/privacy");
  await expect(
    page.getByRole("heading", { name: "Cookies and storage on your device" }),
  ).toBeVisible();
  await expect(
    page.getByText(
      "Crash reporting is separate from that notification setting.",
      { exact: false },
    ),
  ).toBeVisible();

  const sitemap = await (await request.get("/sitemap.xml")).text();
  for (const location of sitemap.matchAll(/<loc>(.*?)<\/loc>/g)) {
    const path = new URL(location[1]).pathname;
    const response = await request.get(path, { maxRedirects: 0 });
    expect(response.status(), path).toBe(200);
    expect(response.headers()["x-robots-tag"] ?? "", path).not.toContain(
      "noindex",
    );
  }
  const robots = await (await request.get("/robots.txt")).text();
  expect(robots).toContain("Sitemap: https://catdo.workercat.com/sitemap.xml");
  // Let crawlers see the app's noindex; robots exclusion is not access control.
  expect(robots).not.toContain("Disallow: /app");
});

test("offline tasks, recurrence, subtasks, history, URLs, reload, theme and account separation", async ({
  page,
}) => {
  const data = await seed(page);
  await openOffline(page);
  await expect(
    page.getByRole("button", {
      name: "Plan the next small step Today",
      exact: false,
    }),
  ).toBeVisible();
  await page
    .getByRole("textbox", { name: "Quick add task" })
    .fill("Saved while offline");
  await page.getByRole("textbox", { name: "Quick add task" }).press("Enter");
  await expect(
    page.getByRole("button", {
      name: "Complete Saved while offline",
      exact: true,
    }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Inbox", exact: true }).click();
  await expect(page).toHaveURL(/\/workspaces\/.*\/inbox$/);
  await page.goBack();
  await expect(
    page.getByRole("heading", { name: "Today", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Add task", exact: true }).click();
  await page
    .getByRole("textbox", { name: "Task", exact: true })
    .fill("A recurring task");
  await page
    .getByRole("combobox", { name: "Repeat", exact: true })
    .selectOption("Days:false");
  await page.getByRole("button", { name: "Save task", exact: true }).click();
  await page
    .getByRole("button", { name: "Complete A recurring task", exact: true })
    .click();
  await page.getByRole("link", { name: "Upcoming", exact: true }).click();
  await expect(
    page.getByRole("button", {
      name: "Complete A recurring task",
      exact: true,
    }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Completed", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Completion history" }),
  ).toBeVisible();
  await expect(
    page.getByText("A recurring task", { exact: true }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Today", exact: false }).first().click();
  await page
    .getByRole("button", { name: "Saved while offline Today", exact: false })
    .click();
  await page.getByRole("button", { name: "Add subtask" }).click();
  await expect(
    page.getByRole("textbox", { name: "Task", exact: true }),
  ).toHaveValue("");
  await page
    .getByRole("textbox", { name: "Task", exact: true })
    .fill("A smaller step");
  await page.getByRole("button", { name: "Save task", exact: true }).click();
  await page.getByRole("link", { name: "Calendar", exact: true }).click();
  const path = new URL(page.url()).pathname;
  await page.reload();
  await page.getByRole("button", { name: "Open saved tasks" }).click();
  await expect(page).toHaveURL(new RegExp(path + "$"));
  await expect(
    page.getByRole("button", { name: "Previous month" }),
  ).toBeVisible();
  await page.getByLabel("Appearance", { exact: true }).selectOption("dark");
  await expect(page.locator("html")).toHaveClass("dark");
  await page.reload();
  await expect(page.locator("html")).toHaveClass("dark");
  await page.getByRole("button", { name: "Open saved tasks" }).click();
  await page.getByRole("link", { name: "Today", exact: false }).first().click();
  await expect(
    page.getByRole("button", {
      name: "Complete Saved while offline",
      exact: true,
    }),
  ).toBeVisible();
  await page.evaluate(() =>
    localStorage.setItem("catdo:last-user", "different-test-account"),
  );
  await page.reload();
  await page.getByRole("button", { name: "Open saved tasks" }).click();
  await expect(
    page.getByRole("button", {
      name: "Complete Saved while offline",
      exact: true,
    }),
  ).toHaveCount(0);
  expect(data.workspaces.length).toBeGreaterThan(0);
});

test("mobile navigation, editor focus, unsaved changes and reduced motion", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await seed(page);
  await openOffline(page);
  await page.getByRole("button", { name: "Open navigation" }).click();
  await expect(page.getByRole("dialog", { name: "Navigation" })).toBeVisible();
  await page
    .getByRole("link", { name: "Calendar", exact: true })
    .last()
    .click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.getByRole("button", { name: "Add task", exact: true }).click();
  await expect(
    page.getByRole("textbox", { name: "Task", exact: true }),
  ).toBeFocused();
  await page
    .getByRole("textbox", { name: "Task", exact: true })
    .fill("Don’t lose this");
  page.once("dialog", (dialog) => dialog.dismiss());
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toBeVisible();
  page.once("dialog", (dialog) => dialog.accept());
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.getByRole("button", { name: "Add task", exact: true }).click();
  expect(
    await page
      .getByRole("dialog")
      .evaluate((el) => getComputedStyle(el).transform),
  ).toBe("none");
  const axe = await new AxeBuilder({ page }).analyze();
  expect(axe.violations).toEqual([]);
});
