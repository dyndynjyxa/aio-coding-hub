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
requireScript(rootPackage, "test:e2e", "vitest run src/e2e");

const workspace = readText("pnpm-workspace.yaml");
if (!workspace.includes("packages/*")) {
  failures.push("pnpm-workspace.yaml: packages/* workspace is required");
}

requireFile("packages/plugin-sdk/src/index.ts");
requireFile("packages/create-aio-plugin/src/scaffold.ts");
requireFile("packages/create-aio-plugin/src/devtools.ts");

const ci = readText(".github/workflows/ci.yml");
for (const phrase of [
  "pnpm check:plugin-api-contract",
  "pnpm check:plugin-system-docs",
  "pnpm check:generated-bindings",
  "pnpm --filter @aio-coding-hub/plugin-sdk typecheck",
  "pnpm --filter create-aio-plugin test",
  "pnpm test:e2e",
]) {
  if (!ci.includes(phrase)) {
    failures.push(`.github/workflows/ci.yml: missing "${phrase}"`);
  }
}

const docs = [
  "docs/plugin-manifest-v1.md",
  "docs/plugins/reference/sdk.md",
  "docs/plugins/developer-guide.md",
  "docs/plugins/runtime/README.md",
];
for (const doc of docs) {
  const text = readText(doc);
  if (!text.includes("Extension Host")) {
    failures.push(`${doc}: must reference Extension Host`);
  }
}

for (const [doc, phrases] of Object.entries({
  "docs/plugin-manifest-v1.md": [
    'runtime.kind = "extensionHost"',
    "`main` points at bundled JavaScript output",
    "`contributes.gatewayHooks`",
    '`capabilities: ["gateway.hooks"]`',
    "`api.gateway.registerHook`",
  ],
  "docs/plugins/developer-guide.md": [
    "Extension Host 是唯一 community runtime",
    '`runtime.kind = "extensionHost"`',
    "`main` 指向打包后的 JavaScript 输出",
    "`contributes.gatewayHooks`",
    "`api.gateway.registerHook`",
    "PLUGIN_REPLAY_UNSUPPORTED",
  ],
  "docs/plugins/plugin-api-v1-contract.json": [
    '"communityRuntimes": [',
    '"extensionHost"',
    '"unsupportedLegacyRuntimes"',
    '"gatewayHooks"',
    '"protocolBridges"',
  ],
})) {
  const text = readText(doc);
  for (const phrase of phrases) {
    if (!text.includes(phrase)) {
      failures.push(`${doc}: missing "${phrase}"`);
    }
  }
}

for (const [doc, phrases] of Object.entries({
  "docs/plugins/developer-guide.md": [
    "WASM 执行受宿主策略控制",
    "`plugin.wasm`",
    "pnpm --filter create-aio-plugin cli replay",
  ],
  "docs/plugin-manifest-v1.md": ['"kind": "wasm"'],
  "docs/plugins/plugin-api-v1-contract.json": ['"policyGatedRuntimes"'],
})) {
  const text = readText(doc);
  for (const phrase of phrases) {
    if (text.includes(phrase)) {
      failures.push(`${doc}: forbidden "${phrase}"`);
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
