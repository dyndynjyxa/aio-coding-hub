import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = dirname(scriptDir);

const failures = [];

function readJson(path) {
  const fullPath = join(repoRoot, path);
  if (!existsSync(fullPath)) {
    failures.push(`${path}: missing`);
    return null;
  }
  try {
    return JSON.parse(readFileSync(fullPath, "utf8"));
  } catch (error) {
    failures.push(`${path}: invalid JSON: ${error.message}`);
    return null;
  }
}

function readText(path) {
  const fullPath = join(repoRoot, path);
  if (!existsSync(fullPath)) {
    failures.push(`${path}: missing`);
    return "";
  }
  return readFileSync(fullPath, "utf8");
}

function requireFile(path) {
  if (!existsSync(join(repoRoot, path))) failures.push(`${path}: missing`);
}

function requireScript(packageJson, name, expected) {
  if (packageJson?.scripts?.[name] !== expected) {
    failures.push(`package.json: expected script "${name}" to be "${expected}"`);
  }
}

const rootPackage = readJson("package.json");
requireScript(
  rootPackage,
  "check:plugin-api-contract",
  "node scripts/check-plugin-api-contract.mjs"
);
requireScript(
  rootPackage,
  "plugin-wasm-sdk:test",
  "cargo test --manifest-path packages/plugin-wasm-sdk/Cargo.toml && cargo test --manifest-path packages/plugin-wasm-sdk/examples/redactor/Cargo.toml"
);
requireScript(rootPackage, "test:e2e", "vitest run src/e2e");

const workspace = readText("pnpm-workspace.yaml");
if (!workspace.includes("packages/*")) {
  failures.push("pnpm-workspace.yaml: packages/* workspace is required");
}

const sdkCargo = readText("packages/plugin-wasm-sdk/Cargo.toml");
for (const phrase of [
  'name = "aio-plugin-wasm-sdk"',
  "serde",
  "serde_json",
  "crate-type",
]) {
  if (!sdkCargo.includes(phrase)) {
    failures.push(`packages/plugin-wasm-sdk/Cargo.toml: missing "${phrase}"`);
  }
}

requireFile("packages/plugin-wasm-sdk/src/lib.rs");
requireFile("packages/plugin-wasm-sdk/examples/redactor/Cargo.toml");
requireFile("packages/plugin-wasm-sdk/examples/redactor/src/lib.rs");
requireFile("packages/plugin-wasm-sdk/tests/sdk_contract.rs");

const sdkLib = readText("packages/plugin-wasm-sdk/src/lib.rs");
for (const phrase of [
  "HookRequest",
  "HookResult",
  "PluginManifest",
  "aio_plugin_entrypoint",
  "serde",
]) {
  if (!sdkLib.includes(phrase)) {
    failures.push(`packages/plugin-wasm-sdk/src/lib.rs: missing "${phrase}"`);
  }
}

const ci = readText(".github/workflows/ci.yml");
for (const phrase of [
  "pnpm check:plugin-api-contract",
  "pnpm check:plugin-system-docs",
  "pnpm check:generated-bindings",
  "pnpm plugin-sdk:typecheck",
  "cargo test --manifest-path packages/plugin-wasm-sdk/Cargo.toml",
  "cargo test --manifest-path packages/plugin-wasm-sdk/examples/redactor/Cargo.toml",
  "pnpm create-aio-plugin:test",
  "pnpm test:e2e",
]) {
  if (!ci.includes(phrase)) {
    failures.push(`.github/workflows/ci.yml: missing "${phrase}"`);
  }
}

const docs = [
  "docs/plugins/reference/sdk.md",
  "docs/plugins/developer-guide.md",
  "docs/plugins/runtime/wasm.md",
];
for (const doc of docs) {
  const text = readText(doc);
  if (!text.includes("plugin-wasm-sdk")) {
    failures.push(`${doc}: must reference plugin-wasm-sdk`);
  }
}

for (const [doc, phrases] of Object.entries({
  "docs/plugins/developer-guide.md": [
    "`declarativeRules` 是默认社区运行时",
    "WASM 执行受宿主策略控制",
    "`plugin.wasm`",
  ],
  "docs/plugins/runtime/wasm.md": [
    "`declarativeRules` 是默认社区运行时",
    "WASM 只用于宿主策略启用后",
    "`plugin.wasm` artifacts 会由 `create-aio-plugin pack` 作为 binary files 打包",
  ],
})) {
  const text = readText(doc);
  for (const phrase of phrases) {
    if (!text.includes(phrase)) {
      failures.push(`${doc}: missing "${phrase}"`);
    }
  }
}

if (failures.length > 0) {
  console.error("Plugin system completion contract failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}
