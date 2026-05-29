import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { resolve } from "node:path";

export default defineConfig({
  plugins: [svelte()],
  base: "/",
  resolve: {
    alias: {
      $lib: resolve(__dirname, "../../packages/ui/src/lib"),
    },
  },
  build: {
    outDir: resolve(__dirname, "../../../web-aggregate/dist"),
    emptyOutDir: true,
  },
  server: { proxy: { "/api": "http://127.0.0.1:8787" } },
});
