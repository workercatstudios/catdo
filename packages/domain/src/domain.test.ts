import { describe, it, expect } from "vitest";
import {
  initialData,
  newTask,
  saveTask,
  removeTask,
  completeTask,
  nextDate,
  validateData,
  type Recurrence,
} from "./model";
import { merge } from "./sync";
const setup = () => {
  const data = initialData();
  saveTask(data, newTask(data.workspaces[0].id, "Plan"));
  saveTask(data, newTask(data.workspaces[0].id, "Walk"));
  return data;
};
const rule: Recurrence = {
  unit: "Months",
  interval: 1,
  month_day: 31,
  after_completion: false,
  weekdays: [],
};
describe("recurrence and validation", () => {
  it("preserves month-end anchors and skips missed fixed occurrences", () => {
    expect(nextDate(rule, "2026-01-31", "2026-01-31")).toBe("2026-02-28");
    expect(nextDate(rule, "2026-02-28", "2026-02-28")).toBe("2026-03-31");
    expect(nextDate(rule, "2026-01-31", "2026-05-01")).toBe("2026-05-31");
  });
  it("uses actual completion dates for relative recurrence", () =>
    expect(
      nextDate({ ...rule, after_completion: true }, "2026-01-31", "2026-02-10"),
    ).toBe("2026-03-10"));
  it("keeps scheduled/deadline offsets and immutable history", () => {
    const d = setup(),
      t = d.tasks[0];
    t.scheduled = "2026-01-31";
    t.due = "2026-02-02";
    t.recurrence = rule;
    completeTask(d, t.id, new Date("2026-01-31T12:00:00Z"), "2026-01-31");
    expect(t.scheduled).toBe("2026-02-28");
    expect(t.due).toBe("2026-03-02");
    expect(d.history[0].task.scheduled).toBe("2026-01-31");
  });
  it("moves and deletes complete subtrees", () => {
    const d = setup(),
      parent = d.tasks[0],
      child = newTask(parent.workspace_id, "Child");
    child.parent_id = parent.id;
    saveTask(d, child);
    const other = crypto.randomUUID();
    d.workspaces.push({ id: other, name: "Work", archived: false });
    saveTask(d, { ...parent, workspace_id: other });
    expect(d.tasks.find((t) => t.id === child.id)?.workspace_id).toBe(other);
    removeTask(d, parent.id);
    expect(d.tasks.some((t) => t.id === child.id)).toBe(false);
  });
  it("rejects cycles, invalid dates and the last workspace archive", () => {
    const d = setup();
    d.tasks[0].parent_id = d.tasks[0].id;
    expect(() => validateData(d)).toThrow();
    d.tasks[0].parent_id = null;
    d.tasks[0].due = "2026-02-31";
    expect(() => validateData(d)).toThrow();
    d.tasks[0].due = null;
    d.workspaces[0].archived = true;
    expect(() => validateData(d)).toThrow();
  });
});
describe("three-way synchronization", () => {
  it("combines edits to different tasks and keeps remote deletions", () => {
    const base = setup(),
      local = structuredClone(base),
      remote = structuredClone(base);
    local.tasks[0].title = "Local";
    remote.tasks.splice(1, 1);
    const result = merge(base, local, remote);
    expect(result.conflicts).toEqual([]);
    expect(result.data.tasks.map((t) => t.title)).toEqual(["Local"]);
  });
  it("requires an explicit choice for edits versus deletions", () => {
    const base = setup(),
      local = structuredClone(base),
      remote = structuredClone(base);
    local.tasks[0].notes = "Keep this";
    remote.tasks.splice(0, 1);
    expect(merge(base, local, remote).conflicts).toHaveLength(1);
    expect(merge(base, local, remote, "remote").data.tasks).toHaveLength(1);
    expect(
      merge(base, local, remote, "local").data.tasks.find(
        (t) => t.id === base.tasks[0].id,
      )?.notes,
    ).toBe("Keep this");
  });
  it("preserves edits made during an in-flight upload", () => {
    const sent = setup(),
      local = structuredClone(sent),
      accepted = structuredClone(sent);
    local.tasks[0].title = "Typed while syncing";
    accepted.tasks[1].title = "Other device";
    expect(merge(sent, local, accepted).data.tasks.map((t) => t.title)).toEqual(
      ["Typed while syncing", "Other device"],
    );
  });
});
it("resolves concurrent completions as one occurrence, including its history", () => {
  const base = setup();
  base.tasks[0].scheduled = "2026-01-31";
  base.tasks[0].recurrence = rule;
  const local = structuredClone(base),
    remote = structuredClone(base);
  completeTask(
    local,
    local.tasks[0].id,
    new Date("2026-01-31T12:00:00Z"),
    "2026-01-31",
  );
  completeTask(
    remote,
    remote.tasks[0].id,
    new Date("2026-01-31T13:00:00Z"),
    "2026-01-31",
  );
  expect(merge(base, local, remote).conflicts).toHaveLength(1);
  expect(merge(base, local, remote, "remote").data.history).toEqual(
    remote.history,
  );
  expect(merge(base, local, remote, "local").data.history).toEqual(
    local.history,
  );
});
