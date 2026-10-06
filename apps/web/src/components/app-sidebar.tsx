import { Link } from "@tanstack/react-router";
import { Settings, Plus, Search } from "lucide-react";
import type { Data, Task } from "../../../../packages/domain/src/model";
import { Icon } from "../icons";
import { appPath } from "../lib/app-route";
import { Button } from "./ui/pop-button";
import { ScrollArea } from "./ui/pop-scroll-area";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "./ui/pop-select";
import { ThemeControl } from "../lib/theme";
import { projectColorIndex } from "../lib/colors";

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
  const workspaces = data.workspaces
    .filter((w) => !w.archived)
    .map((w) => ({ value: w.id, label: w.name }));
  const navLink = (id: string, label: string, count?: number) => (
    <Link
      key={id}
      to={appPath(workspace, id)}
      className={`${id.startsWith("project:") ? "view-project" : `view-${id}`} ${view === id && !search ? "selected" : ""}`}
      style={
        id.startsWith("project:")
          ? ({
              "--project-color": `var(--c-p${projectColorIndex(id.slice(8))})`,
            } as React.CSSProperties)
          : undefined
      }
      aria-current={view === id && !search ? "page" : undefined}
      onClick={(e) => {
        if (e.ctrlKey || e.metaKey || e.shiftKey || e.altKey) return;
        e.preventDefault();
        navigate(id);
      }}
    >
      <span className="nav-icon">
        <Icon name={id.startsWith("project:") ? "project" : id} />
      </span>
      <span>{label}</span>
      {!!count && <small>{count}</small>}
    </Link>
  );
  return (
    <div className="sidebar-inner">
      <a className="app-brand" href="/" aria-label="CatDo home">
        <img src="/icon.png" alt="" />
        <span>CatDo</span>
        <span className="app-brand-spark" aria-hidden="true">
          ✦
        </span>
      </a>
      <div className="workspace-control">
        <Select
          items={workspaces}
          value={workspace}
          onValueChange={(value) => value && navigate("today", value)}
        >
          <SelectTrigger aria-label="Workspace" className="workspace-trigger">
            <SelectValue />
          </SelectTrigger>
          <SelectContent alignItemWithTrigger={false}>
            {workspaces.map((w) => (
              <SelectItem key={w.value} value={w.value}>
                {w.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label="New workspace"
          title="New workspace"
          onClick={() => naming("workspace")}
        >
          <Plus />
        </Button>
      </div>
      <Button className="sidebar-add" onClick={addTask}>
        <Plus />
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
            size="icon-xs"
            aria-label="New project"
            title="New project"
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
          className={`view-settings ${view === "settings" ? "selected" : ""}`}
          aria-current={view === "settings" ? "page" : undefined}
          onClick={(e) => {
            e.preventDefault();
            navigate("settings");
          }}
        >
          <span className="nav-icon">
            <Settings size={18} strokeWidth={1.8} aria-hidden="true" />
          </span>
          <span>Settings</span>
        </Link>
      </nav>
      <div className="sidebar-footer">
        <ThemeControl compact />
        {syncStatus}
        <div className="account">
          {account}
          <a href="/help">Help ↗</a>
        </div>
      </div>
    </div>
  );
}
