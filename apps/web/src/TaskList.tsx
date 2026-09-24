import type { Data, Task } from "../../../packages/domain/src/model";
import { addDays } from "../../../packages/domain/src/model";
import { CalendarDays, Flag, Repeat2, ListTree, Check } from "lucide-react";

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
                const project = data.projects.find(
                  (p) => p.id === t.project_id,
                );
                const children = data.tasks.filter((c) => c.parent_id === t.id);
                const scheduleVisible =
                  t.scheduled && view !== "today" && view !== "upcoming";
                return (
                  <div
                    className={`task-row ${t.completed_at ? "is-complete" : ""}`}
                    role="listitem"
                    key={t.id}
                  >
                    <button
                      className={`check ${t.completed_at ? "done" : ""}`}
                      aria-label={`${t.completed_at ? "Reopen" : "Complete"} ${t.title}`}
                      onClick={() => complete(t.id)}
                    >
                      <Check size={12} aria-hidden="true" />
                    </button>
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
                              <span className="project-name">
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
                              <span title="Repeating task">
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
                                : "due-date"
                            }
                          >
                            <Flag size={12} aria-hidden="true" />
                            {dateLabel(t.due, today)}
                          </span>
                        )}
                        {scheduleVisible && (
                          <span>
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
