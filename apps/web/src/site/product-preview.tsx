import { useState } from "react";
import { Sun, Folder, CalendarDays } from "lucide-react";

const views = [
  {
    id: "today",
    label: "Your day",
    icon: Sun,
    caption: "A short list. A clear place to start.",
    alt: "CatDo’s Today view with sample tasks, scheduled dates, and deadlines.",
  },
  {
    id: "projects",
    label: "Your projects",
    icon: Folder,
    caption: "Work and life apart. Related tasks together.",
    alt: "CatDo’s Weekend away project with a packing and planning task list.",
  },
  {
    id: "calendar",
    label: "Your calendar",
    icon: CalendarDays,
    caption: "See what’s coming. Make room for it.",
    alt: "CatDo’s calendar showing scheduled tasks and deadlines across the month.",
  },
] as const;

export function ProductPreview() {
  const [selected, setSelected] = useState<(typeof views)[number]>(views[0]);
  return (
    <section
      id="features"
      className="product-tour"
      aria-label="See CatDo in action"
    >
      <div
        className="preview-controls"
        role="group"
        aria-label="Choose an app screenshot"
      >
        {views.map((view) => (
          <button
            key={view.id}
            aria-pressed={view.id === selected.id}
            aria-controls="product-screen"
            onClick={() => setSelected(view)}
          >
            <view.icon size={16} /> {view.label}
          </button>
        ))}
      </div>
      <figure
        className="app-screenshot"
        id="product-screen"
        aria-label={selected.label}
      >
        <div className="screenshot-frame">
          {views.map((view, index) => (
            <picture
              key={view.id}
              className="screenshot-layer"
              aria-hidden={view.id !== selected.id}
              data-active={view.id === selected.id}
            >
              <source
                media="(max-width: 600px)"
                srcSet={`/screenshots/${view.id}-mobile.png`}
                width="780"
                height="1280"
              />
              <img
                src={`/screenshots/${view.id}.png`}
                width="1440"
                height="880"
                fetchPriority={index === 0 ? "high" : "low"}
                loading="eager"
                alt={view.alt}
              />
            </picture>
          ))}
        </div>
        <figcaption aria-live="polite">{selected.caption}</figcaption>
      </figure>
      <p className="product-note">
        Works offline. Syncs across devices. Actual app screenshots with sample
        tasks.
      </p>
    </section>
  );
}
