import { useEffect, useSyncExternalStore } from "react";
export type Theme = "system" | "light" | "dark";
export const themeScript = `try{let t=localStorage.getItem('catdo:theme')||'system';document.documentElement.classList.toggle('dark',t==='dark'||(t==='system'&&matchMedia('(prefers-color-scheme: dark)').matches))}catch{}`;
function snapshot(): Theme {
  try {
    const t = localStorage.getItem("catdo:theme");
    return t === "light" || t === "dark" ? t : "system";
  } catch {
    return "system";
  }
}
function subscribe(fn: () => void) {
  window.addEventListener("catdo-theme", fn);
  window.addEventListener("storage", fn);
  return () => {
    window.removeEventListener("catdo-theme", fn);
    window.removeEventListener("storage", fn);
  };
}
const serverSnapshot = () => "system" as const;
export function ThemeListener() {
  useEffect(() => {
    const media = matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const t = snapshot();
      document.documentElement.classList.toggle(
        "dark",
        t === "dark" || (t === "system" && media.matches),
      );
    };
    apply();
    media.addEventListener("change", apply);
    const unsubscribe = subscribe(apply);
    return () => {
      media.removeEventListener("change", apply);
      unsubscribe();
    };
  }, []);
  return null;
}
export function ThemeControl() {
  const theme = useSyncExternalStore(subscribe, snapshot, serverSnapshot);
  return (
    <label className="theme-control">
      <span>Appearance</span>
      <select
        aria-label="Appearance"
        value={theme}
        onChange={(e) => {
          try {
            localStorage.setItem("catdo:theme", e.target.value);
          } catch {}
          window.dispatchEvent(new Event("catdo-theme"));
        }}
      >
        <option value="system">System</option>
        <option value="light">Light</option>
        <option value="dark">Dark</option>
      </select>
    </label>
  );
}
