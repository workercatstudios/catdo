import { useBlocker, useLocation, useNavigate } from "@tanstack/react-router";
import { AppSidebar } from "./components/app-sidebar";
import { NamingDialog, type Naming } from "./components/naming-dialog";
import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogDescription,
} from "./components/ui/dialog";
import { Button } from "./components/ui/button";
import { appPath, parseAppPath } from "./lib/app-route";
import {
  useEffect,
  useMemo,
  useReducer,
  useState,
  type ReactNode,
} from "react";
import {
  type Data,
  type Task,
  newTask,
  saveTask,
  completeTask,
  activeTask,
  localDay,
  validateData,
} from "../../../packages/domain/src/model";
import { merge } from "../../../packages/domain/src/sync";
import { TaskStore, message } from "./storage";
import { Icon } from "./icons";
import { TaskEditor } from "./TaskEditor";
import { TaskList } from "./TaskList";
import { Management } from "./Management";
import { Calendar } from "./Calendar";
type Undo = { before: Data; after: Data };
export function App({
  owner,
  getToken,
  account,
}: {
  owner: string;
  getToken: () => Promise<string | null>;
  account: ReactNode;
}) {
  const store = useMemo(
    () => new TaskStore(owner, getToken),
    [owner, getToken],
  );
  const [, render] = useReducer((n) => n + 1, 0);
  const location = useLocation();
  const routerNavigate = useNavigate();
  const { workspace, view, valid } = parseAppPath(location.pathname);
  const [namingTarget, setNamingTarget] = useState<Naming | null>(null);
  const [search, setSearch] = useState(""),
    [quick, setQuick] = useState(""),
    [editor, setEditor] = useState<Task | null>(null),
    [editorDirty, setEditorDirty] = useState(false),
    [error, setError] = useState(""),
    [undo, setUndo] = useState<Undo[]>([]),
    [menu, setMenu] = useState(false);
  useEffect(() => {
    const unsubscribe = store.subscribe(render);
    void store.open();
    return () => {
      unsubscribe();
      store.stop();
    };
  }, [store]);
  const data = store.state?.data;
  const activeWorkspace =
    data?.workspaces.find((w) => w.id === workspace && !w.archived) ??
    data?.workspaces.find((w) => !w.archived);
  const workspaceId = activeWorkspace?.id ?? "";
  const change = async (fn: (d: Data) => void) => {
    let entry: Undo | undefined;
    await store.change((d) => {
      const before = structuredClone(d);
      fn(d);
      entry = { before, after: structuredClone(d) };
    });
    if (entry) setUndo((u) => [...u.slice(-29), entry!]);
  };
  const act = (fn: () => Promise<unknown>) => {
    setError("");
    void fn().catch((e) => setError(message(e)));
  };
  const canLeave = () =>
    !editorDirty || confirm("Discard unsaved task changes?");
  const openEditor = (task: Task) => {
    if (canLeave()) {
      setEditorDirty(false);
      setEditor(structuredClone(task));
    }
  };
  const add = () =>
    openEditor(
      newTask(
        workspaceId,
        "",
        view.startsWith("project:") ? view.slice(8) : null,
        view === "today" ? localDay() : null,
      ),
    );
  useBlocker({
    shouldBlockFn: () =>
      editorDirty && !confirm("Discard unsaved task changes?"),
    enableBeforeUnload: editorDirty,
  });
  useEffect(() => {
    setEditorDirty(false);
    setEditor(null);
    setSearch("");
    setMenu(false);
  }, [location.pathname]);
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (
        (e.ctrlKey || e.metaKey) &&
        e.key === "k" &&
        !editor &&
        !namingTarget
      ) {
        e.preventDefault();
        const mobile = matchMedia("(max-width: 720px)").matches;
        if (mobile) setMenu(true);
        setTimeout(
          () =>
            document
              .querySelector<HTMLInputElement>(
                mobile ? ".mobile-sidebar #search" : ".desktop-sidebar #search",
              )
              ?.focus(),
          0,
        );
      }
      if (
        (e.ctrlKey || e.metaKey) &&
        e.key === "Enter" &&
        !editor &&
        !namingTarget
      ) {
        e.preventDefault();
        add();
      }
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  });
  if (!data || !activeWorkspace)
    return (
      <main className="loading">
        <img src="/cat.png" alt="" />
        <p>{store.status}</p>
      </main>
    );
  const tasks = data.tasks.filter(
    (t) => t.workspace_id === workspaceId && activeTask(data, t),
  );
  const today = localDay();
  const projects = data.projects.filter(
    (p) => p.workspace_id === workspaceId && !p.archived,
  );
  const nav = [
    ["inbox", "Inbox"],
    ["today", "Today"],
    ["upcoming", "Upcoming"],
    ["calendar", "Calendar"],
  ];
  let visible = data.tasks.filter(
    (t) =>
      t.workspace_id === workspaceId &&
      (view === "completed" ? !!t.completed_at : activeTask(data, t)),
  );
  if (search.trim())
    visible = tasks.filter((t) =>
      `${t.title} ${t.notes}`
        .toLocaleLowerCase()
        .includes(search.trim().toLocaleLowerCase()),
    );
  else if (view === "inbox")
    visible = visible.filter((t) => !t.project_id && !t.parent_id);
  else if (view === "today")
    visible = visible.filter(
      (t) => (t.scheduled && t.scheduled <= today) || (t.due && t.due <= today),
    );
  else if (view === "upcoming")
    visible = visible.filter(
      (t) => (t.scheduled && t.scheduled > today) || (t.due && t.due > today),
    );
  else if (view.startsWith("project:"))
    visible = visible.filter(
      (t) => t.project_id === view.slice(8) && !t.parent_id,
    );
  visible.sort((a, b) =>
    (a.scheduled ?? a.due ?? "9999").localeCompare(
      b.scheduled ?? b.due ?? "9999",
    ),
  );
  const title = search
    ? "Search"
    : view.startsWith("project:")
      ? (projects.find((p) => p.id === view.slice(8))?.name ?? "Project")
      : view === "settings"
        ? "Settings"
        : view === "completed"
          ? "Completed"
          : (nav.find((n) => n[0] === view)?.[1] ?? "Today");
  const navigate = (value: string, nextWorkspace = workspaceId) => {
    void routerNavigate({ to: appPath(nextWorkspace, value) });
  };
  if (
    !valid ||
    (workspace && workspace !== workspaceId) ||
    (view.startsWith("project:") &&
      !projects.some((p) => p.id === view.slice(8)))
  )
    return (
      <main className="not-found">
        <h1>This space isn’t available.</h1>
        <p>It may be archived, or belong to another account.</p>
        <Button onClick={() => navigate("today")}>Back to Today</Button>
      </main>
    );
  const naming = (kind: "workspace" | "project", id?: string) => {
    const rows = kind === "workspace" ? data.workspaces : data.projects;
    const existing = rows.find((r) => r.id === id);
    setNamingTarget({ kind, id, name: existing?.name ?? "" });
  };
  const saveName = async (name: string) => {
    if (!namingTarget || !name.trim()) return;
    const { kind, id } = namingTarget;
    const projectWorkspace = id
      ? data.projects.find((p) => p.id === id)?.workspace_id
      : workspaceId;
    await change((d) => {
      const rows = kind === "workspace" ? d.workspaces : d.projects;
      if (
        rows.some(
          (r) =>
            r.id !== id &&
            r.name.toLowerCase() === name.trim().toLowerCase() &&
            (kind === "workspace" ||
              ("workspace_id" in r && r.workspace_id === projectWorkspace)),
        )
      )
        throw Error("That name is already used here.");
      if (id) {
        rows.find((r) => r.id === id)!.name = name.trim();
      } else if (kind === "workspace")
        d.workspaces.push({
          id: crypto.randomUUID(),
          name: name.trim(),
          archived: false,
        });
      else
        d.projects.push({
          id: crypto.randomUUID(),
          name: name.trim(),
          archived: false,
          workspace_id: workspaceId,
        });
    });
  };
  const sidebar = (
    <AppSidebar
      data={data}
      workspace={workspaceId}
      view={view}
      tasks={tasks}
      today={today}
      search={search}
      setSearch={setSearch}
      navigate={navigate}
      naming={naming}
      account={account}
    />
  );
  return (
    <div className="app-shell">
      <aside className="sidebar desktop-sidebar">{sidebar}</aside>
      <Dialog open={menu} onOpenChange={setMenu}>
        <DialogContent className="mobile-sidebar">
          <DialogTitle className="sr-only">Navigation</DialogTitle>
          <DialogDescription className="sr-only">
            Workspaces and task views
          </DialogDescription>
          {sidebar}
        </DialogContent>
      </Dialog>
      <main className="main">
        <header className="topbar">
          <button
            className="icon-button mobile-menu"
            aria-label="Open navigation"
            onClick={() => setMenu(!menu)}
          >
            <Icon name="menu" />
          </button>
          <span>
            {activeWorkspace.name}
            <span className="slash">/</span>
            {title}
          </span>
          <Button onClick={add}>
            <Icon name="plus" />
            Add task
          </Button>
        </header>
        <div
          className={`content ${view === "calendar" && !search ? "wide" : ""}`}
        >
          {store.state?.conflict && (
            <div className="conflict" role="alert">
              <strong>Changed on two devices</strong>
              <p>
                {store.conflicts.length
                  ? store.conflicts.map((c) => c.name).join(", ")
                  : "A project or task was moved while this device was offline."}
                .
              </p>
              <p>
                {store.structuralConflict
                  ? "These moves cannot be combined. Choose one complete task list, or export a backup first."
                  : "Choose which conflicting versions to keep. Other changes will be combined."}
              </p>
              <button onClick={() => act(() => store.resolve("local"))}>
                {store.structuralConflict
                  ? "Keep this device’s entire list"
                  : "Keep this device’s versions"}
              </button>
              <button onClick={() => act(() => store.resolve("remote"))}>
                {store.structuralConflict
                  ? "Use the entire synced list"
                  : "Use synced versions"}
              </button>
              <button onClick={() => act(() => store.export())}>
                Export a backup
              </button>
            </div>
          )}
          {error && (
            <div className="error" role="alert">
              {error}
              <button aria-label="Dismiss error" onClick={() => setError("")}>
                ×
              </button>
            </div>
          )}
          {view === "settings" && !search ? (
            <Management
              data={data}
              naming={naming}
              act={act}
              change={change}
              error={error}
              exportTasks={() => act(() => store.export())}
            />
          ) : view === "calendar" && !search ? (
            <Calendar
              tasks={tasks}
              open={openEditor}
              move={(id, date) =>
                act(() =>
                  change((d) => {
                    const t = d.tasks.find((t) => t.id === id)!;
                    saveTask(d, {
                      ...t,
                      scheduled: date,
                      recurrence: t.recurrence
                        ? { ...t.recurrence, month_day: Number(date.slice(8)) }
                        : null,
                    });
                  }),
                )
              }
            />
          ) : (
            <>
              <div className="page-heading">
                <p className="eyebrow">
                  {view === "today"
                    ? new Date().toLocaleDateString(undefined, {
                        weekday: "long",
                        month: "long",
                        day: "numeric",
                      })
                    : activeWorkspace.name}
                </p>
                <h1>{title}</h1>
                <p className="muted">
                  {view === "today"
                    ? "A clear place to start."
                    : view === "inbox"
                      ? "Get it out of your head. Give it a home later."
                      : view === "upcoming"
                        ? "A little room to look ahead."
                        : view === "completed"
                          ? "The things you’ve taken care of."
                          : `${visible.length} open ${visible.length === 1 ? "task" : "tasks"}`}
                </p>
              </div>
              {view !== "completed" && (
                <form
                  className="quick-add"
                  onSubmit={(e) => {
                    e.preventDefault();
                    if (!quick.trim()) return;
                    const task = newTask(
                      workspaceId,
                      quick,
                      view.startsWith("project:") ? view.slice(8) : null,
                      view === "today" ? today : null,
                    );
                    act(async () => {
                      await change((d) => saveTask(d, task));
                      setQuick((current) => (current === quick ? "" : current));
                    });
                  }}
                >
                  <Icon name="plus" />
                  <input
                    aria-label="Quick add task"
                    placeholder="Add a task…"
                    value={quick}
                    onChange={(e) => setQuick(e.target.value)}
                  />
                  <kbd>Enter ↵</kbd>
                </form>
              )}
              <TaskList
                tasks={visible}
                data={data}
                today={today}
                complete={(id) => act(() => change((d) => completeTask(d, id)))}
                open={openEditor}
              />
              {visible.length === 0 && (
                <div className="empty-state">
                  <img src="/cat.png" alt="" />
                  <h2>
                    {search
                      ? "No matching tasks"
                      : view === "completed"
                        ? "Small steps add up."
                        : "A little breathing room."}
                  </h2>
                  <p>
                    {search
                      ? "Try another word, or switch workspaces."
                      : view === "completed"
                        ? "Completed tasks will show up here."
                        : "Add something to do, or enjoy the clear space."}
                  </p>
                </div>
              )}
              {view === "completed" &&
                data.history.some(
                  (h) => h.task.workspace_id === workspaceId,
                ) && (
                  <div className="history">
                    <h2>Completion history</h2>
                    {data.history
                      .filter((h) => h.task.workspace_id === workspaceId)
                      .toReversed()
                      .map((h) => (
                        <div key={h.id}>
                          <span>{h.task.title}</span>
                          <time>
                            {new Date(h.completed_at).toLocaleString()}
                          </time>
                        </div>
                      ))}
                  </div>
                )}
            </>
          )}
        </div>
        <footer className="status" aria-live="polite">
          <span>{store.status}</span>
          <button onClick={() => void store.sync()}>Sync now</button>
          {undo.length > 0 && (
            <button
              onClick={() =>
                act(async () => {
                  const last = undo.at(-1)!;
                  await store.change((d) => {
                    const result = merge(last.after, last.before, d);
                    if (result.conflicts.length)
                      throw Error(
                        "This change cannot be undone because it was edited again.",
                      );
                    validateData(result.data);
                    Object.assign(d, result.data);
                  });
                  setUndo((u) => u.slice(0, -1));
                })
              }
            >
              Undo
            </button>
          )}
        </footer>
      </main>
      {editor && (
        <TaskEditor
          key={editor.id}
          task={editor}
          data={data}
          save={change}
          close={() => {
            setEditorDirty(false);
            setEditor(null);
          }}
          dismiss={() => {
            if (canLeave()) {
              setEditorDirty(false);
              setEditor(null);
            }
          }}
          open={(task) => {
            setEditorDirty(false);
            setEditor(structuredClone(task));
          }}
          onDirty={setEditorDirty}
        />
      )}
      {namingTarget && (
        <NamingDialog
          key={namingTarget.kind + (namingTarget.id ?? "")}
          target={namingTarget}
          close={() => setNamingTarget(null)}
          save={saveName}
        />
      )}
    </div>
  );
}
