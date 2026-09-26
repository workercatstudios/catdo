import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
import { seed } from "./fixtures";

test("age eligibility precedes sign-in and is an explicit unchecked choice", async ({
  page,
}) => {
  let configRequests = 0;
  await page.route("**/api/config", async (route) => {
    configRequests += 1;
    await route.fulfill({ status: 503, body: "Unavailable" });
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/app");
  await expect(
    page.getByRole("heading", { name: "Before you sign in" }),
  ).toBeVisible();
  const eligibility = page.getByRole("checkbox", { name: /I’m at least 13/ });
  const proceed = page.getByRole("button", { name: "Continue to sign in" });
  await expect(eligibility).not.toBeChecked();
  await expect(proceed).toBeDisabled();
  expect(configRequests).toBe(0);
  await expect(
    page.getByRole("link", { name: "Terms of service" }),
  ).toHaveAttribute("href", "https://workercat.com/legal/2026-09-26/terms");
  await expect(
    page.getByRole("link", { name: "Privacy requests", exact: true }),
  ).toHaveAttribute("href", "/privacy-requests");
  expect((await new AxeBuilder({ page }).analyze()).violations).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await eligibility.check();
  await expect(proceed).toBeEnabled();
  expect(configRequests).toBe(0);
  await proceed.click();
  await expect(
    page.getByRole("heading", { name: "Connection unavailable." }),
  ).toBeVisible();
  expect(configRequests).toBe(1);
});

test("saved tasks and export remain available without confirming eligibility or terms", async ({
  page,
}) => {
  await seed(page);
  let apiRequests = 0;
  await page.route("**/api/**", async (route) => {
    apiRequests += 1;
    await route.abort();
  });
  await page.goto("/app");
  await expect(
    page.getByRole("checkbox", { name: /I’m at least 13/ }),
  ).not.toBeChecked();
  await page
    .getByRole("button", { name: "Open saved tasks", exact: true })
    .click();
  await expect(
    page.getByRole("button", {
      name: "Complete Plan the next small step",
      exact: true,
    }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export all tasks" }).click();
  expect((await download).suggestedFilename()).toBe("catdo-tasks.json");
  expect(apiRequests).toBe(0);
});

test("privacy account access does not require age or terms confirmation", async ({
  page,
}) => {
  await page.route("**/api/config", (route) =>
    route.fulfill({ status: 503, body: "Unavailable" }),
  );
  await page.goto("/privacy-requests");
  await expect(
    page.getByRole("heading", { name: "Connection unavailable." }),
  ).toBeVisible();
  await expect(page.getByRole("checkbox")).toHaveCount(0);
  await expect(
    page.getByText("Connect to sign in and manage your private requests."),
  ).toBeVisible();
});
