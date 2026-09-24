import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogDescription,
} from "./components/ui/dialog";
import { Button } from "./components/ui/button";
import { Input, Textarea } from "./components/ui/input";
import { useEffect, useState } from "react";
import { Check, Circle, Plus, Trash2 } from "lucide-react";
import {
  type Data,
  type Task,
  saveTask,
  newTask,
  removeTask,
} from "../../../packages/domain/src/model";
import { equal } from "../../../packages/domain/src/sync";
export function TaskEditor({
  task,
  data,
  save,
  close,
  dismiss,
  open,
  onDirty,
}: {
  task: Task;
  data: Data;
  save: (fn: (d: Data) => void) => Promise<void>;
  close: () => void;
  dismiss: () => void;
  open: (task: Task) => void;
  onDirty: (dirty: boolean) => void;
}) {
  const [wasExisting] = useState(() =>
    data.tasks.some((t) => t.id === task.id),
  );
  const [draft, setDraft] = useState(task),
    [error, setError] = useState(""),
    [saving, setSaving] = useState(false);
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
    setSaving(true);
    try {
      await persist();
      close();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Could not save.");
    } finally {
      setSaving(false);
    }
  };
  const repeat = draft.recurrence;
  return (
    <Dialog
      open
      onOpenChange={(isOpen) => {
        if (!isOpen && !saving) dismiss();
      }}
    >
      <DialogContent
        className="editor"
        onInteractOutside={(e) => e.preventDefault()}
      >
        <header className="editor-header">
          <DialogTitle>Task details</DialogTitle>
        </header>
        <DialogDescription className="sr-only">
          Edit your task, dates, recurrence, and subtasks.
        </DialogDescription>
        <form onSubmit={submit}>
          <fieldset disabled={saving} className="editor-fields">
            <div className="editor-writing">
              <label className="editor-title-field">
                <span className="sr-only">Task</span>
                <Textarea
                  className="editor-title-input"
                  autoFocus
                  required
                  rows={1}
                  maxLength={200}
                  value={draft.title}
                  onChange={(e) => update({ title: e.target.value })}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !e.nativeEvent.isComposing) {
                      e.preventDefault();
                      e.currentTarget.form?.requestSubmit();
                    }
                  }}
                  placeholder="What needs doing?"
                />
              </label>
              <label className="editor-notes-field">
                <span>Notes</span>
                <Textarea
                  rows={4}
                  value={draft.notes}
                  onChange={(e) => update({ notes: e.target.value })}
                  placeholder="A little context, if you need it."
                />
              </label>
              {draft.parent_id && (
                <Button
                  type="button"
                  className="text-button"
                  onClick={() => update({ parent_id: null })}
                >
                  Make a standalone task
                </Button>
              )}
              {wasExisting && (
                <div className="editor-subtasks">
                  <div className="editor-section-heading">Subtasks</div>
                  {data.tasks
                    .filter((t) => t.parent_id === task.id)
                    .map((child) => (
                      <Button
                        type="button"
                        variant="outline"
                        className="subtask-link"
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
                        {child.completed_at ? (
                          <Check size={16} aria-hidden="true" />
                        ) : (
                          <Circle size={16} aria-hidden="true" />
                        )}
                        <span>{child.title}</span>
                      </Button>
                    ))}
                  <Button
                    type="button"
                    variant="ghost"
                    className="editor-add-subtask"
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
                    <Plus size={16} aria-hidden="true" />
                    Add subtask
                  </Button>
                </div>
              )}
            </div>
            <div className="editor-properties">
              <div className="editor-property-group">
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
                      update({
                        project_id: e.target.value || null,
                        parent_id: null,
                      })
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
              </div>
              <div className="editor-property-group">
                <label>
                  Scheduled
                  <Input
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
                                  (
                                    e.target.value ||
                                    draft.due ||
                                    "2000-01-01"
                                  ).slice(8),
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
                  <Input
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
                  value={
                    repeat ? `${repeat.unit}:${repeat.after_completion}` : ""
                  }
                  onChange={(e) => {
                    const [unit, after] = e.target.value.split(":");
                    update({
                      recurrence: unit
                        ? {
                            unit: unit as NonNullable<
                              Task["recurrence"]
                            >["unit"],
                            after_completion: after === "true",
                            interval: repeat?.interval ?? 1,
                            month_day: Number(
                              (
                                draft.scheduled ??
                                draft.due ??
                                "2000-01-01"
                              ).slice(8),
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
                  <Input
                    type="number"
                    min="1"
                    max="999"
                    value={repeat.interval}
                    onChange={(e) =>
                      update({
                        recurrence: {
                          ...repeat,
                          interval: Number(e.target.value),
                        },
                      })
                    }
                  />
                </label>
              )}
              {repeat?.unit === "Weekdays" && (
                <div className="weekdays">
                  {["M", "T", "W", "T", "F", "S", "S"].map((day, i) => (
                    <Button
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
                    </Button>
                  ))}
                </div>
              )}
              <label>
                Reminder
                <Input
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
              <p className="field-help">
                Desktop reminders run while CatDo is open.
              </p>
            </div>
          </fieldset>
          {error && (
            <p role="alert" className="error">
              {error}
            </p>
          )}

          <div className="editor-actions">
            {wasExisting && (
              <Button
                type="button"
                variant="ghost"
                className="editor-delete"
                disabled={saving}
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
                <Trash2 size={15} aria-hidden="true" />
                Delete task
              </Button>
            )}
            <div className="editor-submit-actions">
              <Button
                type="button"
                disabled={saving}
                variant="outline"
                onClick={dismiss}
              >
                Cancel
              </Button>
              <Button type="submit" disabled={saving}>
                Save task
              </Button>
            </div>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
function toLocal(iso: string) {
  const d = new Date(iso);
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}T${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}
