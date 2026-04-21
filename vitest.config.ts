import react from "@vitejs/plugin-react";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "vitest/config";

const ROOT_DIR = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.join(ROOT_DIR, "src"),
    },
  },
  test: {
    environment: "jsdom",
    testTimeout: 15000,
    setupFiles: ["src/test/setup.ts"],
    restoreMocks: true,
    exclude: ["**/node_modules/**", ".codex-temp/**"],
    coverage: {
      provider: "v8",
      reporter: ["text", "lcov"],
      reportsDirectory: "coverage",
      all: true,
      thresholds: {
        statements: 90,
        branches: 85,
        functions: 90,
        lines: 90,
      },
      include: ["src/**/*.{ts,tsx}"],
      exclude: [
        "**/*.d.ts",
        "**/node_modules/**",
        "**/__tests__/**",
        "**/*.{test,spec}.{ts,tsx}",
        "src/components/ClaudeModelValidation*.tsx",
        "src/components/claude-model-validation/**",
        "src/services/claude/claudeModelValidation*.ts",
        "src/services/claude/claudeValidationTemplates.ts",
        "src/test/**",
        "src/generated/**",
        "src/pages/providers/types.ts",
      ],
    },
  },
});
