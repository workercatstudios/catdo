import { expect, it } from "vitest";
import { appPath, parseAppPath } from "./app-route";
it("round trips every view, project and workspace without ambiguous paths", () => {
  for (const view of [
    "today",
    "inbox",
    "upcoming",
    "calendar",
    "completed",
    "settings",
    "project:a/b",
  ]) {
    expect(parseAppPath(appPath("work / life", view))).toEqual({
      workspace: "work / life",
      view,
      valid: true,
    });
  }
});
it("rejects malformed links and keeps the existing app entry", () => {
  expect(parseAppPath("/app")).toEqual({
    workspace: "",
    view: "today",
    valid: true,
  });
  for (const path of [
    "/application",
    "/app/workspaces/a/anything",
    "/app/workspaces/a/projects",
    "/app/workspaces/a/today/extra",
    "/app/workspaces/%XX/today",
  ])
    expect(parseAppPath(path).valid).toBe(false);
});
