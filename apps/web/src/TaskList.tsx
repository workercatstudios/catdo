import type { Data, Task } from "../../../packages/domain/src/model";
import { Icon } from "./icons";
export function TaskList({
  tasks,
  data,
  today,
  complete,
  open,
}: {
  tasks: Task[];
  data: Data;
  today: string;
  complete: (id: string) => void;
  open: (task: Task) => void;
}) {
  return (
    <div className="task-list">
      {tasks.map((t) => (
        <div className="task-row" key={t.id}>
          <button
            className={`check ${t.completed_at ? "done" : ""}`}
            aria-label={`${t.completed_at ? "Reopen" : "Complete"} ${t.title}`}
            onClick={() => complete(t.id)}
          >
            {t.completed_at ? "✓" : ""}
          </button>
          <button className="task-body" onClick={() => open(t)}>
            <span className={t.completed_at ? "finished" : ""}>{t.title}</span>
            <span className="task-meta">
              {t.scheduled && (
                <span>{t.scheduled === today ? "Today" : t.scheduled}</span>
              )}
              {t.due && (
                <span className={t.due < today ? "deadline" : ""}>
                  ◆ Due {t.due}
                </span>
              )}
              {t.recurrence && <Icon name="repeat" />}
              {t.parent_id && <span>Subtask</span>}
              {data.tasks.some((c) => c.parent_id === t.id) && (
                <span>
                  {data.tasks.filter((c) => c.parent_id === t.id).length}{" "}
                  subtasks
                </span>
              )}
            </span>
          </button>
          <span className="project-name">
            {data.projects.find((p) => p.id === t.project_id)?.name}
          </span>
        </div>
      ))}
    </div>
  );
}
