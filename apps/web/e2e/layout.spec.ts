import { test, expect } from "@playwright/test";
import { fixtureData, seed, openOffline } from "./fixtures";
import { addDays, localDay, newTask } from "../../../packages/domain/src/model";

test("busy days remain usable through the agenda and a narrow task editor", async ({
  page,
}) => {
  const data = fixtureData();
  const today = localDay();
  const busyDay = addDays(today, Number(today.slice(8)) < 28 ? 1 : -1);
  const longTitle =
    "Review the entire release checklist with the team and prepare the final notes for next week's launch";
  data.tasks[0].due = addDays(today, -1);
  for (let i = 0; i < 5; i++)
    data.tasks.push(
      newTask(
        data.workspaces[0].id,
        i === 4 ? longTitle : `Busy-day task ${i + 1}`,
        null,
        busyDay,
      ),
    );
  await seed(page, data);
  await openOffline(page);
  await expect(
    page
      .getByRole("list", { name: "Overdue", exact: true })
      .getByRole("button", { name: /Complete Plan the next small step/ }),
  ).toBeVisible();
  await page.getByRole("link", { name: "Calendar", exact: true }).click();
  await page
    .getByRole("button", { name: `Show all 5 tasks on ${busyDay}` })
    .click();
  const agenda = page.locator(".agenda");
  await expect(agenda.getByRole("button")).toHaveCount(5);
  await agenda
    .getByRole("button", { name: "Busy-day task 4", exact: false })
    .dragTo(
      page.getByRole("button", {
        name: `Show tasks for ${today}`,
        exact: true,
      }),
    );
  await expect(agenda.getByRole("button")).toHaveCount(4);
  await page
    .getByRole("button", { name: `Show tasks for ${today}`, exact: true })
    .click();
  await expect(
    agenda.getByRole("button", { name: "Busy-day task 4", exact: false }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: `Show tasks for ${busyDay}`, exact: true })
    .click();
  await agenda.getByRole("button", { name: longTitle, exact: false }).click();
  await page.setViewportSize({ width: 320, height: 640 });
  const title = page.getByRole("textbox", { name: "Task", exact: true });
  await expect(title).toHaveValue(longTitle);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  const save = page.getByRole("button", { name: "Save task", exact: true });
  await expect(save).toBeInViewport();
  await title.fill(`${longTitle} — confirmed`);
  await save.click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(
    agenda.getByRole("button", {
      name: `${longTitle} — confirmed`,
      exact: false,
    }),
  ).toBeVisible();
});
