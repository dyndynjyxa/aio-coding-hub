import { describe, expect, it } from "vitest";

const capabilitySources = import.meta.glob("../../src-tauri/capabilities/*.json", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

type CapabilityDefinition = {
  identifier: string;
  windows: string[];
  permissions: unknown[];
};

function parseCapabilityDefinitions() {
  return Object.entries(capabilitySources)
    .map(([path, source]) => ({
      path: path.split("/").pop() ?? path,
      data: JSON.parse(source) as CapabilityDefinition,
    }))
    .sort((left, right) => left.path.localeCompare(right.path));
}

describe("tauri capabilities contract", () => {
  it("keeps only the renderer-owned main-core capability", () => {
    const definitions = parseCapabilityDefinitions();

    expect(definitions.map((item) => item.path)).toEqual(["main-core.json"]);

    expect(definitions.every((item) => item.data.windows.includes("main"))).toBe(true);

    const coreCapability = definitions.find((item) => item.data.identifier === "main-core");
    expect(coreCapability?.data.permissions).toEqual([
      "core:event:allow-listen",
      "core:event:allow-unlisten",
      "core:window:allow-start-dragging",
    ]);
  });
});
