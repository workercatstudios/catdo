import { Link } from "@tanstack/react-router";
import { Settings, Plus, Search } from "lucide-react";
import type { Data, Task } from "../../../../packages/domain/src/model";
import { Icon } from "../icons";
import { appPath } from "../lib/app-route";
import { Button } from "./ui/button";
import { ScrollArea } from "./ui/scroll-area";
import { ThemeControl } from "../lib/theme";

import type { ReactNode } from "react";
export function AppSidebar({
  data,
  workspace,
  view,
  tasks,
  today,
  search,
  setSearch,
  navigate,
  naming,
  account,
  addTask,
  syncStatus,
}: {
  data: Data;
  workspace: string;
  view: string;
  tasks: Task[];
  today: string;
  search: string;
  setSearch: (value: string) => void;
  navigate: (view: string, workspace?: string) => void;
  naming: (kind: "workspace" | "project", id?: string) => void;
  account: ReactNode;
  addTask: () => void;
  syncStatus: ReactNode;
}) {
  const nav = [
    ["inbox", "Inbox"],
    ["today", "Today"],
    ["upcoming", "Upcoming"],
    ["calendar", "Calendar"],
  ];
  const navLink = (id: string, label: string, count?: number) => (
    <Link
      key={id}
      to={appPath(workspace, id)}
      className={view === id && !search ? "selected" : ""}
      aria-current={view === id && !search ? "page" : undefined}
      onClick={(e) => {
        if (e.ctrlKey || e.metaKey || e.shiftKey || e.altKey) return;
        e.preventDefault();
        navigate(id);
      }}
    >
      <Icon name={id.startsWith("project:") ? "project" : id} />
      <span>{label}</span>
      {!!count && <small>{count}</small>}
    </Link>
  );
  return (
    <div className="sidebar-inner">
      <a className="app-brand" href="/" aria-label="CatDo home">
        <img src="/icon.png" alt="" />
        <span>CatDo</span>
      </a>
      <div className="workspace-control">
        <select
          aria-label="Workspace"
          value={workspace}
          onChange={(e) => navigate("today", e.target.value)}
        >
          {data.workspaces
            .filter((w) => !w.archived)
            .map((w) => (
              <option key={w.id} value={w.id}>
                {w.name}
              </option>
            ))}
        </select>
        <Button
          variant="ghost"
          size="icon"
          aria-label="New workspace"
          onClick={() => naming("workspace")}
        >
          <Plus />
        </Button>
      </div>
      <Button className="sidebar-add" onClick={addTask}>
        <Plus size={16} />
        Add task<kbd aria-hidden="true">⌃ ↵</kbd>
      </Button>
      <div className="search">
        <Search size={16} />
        <input
          id="search"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder="Search workspace"
          aria-label="Search workspace"
        />
        <kbd>⌃ K</kbd>
      </div>
      <ScrollArea className="sidebar-navigation">
        <nav aria-label="Task views">
          {nav.map(([id, label]) =>
            navLink(
              id,
              label,
              id === "today"
                ? tasks.filter(
                    (t) =>
                      (t.scheduled && t.scheduled <= today) ||
                      (t.due && t.due <= today),
                  ).length
                : undefined,
            ),
          )}
        </nav>
        <div className="section-label">
          <span>Projects</span>
          <Button
            variant="ghost"
            size="icon"
            aria-label="New project"
            onClick={() => naming("project")}
          >
            <Plus />
          </Button>
        </div>
        <div className="projects">
          <nav aria-label="Projects">
            {data.projects
              .filter((p) => p.workspace_id === workspace && !p.archived)
              .map((p) =>
                navLink(
                  "project:" + p.id,
                  p.name,
                  tasks.filter((t) => t.project_id === p.id && !t.parent_id)
                    .length,
                ),
              )}
          </nav>
          {!data.projects.some(
            (p) => p.workspace_id === workspace && !p.archived,
          ) && (
            <p className="sidebar-hint">
              A home for your next idea.
              <br />
              Add a project with +.
            </p>
          )}
        </div>
      </ScrollArea>
      <nav aria-label="Workspace tools">
        {navLink("completed", "Completed")}
        <Link
          to={appPath(workspace, "settings")}
          className={view === "settings" ? "selected" : ""}
          aria-current={view === "settings" ? "page" : undefined}
          onClick={(e) => {
            e.preventDefault();
            navigate("settings");
          }}
        >
          <Settings size={18} />
          <span>Settings</span>
        </Link>
      </nav>
      <div className="sidebar-footer">
        <ThemeControl />
        {syncStatus}
        <div className="account">
          {account}
          <a href="/help">Help ↗</a>
        </div>
      </div>
    </div>
  );
}
