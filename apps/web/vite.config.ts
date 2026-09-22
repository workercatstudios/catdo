import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { readFileSync } from "node:fs";
import { createHash } from "node:crypto";
export default defineConfig({
  plugins: [
    react(),
    {
      name: "catdo-offline-shell",
      generateBundle(_, bundle) {
        const files = Object.keys(bundle).filter((name) =>
          /\.(js|css)$/.test(name),
        );
        const version = createHash("sha256")
          .update(files.join(","))
          .digest("hex")
          .slice(0, 12);
        const template = readFileSync(
          new URL("./src/service-worker.js", import.meta.url),
          "utf8",
        );
        this.emitFile({
          type: "asset",
          fileName: "sw.js",
          source: template
            .replace("__VERSION__", version)
            .replace(
              "__ASSETS__",
              JSON.stringify([
                "/app",
                "/cat.png",
                ...files.map((name) => "/" + name),
              ]),
            ),
        });
      },
    },
  ],
  server: { proxy: { "/api": "http://127.0.0.1:8787" } },
});
