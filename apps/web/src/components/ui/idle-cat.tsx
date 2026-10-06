"use client";

import type { ComponentProps, CSSProperties } from "react";
import { cn } from "@/lib/utils";

// The rig: every <g data-part> owns one motion, so nested parts never fight over `transform`.
// Body-level parts scale from the feet (64, 112 in the viewBox); small parts use their own box.
const css = `
@layer theme, base, components, utilities;
@layer components {
:where([data-slot="idle-cat"]) {
  --kk-cat-ink: #4b3832;
  overflow: visible;
}
:where([data-slot="idle-cat"] [data-part]) {
  transform-box: view-box;
  transform-origin: 64px 112px;
}
:where([data-slot="idle-cat"] [data-part="breathe"]) {
  animation: kk-idle-cat-breathe 1.1s ease-in-out infinite;
}
:where([data-slot="idle-cat"] [data-part="eyes"]) {
  transform-box: fill-box;
  transform-origin: 50% 50%;
  animation: kk-idle-cat-blink 5s ease-in-out infinite;
}
:where([data-slot="idle-cat"] [data-part="ear"]) {
  --kk-ear: -1;
  transform-box: fill-box;
  transform-origin: 50% 76%;
  animation: kk-idle-cat-twitch 4.4s ease-in-out infinite;
}
/* Offset by half a cycle so the ears twitch one at a time. */
:where([data-slot="idle-cat"] [data-part="ear"] + [data-part="ear"]) {
  --kk-ear: 1;
  animation-delay: -2.2s;
}
:where([data-slot="idle-cat"] [data-part="tail"]) {
  transform-origin: 96px 101px;
  animation: kk-idle-cat-wag 3.2s ease-in-out infinite;
}
:where([data-slot="idle-cat"] [data-part="shadow"], [data-slot="idle-cat"] [data-part="z"]) {
  transform-box: fill-box;
  transform-origin: 50% 50%;
}
:where([data-slot="idle-cat"] [data-part="z"]) {
  opacity: 0.6;
  animation: kk-idle-cat-z 2.8s ease-out infinite;
}
:where([data-slot="idle-cat"] [data-part="z"] + [data-part="z"]) {
  animation-delay: -1.4s;
}
:where([data-slot="idle-cat"][data-mood="happy"] [data-part="hop"]) {
  animation: kk-idle-cat-hop 1.2s infinite;
}
:where([data-slot="idle-cat"][data-mood="happy"] [data-part="squash"]) {
  animation: kk-idle-cat-squash 1.2s ease-in-out infinite;
}
:where([data-slot="idle-cat"][data-mood="happy"] [data-part="shadow"]) {
  animation: kk-idle-cat-shadow 1.2s ease-in-out infinite;
}
:where([data-slot="idle-cat"][data-mood="happy"] [data-part="tail"]) {
  animation-duration: 1.2s;
}
:where([data-slot="idle-cat"][data-mood="sleepy"] [data-part="breathe"]) {
  animation-name: kk-idle-cat-breathe-deep;
  animation-duration: 2.6s;
}
:where([data-slot="idle-cat"][data-mood="sleepy"] [data-part="ear"]) {
  animation-duration: 7s;
}
:where([data-slot="idle-cat"][data-mood="sleepy"] [data-part="tail"]) {
  animation-duration: 6.4s;
}
@keyframes kk-idle-cat-breathe {
  50% { transform: scale(1.024, 1.04); }
}
@keyframes kk-idle-cat-breathe-deep {
  50% { transform: scale(1.036, 1.06); }
}
/* One blink, then a double blink, each about 150 ms. */
@keyframes kk-idle-cat-blink {
  0%, 30%, 33%, 86%, 89%, 92%, 95%, 100% { transform: none; }
  31.5%, 87.5%, 93.5% { transform: scaleY(0.1); }
}
/* Idle most of the cycle, then two quick flicks outward. */
@keyframes kk-idle-cat-twitch {
  0%, 56%, 61%, 66%, 100% { transform: none; }
  58.5% { transform: rotate(calc(var(--kk-ear) * 16deg)); }
  63.5% { transform: rotate(calc(var(--kk-ear) * 9deg)); }
}
@keyframes kk-idle-cat-wag {
  0%, 62%, 100% { transform: none; }
  12% { transform: rotate(-14deg); }
  26% { transform: rotate(6deg); }
  40% { transform: rotate(-10deg); }
  52% { transform: rotate(3deg); }
}
/* Rise on an ease-out and fall on an ease-in, like a thrown ball. */
@keyframes kk-idle-cat-hop {
  0%, 22% { transform: none; animation-timing-function: cubic-bezier(0.33, 1, 0.68, 1); }
  44% { transform: translateY(-20px); animation-timing-function: cubic-bezier(0.32, 0, 0.67, 0); }
  66%, 100% { transform: none; }
}
/* Anticipation, launch stretch, fall stretch, landing squash, settle. */
@keyframes kk-idle-cat-squash {
  0%, 90%, 100% { transform: none; }
  14% { transform: scale(1.16, 0.84); }
  26% { transform: scale(0.88, 1.14); }
  44% { transform: scale(0.98, 1.02); }
  62% { transform: scale(0.92, 1.08); }
  70% { transform: scale(1.18, 0.8); }
  80% { transform: scale(0.97, 1.03); }
}
@keyframes kk-idle-cat-shadow {
  0%, 22%, 66%, 100% { transform: none; opacity: 1; }
  44% { transform: scale(0.7); opacity: 0.5; }
  70% { transform: scale(1.12, 1); }
}
@keyframes kk-idle-cat-z {
  from { transform: translate(-2px, 6px) scale(0.6); opacity: 0; }
  30% { opacity: 0.6; }
  to { transform: translate(6px, -10px); opacity: 0; }
}
@media (prefers-reduced-motion: reduce) {
  :where([data-slot="idle-cat"] *) { animation: none !important; }
}
}
`;

export type IdleCatMood = "idle" | "happy" | "sleepy";

export type IdleCatProps = Omit<ComponentProps<"svg">, "color"> & {
  /** Fur colour. */
  color?: string;
  /** Width in px. */
  size?: number;
  mood?: IdleCatMood;
  /** Accessible name. */
  label?: string;
};

const ink = "var(--kk-cat-ink)";
const pink = "var(--kk-pink, #ec5f8f)";
const fur = "var(--kk-cat)";

function Eyes({ mood }: { mood: IdleCatMood }) {
  if (mood === "idle") {
    return (
      <g data-part="eyes">
        {[48, 80].map((x) => (
          <g key={x}>
            <ellipse cx={x} cy={72} rx={4.6} ry={5.6} fill={ink} />
            <circle cx={x + 1.6} cy={70} r={1.6} fill="#fff" />
          </g>
        ))}
      </g>
    );
  }
  // Happy eyes arch up like ^ ^; sleepy eyes dip like closed lids.
  const bend = mood === "happy" ? -7.5 : 5.5;
  const y = mood === "happy" ? 74 : 71;
  return (
    <g fill="none" stroke={ink} strokeWidth={3} strokeLinecap="round">
      {[48, 80].map((x) => (
        <path key={x} d={`M${x - 5} ${y} Q${x} ${y + bend} ${x + 5} ${y}`} />
      ))}
    </g>
  );
}

/** An idling SVG cat: breathes, blinks, twitches its ears and wags its tail. */
export function IdleCat({
  color = "var(--kk-orange, #f4a35f)",
  size = 160,
  mood = "idle",
  label = "Cat",
  className,
  style,
  ...props
}: IdleCatProps) {
  return (
    <>
      <style href="kk-idle-cat" precedence="kirakira">
        {css}
      </style>
      <svg
        viewBox="0 0 140 120"
        width={size}
        height={(size * 120) / 140}
        role="img"
        aria-label={label}
        {...props}
        data-slot="idle-cat"
        data-mood={mood}
        className={cn("shrink-0", className)}
        style={{ "--kk-cat": color, ...style } as CSSProperties}
      >
        <ellipse
          data-part="shadow"
          cx={64}
          cy={113}
          rx={40}
          ry={4.5}
          fill={ink}
          fillOpacity={0.12}
        />
        <g data-part="hop">
          <g data-part="squash">
            <g data-part="tail">
              <path
                d="M94 101C116 103 129 92 127 77C125.5 66 113 64.5 112 73"
                fill="none"
                stroke={fur}
                strokeWidth={10}
                strokeLinecap="round"
              />
            </g>
            <g data-part="breathe">
              {[
                ["M30 60L31 19L60 40Z", "M35 47L35.5 28L50 39Z"],
                ["M98 60L97 19L68 40Z", "M93 47L92.5 28L78 39Z"],
              ].map(([outer, inner]) => (
                <g key={outer} data-part="ear" strokeLinejoin="round">
                  <path d={outer} fill={fur} stroke={fur} strokeWidth={8} />
                  <path d={inner} fill={pink} stroke={pink} strokeWidth={3} opacity={0.5} />
                </g>
              ))}
              <path
                d="M20 86C20 52 40 34 64 34C88 34 108 52 108 86C108 104 94 112 64 112C34 112 20 104 20 86Z"
                fill={fur}
              />
              <ellipse cx={64} cy={101} rx={20} ry={9} fill="#fff" fillOpacity={0.3} />
              <path
                d="M58 41V47M64 39V47.5M70 41V47"
                stroke={ink}
                strokeOpacity={0.14}
                strokeWidth={3.2}
                strokeLinecap="round"
              />
              <Eyes mood={mood} />
              <ellipse cx={38} cy={82} rx={6.5} ry={3.8} fill={pink} fillOpacity={0.45} />
              <ellipse cx={90} cy={82} rx={6.5} ry={3.8} fill={pink} fillOpacity={0.45} />
              <path
                d="M61.6 75.2h4.8l-2.4 2.6z"
                fill={pink}
                stroke={pink}
                strokeWidth={1.6}
                strokeLinejoin="round"
              />
              <path
                d="M64 77.6V79.6M58.5 79.6Q61.25 83.4 64 79.6Q66.75 83.4 69.5 79.6"
                fill="none"
                stroke={ink}
                strokeWidth={2}
                strokeLinecap="round"
                strokeLinejoin="round"
              />
              <path
                d="M32 76L15 73.5M32 80.5L16 83M96 76L113 73.5M96 80.5L112 83"
                stroke={ink}
                strokeOpacity={0.4}
                strokeWidth={1.6}
                strokeLinecap="round"
              />
              {[50, 78].map((x) => (
                <ellipse
                  key={x}
                  cx={x}
                  cy={109}
                  rx={8.5}
                  ry={5}
                  fill="#fffaf3"
                  stroke={ink}
                  strokeOpacity={0.12}
                  strokeWidth={1.4}
                />
              ))}
            </g>
          </g>
        </g>
        {mood === "sleepy" && (
          <g fill="var(--kk-ink, #4b3832)" fontWeight={800} fontFamily="system-ui, sans-serif">
            <text data-part="z" x={102} y={38} fontSize={13}>
              z
            </text>
            <text data-part="z" x={113} y={25} fontSize={10}>
              z
            </text>
          </g>
        )}
      </svg>
    </>
  );
}
