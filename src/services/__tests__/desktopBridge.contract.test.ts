import { describe, expect, it } from "vitest";
import bindingsSource from "../../generated/bindings.ts?raw";
import updaterSource from "../desktop/updater.ts?raw";

const allowedRawTauriImportFiles = new Set([
  "generated/bindings.ts",
  "services/desktop/event.ts",
  "services/desktop/updater.ts",
  "services/tauriInvoke.ts",
]);

const allowedRawInvokeFiles = new Set([
  "services/tauriInvoke.ts",
  "services/desktop/updater.ts",
]);

const rawTauriImportPattern =
  /from\s+["']@tauri-apps\/(?:api|plugin)[^"']*["']|import\s*\(\s*["']@tauri-apps\/(?:api|plugin)[^"']*["']\s*\)/;
const rawInvokePattern = /invokeTauriOrNull(?:<[^>]+>)?\s*\(/;

const rawSourceModules = import.meta.glob("/src/**/*.{ts,tsx}", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

function listSourceFiles() {
  return Object.entries(rawSourceModules)
    .map(([path, source]) => {
      const relPath = path.replace(/^\/src\//, "");
      return { relPath, source };
    })
    .filter(({ relPath }) => !relPath.endsWith(".d.ts"))
    .filter(({ relPath }) => !relPath.includes("__tests__"))
    .filter(({ relPath }) => !relPath.startsWith("test/"));
}

describe("services desktop bridge contract", () => {
  it("keeps raw tauri and plugin imports inside dedicated desktop/service adapters", () => {
    const violations: string[] = [];

    for (const { relPath, source } of listSourceFiles()) {
      if (!rawTauriImportPattern.test(source)) {
        continue;
      }

      if (!allowedRawTauriImportFiles.has(relPath)) {
        violations.push(relPath);
      }
    }

    expect(violations).toEqual([]);
  });

  it("keeps low-level invoke ownership in the shared service boundary", () => {
    const violations: string[] = [];

    for (const { relPath, source } of listSourceFiles()) {
      if (!relPath.startsWith("services/")) {
        continue;
      }
      if (!rawInvokePattern.test(source)) {
        continue;
      }

      if (!allowedRawInvokeFiles.has(relPath)) {
        violations.push(relPath);
      }
    }

    expect(violations).toEqual([]);
  });

  it("removes the deprecated invokeServiceCommand bridge", () => {
    const files = listSourceFiles().map(({ relPath }) => relPath);
    expect(files).not.toContain("services/invokeServiceCommand.ts");
  });

  it("keeps updater install as the only handwritten desktop ipc exception", () => {
    expect(bindingsSource).toContain("desktop_updater_check");
    expect(bindingsSource).not.toContain("desktop_updater_download_and_install");
    expect(updaterSource).toContain("DESKTOP_UPDATER_HANDWRITTEN_COMMAND");
    expect(updaterSource).toContain("DESKTOP_UPDATER_HANDWRITTEN_REASON");
    expect(updaterSource).toContain(
      "Requires a Tauri Channel callback, so this desktop updater path stays as the single handwritten desktop IPC exception."
    );
    expect(updaterSource.match(/invokeTauriOrNull(?:<[^>]+>)?\s*\(/g) ?? []).toHaveLength(1);
  });
});
