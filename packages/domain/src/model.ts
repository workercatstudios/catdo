import { z } from "zod";
const id = z.uuid();
const date = z.iso
  .date()
  .refine(
    (s) => new Date(`${s}T12:00:00Z`).toISOString().slice(0, 10) === s,
    "Invalid date",
  );
// Match chrono's RFC3339 precision (0, 3, 6 or 9 fractional digits),
// preserving nanoseconds from native clients instead of truncating through Date.
const timestamp = z.iso.datetime({ offset: true }).transform((value) => {
  const fraction = /\.(\d+)/.exec(value)?.[1] ?? "";
  let digits = fraction.padEnd(9, "0");
  while (digits.endsWith("000")) digits = digits.slice(0, -3);
  return (
    new Date(value).toISOString().slice(0, 19) +
    (digits ? "." + digits : "") +
    "Z"
  );
});
const name = z.string().trim().min(1).max(200);
export const recurrenceSchema = z
  .object({
    unit: z.enum(["Days", "Weeks", "Months", "Weekdays"]),
    interval: z.number().int().min(1).max(999),
    after_completion: z.boolean(),
    weekdays: z.array(z.number().int().min(0).max(6)).max(7),
    month_day: z.number().int().min(1).max(31),
  })
  .refine(
    (r) => r.unit !== "Weekdays" || r.weekdays.length > 0,
    "Choose at least one weekday",
  );
const workspaceSchema = z.object({
  id,
  name,
  archived: z.boolean().default(false),
});
const projectSchema = z.object({
  id,
  workspace_id: id,
  name,
  archived: z.boolean().default(false),
});
export const taskSchema = z.object({
  id,
  workspace_id: id,
  project_id: id.nullable(),
  parent_id: id.nullable(),
  title: name,
  notes: z.string().max(20000),
  scheduled: date.nullable(),
  due: date.nullable(),
  recurrence: recurrenceSchema.nullable(),
  reminder: z
    .object({ at: timestamp, delivered: z.boolean() })
    .nullable()
    .default(null),
  completed_at: timestamp.nullable(),
  created_at: timestamp,
});
export const dataSchema = z.object({
  workspaces: z.array(workspaceSchema).max(100),
  projects: z.array(projectSchema).max(2000),
  tasks: z.array(taskSchema).max(10000),
  history: z
    .array(
      z.object({
        id,
        task: taskSchema,
        completed_at: timestamp,
      }),
    )
    .max(20000),
});
export type Data = z.infer<typeof dataSchema>;
export type Task = z.infer<typeof taskSchema>;
export type Recurrence = z.infer<typeof recurrenceSchema>;
export const emptyData = (): Data => ({
  workspaces: [],
  projects: [],
  tasks: [],
  history: [],
});
export function initialData(): Data {
  return {
    ...emptyData(),
    workspaces: [
      { id: crypto.randomUUID(), name: "Personal", archived: false },
    ],
  };
}
export function validateData(input: unknown, allowEmpty = false): Data {
  const d = dataSchema.parse(input);
  const ids = new Set<string>();
  for (const rows of [d.workspaces, d.projects, d.tasks, d.history])
    for (const row of rows) {
      if (ids.has(row.id)) throw Error("Duplicate record ID");
      ids.add(row.id);
    }
  if (!d.workspaces.some((w) => !w.archived) && !(allowEmpty && ids.size === 0))
    throw Error("Keep at least one active workspace.");
  for (const p of d.projects)
    if (!d.workspaces.some((w) => w.id === p.workspace_id))
      throw Error("Project workspace does not exist.");
  const tasks = new Map(d.tasks.map((t) => [t.id, t]));
  for (const t of d.tasks) {
    if (!d.workspaces.some((w) => w.id === t.workspace_id))
      throw Error("Task workspace does not exist.");
    if (
      t.project_id &&
      !d.projects.some(
        (p) => p.id === t.project_id && p.workspace_id === t.workspace_id,
      )
    )
      throw Error("Project must belong to the task workspace.");
    let parent = t.parent_id;
    const seen = new Set([t.id]);
    while (parent) {
      if (seen.has(parent)) throw Error("A task cannot be its own ancestor.");
      seen.add(parent);
      const p = tasks.get(parent);
      if (
        !p ||
        p.workspace_id !== t.workspace_id ||
        p.project_id !== t.project_id
      )
        throw Error("Subtasks must stay with their parent.");
      parent = p.parent_id;
    }
    if (t.recurrence && !t.scheduled && !t.due)
      throw Error("Repeating tasks need a date.");
  }
  return d;
}
export function newTask(
  workspace_id: string,
  title = "",
  project_id: string | null = null,
  scheduled: string | null = null,
): Task {
  return {
    id: crypto.randomUUID(),
    workspace_id,
    project_id,
    parent_id: null,
    title,
    notes: "",
    scheduled,
    due: null,
    recurrence: null,
    reminder: null,
    completed_at: null,
    created_at: new Date().toISOString(),
  };
}
export function descendants(d: Data, id: string): Set<string> {
  const ids = new Set([id]);
  let count = 0;
  while (count !== ids.size) {
    count = ids.size;
    for (const t of d.tasks)
      if (t.parent_id && ids.has(t.parent_id)) ids.add(t.id);
  }
  ids.delete(id);
  return ids;
}
export function saveTask(d: Data, task: Task) {
  const children = descendants(d, task.id);
  d.tasks = d.tasks.map((t) =>
    children.has(t.id)
      ? { ...t, workspace_id: task.workspace_id, project_id: task.project_id }
      : t.id === task.id
        ? task
        : t,
  );
  if (!d.tasks.some((t) => t.id === task.id)) d.tasks.push(task);
  validateData(d);
}
export function removeTask(d: Data, id: string) {
  const children = descendants(d, id);
  d.tasks = d.tasks.filter((t) => t.id !== id && !children.has(t.id));
}
export function activeTask(d: Data, t: Task) {
  return (
    !t.completed_at &&
    !d.workspaces.find((w) => w.id === t.workspace_id)?.archived &&
    !d.projects.find((p) => p.id === t.project_id)?.archived
  );
}
export const localDay = (date = new Date()) =>
  `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
export function addDays(day: string, n: number) {
  const d = new Date(`${day}T12:00:00Z`);
  d.setUTCDate(d.getUTCDate() + n);
  return d.toISOString().slice(0, 10);
}
export function nextDate(
  rule: Recurrence,
  current: string,
  completed: string,
): string {
  recurrenceSchema.parse(rule);
  let candidate = rule.after_completion ? completed : current;
  const threshold = rule.after_completion
    ? completed
    : current > completed
      ? current
      : completed;
  for (let i = 0; i < 4000000; i++) {
    if (rule.unit === "Months") {
      const d = new Date(`${candidate}T12:00:00Z`);
      d.setUTCDate(1);
      d.setUTCMonth(d.getUTCMonth() + rule.interval);
      const last = new Date(
        Date.UTC(d.getUTCFullYear(), d.getUTCMonth() + 1, 0),
      ).getUTCDate();
      d.setUTCDate(
        Math.min(
          last,
          rule.after_completion ? Number(completed.slice(8)) : rule.month_day,
        ),
      );
      candidate = d.toISOString().slice(0, 10);
    } else
      candidate = addDays(
        candidate,
        rule.unit === "Weekdays"
          ? 1
          : rule.interval * (rule.unit === "Weeks" ? 7 : 1),
      );
    if (candidate.length !== 10)
      throw Error("Next occurrence is outside the supported date range.");
    if (
      candidate > threshold &&
      (rule.unit !== "Weekdays" ||
        rule.weekdays.includes(
          (new Date(`${candidate}T12:00:00Z`).getUTCDay() + 6) % 7,
        ))
    )
      return candidate;
  }
  throw Error("Next occurrence is outside the supported date range.");
}
export function completeTask(
  d: Data,
  id: string,
  at = new Date(),
  day = localDay(at),
) {
  const t = d.tasks.find((t) => t.id === id);
  if (!t) throw Error("Task no longer exists.");
  if (t.completed_at) {
    t.completed_at = null;
    return;
  }
  const before = structuredClone(t);
  if (t.recurrence) {
    const anchor = t.scheduled ?? t.due!;
    const next = nextDate(t.recurrence, anchor, day);
    const shift = Math.round(
      (Date.parse(next) - Date.parse(anchor)) / 86400000,
    );
    if (t.scheduled) t.scheduled = addDays(t.scheduled, shift);
    if (t.due) t.due = addDays(t.due, shift);
    if (t.reminder) {
      const date = new Date(t.reminder.at);
      const hours = date.getHours(),
        minutes = date.getMinutes();
      date.setDate(date.getDate() + shift);
      if (date.getHours() !== hours || date.getMinutes() !== minutes) {
        date.setHours(hours, minutes, 0, 0); // JS shifts through gaps; select the first available minute instead.
        const requested = hours * 60 + minutes;
        if (date.getHours() * 60 + date.getMinutes() > requested)
          date.setMinutes(0, 0, 0);
      }
      t.reminder = { at: date.toISOString(), delivered: false };
    }
  } else t.completed_at = at.toISOString();
  d.history.push({
    id: crypto.randomUUID(),
    task: before,
    completed_at: at.toISOString(),
  });
  validateData(d);
}
