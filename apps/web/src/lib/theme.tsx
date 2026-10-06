import { useEffect, useId, useSyncExternalStore } from "react";
import { Monitor, Moon, Sun } from "lucide-react";
import {
  ToggleGroup,
  ToggleGroupItem,
} from "../components/ui/pop-toggle-group";
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
const choices = [
  ["system", "System", Monitor],
  ["light", "Light", Sun],
  ["dark", "Dark", Moon],
] as const;
export function ThemeControl({ compact = false }: { compact?: boolean }) {
  const theme = useSyncExternalStore(subscribe, snapshot, serverSnapshot);
  const id = useId();
  return (
    <div className="theme-control">
      <span id={id}>Appearance</span>
      <ToggleGroup
        variant="outline"
        size="sm"
        aria-labelledby={id}
        value={[theme]}
        onValueChange={([value]) => {
          if (!value) return;
          try {
            localStorage.setItem("catdo:theme", value);
          } catch {}
          window.dispatchEvent(new Event("catdo-theme"));
        }}
      >
        {choices.map(([value, label, Icon]) => (
          <ToggleGroupItem
            key={value}
            value={value}
            aria-label={label}
            title={label}
          >
            <Icon aria-hidden="true" />
            {!compact && label}
          </ToggleGroupItem>
        ))}
      </ToggleGroup>
    </div>
  );
}
