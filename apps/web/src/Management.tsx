import { useEffect, useRef } from "react";
import type { Data } from "../../../packages/domain/src/model";
export function Management({
  data,
  close,
  naming,
  act,
  change,
  error,
  exportTasks,
}: {
  data: Data;
  close: () => void;
  naming: (kind: "workspace" | "project", id?: string) => void;
  act: (fn: () => Promise<unknown>) => void;
  change: (fn: (d: Data) => void) => Promise<void>;
  error: string;
  exportTasks: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    dialog.current?.showModal();
  }, []);
  return (
    <dialog
      className="manage"
      ref={dialog}
      onCancel={close}
      aria-labelledby="manage-title"
    >
      {" "}
      <div className="editor-head">
        <h2 id="manage-title">Workspaces & projects</h2>
        <button aria-label="Close management" onClick={() => close()}>
          ×
        </button>
      </div>
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
            <button onClick={() => naming("workspace", w.id)}>Rename</button>
            <button
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
            </button>
          </div>
          {data.projects
            .filter((p) => p.workspace_id === w.id)
            .map((p) => (
              <div className="manage-row indented" key={p.id}>
                <span>
                  {p.name}
                  {p.archived ? " · archived" : ""}
                </span>
                <button onClick={() => naming("project", p.id)}>Rename</button>
                <button
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
                </button>
              </div>
            ))}
        </div>
      ))}
      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}
      <button onClick={() => exportTasks()}>Export all tasks</button>
    </dialog>
  );
}
