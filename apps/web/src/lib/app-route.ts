export function appPath(workspace: string, view: string) {
  const root = "/app/workspaces/" + encodeURIComponent(workspace);
  return (
    root +
    (view.startsWith("project:")
      ? "/projects/" + encodeURIComponent(view.slice(8))
      : "/" + view)
  );
}
export function parseAppPath(path: string): {
  workspace: string;
  view: string;
  valid: boolean;
} {
  if (path === "/app" || path === "/app/")
    return { workspace: "", view: "today", valid: true };
  const parts = path.split("/").filter(Boolean);
  try {
    const workspace = decodeURIComponent(parts[2] || "");
    const view =
      parts[3] === "projects" && parts.length === 5
        ? "project:" + decodeURIComponent(parts[4])
        : parts[3];
    const valid =
      parts[0] === "app" &&
      parts[1] === "workspaces" &&
      !!workspace &&
      ((parts.length === 4 &&
        [
          "inbox",
          "today",
          "upcoming",
          "calendar",
          "completed",
          "settings",
        ].includes(view)) ||
        (parts.length === 5 && view.startsWith("project:")));
    return { workspace, view: view || "today", valid };
  } catch {
    return { workspace: "", view: "today", valid: false };
  }
}
