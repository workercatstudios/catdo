import { useState } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";
import { Button } from "./components/ui/button";
import {
  type Task,
  addDays,
  localDay,
} from "../../../packages/domain/src/model";
export function Calendar({
  tasks,
  open,
  move,
}: {
  tasks: Task[];
  open: (t: Task) => void;
  move: (id: string, date: string) => void;
}) {
  const [month, setMonth] = useState(localDay().slice(0, 7) + "-01"),
    [selected, setSelected] = useState(localDay());
  const first = new Date(`${month}T12:00:00`);
  const start = addDays(month, -((first.getDay() + 6) % 7));
  const days = new Date(first.getFullYear(), first.getMonth() + 1, 0).getDate();
  const count = Math.ceil((days + ((first.getDay() + 6) % 7)) / 7) * 7;
  const shift = (by: number) => {
    const date = new Date(first);
    date.setMonth(date.getMonth() + by);
    setMonth(localDay(date));
  };
  return (
    <>
      <div className="calendar-controls">
        <h1>
          {first.toLocaleDateString(undefined, {
            month: "long",
            year: "numeric",
          })}
        </h1>
        <div>
          <Button
            variant="outline"
            size="icon"
            aria-label="Previous month"
            onClick={() => shift(-1)}
          >
            <ChevronLeft />
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => {
              setMonth(localDay().slice(0, 7) + "-01");
              setSelected(localDay());
            }}
          >
            Today
          </Button>
          <Button
            variant="outline"
            size="icon"
            aria-label="Next month"
            onClick={() => shift(1)}
          >
            <ChevronRight />
          </Button>
        </div>
      </div>
      <p className="muted calendar-key">
        Scheduled · <span className="deadline">◆ Due date</span>
        <span>Drag a scheduled task to move its date.</span>
      </p>
      <div className="calendar-layout">
        <div className="calendar-grid">
          {["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].map((day) => (
            <div className="weekday" key={day}>
              {day}
            </div>
          ))}
          {Array.from({ length: count }, (_, i) => {
            const date = addDays(start, i);
            const dayTasks = tasks.filter(
              (t) => t.scheduled === date || t.due === date,
            );
            return (
              <div
                key={date}
                className={`calendar-day ${date.slice(0, 7) !== month.slice(0, 7) ? "outside" : ""} ${date === selected ? "selected" : ""}`}
                onDragOver={(e) => e.preventDefault()}
                onDrop={(e) => {
                  e.preventDefault();
                  const id = e.dataTransfer.getData("text/plain");
                  if (tasks.some((t) => t.id === id && t.scheduled))
                    move(id, date);
                }}
              >
                <button
                  className={`day-number ${date === localDay() ? "today" : ""}`}
                  aria-label={`Show tasks for ${date}`}
                  aria-pressed={date === selected}
                  onClick={() => setSelected(date)}
                >
                  {Number(date.slice(8))}
                </button>
                {dayTasks.length > 0 && (
                  <button
                    className="calendar-count"
                    aria-label={`${dayTasks.length} tasks on ${date}`}
                    onClick={() => setSelected(date)}
                  >
                    {dayTasks.length}
                    <span className="sr-only"> tasks</span>
                  </button>
                )}
                <div className="calendar-entries">
                  {dayTasks.slice(0, 3).map((t) => (
                    <button
                      key={t.id}
                      className="calendar-task"
                      draggable={t.scheduled === date}
                      onDragStart={(e) =>
                        e.dataTransfer.setData("text/plain", t.id)
                      }
                      onClick={() => open(t)}
                      title={t.title}
                    >
                      {t.due === date && <span className="deadline">◆ </span>}
                      {t.title}
                    </button>
                  ))}
                  {dayTasks.length > 3 && (
                    <button
                      className="calendar-more"
                      onClick={() => setSelected(date)}
                      aria-label={`Show all ${dayTasks.length} tasks on ${date}`}
                    >
                      +{dayTasks.length - 3} more
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
        <div className="agenda">
          <h2>
            {new Date(`${selected}T12:00:00`).toLocaleDateString(undefined, {
              weekday: "long",
              month: "long",
              day: "numeric",
            })}
          </h2>
          {tasks
            .filter((t) => t.scheduled === selected || t.due === selected)
            .map((t) => (
              <button
                className="agenda-task"
                key={t.id}
                draggable={t.scheduled === selected}
                onDragStart={(e) => e.dataTransfer.setData("text/plain", t.id)}
                onClick={() => open(t)}
              >
                {t.title}
                <span>{t.due === selected ? "Due" : "Scheduled"}</span>
              </button>
            ))}
          {!tasks.some(
            (t) => t.scheduled === selected || t.due === selected,
          ) && <p className="muted">Nothing planned for this day.</p>}
        </div>
      </div>
    </>
  );
}
