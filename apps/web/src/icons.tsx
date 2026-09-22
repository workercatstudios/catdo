export function Icon({ name }: { name: string }) {
  const paths: Record<string, string> = {
    inbox: "M4 4h16v16H4z M4 13h5l2 3h2l2-3h5",
    today:
      "M12 3v2m0 14v2M3 12h2m14 0h2M5.6 5.6 7 7m10 10 1.4 1.4M5.6 18.4 7 17M17 7l1.4-1.4 M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0",
    calendar: "M4 5h16v16H4z M4 10h16M8 3v4m8-4v4",
    upcoming: "M4 5h16v16H4z M4 10h16m-9 4 3 2-3 2",
    project: "M3 6h7l2 3h9v11H3z",
    completed: "m8 12 3 3 6-7 M21 12a9 9 0 1 1-4-7.5",
    search: "M15 15l6 6 M17 10a7 7 0 1 1-14 0 7 7 0 0 1 14 0",
    plus: "M12 5v14M5 12h14",
    close: "m6 6 12 12M6 18 18 6",
    repeat: "m17 2 4 4-4 4 M3 11V6h18 M7 22l-4-4 4-4 M21 13v5H3",
    menu: "M4 6h16M4 12h16M4 18h16",
  };
  return (
    <svg
      width="18"
      height="18"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={paths[name] ?? paths.project} />
    </svg>
  );
}
