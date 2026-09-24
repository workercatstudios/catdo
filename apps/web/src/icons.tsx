import {
  CalendarDays,
  CalendarRange,
  CircleCheck,
  Folder,
  Inbox,
  Menu,
  Plus,
  Repeat2,
  Search,
  Sun,
  X,
  type LucideIcon,
} from "lucide-react";

const icons: Record<string, LucideIcon> = {
  inbox: Inbox,
  today: Sun,
  calendar: CalendarDays,
  upcoming: CalendarRange,
  project: Folder,
  completed: CircleCheck,
  search: Search,
  plus: Plus,
  close: X,
  repeat: Repeat2,
  menu: Menu,
};

export function Icon({ name }: { name: string }) {
  const Component = icons[name] ?? Folder;
  return <Component size={18} strokeWidth={1.6} aria-hidden="true" />;
}
