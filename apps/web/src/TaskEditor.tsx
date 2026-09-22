import { useEffect, useState } from "react";
import {
  type Data,
  type Task,
  saveTask,
  newTask,
  removeTask,
} from "../../../packages/domain/src/model";
import { equal } from "../../../packages/domain/src/sync";
import { Icon } from "./icons";
export function TaskEditor({
  task,
  data,
  save,
  close,
  open,
  onDirty,
}: {
  task: Task;
  data: Data;
  save: (fn: (d: Data) => void) => Promise<void>;
  close: () => void;
  open: (task: Task) => void;
  onDirty: (dirty: boolean) => void;
}) {
  const [wasExisting] = useState(() =>
    data.tasks.some((t) => t.id === task.id),
  );
  const [draft, setDraft] = useState(task),
    [error, setError] = useState("");
  useEffect(() => onDirty(!equal(task, draft)), [task, draft, onDirty]);
  const update = (patch: Partial<Task>) =>
    setDraft((d) => ({ ...d, ...patch }));
  const persist = async () => {
    await save((d) => {
      const current = d.tasks.find((t) => t.id === task.id);
      if (current && !equal(current, task))
        throw Error(
          "This task changed on another device while you were editing. Copy your changes, close this panel, and reopen the task.",
        );
      if (wasExisting && !current)
        throw Error(
          "This task was deleted on another device. Copy your changes before closing.",
        );
      saveTask(d, draft);
    });
  };
  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await persist();
      close();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not save.");
    }
  };
  const repeat = draft.recurrence;
  return (
    <aside className="editor" aria-label="Task details">
      <form onSubmit={submit}>
        <div className="editor-head">
          <span>Task details</span>
          <button
            type="button"
            className="icon-button"
            aria-label="Close without saving"
            onClick={close}
          >
            <Icon name="close" />
          </button>
        </div>
        <label>
          Task
          <input
            autoFocus
            required
            maxLength={200}
            value={draft.title}
            onChange={(e) => update({ title: e.target.value })}
            placeholder="What needs doing?"
          />
        </label>
        <label>
          Notes
          <textarea
            rows={5}
            value={draft.notes}
            onChange={(e) => update({ notes: e.target.value })}
            placeholder="A little context, if you need it."
          />
        </label>
        <label>
          Workspace
          <select
            value={draft.workspace_id}
            onChange={(e) =>
              update({
                workspace_id: e.target.value,
                project_id: null,
                parent_id: null,
              })
            }
          >
            {data.workspaces.map((w) => (
              <option key={w.id} value={w.id}>
                {w.name}
                {w.archived ? " (archived)" : ""}
              </option>
            ))}
          </select>
        </label>
        <label>
          Project
          <select
            value={draft.project_id ?? ""}
            onChange={(e) =>
              update({ project_id: e.target.value || null, parent_id: null })
            }
          >
            <option value="">Inbox</option>
            {data.projects
              .filter((p) => p.workspace_id === draft.workspace_id)
              .map((p) => (
                <option key={p.id} value={p.id}>
                  {p.name}
                  {p.archived ? " (archived)" : ""}
                </option>
              ))}
          </select>
        </label>
        <div className="two-fields">
          <label>
            Scheduled
            <input
              type="date"
              value={draft.scheduled ?? ""}
              onChange={(e) =>
                update({
                  scheduled: e.target.value || null,
                  ...(repeat
                    ? {
                        recurrence: {
                          ...repeat,
                          month_day: Number(
                            (e.target.value || draft.due || "2000-01-01").slice(
                              8,
                            ),
                          ),
                        },
                      }
                    : {}),
                })
              }
            />
          </label>
          <label>
            Due date
            <input
              type="date"
              value={draft.due ?? ""}
              onChange={(e) =>
                update({
                  due: e.target.value || null,
                  ...(!draft.scheduled && repeat
                    ? {
                        recurrence: {
                          ...repeat,
                          month_day: Number(
                            (e.target.value || "2000-01-01").slice(8),
                          ),
                        },
                      }
                    : {}),
                })
              }
            />
          </label>
        </div>
        <p className="field-help">
          Scheduled is when you plan to work. Due is the deadline.
        </p>
        <label>
          Repeat
          <select
            value={repeat ? `${repeat.unit}:${repeat.after_completion}` : ""}
            onChange={(e) => {
              const [unit, after] = e.target.value.split(":");
              update({
                recurrence: unit
                  ? {
                      unit: unit as NonNullable<Task["recurrence"]>["unit"],
                      after_completion: after === "true",
                      interval: repeat?.interval ?? 1,
                      month_day: Number(
                        (draft.scheduled ?? draft.due ?? "2000-01-01").slice(8),
                      ),
                      weekdays: repeat?.weekdays.length
                        ? repeat.weekdays
                        : [0, 1, 2, 3, 4],
                    }
                  : null,
              });
            }}
          >
            <option value="">Does not repeat</option>
            <option value="Days:false">Every day</option>
            <option value="Weeks:false">Every week</option>
            <option value="Months:false">Every month</option>
            <option value="Weekdays:false">Selected weekdays</option>
            <option value="Days:true">Days after completion</option>
            <option value="Weeks:true">Weeks after completion</option>
            <option value="Months:true">Months after completion</option>
          </select>
        </label>
        {repeat && repeat.unit !== "Weekdays" && (
          <label>
            Interval
            <input
              type="number"
              min="1"
              max="999"
              value={repeat.interval}
              onChange={(e) =>
                update({
                  recurrence: { ...repeat, interval: Number(e.target.value) },
                })
              }
            />
          </label>
        )}
        {repeat?.unit === "Weekdays" && (
          <div className="weekdays">
            {["M", "T", "W", "T", "F", "S", "S"].map((day, i) => (
              <button
                key={i}
                type="button"
                aria-label={
                  [
                    "Monday",
                    "Tuesday",
                    "Wednesday",
                    "Thursday",
                    "Friday",
                    "Saturday",
                    "Sunday",
                  ][i]
                }
                aria-pressed={repeat.weekdays.includes(i)}
                onClick={() =>
                  update({
                    recurrence: {
                      ...repeat,
                      weekdays: repeat.weekdays.includes(i)
                        ? repeat.weekdays.filter((d) => d !== i)
                        : [...repeat.weekdays, i],
                    },
                  })
                }
              >
                {day}
              </button>
            ))}
          </div>
        )}
        <label>
          Reminder
          <input
            type="datetime-local"
            value={draft.reminder ? toLocal(draft.reminder.at) : ""}
            onChange={(e) =>
              update({
                reminder: e.target.value
                  ? {
                      at: new Date(e.target.value).toISOString(),
                      delivered: false,
                    }
                  : null,
              })
            }
          />
        </label>
        <p className="field-help">Desktop reminders run while CatDo is open.</p>
        {draft.parent_id && (
          <button
            type="button"
            className="text-button"
            onClick={() => update({ parent_id: null })}
          >
            Make a standalone task
          </button>
        )}
        {error && (
          <p role="alert" className="error">
            {error}
          </p>
        )}
        <div className="editor-actions">
          <button type="button" onClick={close}>
            Cancel
          </button>
          <button className="primary" type="submit">
            Save task
          </button>
        </div>
        {data.tasks.some((t) => t.id === task.id) && (
          <>
            <hr />
            {data.tasks
              .filter((t) => t.parent_id === task.id)
              .map((child) => (
                <button
                  type="button"
                  key={child.id}
                  onClick={async () => {
                    try {
                      await persist();
                      open(child);
                    } catch (e) {
                      setError(String(e));
                    }
                  }}
                >
                  {child.completed_at ? "✓ " : ""}
                  {child.title}
                </button>
              ))}
            <button
              type="button"
              onClick={async () => {
                try {
                  const child = newTask(
                    draft.workspace_id,
                    "",
                    draft.project_id,
                  );
                  child.parent_id = draft.id;
                  await persist();
                  open(child);
                } catch (e) {
                  setError(String(e));
                }
              }}
            >
              Add subtask
            </button>
            <button
              type="button"
              className="danger"
              onClick={async () => {
                if (!confirm("Delete this task and its subtasks?")) return;
                try {
                  await save((d) => removeTask(d, task.id));
                  close();
                } catch (e) {
                  setError(String(e));
                }
              }}
            >
              Delete task
            </button>
          </>
        )}
      </form>
    </aside>
  );
}
function toLocal(iso: string) {
  const d = new Date(iso);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}T${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}
