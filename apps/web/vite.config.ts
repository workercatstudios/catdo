import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { cloudflare } from "@cloudflare/vite-plugin";
import { tanstackStart } from "@tanstack/react-start/plugin/vite";
import tailwindcss from "@tailwindcss/vite";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
export default defineConfig({
  resolve: { alias: { "@": new URL("./src", import.meta.url).pathname } },
  plugins: [
    cloudflare({ viteEnvironment: { name: "ssr" } }),
    tanstackStart(),
    tailwindcss(),
    react(),
    {
      name: "catdo-offline-shell",
      apply: "build",
      generateBundle(_, bundle) {
        if (this.environment.name !== "client") return;
        const files = Object.keys(bundle)
          .filter((name) => /\.(js|css|woff2)$/.test(name))
          .sort();
        const version = createHash("sha256")
          .update(
            files
              .map((name) => {
                const entry = bundle[name];
                return entry.type === "chunk"
                  ? entry.code
                  : String(entry.source);
              })
              .join("\n"),
          )
          .digest("hex")
          .slice(0, 12);
        this.emitFile({
          type: "asset",
          fileName: "sw.js",
          source: readFileSync(
            new URL("./src/service-worker.js", import.meta.url),
            "utf8",
          )
            .replace("__VERSION__", version)
            .replace(
              "__ASSETS__",
              JSON.stringify([
                "/app",
                "/icon.png",
                "/cat.png",
                ...files.map((name) => "/" + name),
              ]),
            ),
        });
      },
    },
  ],
});
