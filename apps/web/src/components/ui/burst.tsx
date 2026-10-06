"use client";

import { useState, type ComponentProps, type CSSProperties } from "react";
import { cn } from "@/lib/utils";

export type BurstEffect = "ring" | "streaks" | "confetti";

const css = `
@layer theme, base, components, utilities;
@layer components {
@property --kk-burst-hole {
  syntax: "<percentage>";
  inherits: false;
  initial-value: 0%;
}
:where(.kk-burst-layer) {
  position: absolute;
  top: 50%;
  left: 50%;
  z-index: -1;
  width: 0;
  height: 0;
  pointer-events: none;
  --kk-burst-r: calc(var(--kk-burst-size, 1) * 3em);
}
:where(.kk-burst-layer > *) {
  position: absolute;
}
/* Ring: a disc grows, then a hole opens a beat behind it and eats it from the inside. */
:where(.kk-burst-ring) {
  left: calc(var(--kk-burst-r) * -1);
  top: calc(var(--kk-burst-r) * -1);
  width: calc(var(--kk-burst-r) * 2);
  height: calc(var(--kk-burst-r) * 2);
  border-radius: 50%;
  background: var(--kk-burst-c0);
  mask: radial-gradient(circle closest-side, transparent var(--kk-burst-hole), #000 calc(var(--kk-burst-hole) + 1px));
  animation:
    kk-burst-grow 0.45s var(--kk-ease-out, cubic-bezier(0.05, 0.3, 0.1, 1)) both,
    kk-burst-eat 0.45s var(--kk-ease-out, cubic-bezier(0.05, 0.3, 0.1, 1)) 0.07s both;
}
/* Streaks: a bar from the centre to the rim holds a rounded window on its outer end. */
:where(.kk-burst-streak) {
  --kk-burst-w: calc(var(--kk-burst-size) * var(--kk-burst-weight) * 1px + 0.5px);
  left: calc(var(--kk-burst-w) / -2);
  bottom: 0;
  width: var(--kk-burst-w);
  height: calc(var(--kk-burst-r) * var(--kk-burst-reach));
  transform-origin: bottom center;
  rotate: var(--kk-burst-a);
}
:where(.kk-burst-window) {
  display: block;
  height: 42%;
  overflow: hidden;
  border-radius: 999px;
}
:where(.kk-burst-window)::after {
  content: "";
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--kk-burst-c);
  animation: kk-burst-slide 0.5s var(--kk-ease-snap, cubic-bezier(0.85, 0, 0.15, 1)) var(--kk-burst-delay) both;
}
/* Confetti: each dot flies along its spoke, scaling in on the way and out at the end. */
:where(.kk-burst-dot) {
  --kk-burst-d: calc(var(--kk-burst-size) * var(--kk-burst-weight) * 1px + 1px);
  left: calc(var(--kk-burst-d) / -2);
  top: calc(var(--kk-burst-d) / -2);
  width: var(--kk-burst-d);
  height: var(--kk-burst-d);
  border-radius: 50%;
  background: var(--kk-burst-c);
  animation:
    kk-burst-fly 0.7s var(--kk-ease-out, cubic-bezier(0.05, 0.3, 0.1, 1)) var(--kk-burst-delay) both,
    kk-burst-in 0.3s ease-out var(--kk-burst-delay) both,
    kk-burst-out 0.3s var(--kk-ease-in, cubic-bezier(0.8, 0, 1, 1)) calc(var(--kk-burst-delay) + 0.45s) forwards;
}
@keyframes kk-burst-grow {
  from { scale: 0; }
  to { scale: 1; }
}
@keyframes kk-burst-eat {
  from { --kk-burst-hole: 0%; }
  to { --kk-burst-hole: 100%; }
}
@keyframes kk-burst-slide {
  from { translate: 0 101%; }
  to { translate: 0 -101%; }
}
@keyframes kk-burst-fly {
  from { translate: 0 0; }
  to { translate: calc(var(--kk-burst-x) * var(--kk-burst-r)) calc(var(--kk-burst-y) * var(--kk-burst-r)); }
}
@keyframes kk-burst-in {
  from { scale: 0; }
  to { scale: 1; }
}
@keyframes kk-burst-out {
  from { scale: 1; }
  to { scale: 0; }
}
@media (prefers-reduced-motion: reduce) {
  :where(.kk-burst-layer) { display: none; }
}
}
`;

const defaultColors = [
  "var(--kk-pink, #ec5f8f)",
  "var(--kk-yellow, #f7d35c)",
  "var(--kk-sky, #5aa9e6)",
  "var(--kk-lime, #b9cc5a)",
];

const allEffects: BurstEffect[] = ["ring", "streaks", "confetti"];

type Vars = CSSProperties & Record<`--${string}`, string | number>;

// Two crosses of four: thick and long first, thin and short a beat later, turned 45°.
const streaks = Array.from({ length: 8 }, (_, i) => {
  const late = i >= 4;
  return {
    angle: (i % 4) * 90 + (late ? 45 : 0),
    weight: late ? 1.5 : 2.5,
    reach: late ? 0.85 : 1,
    delay: late ? 0.1 : 0,
  };
});

// Ten dots on 36° spokes. Index rules vary size, reach and delay so the ring of dots breaks up.
const confetti = Array.from({ length: 10 }, (_, i) => {
  const angle = ((i * 36 - 90) * Math.PI) / 180;
  const reach = i % 3 === 2 ? 0.8 : i % 2 ? 0.92 : 1.05;
  return {
    x: Number((Math.cos(angle) * reach).toFixed(4)),
    y: Number((Math.sin(angle) * reach).toFixed(4)),
    weight: i % 3 === 2 ? 3 : i % 2 ? 4 : 6,
    delay: i % 3 === 2 ? 0.1 : i % 2 ? 0.05 : 0,
  };
});

export interface BurstProps extends ComponentProps<"span"> {
  /** Which effects to fire. */
  effects?: BurstEffect[];
  /** `click` fires on every click inside; `mount` fires once when it mounts. */
  trigger?: "click" | "mount";
  /** Controlled firing: each change of this number fires once. Turns off click firing. */
  fire?: number;
  /** Ring, streak and confetti colours, cycled in order. */
  colors?: string[];
  /** Scales the burst radius, which is 3em at 1. */
  size?: number;
}

export function Burst({
  effects = allEffects,
  trigger = "click",
  fire,
  colors = defaultColors,
  size = 1,
  className,
  onClick,
  children,
  ...props
}: BurstProps) {
  const [shot, setShot] = useState(trigger === "mount" ? 1 : 0);
  const [lastFire, setLastFire] = useState(fire);
  if (fire !== lastFire) {
    setLastFire(fire);
    setShot((n) => n + 1);
  }
  const color = (i: number) => colors[i % colors.length] ?? defaultColors[0];

  return (
    <span
      data-slot="burst"
      className={cn("relative isolate inline-flex", className)}
      onClick={(event) => {
        onClick?.(event);
        if (trigger === "click" && fire === undefined) setShot((n) => n + 1);
      }}
      {...props}
    >
      <style href="kk-burst" precedence="kirakira">
        {css}
      </style>
      {children}
      {shot > 0 && (
        <span
          key={shot}
          aria-hidden
          className="kk-burst-layer"
          style={{ "--kk-burst-size": size, "--kk-burst-c0": color(0) } as Vars}
        >
          {effects.includes("ring") && <span className="kk-burst-ring" />}
          {effects.includes("streaks") &&
            streaks.map((streak, i) => (
              <span
                key={`s${i}`}
                className="kk-burst-streak"
                style={
                  {
                    "--kk-burst-a": `${streak.angle}deg`,
                    "--kk-burst-weight": streak.weight,
                    "--kk-burst-reach": streak.reach,
                    "--kk-burst-delay": `${streak.delay}s`,
                    "--kk-burst-c": color(streak.delay ? 2 : 1),
                  } as Vars
                }
              >
                <span className="kk-burst-window" />
              </span>
            ))}
          {effects.includes("confetti") &&
            confetti.map((dot, i) => (
              <span
                key={`c${i}`}
                className="kk-burst-dot"
                style={
                  {
                    "--kk-burst-x": dot.x,
                    "--kk-burst-y": dot.y,
                    "--kk-burst-weight": dot.weight,
                    "--kk-burst-delay": `${dot.delay}s`,
                    "--kk-burst-c": color(i),
                  } as Vars
                }
              />
            ))}
        </span>
      )}
    </span>
  );
}
