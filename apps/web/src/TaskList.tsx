import type { Data, Task } from "../../../packages/domain/src/model";
import { addDays } from "../../../packages/domain/src/model";
import {
  CalendarDays,
  Flag,
  Repeat2,
  ListTree,
  Check,
  AlertCircle,
  Sun,
} from "lucide-react";
import { projectColorIndex } from "./lib/colors";
import { Burst } from "./components/ui/burst";
import { useEffect, useRef, useState } from "react";

const LEAVE_MS = 520;
const BURST_COLORS = [
  "var(--kk-pink, #ec5f8f)",
  "var(--kk-yellow, #f7d35c)",
  "var(--kk-sky, #5aa9e6)",
];
function immediateMotion() {
  return (
    document.documentElement.dataset.input === "keyboard" ||
    matchMedia("(prefers-reduced-motion: reduce)").matches
  );
}

function dateLabel(date: string, today: string) {
  if (date === today) return "Today";
  if (date === addDays(today, 1)) return "Tomorrow";
  return new Date(`${date}T12:00:00`).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
    ...(date.slice(0, 4) !== today.slice(0, 4)
      ? { year: "numeric" as const }
      : {}),
  });
}

export function TaskList({
  tasks,
  data,
  today,
  view,
  complete,
  open,
}: {
  tasks: Task[];
  data: Data;
  today: string;
  view: string;
  complete: (id: string) => void;
  open: (task: Task) => void;
}) {
  // Completing a task lets the check fill and the row fade before it leaves.
  // Anything still pending when the list unmounts completes immediately.
  const [leaving, setLeaving] = useState<string[]>([]);
  const pending = useRef(new Map<string, ReturnType<typeof setTimeout>>());
  const completeRef = useRef(complete);
  completeRef.current = complete;
  useEffect(() => {
    const timers = pending.current;
    return () => {
      for (const [id, timer] of timers) {
        clearTimeout(timer);
        completeRef.current(id);
      }
      timers.clear();
    };
  }, []);
  const finish = (task: Task) => {
    if (task.completed_at || immediateMotion() || pending.current.has(task.id))
      return pending.current.has(task.id) ? undefined : complete(task.id);
    setLeaving((ids) => [...ids, task.id]);
    pending.current.set(
      task.id,
      setTimeout(() => {
        pending.current.delete(task.id);
        setLeaving((ids) => ids.filter((id) => id !== task.id));
        completeRef.current(task.id);
      }, LEAVE_MS),
    );
  };
  let index = 0;
  const groups: { title: string; tasks: Task[]; overdue?: boolean }[] = [];
  if (view === "today" && tasks.some((t) => t.due && t.due < today)) {
    groups.push({
      title: "Overdue",
      tasks: tasks.filter((t) => t.due && t.due < today),
      overdue: true,
    });
    groups.push({
      title: "Today",
      tasks: tasks.filter((t) => !t.due || t.due >= today),
    });
  } else if (view === "upcoming") {
    const dates = new Map<string, Task[]>();
    for (const task of tasks) {
      const date =
        task.scheduled && task.scheduled > today
          ? task.scheduled
          : (task.due ?? task.scheduled ?? "");
      dates.set(date, [...(dates.get(date) ?? []), task]);
    }
    for (const [date, entries] of [...dates].sort(([a], [b]) =>
      a.localeCompare(b),
    ))
      groups.push({
        title: date ? dateLabel(date, today) : "Unscheduled",
        tasks: entries,
      });
  } else groups.push({ title: "", tasks });
  return (
    <div className="task-groups">
      {groups
        .filter((g) => g.tasks.length)
        .map((group) => (
          <section className="task-group" key={group.title}>
            {group.title && (
              <div
                className={`task-group-heading ${group.overdue ? "deadline" : ""}`}
              >
                {group.overdue && <AlertCircle aria-hidden="true" />}
                {view === "today" && !group.overdue && group.title && (
                  <Sun aria-hidden="true" style={{ color: "var(--today)" }} />
                )}
                <h2>{group.title}</h2>
                <span>{group.tasks.length}</span>
              </div>
            )}
            <div
              className="task-list"
              role="list"
              aria-label={group.title || "Tasks"}
            >
              {group.tasks.map((t) => {
                const order = index++;
                const project = data.projects.find(
                  (p) => p.id === t.project_id,
                );
                const children = data.tasks.filter((c) => c.parent_id === t.id);
                const scheduleVisible =
                  t.scheduled && view !== "today" && view !== "upcoming";
                return (
                  <div
                    className={`task-row ${t.completed_at ? "is-complete" : ""} ${leaving.includes(t.id) ? "is-leaving" : ""}`}
                    role="listitem"
                    key={t.id}
                    style={{ "--i": order } as React.CSSProperties}
                  >
                    <Burst
                      className="check-burst"
                      effects={["ring", "confetti"]}
                      colors={BURST_COLORS}
                      size={0.62}
                      fire={leaving.includes(t.id) ? 1 : 0}
                    >
                      <button
                        className={`check ${t.completed_at ? "done" : ""}`}
                        aria-label={`${t.completed_at ? "Reopen" : "Complete"} ${t.title}`}
                        onClick={() => finish(t)}
                      >
                        <Check size={12} strokeWidth={3} aria-hidden="true" />
                      </button>
                    </Burst>
                    <button
                      className="task-body"
                      onClick={() => open(t)}
                      aria-label={[
                        `${t.title}${t.scheduled ? ` ${dateLabel(t.scheduled, today)}` : ""}`,
                        t.due && `Due ${dateLabel(t.due, today)}`,
                        project?.name,
                        t.recurrence && "Repeats",
                        children.length > 0 &&
                          `${children.filter((c) => c.completed_at).length} of ${children.length} subtasks complete`,
                        t.parent_id && "Subtask",
                      ]
                        .filter(Boolean)
                        .join(" · ")}
                      aria-describedby={
                        t.notes.trim() ? `task-note-${t.id}` : undefined
                      }
                    >
                      <span className="task-copy">
                        <span
                          className={`task-title ${t.completed_at ? "finished" : ""}`}
                        >
                          {t.title}
                        </span>
                        {(t.notes.trim() ||
                          (project && !view.startsWith("project:"))) && (
                          <span className="task-context">
                            {project && !view.startsWith("project:") && (
                              <span
                                className="project-name"
                                style={
                                  {
                                    "--project-color": `var(--c-p${projectColorIndex(project.id)})`,
                                  } as React.CSSProperties
                                }
                              >
                                <span
                                  className="color-dot"
                                  aria-hidden="true"
                                />
                                {project.name}
                              </span>
                            )}
                            {t.notes.trim() && (
                              <span
                                className="task-note"
                                id={`task-note-${t.id}`}
                              >
                                {t.notes}
                              </span>
                            )}
                          </span>
                        )}
                        {(t.recurrence ||
                          children.length > 0 ||
                          t.parent_id) && (
                          <span className="task-indicators">
                            {t.recurrence && (
                              <span className="repeat" title="Repeating task">
                                <Repeat2 size={12} />
                                <span className="sr-only">Repeats</span>
                              </span>
                            )}
                            {children.length > 0 && (
                              <span>
                                <ListTree size={12} />
                                {children.filter((c) => c.completed_at).length}/
                                {children.length}
                              </span>
                            )}
                            {t.parent_id && <span>Subtask</span>}
                          </span>
                        )}
                      </span>
                      <span className="task-meta">
                        {t.due && (
                          <span
                            className={
                              !t.completed_at && t.due < today
                                ? "deadline"
                                : !t.completed_at && t.due === today
                                  ? "due-today"
                                  : "due-date"
                            }
                          >
                            <Flag size={12} aria-hidden="true" />
                            {dateLabel(t.due, today)}
                          </span>
                        )}
                        {scheduleVisible && (
                          <span className="scheduled">
                            <CalendarDays size={12} aria-hidden="true" />
                            {dateLabel(t.scheduled!, today)}
                          </span>
                        )}
                      </span>
                    </button>
                  </div>
                );
              })}
            </div>
          </section>
        ))}
    </div>
  );
}
