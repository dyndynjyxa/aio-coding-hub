import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "tailwindcss";
import autoprefixer from "autoprefixer";
import path from "node:path";
import { fileURLToPath } from "node:url";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;
const ROOT_DIR = path.dirname(fileURLToPath(import.meta.url));

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.join(ROOT_DIR, "src"),
    },
  },

  // Inline PostCSS config (was postcss.config.cjs)
  css: {
    postcss: {
      plugins: [tailwindcss, autoprefixer],
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. code splitting for vendor libraries
  build: {
    rollupOptions: {
      output: {
        manualChunks: {
          "vendor-react": ["react", "react-dom", "react-router-dom"],
          "vendor-query": ["@tanstack/react-query"],
          "vendor-virtual": ["@tanstack/react-virtual"],
          "vendor-charts": ["recharts"],
          "vendor-codemirror": [
            "codemirror",
            "@codemirror/language",
            "@codemirror/state",
            "@codemirror/view",
          ],
          "vendor-dnd": ["@dnd-kit/core", "@dnd-kit/sortable", "@dnd-kit/utilities"],
          "vendor-icons": ["lucide-react"],
          "vendor-ui": [
            "@hookform/resolvers",
            "@radix-ui/react-dialog",
            "@radix-ui/react-label",
            "@radix-ui/react-popover",
            "@radix-ui/react-radio-group",
            "@radix-ui/react-select",
            "@radix-ui/react-slot",
            "@radix-ui/react-switch",
            "@radix-ui/react-tabs",
            "@radix-ui/react-tooltip",
            "class-variance-authority",
            "clsx",
            "react-hook-form",
            "sonner",
            "tailwind-merge",
            "zod",
          ],
        },
      },
    },
  },
  // 3. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
