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

function requirePathMissing(path, label) {
  if (existsSync(join(repoRoot, path))) {
    failures.push(`${path} must not exist as ${label}`);
  }
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

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function rustIntegerLiteral(value) {
  return String(value).replace(/\B(?=(\d{3})+(?!\d))/g, "_");
}

function functionBody(text, functionName) {
  const signature = new RegExp(`function\\s+${escapeRegex(functionName)}\\s*\\(`).exec(text);
  if (!signature) return null;
  const openBrace = text.indexOf("{", signature.index);
  if (openBrace === -1) return null;

  let depth = 0;
  for (let index = openBrace; index < text.length; index += 1) {
    const char = text[index];
    if (char === "{") depth += 1;
    if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return text.slice(openBrace + 1, index);
      }
    }
  }
  return null;
}

function requireObject(path, value) {
  if (value == null || typeof value !== "object" || Array.isArray(value)) {
    failures.push(`${path} must be an object`);
    return null;
  }
  return value;
}

function requireArray(path, value) {
  if (!Array.isArray(value)) {
    failures.push(`${path} must be an array`);
    return [];
  }
  return value;
}

function requireUniqueArray(path, values) {
  const seen = new Set();
  for (const value of values) {
    if (seen.has(value)) {
      failures.push(`${path} contains duplicate ${value}`);
    }
    seen.add(value);
  }
}

function requireArrayEquals(path, values, expected) {
  const array = requireArray(path, values);
  if (array.length !== expected.length || array.some((value, index) => value !== expected[index])) {
    failures.push(`${path} must equal [${expected.join(", ")}]`);
  }
  return array;
}

function requireOneOf(path, value, allowed) {
  if (!allowed.includes(value)) {
    failures.push(`${path} must be one of ${allowed.join(", ")}`);
  }
}

const contractPath = "docs/plugins/plugin-api-v1-contract.json";
const contract = readJson(contractPath);

if (contract) {
  const runtimes = requireObject(`${contractPath}.runtimes`, contract.runtimes) ?? {};
  for (const legacyRuntime of ["wasm", "process", "native"]) {
    if (Object.prototype.hasOwnProperty.call(runtimes, legacyRuntime)) {
      failures.push(`${contractPath}.runtimes must not expose ${legacyRuntime}`);
    }
  }
  const extensionHostRuntime = requireObject(
    `${contractPath}.runtimes.extensionHost`,
    runtimes.extensionHost
  );
  if (extensionHostRuntime) {
    if (extensionHostRuntime.language !== "typescript") {
      failures.push(`${contractPath}.runtimes.extensionHost.language must be typescript`);
    }
    if (extensionHostRuntime.requiresMain !== true) {
      failures.push(`${contractPath}.runtimes.extensionHost.requiresMain must be true`);
    }
    requireIncludes(
      `${contractPath}.runtimes.extensionHost`,
      JSON.stringify(extensionHostRuntime),
      ["mainOutput", "allowedMainExtensions", "lifecycle", "hookTimeoutMs", "dispose"],
      "Extension Host runtime lifecycle"
    );
  }
  const extensionHostContract = requireObject(
    `${contractPath}.extensionHostContract`,
    contract.extensionHostContract
  );
  if (extensionHostContract) {
    if (extensionHostContract.runtime !== "extensionHost") {
      failures.push(`${contractPath}.extensionHostContract.runtime must be extensionHost`);
    }
    if (extensionHostContract.language !== "typescript") {
      failures.push(`${contractPath}.extensionHostContract.language must be typescript`);
    }
    if (extensionHostContract.requiresMain !== true) {
      failures.push(`${contractPath}.extensionHostContract.requiresMain must be true`);
    }
    requireIncludes(
      `${contractPath}.extensionHostContract`,
      JSON.stringify(extensionHostContract),
      [
        "entryField",
        "mainOutput",
        "supportedSourceLanguages",
        "api.gateway.registerHook",
        "api.privacy.redactRequestBody",
      ],
      "Extension Host contract fields"
    );
  }
  requireArrayEquals(`${contractPath}.communityRuntimes`, contract.communityRuntimes, [
    "extensionHost",
  ]);
  const unsupportedLegacyRuntimes = requireArray(
    `${contractPath}.unsupportedLegacyRuntimes`,
    contract.unsupportedLegacyRuntimes
  );
  for (const legacyRuntime of ["wasm", "process", "native"]) {
    if (!unsupportedLegacyRuntimes.includes(legacyRuntime)) {
      failures.push(`${contractPath}.unsupportedLegacyRuntimes must include ${legacyRuntime}`);
    }
  }
  if (contract.policyGatedRuntimes !== undefined) {
    failures.push(`${contractPath}.policyGatedRuntimes must not expose public community runtimes`);
  }
  const capabilities = requireArray(`${contractPath}.capabilities`, contract.capabilities);
  const reservedHostMediatedLabels = ["network.fetch", "file.read", "file.write", "secret.read"];
  for (const label of reservedHostMediatedLabels) {
    if (capabilities.includes(label)) {
      failures.push(
        `${contractPath}.capabilities must not include reserved host-mediated label ${label}`
      );
    }
    if ((contract.activePermissions ?? []).includes(label)) {
      failures.push(
        `${contractPath}.activePermissions must not include reserved host-mediated label ${label}`
      );
    }
  }
  for (const capability of [
    "gateway.hooks",
    "protocol.bridge",
    "commands.execute",
    "provider.extensionValues",
    "privacy.redact",
  ]) {
    if (!capabilities.includes(capability)) {
      failures.push(`${contractPath}.capabilities must include ${capability}`);
    }
  }
  const contributionPoints = requireArray(
    `${contractPath}.contributionPoints`,
    contract.contributionPoints
  );
  for (const contribution of ["gatewayHooks", "protocolBridges", "commands"]) {
    if (!contributionPoints.includes(contribution)) {
      failures.push(`${contractPath}.contributionPoints must include ${contribution}`);
    }
  }
  const capabilityDependencies =
    requireObject(`${contractPath}.capabilityDependencies`, contract.capabilityDependencies) ?? {};
  const expectedCapabilityDependencies = {
    commands: ["commands.execute"],
    providers: ["provider.extensionValues"],
    "ui.providers.editor.sections": ["provider.extensionValues"],
    "ui.providers.editor.fields": ["provider.extensionValues"],
    "ui.buttonCommandFields": ["commands.execute"],
    gatewayHooks: ["gateway.hooks"],
    protocolBridges: ["protocol.bridge"],
  };
  for (const key of Object.keys(capabilityDependencies)) {
    if (!Object.prototype.hasOwnProperty.call(expectedCapabilityDependencies, key)) {
      failures.push(`${contractPath}.capabilityDependencies must not include ${key}`);
    }
  }
  for (const [contribution, requiredCapabilities] of Object.entries(
    expectedCapabilityDependencies
  )) {
    requireArrayEquals(
      `${contractPath}.capabilityDependencies.${contribution}`,
      capabilityDependencies[contribution],
      requiredCapabilities
    );
  }
  const protocolBridgeContribution = requireObject(
    `${contractPath}.protocolBridgeContribution`,
    contract.protocolBridgeContribution
  );
  if (protocolBridgeContribution) {
    if (protocolBridgeContribution.status !== "mvp-skeleton") {
      failures.push(`${contractPath}.protocolBridgeContribution.status must be mvp-skeleton`);
    }
    if (
      typeof protocolBridgeContribution.executionBoundary !== "string" ||
      !protocolBridgeContribution.executionBoundary.includes("future host integration")
    ) {
      failures.push(
        `${contractPath}.protocolBridgeContribution.executionBoundary must describe future host integration`
      );
    }
  }

  const matrix = requireObject(`${contractPath}.hookMatrix`, contract.hookMatrix) ?? {};
  for (const hook of contract.activeHooks ?? []) {
    const entry = requireObject(`hookMatrix.${hook}`, matrix[hook]);
    if (!entry) continue;
    requireOneOf(`hookMatrix.${hook}.kind`, entry.kind, ["request", "response", "stream", "log"]);
    requireOneOf(`hookMatrix.${hook}.status`, entry.status, ["active", "reserved"]);
    if (entry.status !== "active") {
      failures.push(`hookMatrix.${hook}.status must be active`);
    }
    const readPermissions = requireArray(
      `hookMatrix.${hook}.readPermissions`,
      entry.readPermissions
    );
    const writePermissions = requireArray(
      `hookMatrix.${hook}.writePermissions`,
      entry.writePermissions
    );
    requireUniqueArray(`hookMatrix.${hook}.readPermissions`, readPermissions);
    requireUniqueArray(`hookMatrix.${hook}.writePermissions`, writePermissions);
    const permissionDependencies =
      requireObject(`hookMatrix.${hook}.permissionDependencies`, entry.permissionDependencies) ??
      {};
    for (const [permission, requires] of Object.entries(permissionDependencies)) {
      if (!writePermissions.includes(permission)) {
        failures.push(
          `hookMatrix.${hook}.permissionDependencies.${permission} must be a write permission`
        );
      }
      const requiredPermissions = requireArray(
        `hookMatrix.${hook}.permissionDependencies.${permission}`,
        requires
      );
      for (const requiredPermission of requiredPermissions) {
        if (!readPermissions.includes(requiredPermission)) {
          failures.push(
            `hookMatrix.${hook}.permissionDependencies.${permission} requires unknown read permission ${requiredPermission}`
          );
        }
      }
    }
    const mutationFields = requireArray(`hookMatrix.${hook}.mutationFields`, entry.mutationFields);
    const contextFields = requireArray(`hookMatrix.${hook}.contextFields`, entry.contextFields);
    requireUniqueArray(`hookMatrix.${hook}.mutationFields`, mutationFields);
    requireUniqueArray(`hookMatrix.${hook}.contextFields`, contextFields);
    if (entry.defaultFailurePolicy !== contract.defaultFailurePolicy) {
      failures.push(`hookMatrix.${hook}.defaultFailurePolicy must equal defaultFailurePolicy`);
    }
    if (entry.timeoutMs !== contract.defaultHookTimeoutMs) {
      failures.push(`hookMatrix.${hook}.timeoutMs must equal defaultHookTimeoutMs`);
    }
    requireOneOf(`hookMatrix.${hook}.reservedHeaderPolicy`, entry.reservedHeaderPolicy, [
      "block-gateway-owned",
    ]);
  }

  const sdk = readText("packages/plugin-sdk/src/index.ts");
  const generatedBindings = readText("src/generated/bindings.ts");
  requireIncludes(
    "src/generated/bindings.ts",
    generatedBindings,
    ['kind: "extensionHost"'],
    "generated Extension Host runtime"
  );
  requireNotIncludes(
    "src/generated/bindings.ts",
    generatedBindings,
    ['kind: "native"', 'kind: "wasm"', 'kind: "process"'],
    "public runtime"
  );
  const pluginModules = readText("src-tauri/src/app/plugins/mod.rs");
  requireNotIncludes(
    "src-tauri/src/app/plugins/mod.rs",
    pluginModules,
    ["mod process_runtime", "mod wasm_runtime"],
    "alternate plugin runtime module"
  );
  const cargoToml = readText("src-tauri/Cargo.toml");
  requireNotIncludes(
    "src-tauri/Cargo.toml",
    cargoToml,
    ["wasmtime", "wat ="],
    "WASM runtime dependency"
  );
  const packageJson = readText("package.json");
  requireNotIncludes("package.json", packageJson, ["plugin-wasm-sdk:test"], "WASM SDK script");
  requirePathMissing("packages/plugin-wasm-sdk", "WASM SDK package");
  const pluginService = readText("src/services/plugins.ts");
  requireNotIncludes(
    "src/services/plugins.ts",
    pluginService,
    [
      "pluginGrantPermissions",
      "pluginRevokePermission",
      "plugin_grant_permissions",
      "plugin_revoke_permission",
    ],
    "manual permission frontend service wrapper"
  );
  const pluginQuery = readText("src/query/plugins.ts");
  requireNotIncludes(
    "src/query/plugins.ts",
    pluginQuery,
    ["usePluginGrantPermissionsMutation", "usePluginRevokePermissionMutation"],
    "manual permission query mutation"
  );
  requireIncludes(
    "packages/plugin-sdk/src/index.ts",
    sdk,
    [
      "export type ExtensionRuntime",
      'kind: "extensionHost"',
      "export type PluginRuntime = ExtensionRuntime",
      "export type PluginCapability",
      '"gateway.hooks"',
      '"privacy.redact"',
      '"protocol.bridge"',
      "export type GatewayHookContribution",
      "export type ProtocolBridgeContribution",
      "export type PrivacyApi",
      "gatewayHooks?: GatewayHookContribution[]",
      "protocolBridges?: ProtocolBridgeContribution[]",
      "validateCapabilityDependencies",
    ],
    "Extension Host SDK contract"
  );
  const capabilityDependencyBody = functionBody(sdk, "validateCapabilityDependencies");
  if (capabilityDependencyBody == null) {
    failures.push(
      "packages/plugin-sdk/src/index.ts is missing validateCapabilityDependencies body"
    );
  } else {
    requireIncludes(
      "packages/plugin-sdk/src/index.ts",
      capabilityDependencyBody,
      [
        "requireCapability",
        '"commands.execute"',
        "commands contribution",
        '"provider.extensionValues"',
        "provider contribution",
        '"providers.editor.sections"',
        "providers.editor.sections UI contribution",
        '"providers.editor.fields"',
        "providers.editor.fields UI contribution",
        "uiHasButtonCommand",
        "UI command field",
        '"gateway.hooks"',
        "gatewayHooks contribution",
        '"protocol.bridge"',
        "protocolBridges contribution",
      ],
      "capability dependency validation"
    );
  }

  const scaffold = readText("packages/create-aio-plugin/src/scaffold.ts");
  requireIncludes(
    "packages/create-aio-plugin/src/scaffold.ts",
    scaffold,
    [
      'main: "dist/extension.js"',
      'runtime: { kind: "extensionHost", language: "typescript" }',
      'hostCompatibility: { app: ">=0.60.0 <1.0.0", pluginApi: "^1.0.0" }',
      'capabilities: ["gateway.hooks"]',
      "contributes",
      "gatewayHooks:",
      "api.gateway.registerHook",
      'capabilities = ["commands.execute"]',
    ],
    "Extension Host scaffold package shape"
  );

  const devtools = readText("packages/create-aio-plugin/src/devtools.ts");
  requireIncludes(
    "packages/create-aio-plugin/src/devtools.ts",
    devtools,
    [
      "doctorPluginFiles",
      "validatePluginFilesStrict",
      "packPluginBytes",
      "publishCheckPluginBytes",
      "normalizeExtensionMainPath",
      "storedZipEntryNames",
      "dist/extension.js",
      "gateway.hooks",
      "contributes.gatewayHooks",
      "PLUGIN_INVALID_MAIN",
      "PLUGIN_UNSUPPORTED_LEGACY_RUNTIME",
      "PLUGIN_REPLAY_UNSUPPORTED",
    ],
    "Extension Host developer tool package shape"
  );
  requireNotIncludes(
    "packages/create-aio-plugin/src/devtools.ts",
    devtools,
    ["contextPatch"],
    "legacy mutation field"
  );

  const rustContract = readText("src-tauri/src/gateway/plugins/contract.rs");
  requireIncludes(
    "src-tauri/src/gateway/plugins/contract.rs",
    rustContract,
    contract.activeHooks,
    "active hook"
  );
  requireIncludes(
    "src-tauri/src/gateway/plugins/contract.rs",
    rustContract,
    contract.reservedHooks,
    "reserved hook"
  );
  requireIncludes(
    "src-tauri/src/gateway/plugins/contract.rs",
    rustContract,
    contract.activePermissions,
    "active permission"
  );
  requireIncludes(
    "src-tauri/src/gateway/plugins/contract.rs",
    rustContract,
    contract.reservedPermissions,
    "reserved permission"
  );

  const rust = readText("src-tauri/src/domain/plugins.rs");
  requireIncludes(
    "src-tauri/src/domain/plugins.rs",
    rust,
    [...contract.activePermissions, ...contract.reservedPermissions],
    "permission risk"
  );
  requireIncludes(
    "src-tauri/src/domain/plugins.rs",
    rust,
    [
      "crate::gateway::plugins::contract::is_active_hook",
      "crate::gateway::plugins::contract::is_reserved_hook",
      "crate::gateway::plugins::contract::hook_contract",
      "providers.editor.sections",
      "providers.editor.fields",
      "UI command field",
    ],
    "contract metadata call-through"
  );
  requireIncludesCaseInsensitive(
    "src-tauri/src/domain/plugins.rs",
    rust,
    ["extensionHost"],
    "runtime"
  );
  requireIncludes(
    "src-tauri/src/gateway/plugins/contract.rs",
    readText("src-tauri/src/gateway/plugins/contract.rs"),
    [
      `DEFAULT_HOOK_TIMEOUT_MS: u64 = ${rustIntegerLiteral(contract.defaultHookTimeoutMs)}`,
      'DEFAULT_FAILURE_POLICY: &str = "fail-open"',
      "timeout_ms: DEFAULT_HOOK_TIMEOUT_MS",
      "default_failure_policy: DEFAULT_FAILURE_POLICY",
    ],
    "default hook contract policy"
  );
  requireIncludes(
    "src-tauri/src/gateway/plugins/pipeline.rs",
    readText("src-tauri/src/gateway/plugins/pipeline.rs"),
    ["Duration::from_millis(DEFAULT_HOOK_TIMEOUT_MS)", "FailurePolicy::FailOpen"],
    "default hook pipeline policy"
  );

  const manifestSpec = readText("docs/plugin-manifest-v1.md");
  requireIncludes("docs/plugin-manifest-v1.md", manifestSpec, contract.activeHooks, "active hook");
  requireIncludes(
    "docs/plugin-manifest-v1.md",
    manifestSpec,
    contract.reservedHooks,
    "reserved hook"
  );
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
  for (const hook of contract.activeHooks ?? []) {
    const entry = matrix?.[hook];
    if (!entry) continue;
    const timeoutToken = `${entry.timeoutMs} ms`;
    const hookRow = manifestSpec.split("\n").find((line) => line.includes(`| \`${hook}\``));
    if (!hookRow) {
      failures.push(`docs/plugin-manifest-v1.md is missing hook table row for ${hook}`);
      continue;
    }
    if (!hookRow.includes(timeoutToken)) {
      failures.push(
        `docs/plugin-manifest-v1.md hook ${hook} row must include timeout ${timeoutToken}`
      );
    }
  }

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
    /pub fn is_reserved_gateway_hook\(hook: &str\)([\s\S]*?)\}/,
    "reserved hook validation helper"
  );
  requireIncludes(
    "src-tauri/src/domain/plugins.rs",
    rust,
    ["PLUGIN_RESERVED_HOOK"],
    "reserved validation error"
  );
  requireNotIncludes(
    "packages/plugin-sdk/src/index.ts",
    sdk,
    ["contextPatch"],
    "legacy mutation field"
  );
  requireNotIncludes(
    "packages/create-aio-plugin/src/scaffold.ts",
    scaffold,
    ["contextPatch"],
    "legacy mutation field"
  );
}

if (failures.length > 0) {
  console.error("Plugin API contract check failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}
