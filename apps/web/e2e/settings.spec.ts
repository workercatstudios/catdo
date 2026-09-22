import { test, expect } from "@playwright/test";
import { seed, openOffline } from "./fixtures";
test("workspace and project management, undo, keyboard navigation and dirty history guard", async ({
  page,
}) => {
  await seed(page);
  await openOffline(page);
  await page
    .getByRole("button", { name: "New workspace", exact: true })
    .click();
  await page.getByRole("textbox", { name: "Name", exact: true }).fill("Work");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page
    .getByRole("combobox", { name: "Workspace", exact: true })
    .selectOption({ label: "Work" });
  await expect(
    page.getByRole("button", { name: "Complete Make time for a walk" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "New project", exact: true }).click();
  await page
    .getByRole("textbox", { name: "Name", exact: true })
    .fill("Release planning");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await page
    .getByRole("link", { name: "Release planning", exact: true })
    .click();
  await expect(page).toHaveURL(/\/projects\//);
  await page
    .getByRole("textbox", { name: "Quick add task" })
    .fill("Ship a small improvement");
  await page.getByRole("textbox", { name: "Quick add task" }).press("Enter");
  await page
    .getByRole("button", { name: "Complete Ship a small improvement" })
    .click();
  await expect(
    page.getByRole("button", { name: "Complete Ship a small improvement" }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Undo", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Complete Ship a small improvement" }),
  ).toBeVisible();
  await page.keyboard.press("Control+k");
  await expect(
    page.getByRole("textbox", { name: "Search workspace" }),
  ).toBeFocused();
  await page.keyboard.press("Control+Enter");
  await expect(
    page.getByRole("textbox", { name: "Task", exact: true }),
  ).toBeFocused();
  expect(
    await page
      .getByRole("dialog")
      .evaluate((el) => getComputedStyle(el).transitionDuration),
  ).toBe("0s");
  await page
    .getByRole("textbox", { name: "Task", exact: true })
    .fill("Still editing");
  const url = page.url();
  page.once("dialog", (d) => d.dismiss());
  await page.goBack();
  await expect(page).toHaveURL(url);
  await expect(
    page.getByRole("textbox", { name: "Task", exact: true }),
  ).toHaveValue("Still editing");
  page.once("dialog", (d) => d.accept());
  await page.keyboard.press("Escape");
  await page.getByRole("link", { name: "Settings", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Settings", exact: true }),
  ).toBeVisible();
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export all tasks" }).click();
  expect((await download).suggestedFilename()).toBe("catdo-tasks.json");
});
