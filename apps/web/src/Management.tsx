import { Button } from "./components/ui/button";
import { ThemeControl } from "./lib/theme";
import type { Data } from "../../../packages/domain/src/model";
export function Management({
  data,
  naming,
  act,
  change,
  error,
  exportTasks,
}: {
  data: Data;
  naming: (kind: "workspace" | "project", id?: string) => void;
  act: (fn: () => Promise<unknown>) => void;
  change: (fn: (d: Data) => void) => Promise<void>;
  error: string;
  exportTasks: () => void;
}) {
  return (
    <section className="manage">
      <div className="page-heading">
        <p className="eyebrow">Make yourself at home</p>
        <h1>Settings</h1>
        <p className="muted">Your spaces, your preferences, your data.</p>
      </div>
      <ThemeControl />
      <h2>Workspaces & projects</h2>
      <p className="muted">
        Archives keep your tasks and history. Restore them whenever you need.
      </p>
      {data.workspaces.map((w) => (
        <div className="manage-group" key={w.id}>
          <div className="manage-row">
            <strong>
              {w.name}
              {w.archived ? " · archived" : ""}
            </strong>
            <Button onClick={() => naming("workspace", w.id)}>Rename</Button>
            <Button
              onClick={() =>
                act(() =>
                  change((d) => {
                    d.workspaces.find((x) => x.id === w.id)!.archived =
                      !w.archived;
                  }),
                )
              }
            >
              {w.archived ? "Restore" : "Archive"}
            </Button>
          </div>
          {data.projects
            .filter((p) => p.workspace_id === w.id)
            .map((p) => (
              <div className="manage-row indented" key={p.id}>
                <span>
                  {p.name}
                  {p.archived ? " · archived" : ""}
                </span>
                <Button onClick={() => naming("project", p.id)}>Rename</Button>
                <Button
                  onClick={() =>
                    act(() =>
                      change((d) => {
                        d.projects.find((x) => x.id === p.id)!.archived =
                          !p.archived;
                      }),
                    )
                  }
                >
                  {p.archived ? "Restore" : "Archive"}
                </Button>
              </div>
            ))}
        </div>
      ))}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <Button onClick={() => exportTasks()}>Export all tasks</Button>
    </section>
  );
}
