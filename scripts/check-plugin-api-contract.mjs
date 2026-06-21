import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const defaultRepoRoot = dirname(scriptDir);
const repoRoot = process.env.AIO_PLUGIN_CONTRACT_TEST_ROOT ?? defaultRepoRoot;

const failures = [];

function readText(path) {
  const fullPath = join(repoRoot, path);
  if (!existsSync(fullPath)) {
    failures.push(`${path} is missing`);
    return "";
  }
  return readFileSync(fullPath, "utf8");
}

function readJson(path) {
  const text = readText(path);
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch (error) {
    failures.push(`${path} is invalid JSON: ${error.message}`);
    return null;
  }
}

function requireIncludes(path, text, values, label) {
  for (const value of values) {
    if (!text.includes(value)) {
      failures.push(`${path} is missing ${label} ${value}`);
    }
  }
}

function requireIncludesCaseInsensitive(path, text, values, label) {
  const haystack = text.toLowerCase();
  for (const value of values) {
    if (!haystack.includes(value.toLowerCase())) {
      failures.push(`${path} is missing ${label} ${value}`);
    }
  }
}

function requireNotIncludes(path, text, values, label) {
  for (const value of values) {
    if (text.includes(value)) {
      failures.push(`${path} must not include ${label} ${value}`);
    }
  }
}

function requireRegex(path, text, regex, label) {
  if (!regex.test(text)) {
    failures.push(`${path} is missing ${label}`);
  }
}

function runtimeTokens(contract) {
  return [...contract.communityRuntimes, ...contract.policyGatedRuntimes];
}

function officialRuntimeTokens(contract) {
  return contract.officialRuntimes.flatMap((runtime) => runtime.split(":"));
}

function snakeCase(value) {
  return value.replace(/[A-Z]/g, (match) => `_${match.toLowerCase()}`);
}

const contractPath = "docs/plugins/plugin-api-v1-contract.json";
const contract = readJson(contractPath);

if (contract) {
  const sdk = readText("packages/plugin-sdk/src/index.ts");
  requireIncludes("packages/plugin-sdk/src/index.ts", sdk, contract.activeHooks, "active hook");
  requireIncludes("packages/plugin-sdk/src/index.ts", sdk, contract.reservedHooks, "reserved hook");
  requireIncludes(
    "packages/plugin-sdk/src/index.ts",
    sdk,
    contract.activePermissions,
    "active permission"
  );
  requireIncludes(
    "packages/plugin-sdk/src/index.ts",
    sdk,
    contract.reservedPermissions,
    "reserved permission"
  );
  requireIncludes("packages/plugin-sdk/src/index.ts", sdk, runtimeTokens(contract), "runtime");
  requireIncludes(
    "packages/plugin-sdk/src/index.ts",
    sdk,
    contract.activeMutationFields ?? [],
    "active mutation field"
  );

  const scaffold = readText("packages/create-aio-plugin/src/scaffold.ts");
  requireIncludes(
    "packages/create-aio-plugin/src/scaffold.ts",
    scaffold,
    contract.communityRuntimes,
    "community runtime"
  );
  requireIncludes(
    "packages/create-aio-plugin/src/scaffold.ts",
    scaffold,
    contract.policyGatedRuntimes,
    "policy-gated runtime"
  );
  requireIncludes(
    "packages/create-aio-plugin/src/scaffold.ts",
    scaffold,
    ["gateway.request.afterBodyRead", "request.body.read", "request.body.write"],
    "default scaffold contract token"
  );

  const rust = readText("src-tauri/src/domain/plugins.rs");
  requireIncludes("src-tauri/src/domain/plugins.rs", rust, contract.activeHooks, "active hook");
  requireIncludes("src-tauri/src/domain/plugins.rs", rust, contract.reservedHooks, "reserved hook");
  requireIncludes(
    "src-tauri/src/domain/plugins.rs",
    rust,
    contract.activePermissions,
    "active permission"
  );
  requireIncludes(
    "src-tauri/src/domain/plugins.rs",
    rust,
    contract.reservedPermissions,
    "reserved permission"
  );
  requireIncludesCaseInsensitive(
    "src-tauri/src/domain/plugins.rs",
    rust,
    [...runtimeTokens(contract), ...officialRuntimeTokens(contract)],
    "runtime"
  );
  requireIncludes(
    "src-tauri/src/gateway/plugins/pipeline.rs",
    readText("src-tauri/src/gateway/plugins/pipeline.rs"),
    [`Duration::from_millis(${contract.defaultHookTimeoutMs})`, "FailurePolicy::FailOpen"],
    "default hook policy"
  );

  const manifestSpec = readText("docs/plugin-manifest-v1.md");
  requireIncludes("docs/plugin-manifest-v1.md", manifestSpec, contract.activeHooks, "active hook");
  requireIncludes("docs/plugin-manifest-v1.md", manifestSpec, contract.reservedHooks, "reserved hook");
  requireIncludes(
    "docs/plugin-manifest-v1.md",
    manifestSpec,
    contract.activePermissions,
    "active permission"
  );
  requireIncludes(
    "docs/plugin-manifest-v1.md",
    manifestSpec,
    contract.reservedPermissions,
    "reserved permission"
  );

  const hooksDocPath = "docs/plugins/reference/hooks.md";
  const hooksDoc = readText(hooksDocPath);
  requireIncludes(hooksDocPath, hooksDoc, contract.activeHooks, "active hook");
  requireIncludes(hooksDocPath, hooksDoc, contract.reservedHooks, "reserved hook");

  const permissionsDocPath = "docs/plugins/reference/permissions.md";
  const permissionsDoc = readText(permissionsDocPath);
  requireIncludes(
    permissionsDocPath,
    permissionsDoc,
    contract.activePermissions,
    "active permission"
  );
  requireIncludes(
    permissionsDocPath,
    permissionsDoc,
    contract.reservedPermissions,
    "reserved permission"
  );

  const manifestGuidePath = "docs/plugins/reference/manifest.md";
  const manifestGuide = readText(manifestGuidePath);
  requireIncludes(
    manifestGuidePath,
    manifestGuide,
    [...runtimeTokens(contract), ...officialRuntimeTokens(contract)],
    "runtime"
  );

  const wasmGuidePath = "docs/plugins/runtime/wasm.md";
  const wasmGuide = readText(wasmGuidePath);
  requireIncludes(wasmGuidePath, wasmGuide, ["wasm", "PLUGIN_RUNTIME_DISABLED"], "WASM policy token");

  requireRegex(
    "packages/plugin-sdk/src/index.ts",
    sdk,
    /export type ActiveGatewayHookName\s*=([\s\S]*?)export type ReservedGatewayHookName/,
    "ActiveGatewayHookName union"
  );
  requireRegex(
    "packages/plugin-sdk/src/index.ts",
    sdk,
    /export type ReservedGatewayHookName\s*=([\s\S]*?)export type GatewayHookName/,
    "ReservedGatewayHookName union"
  );
  requireRegex(
    "src-tauri/src/domain/plugins.rs",
    rust,
    /pub fn is_active_gateway_hook\(hook: &str\)([\s\S]*?)pub fn is_reserved_gateway_hook/,
    "active hook validation helper"
  );
  requireRegex(
    "src-tauri/src/domain/plugins.rs",
    rust,
    /pub fn is_reserved_gateway_hook\(hook: &str\)([\s\S]*?)pub fn is_reserved_permission/,
    "reserved hook validation helper"
  );
  requireIncludes(
    "src-tauri/src/domain/plugins.rs",
    rust,
    ["PLUGIN_RESERVED_HOOK", "PLUGIN_RESERVED_PERMISSION"],
    "reserved validation error"
  );
  requireNotIncludes("packages/plugin-sdk/src/index.ts", sdk, ["contextPatch"], "legacy mutation field");
  requireNotIncludes(
    "packages/create-aio-plugin/src/scaffold.ts",
    scaffold,
    ["contextPatch"],
    "legacy mutation field"
  );

  const wasmSdk = readText("packages/plugin-wasm-sdk/src/lib.rs");
  requireIncludes(
    "packages/plugin-wasm-sdk/src/lib.rs",
    wasmSdk,
    (contract.activeMutationFields ?? []).map(snakeCase),
    "active mutation field"
  );
  requireIncludes(
    "packages/plugin-wasm-sdk/src/lib.rs",
    wasmSdk,
    ['#[serde(rename_all = "camelCase")]'],
    "camelCase serde ABI"
  );
}

if (failures.length > 0) {
  console.error("Plugin API contract check failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}
