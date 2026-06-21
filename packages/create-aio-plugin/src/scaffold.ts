import type { PluginManifest } from "@aio-coding-hub/plugin-sdk";

export type ScaffoldTemplate = "rule" | "wasm";

export type ScaffoldInput = {
  id: string;
  name: string;
  template: ScaffoldTemplate;
};

export type ScaffoldFiles = Record<string, string>;

export function createPluginScaffold(input: ScaffoldInput): ScaffoldFiles {
  const id = normalizeId(input.id);
  const name = normalizeName(input.name);
  return input.template === "wasm" ? wasmTemplate(id, name) : ruleTemplate(id, name);
}

function ruleTemplate(id: string, name: string): ScaffoldFiles {
  const manifest: PluginManifest = {
    id,
    name,
    version: "0.1.0",
    apiVersion: "1.0.0",
    runtime: { kind: "declarativeRules", rules: ["rules/main.json"] },
    hooks: [{ name: "gateway.request.afterBodyRead", priority: 100 }],
    permissions: ["request.body.read", "request.body.write"],
    hostCompatibility: { app: ">=0.56.0 <1.0.0", pluginApi: "^1.0.0" },
    description: "Declarative rule plugin scaffold.",
  };

  return {
    "plugin.json": `${JSON.stringify(manifest, null, 2)}\n`,
    "rules/main.json": `${JSON.stringify(
      {
        rules: [
          {
            id: "redact-placeholder",
            hook: "gateway.request.afterBodyRead",
            target: { field: "request.body", jsonPath: "$.messages[*].content" },
            match: { regex: "SECRET_[A-Za-z0-9_]+", caseSensitive: true },
            action: { kind: "replace", replacement: "[REDACTED]" },
          },
        ],
      },
      null,
      2
    )}\n`,
    "README.md": `# ${name}\n\nPlugin ID: \`${id}\`.\n\nThis scaffold uses declarative rules and does not execute JavaScript in the host.\n`,
  };
}

function wasmTemplate(id: string, name: string): ScaffoldFiles {
  const manifest: PluginManifest = {
    id,
    name,
    version: "0.1.0",
    apiVersion: "1.0.0",
    runtime: { kind: "wasm", abiVersion: "1.0.0", memoryLimitBytes: 16 * 1024 * 1024 },
    hooks: [{ name: "gateway.request.afterBodyRead", priority: 100 }],
    permissions: ["request.meta.read"],
    hostCompatibility: { app: ">=0.56.0 <1.0.0", pluginApi: "^1.0.0" },
    entry: "plugin.wasm",
    description: "Experimental WASM plugin scaffold.",
  };

  return {
    "plugin.json": `${JSON.stringify(manifest, null, 2)}\n`,
    "src/lib.rs": `#[no_mangle]\npub extern "C" fn aio_plugin_handle(_ptr: i32, _len: i32) -> i64 {\n    0\n}\n`,
    "README.md": `# ${name}\n\nPlugin ID: \`${id}\`.\n\nThis template packages a WASM artifact and validates the ABI, but gateway execution remains policy-gated. The host rejects enablement with PLUGIN_RUNTIME_DISABLED until WASM execution is enabled by host policy.\n`,
  };
}

function normalizeId(value: string): string {
  const id = value.trim();
  if (!/^[a-z0-9][a-z0-9-]*(\.[a-z0-9][a-z0-9-]*)+$/.test(id)) {
    throw new Error("PLUGIN_INVALID_ID: expected publisher.plugin-name");
  }
  return id;
}

function normalizeName(value: string): string {
  const name = value.trim();
  if (!name) {
    throw new Error("PLUGIN_INVALID_NAME: plugin name is required");
  }
  return name;
}
