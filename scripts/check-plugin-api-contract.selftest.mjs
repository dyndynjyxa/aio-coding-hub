import { spawnSync } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

function writeJson(root, path, value) {
  const output =
    path === "docs/plugins/plugin-api-v1-contract.json" ? withContractDefaults(value) : value;
  writeFileSync(join(root, path), JSON.stringify(output, null, 2));
}

function withContractDefaults(value) {
  const mergeUnique = (left, right) => [...new Set([...(left ?? []), ...(right ?? [])])];
  return {
    ...value,
    runtimes: {
      extensionHost: {
        language: "typescript",
        requiresMain: true,
        mainOutput: "bundled JavaScript or CommonJS file",
        allowedMainExtensions: [".js", ".cjs"],
        lifecycle: {
          hookTimeoutMs: 150,
          dispose: "host-managed",
        },
        status: "mainline-contract",
      },
      ...(value.runtimes ?? {}),
    },
    extensionHostContract: {
      runtime: "extensionHost",
      language: "typescript",
      requiresMain: true,
      entryField: "main",
      mainOutput: "dist/extension.js",
      supportedSourceLanguages: ["typescript", "javascript"],
      lifecycle: {
        gatewayRegistration: "api.gateway.registerHook",
        privacyRedaction: "api.privacy.redactRequestBody",
      },
      status: "mainline-contract",
      ...(value.extensionHostContract ?? {}),
    },
    capabilities: mergeUnique(
      [
        "gateway.hooks",
        "protocol.bridge",
        "commands.execute",
        "provider.extensionValues",
        "privacy.redact",
      ],
      value.capabilities
    ),
    contributionPoints: mergeUnique(
      ["gatewayHooks", "protocolBridges", "commands"],
      value.contributionPoints
    ),
    protocolBridgeContribution: {
      requiredFields: ["bridgeType", "inboundProtocol", "outboundProtocol"],
      optionalFields: ["supportsStreaming"],
      status: "mvp-skeleton",
      executionBoundary:
        "manifest declaration, capability dependency, and install preview only; full protocol bridge execution is future host integration",
      ...(value.protocolBridgeContribution ?? {}),
    },
  };
}

function makeRoot(name) {
  const root = join(tmpdir(), `aio-plugin-contract-${name}-${Date.now()}`);
  mkdirSync(join(root, "docs/plugins"), { recursive: true });
  mkdirSync(join(root, "docs/plugins/reference"), { recursive: true });
  mkdirSync(join(root, "docs/plugins/runtime"), { recursive: true });
  mkdirSync(join(root, "packages/plugin-sdk/src"), { recursive: true });
  mkdirSync(join(root, "packages/create-aio-plugin/src"), { recursive: true });
  mkdirSync(join(root, "src/services"), { recursive: true });
  mkdirSync(join(root, "src/query"), { recursive: true });
  mkdirSync(join(root, "src/generated"), { recursive: true });
  mkdirSync(join(root, "src-tauri/src/app/plugins"), { recursive: true });
  mkdirSync(join(root, "src-tauri/src/domain"), { recursive: true });
  mkdirSync(join(root, "src-tauri/src/gateway/plugins"), { recursive: true });
  writeFileSync(
    join(root, "src/generated/bindings.ts"),
    'export type PluginRuntime = { kind: "extensionHost"; language: string };\n'
  );
  writeFileSync(
    join(root, "src-tauri/src/app/plugins/mod.rs"),
    "pub(crate) mod extension_host;\npub(crate) mod extension_host_process;\n"
  );
  writeFileSync(
    join(root, "src-tauri/Cargo.toml"),
    '[dependencies]\nrquickjs = "0.12.0"\n[dev-dependencies]\ntempfile = "3"\n'
  );
  writeFileSync(join(root, "package.json"), '{ "scripts": {} }\n');
  writeFileSync(join(root, "src/services/plugins.ts"), "export function pluginEnable() {}\n");
  writeFileSync(
    join(root, "src/query/plugins.ts"),
    "export function usePluginEnableMutation() {}\n"
  );
  return root;
}

function runCheck(root) {
  return spawnSync("node", ["scripts/check-plugin-api-contract.mjs"], {
    cwd: process.cwd(),
    env: { ...process.env, AIO_PLUGIN_CONTRACT_TEST_ROOT: root },
    encoding: "utf8",
  });
}

function writePassingDevtools(root) {
  writeFileSync(
    join(root, "packages/create-aio-plugin/src/devtools.ts"),
    [
      "doctorPluginFiles validatePluginFilesStrict packPluginBytes publishCheckPluginBytes",
      "normalizeExtensionMainPath storedZipEntryNames",
      "dist/extension.js gateway.hooks contributes.gatewayHooks",
      "PLUGIN_INVALID_MAIN PLUGIN_UNSUPPORTED_LEGACY_RUNTIME PLUGIN_REPLAY_UNSUPPORTED",
    ].join("\n")
  );
}

function writePassingManifestDocs(root) {
  writeFileSync(
    join(root, "docs/plugin-manifest-v1.md"),
    [
      "| `gateway.request.afterBodyRead` | phase | mutation | 150 ms | fail-open | host mediated |",
      "| `gateway.request.beforeSend` | phase | mutation | 150 ms | fail-open | host mediated |",
      "| `gateway.response.chunk` | phase | mutation | 150 ms | fail-open | host mediated |",
      "gateway.response.headers",
      "request.meta.read request.header.read request.header.readSensitive",
      "request.body.read request.body.write stream.inspect stream.modify network.fetch",
    ].join("\n")
  );
  writeFileSync(
    join(root, "docs/plugins/reference/hooks.md"),
    "gateway.request.afterBodyRead gateway.request.beforeSend gateway.response.chunk gateway.response.headers"
  );
  writeFileSync(
    join(root, "docs/plugins/reference/permissions.md"),
    "request.meta.read request.header.read request.header.readSensitive request.body.read request.body.write stream.inspect stream.modify network.fetch"
  );
  writeFileSync(
    join(root, "docs/plugins/reference/manifest.md"),
    "extensionHost wasm process native privacyFilter"
  );
}

function writePassingScaffold(root) {
  writeFileSync(
    join(root, "packages/plugin-sdk/src/index.ts"),
    [
      [
        "gateway.request.afterBodyRead",
        "gateway.request.beforeSend",
        "gateway.response.chunk",
        "gateway.response.after",
        "gateway.error",
        "log.beforePersist",
        "gateway.response.headers",
      ].join(" "),
      [
        "request.meta.read",
        "request.header.read",
        "request.header.readSensitive",
        "request.header.write",
        "request.body.read",
        "request.body.write",
        "response.header.read",
        "response.header.write",
        "response.body.read",
        "response.body.write",
        "stream.inspect",
        "stream.modify",
        "log.redact",
        "network.fetch",
      ].join(" "),
      "requestBody responseBody streamChunk logMessage headers",
      'export type ExtensionRuntime = { kind: "extensionHost"; language: "typescript" };',
      "export type PluginRuntime = ExtensionRuntime;",
      'export type PluginCapability = "gateway.hooks" | "protocol.bridge" | "commands.execute" | "provider.extensionValues" | "privacy.redact";',
      "export type GatewayHookContribution = { name: string };",
      "export type ProtocolBridgeContribution = { bridgeType: string };",
      "export type PrivacyApi = { redactText(text: string): unknown; redactRequestBody(body: string): unknown };",
      "type PluginContributes = { commands?: unknown[]; providers?: unknown[]; gatewayHooks?: GatewayHookContribution[]; protocolBridges?: ProtocolBridgeContribution[]; ui?: Record<string, unknown[]> };",
      [
        "export type ActiveGatewayHookName =",
        "'gateway.request.afterBodyRead' |",
        "'gateway.request.beforeSend' |",
        "'gateway.response.chunk' |",
        "'gateway.response.after' |",
        "'gateway.error' |",
        "'log.beforePersist';",
      ].join(" "),
      "export type ReservedGatewayHookName = 'gateway.response.headers';",
      "export type GatewayHookName = ActiveGatewayHookName | ReservedGatewayHookName;",
      "type PluginManifest = { permissions: string[]; hooks: { name: string }[] };",
      "function validateManifest(manifest: PluginManifest) {",
      "  return validatePermissionSet(manifest);",
      "}",
      "function validatePermissionSet(manifest: PluginManifest) {",
      "  const set = new Set(manifest.permissions);",
      "  const hooks = new Set(manifest.hooks.map((hook) => hook.name));",
      "  if (hooks.has('gateway.request.afterBodyRead') && set.has('request.body.write') && !set.has('request.body.read')) return 'request.body.write requires request.body.read';",
      "  if (hooks.has('gateway.response.after') && set.has('response.body.write') && !set.has('response.body.read')) return 'response.body.write requires response.body.read';",
      "  if (hooks.has('gateway.response.chunk') && set.has('stream.modify') && !set.has('stream.inspect')) return 'stream.modify requires stream.inspect';",
      "  return null;",
      "}",
      "function validateCapabilityDependencies(contributes: PluginContributes, capabilities: PluginCapability[]) {",
      "  const requireCapability = (capability: PluginCapability, reason: string) => capabilities.includes(capability) ? null : `${reason} requires ${capability}`;",
      "  if ((contributes.commands?.length ?? 0) > 0) {",
      '    const error = requireCapability("commands.execute", "commands contribution");',
      "    if (error) return error;",
      "  }",
      "  if ((contributes.providers?.length ?? 0) > 0) {",
      '    const error = requireCapability("provider.extensionValues", "provider contribution");',
      "    if (error) return error;",
      "  }",
      "  if ((contributes.gatewayHooks?.length ?? 0) > 0) {",
      '    const error = requireCapability("gateway.hooks", "gatewayHooks contribution");',
      "    if (error) return error;",
      "  }",
      "  if ((contributes.protocolBridges?.length ?? 0) > 0) {",
      '    const error = requireCapability("protocol.bridge", "protocolBridges contribution");',
      "    if (error) return error;",
      "  }",
      '  if ((contributes.ui?.["providers.editor.sections"]?.length ?? 0) > 0) {',
      '    const error = requireCapability("provider.extensionValues", "providers.editor.sections UI contribution");',
      "    if (error) return error;",
      "  }",
      '  if ((contributes.ui?.["providers.editor.fields"]?.length ?? 0) > 0) {',
      '    const error = requireCapability("provider.extensionValues", "providers.editor.fields UI contribution");',
      "    if (error) return error;",
      "  }",
      "  if (uiHasButtonCommand(contributes.ui)) {",
      '    const error = requireCapability("commands.execute", "UI command field");',
      "    if (error) return error;",
      "  }",
      "  return null;",
      "}",
      "function uiHasButtonCommand(ui: unknown) { return String(ui).includes('button'); }",
    ].join("\n")
  );
  writeFileSync(
    join(root, "packages/create-aio-plugin/src/scaffold.ts"),
    [
      'main: "dist/extension.js"',
      'runtime: { kind: "extensionHost", language: "typescript" }',
      'hostCompatibility: { app: ">=0.60.0 <1.0.0", pluginApi: "^1.0.0" }',
      'capabilities: ["gateway.hooks"]',
      "contributes gatewayHooks:",
      "api.gateway.registerHook",
      'capabilities = ["commands.execute"]',
    ].join("\n")
  );
  writeFileSync(
    join(root, "src-tauri/src/gateway/plugins/contract.rs"),
    [
      "pub(crate) const DEFAULT_HOOK_TIMEOUT_MS: u64 = 150;",
      'pub(crate) const DEFAULT_FAILURE_POLICY: &str = "fail-open";',
      "timeout_ms: DEFAULT_HOOK_TIMEOUT_MS default_failure_policy: DEFAULT_FAILURE_POLICY",
      [
        "gateway.request.afterBodyRead",
        "gateway.request.beforeSend",
        "gateway.response.chunk",
        "gateway.response.after",
        "gateway.error",
        "log.beforePersist",
        "gateway.response.headers",
      ].join(" "),
      [
        "request.meta.read",
        "request.header.read",
        "request.header.readSensitive",
        "request.header.write",
        "request.body.read",
        "request.body.write",
        "response.header.read",
        "response.header.write",
        "response.body.read",
        "response.body.write",
        "stream.inspect",
        "stream.modify",
        "log.redact",
        "network.fetch",
      ].join(" "),
    ].join("\n")
  );
  writeFileSync(
    join(root, "src-tauri/src/domain/plugins.rs"),
    [
      "extensionHost wasm process native privacyFilter",
      "crate::gateway::plugins::contract::is_active_hook",
      "crate::gateway::plugins::contract::is_reserved_hook",
      "crate::gateway::plugins::contract::hook_contract",
      [
        "request.meta.read",
        "request.header.read",
        "request.header.readSensitive",
        "request.header.write",
        "request.body.read",
        "request.body.write",
        "response.header.read",
        "response.header.write",
        "response.body.read",
        "response.body.write",
        "stream.inspect",
        "stream.modify",
        "log.redact",
        "network.fetch",
      ].join(" "),
      "providers.editor.sections UI contribution requires provider.extensionValues",
      "providers.editor.fields UI contribution requires provider.extensionValues",
      "UI command field requires commands.execute",
      "pub fn is_active_gateway_hook(hook: &str) -> bool {",
      '  hook == "gateway.request.afterBodyRead" || hook == "gateway.request.beforeSend"',
      "}",
      'pub fn is_reserved_gateway_hook(hook: &str) -> bool { hook == "gateway.response.headers" }',
      "fn permission_risk(permission: &str) { request.body.read; request.body.write; network.fetch; }",
      "PLUGIN_RESERVED_HOOK",
    ].join("\n")
  );
  writeFileSync(
    join(root, "src-tauri/src/gateway/plugins/pipeline.rs"),
    "Duration::from_millis(DEFAULT_HOOK_TIMEOUT_MS) FailurePolicy::FailOpen"
  );
  writeFileSync(
    join(root, "docs/plugin-manifest-v1.md"),
    [
      "gateway.request.afterBodyRead gateway.request.beforeSend gateway.response.headers",
      "request.body.read request.body.write network.fetch",
    ].join("\n")
  );
  writeFileSync(
    join(root, "docs/plugins/reference/hooks.md"),
    "gateway.request.afterBodyRead gateway.request.beforeSend gateway.response.headers"
  );
  writeFileSync(
    join(root, "docs/plugins/reference/permissions.md"),
    "request.body.read request.body.write network.fetch"
  );
  writeFileSync(
    join(root, "docs/plugins/reference/manifest.md"),
    "extensionHost wasm process native privacyFilter"
  );
  writeFileSync(join(root, "docs/plugins/runtime/wasm.md"), "wasm PLUGIN_RUNTIME_DISABLED");
  writePassingManifestDocs(root);
  writePassingDevtools(root);
}

const extensionHostDependencyBaselineRoot = makeRoot("extension-host-dependency-baseline");
writeJson(extensionHostDependencyBaselineRoot, "docs/plugins/plugin-api-v1-contract.json", {
  apiVersion: "1.0.0",
  defaultHookTimeoutMs: 150,
  defaultFailurePolicy: "fail-open",
  activeHooks: [
    "gateway.request.afterBodyRead",
    "gateway.request.beforeSend",
    "gateway.response.chunk",
  ],
  reservedHooks: ["gateway.response.headers"],
  activeMutationFields: ["requestBody", "streamChunk"],
  configSchemaTypes: ["object"],
  activePermissions: [
    "request.meta.read",
    "request.header.read",
    "request.header.readSensitive",
    "request.body.read",
    "request.body.write",
    "stream.inspect",
    "stream.modify",
  ],
  reservedPermissions: ["network.fetch"],
  capabilityDependencies: {
    commands: ["commands.execute"],
    providers: ["provider.extensionValues"],
    "ui.providers.editor.sections": ["provider.extensionValues"],
    "ui.providers.editor.fields": ["provider.extensionValues"],
    "ui.buttonCommandFields": ["commands.execute"],
    gatewayHooks: ["gateway.hooks"],
    protocolBridges: ["protocol.bridge"],
  },
  hookMatrix: {
    "gateway.request.afterBodyRead": {
      phase: "after request body read and before upstream provider send",
      kind: "request",
      status: "active",
      defaultFailurePolicy: "fail-open",
      timeoutMs: 150,
      reservedHeaderPolicy: "block-gateway-owned",
      readPermissions: [
        "request.meta.read",
        "request.header.read",
        "request.header.readSensitive",
        "request.body.read",
      ],
      writePermissions: ["request.body.write"],
      permissionDependencies: { "request.body.write": ["request.body.read"] },
      mutationFields: ["requestBody"],
      contextFields: ["traceId", "request.body"],
    },
    "gateway.request.beforeSend": {
      phase: "after provider resolution and before upstream provider send",
      kind: "request",
      status: "active",
      defaultFailurePolicy: "fail-open",
      timeoutMs: 150,
      reservedHeaderPolicy: "block-gateway-owned",
      readPermissions: ["request.body.read"],
      writePermissions: ["request.body.write"],
      permissionDependencies: {},
      mutationFields: ["requestBody"],
      contextFields: ["traceId", "request.body"],
    },
    "gateway.response.chunk": {
      phase: "for each bounded streaming response chunk",
      kind: "stream",
      status: "active",
      defaultFailurePolicy: "fail-open",
      timeoutMs: 150,
      reservedHeaderPolicy: "block-gateway-owned",
      readPermissions: ["stream.inspect"],
      writePermissions: ["stream.modify"],
      permissionDependencies: { "stream.modify": ["stream.inspect"] },
      mutationFields: ["streamChunk"],
      contextFields: ["traceId", "stream.chunk"],
    },
  },
  communityRuntimes: ["extensionHost"],
  unsupportedLegacyRuntimes: ["wasm", "process", "native"],
});
writePassingScaffold(extensionHostDependencyBaselineRoot);

const extensionHostDependencyBaselineResult = runCheck(extensionHostDependencyBaselineRoot);
if (extensionHostDependencyBaselineResult.status !== 0) {
  throw new Error(
    `expected Extension Host dependency baseline to pass, got status ${extensionHostDependencyBaselineResult.status}\n${extensionHostDependencyBaselineResult.stderr}`
  );
}

const reservedHostMediatedLabelDriftRoot = makeRoot("reserved-host-mediated-label-drift");
writeJson(reservedHostMediatedLabelDriftRoot, "docs/plugins/plugin-api-v1-contract.json", {
  apiVersion: "1.0.0",
  defaultHookTimeoutMs: 150,
  defaultFailurePolicy: "fail-open",
  activeHooks: [
    "gateway.request.afterBodyRead",
    "gateway.request.beforeSend",
    "gateway.response.chunk",
  ],
  reservedHooks: ["gateway.response.headers"],
  activeMutationFields: ["requestBody", "streamChunk"],
  configSchemaTypes: ["object"],
  capabilities: ["network.fetch"],
  activePermissions: [
    "request.meta.read",
    "request.header.read",
    "request.header.readSensitive",
    "request.body.read",
    "request.body.write",
    "stream.inspect",
    "stream.modify",
    "network.fetch",
  ],
  reservedPermissions: ["network.fetch"],
  capabilityDependencies: {
    commands: ["commands.execute"],
    providers: ["provider.extensionValues"],
    "ui.providers.editor.sections": ["provider.extensionValues"],
    "ui.providers.editor.fields": ["provider.extensionValues"],
    "ui.buttonCommandFields": ["commands.execute"],
    gatewayHooks: ["gateway.hooks"],
    protocolBridges: ["protocol.bridge"],
  },
  hookMatrix: JSON.parse(
    readFileSync(
      join(extensionHostDependencyBaselineRoot, "docs/plugins/plugin-api-v1-contract.json"),
      "utf8"
    )
  ).hookMatrix,
  communityRuntimes: ["extensionHost"],
  unsupportedLegacyRuntimes: ["wasm", "process", "native"],
});
writePassingScaffold(reservedHostMediatedLabelDriftRoot);

const reservedHostMediatedLabelDriftResult = runCheck(reservedHostMediatedLabelDriftRoot);
if (
  reservedHostMediatedLabelDriftResult.status === 0 ||
  !reservedHostMediatedLabelDriftResult.stderr.includes(
    "capabilities must not include reserved host-mediated label network.fetch"
  ) ||
  !reservedHostMediatedLabelDriftResult.stderr.includes(
    "activePermissions must not include reserved host-mediated label network.fetch"
  )
) {
  throw new Error(
    `expected reserved host-mediated label drift failure, got status ${reservedHostMediatedLabelDriftResult.status}\n${reservedHostMediatedLabelDriftResult.stderr}`
  );
}

const singlePermissionModelRoot = makeRoot("single-permission-model");
writeJson(singlePermissionModelRoot, "docs/plugins/plugin-api-v1-contract.json", {
  apiVersion: "1.0.0",
  defaultHookTimeoutMs: 150,
  defaultFailurePolicy: "fail-open",
  activeHooks: [
    "gateway.request.afterBodyRead",
    "gateway.request.beforeSend",
    "gateway.response.chunk",
  ],
  reservedHooks: ["gateway.response.headers"],
  activeMutationFields: ["requestBody", "streamChunk"],
  configSchemaTypes: ["object"],
  activePermissions: [
    "request.meta.read",
    "request.header.read",
    "request.header.readSensitive",
    "request.body.read",
    "request.body.write",
    "stream.inspect",
    "stream.modify",
  ],
  reservedPermissions: ["network.fetch"],
  capabilityDependencies: {
    commands: ["commands.execute"],
    providers: ["provider.extensionValues"],
    "ui.providers.editor.sections": ["provider.extensionValues"],
    "ui.providers.editor.fields": ["provider.extensionValues"],
    "ui.buttonCommandFields": ["commands.execute"],
    gatewayHooks: ["gateway.hooks"],
    protocolBridges: ["protocol.bridge"],
  },
  hookMatrix: JSON.parse(
    readFileSync(
      join(extensionHostDependencyBaselineRoot, "docs/plugins/plugin-api-v1-contract.json"),
      "utf8"
    )
  ).hookMatrix,
  communityRuntimes: ["extensionHost"],
  unsupportedLegacyRuntimes: ["wasm", "process", "native"],
});
writePassingScaffold(singlePermissionModelRoot);
writeFileSync(
  join(singlePermissionModelRoot, "src-tauri/src/domain/plugins.rs"),
  [
    "extensionHost wasm process native privacyFilter",
    "crate::gateway::plugins::contract::is_active_hook",
    "crate::gateway::plugins::contract::is_reserved_hook",
    "crate::gateway::plugins::contract::hook_contract",
    [
      "request.meta.read",
      "request.header.read",
      "request.header.readSensitive",
      "request.header.write",
      "request.body.read",
      "request.body.write",
      "response.header.read",
      "response.header.write",
      "response.body.read",
      "response.body.write",
      "stream.inspect",
      "stream.modify",
      "log.redact",
      "network.fetch",
    ].join(" "),
    "providers.editor.sections UI contribution requires provider.extensionValues",
    "providers.editor.fields UI contribution requires provider.extensionValues",
    "UI command field requires commands.execute",
    "pub fn is_active_gateway_hook(hook: &str) -> bool {",
    '  hook == "gateway.request.afterBodyRead" || hook == "gateway.request.beforeSend"',
    "}",
    'pub fn is_reserved_gateway_hook(hook: &str) -> bool { hook == "gateway.response.headers" }',
    "fn permission_risk(permission: &str) { request.body.read; request.body.write; network.fetch; }",
    "PLUGIN_RESERVED_HOOK",
  ].join("\n")
);

const singlePermissionModelResult = runCheck(singlePermissionModelRoot);
if (singlePermissionModelResult.status !== 0) {
  throw new Error(
    `expected Extension Host single permission model without reserved permission manifest validation to pass, got status ${singlePermissionModelResult.status}\n${singlePermissionModelResult.stderr}`
  );
}

const protocolBridgeBoundaryDriftRoot = makeRoot("protocol-bridge-boundary-drift");
writeJson(protocolBridgeBoundaryDriftRoot, "docs/plugins/plugin-api-v1-contract.json", {
  apiVersion: "1.0.0",
  defaultHookTimeoutMs: 150,
  defaultFailurePolicy: "fail-open",
  activeHooks: [
    "gateway.request.afterBodyRead",
    "gateway.request.beforeSend",
    "gateway.response.chunk",
  ],
  reservedHooks: ["gateway.response.headers"],
  activeMutationFields: ["requestBody", "streamChunk"],
  configSchemaTypes: ["object"],
  activePermissions: [
    "request.meta.read",
    "request.header.read",
    "request.header.readSensitive",
    "request.body.read",
    "request.body.write",
    "stream.inspect",
    "stream.modify",
  ],
  reservedPermissions: ["network.fetch"],
  capabilityDependencies: {
    commands: ["commands.execute"],
    providers: ["provider.extensionValues"],
    "ui.providers.editor.sections": ["provider.extensionValues"],
    "ui.providers.editor.fields": ["provider.extensionValues"],
    "ui.buttonCommandFields": ["commands.execute"],
    gatewayHooks: ["gateway.hooks"],
    protocolBridges: ["protocol.bridge"],
  },
  protocolBridgeContribution: {
    status: "active-execution",
    executionBoundary: "protocol bridge execution is fully active",
  },
  hookMatrix: JSON.parse(
    readFileSync(
      join(extensionHostDependencyBaselineRoot, "docs/plugins/plugin-api-v1-contract.json"),
      "utf8"
    )
  ).hookMatrix,
  communityRuntimes: ["extensionHost"],
  unsupportedLegacyRuntimes: ["wasm", "process", "native"],
});
writePassingScaffold(protocolBridgeBoundaryDriftRoot);

const protocolBridgeBoundaryDriftResult = runCheck(protocolBridgeBoundaryDriftRoot);
if (
  protocolBridgeBoundaryDriftResult.status === 0 ||
  !protocolBridgeBoundaryDriftResult.stderr.includes(
    "protocolBridgeContribution.status must be mvp-skeleton"
  ) ||
  !protocolBridgeBoundaryDriftResult.stderr.includes(
    "protocolBridgeContribution.executionBoundary must describe future host integration"
  )
) {
  throw new Error(
    `expected protocol bridge boundary drift failure, got status ${protocolBridgeBoundaryDriftResult.status}\n${protocolBridgeBoundaryDriftResult.stderr}`
  );
}

const capabilityDependencyDriftRoot = makeRoot("capability-dependency-drift");
writeJson(capabilityDependencyDriftRoot, "docs/plugins/plugin-api-v1-contract.json", {
  apiVersion: "1.0.0",
  defaultHookTimeoutMs: 150,
  defaultFailurePolicy: "fail-open",
  activeHooks: [
    "gateway.request.afterBodyRead",
    "gateway.request.beforeSend",
    "gateway.response.chunk",
  ],
  reservedHooks: ["gateway.response.headers"],
  activeMutationFields: ["requestBody", "streamChunk"],
  configSchemaTypes: ["object"],
  activePermissions: [
    "request.meta.read",
    "request.header.read",
    "request.header.readSensitive",
    "request.body.read",
    "request.body.write",
    "stream.inspect",
    "stream.modify",
  ],
  reservedPermissions: ["network.fetch"],
  capabilityDependencies: {
    commands: ["commands.execute"],
    providers: ["provider.extensionValues"],
    "ui.providers.editor.sections": ["provider.extensionValues"],
    "ui.providers.card.badges": ["provider.extensionValues"],
    "ui.providers.card.actions": ["provider.extensionValues"],
    gatewayHooks: ["gateway.hooks"],
    protocolBridges: ["protocol.bridge"],
  },
  hookMatrix:
    extensionHostDependencyBaselineResult.status === 0
      ? JSON.parse(
          readFileSync(
            join(extensionHostDependencyBaselineRoot, "docs/plugins/plugin-api-v1-contract.json"),
            "utf8"
          )
        ).hookMatrix
      : {},
  communityRuntimes: ["extensionHost"],
  unsupportedLegacyRuntimes: ["wasm", "process", "native"],
});
writePassingScaffold(capabilityDependencyDriftRoot);

const capabilityDependencyDriftResult = runCheck(capabilityDependencyDriftRoot);
if (
  capabilityDependencyDriftResult.status === 0 ||
  !capabilityDependencyDriftResult.stderr.includes(
    "capabilityDependencies.ui.providers.editor.fields"
  ) ||
  !capabilityDependencyDriftResult.stderr.includes(
    "capabilityDependencies.ui.buttonCommandFields"
  ) ||
  !capabilityDependencyDriftResult.stderr.includes("ui.providers.card.badges") ||
  !capabilityDependencyDriftResult.stderr.includes("ui.providers.card.actions")
) {
  throw new Error(
    `expected capability dependency drift failure, got status ${capabilityDependencyDriftResult.status}\n${capabilityDependencyDriftResult.stderr}`
  );
}

const reservedHookRoot = makeRoot("reserved-hook");
writeJson(reservedHookRoot, "docs/plugins/plugin-api-v1-contract.json", {
  apiVersion: "1.0.0",
  defaultHookTimeoutMs: 150,
  defaultFailurePolicy: "fail-open",
  activeHooks: ["gateway.request.afterBodyRead"],
  reservedHooks: ["gateway.response.headers"],
  activeMutationFields: ["requestBody"],
  configSchemaTypes: ["object"],
  activePermissions: ["request.body.read"],
  reservedPermissions: ["network.fetch"],
  communityRuntimes: ["extensionHost"],
  unsupportedLegacyRuntimes: ["wasm", "process", "native"],
});
writeFileSync(
  join(reservedHookRoot, "packages/plugin-sdk/src/index.ts"),
  "gateway.request.afterBodyRead request.body.read extensionHost"
);
writeFileSync(
  join(reservedHookRoot, "packages/create-aio-plugin/src/scaffold.ts"),
  "extensionHost gateway.request.afterBodyRead request.body.read"
);
writeFileSync(
  join(reservedHookRoot, "src-tauri/src/domain/plugins.rs"),
  "gateway.request.afterBodyRead request.body.read extensionHost"
);
writeFileSync(
  join(reservedHookRoot, "docs/plugin-manifest-v1.md"),
  "gateway.request.afterBodyRead request.body.read"
);
writeFileSync(
  join(reservedHookRoot, "docs/plugins/reference/hooks.md"),
  "gateway.request.afterBodyRead"
);
writeFileSync(join(reservedHookRoot, "docs/plugins/reference/permissions.md"), "request.body.read");
writeFileSync(
  join(reservedHookRoot, "docs/plugins/reference/manifest.md"),
  "extensionHost wasm process native privacyFilter"
);
writeFileSync(
  join(reservedHookRoot, "docs/plugins/runtime/wasm.md"),
  "wasm PLUGIN_RUNTIME_DISABLED"
);

const reservedHookResult = runCheck(reservedHookRoot);
if (
  reservedHookResult.status === 0 ||
  !reservedHookResult.stderr.includes("gateway.response.headers")
) {
  throw new Error(
    `expected structural contract failure, got status ${reservedHookResult.status}\n${reservedHookResult.stderr}`
  );
}

const missingHookMetadataRoot = makeRoot("missing-hook-metadata");
writeJson(missingHookMetadataRoot, "docs/plugins/plugin-api-v1-contract.json", {
  apiVersion: "1.0.0",
  defaultHookTimeoutMs: 150,
  defaultFailurePolicy: "fail-open",
  activeHooks: ["gateway.request.afterBodyRead"],
  reservedHooks: ["gateway.response.headers"],
  activeMutationFields: ["requestBody"],
  configSchemaTypes: ["object"],
  activePermissions: ["request.body.read"],
  reservedPermissions: ["network.fetch"],
  hookMatrix: {
    "gateway.request.afterBodyRead": {
      phase: "after request body read and before upstream provider send",
      readPermissions: ["request.body.read"],
      writePermissions: [],
      contextFields: ["traceId"],
      timeoutMs: 150,
    },
  },
  communityRuntimes: ["extensionHost"],
  unsupportedLegacyRuntimes: ["wasm", "process", "native"],
});
writePassingScaffold(missingHookMetadataRoot);

const missingHookMetadataResult = runCheck(missingHookMetadataRoot);
if (
  missingHookMetadataResult.status === 0 ||
  !missingHookMetadataResult.stderr.includes("hookMatrix.gateway.request.afterBodyRead.kind") ||
  !missingHookMetadataResult.stderr.includes("hookMatrix.gateway.request.afterBodyRead.status") ||
  !missingHookMetadataResult.stderr.includes(
    "hookMatrix.gateway.request.afterBodyRead.permissionDependencies"
  ) ||
  !missingHookMetadataResult.stderr.includes(
    "hookMatrix.gateway.request.afterBodyRead.mutationFields"
  )
) {
  throw new Error(
    `expected hookMatrix metadata failure, got status ${missingHookMetadataResult.status}\n${missingHookMetadataResult.stderr}`
  );
}

const inconsistentHookMetadataRoot = makeRoot("inconsistent-hook-metadata");
writeJson(inconsistentHookMetadataRoot, "docs/plugins/plugin-api-v1-contract.json", {
  apiVersion: "1.0.0",
  defaultHookTimeoutMs: 150,
  defaultFailurePolicy: "fail-open",
  activeHooks: ["gateway.request.afterBodyRead"],
  reservedHooks: ["gateway.response.headers"],
  activeMutationFields: ["requestBody"],
  configSchemaTypes: ["object"],
  activePermissions: ["request.body.read"],
  reservedPermissions: ["network.fetch"],
  hookMatrix: {
    "gateway.request.afterBodyRead": {
      phase: "after request body read and before upstream provider send",
      kind: "request",
      status: "reserved",
      defaultFailurePolicy: "fail-closed",
      timeoutMs: 150,
      reservedHeaderPolicy: "allow-all",
      readPermissions: ["request.body.read"],
      writePermissions: [],
      mutationFields: ["requestBody"],
      contextFields: ["traceId"],
    },
  },
  communityRuntimes: ["extensionHost"],
  unsupportedLegacyRuntimes: ["wasm", "process", "native"],
});
writePassingScaffold(inconsistentHookMetadataRoot);

const inconsistentHookMetadataResult = runCheck(inconsistentHookMetadataRoot);
if (
  inconsistentHookMetadataResult.status === 0 ||
  !inconsistentHookMetadataResult.stderr.includes(
    "hookMatrix.gateway.request.afterBodyRead.status must be active"
  ) ||
  !inconsistentHookMetadataResult.stderr.includes(
    "hookMatrix.gateway.request.afterBodyRead.defaultFailurePolicy must equal defaultFailurePolicy"
  ) ||
  !inconsistentHookMetadataResult.stderr.includes(
    "hookMatrix.gateway.request.afterBodyRead.reservedHeaderPolicy must be one of block-gateway-owned"
  )
) {
  throw new Error(
    `expected hookMatrix consistency failure, got status ${inconsistentHookMetadataResult.status}\n${inconsistentHookMetadataResult.stderr}`
  );
}

const duplicateHookMetadataRoot = makeRoot("duplicate-hook-metadata");
writeJson(duplicateHookMetadataRoot, "docs/plugins/plugin-api-v1-contract.json", {
  apiVersion: "1.0.0",
  defaultHookTimeoutMs: 150,
  defaultFailurePolicy: "fail-open",
  activeHooks: ["gateway.request.afterBodyRead"],
  reservedHooks: ["gateway.response.headers"],
  activeMutationFields: ["requestBody"],
  configSchemaTypes: ["object"],
  activePermissions: ["request.body.read"],
  reservedPermissions: ["network.fetch"],
  hookMatrix: {
    "gateway.request.afterBodyRead": {
      phase: "after request body read and before upstream provider send",
      kind: "request",
      status: "active",
      defaultFailurePolicy: "fail-open",
      timeoutMs: 150,
      reservedHeaderPolicy: "block-gateway-owned",
      readPermissions: ["request.body.read", "request.body.read"],
      writePermissions: [],
      permissionDependencies: {},
      mutationFields: ["requestBody"],
      contextFields: ["traceId"],
    },
  },
  communityRuntimes: ["extensionHost"],
  unsupportedLegacyRuntimes: ["wasm", "process", "native"],
});
writePassingScaffold(duplicateHookMetadataRoot);

const duplicateHookMetadataResult = runCheck(duplicateHookMetadataRoot);
if (
  duplicateHookMetadataResult.status === 0 ||
  !duplicateHookMetadataResult.stderr.includes(
    "hookMatrix.gateway.request.afterBodyRead.readPermissions contains duplicate request.body.read"
  )
) {
  throw new Error(
    `expected hookMatrix duplicate metadata failure, got status ${duplicateHookMetadataResult.status}\n${duplicateHookMetadataResult.stderr}`
  );
}

const missingDevtoolsMetadataRoot = makeRoot("missing-devtools-metadata");
writeJson(missingDevtoolsMetadataRoot, "docs/plugins/plugin-api-v1-contract.json", {
  apiVersion: "1.0.0",
  defaultHookTimeoutMs: 150,
  defaultFailurePolicy: "fail-open",
  activeHooks: ["gateway.request.afterBodyRead"],
  reservedHooks: ["gateway.response.headers"],
  activeMutationFields: ["requestBody"],
  configSchemaTypes: ["object"],
  activePermissions: ["request.body.read", "request.body.write"],
  reservedPermissions: ["network.fetch"],
  hookMatrix: {
    "gateway.request.afterBodyRead": {
      phase: "after request body read and before upstream provider send",
      kind: "request",
      status: "active",
      defaultFailurePolicy: "fail-open",
      timeoutMs: 150,
      reservedHeaderPolicy: "block-gateway-owned",
      readPermissions: ["request.body.read"],
      writePermissions: ["request.body.write"],
      permissionDependencies: { "request.body.write": ["request.body.read"] },
      mutationFields: ["requestBody"],
      contextFields: ["traceId"],
    },
  },
  communityRuntimes: ["extensionHost"],
  unsupportedLegacyRuntimes: ["wasm", "process", "native"],
});
writePassingScaffold(missingDevtoolsMetadataRoot);
writeFileSync(
  join(missingDevtoolsMetadataRoot, "packages/create-aio-plugin/src/devtools.ts"),
  "validatePluginFilesStrict doctorPluginFiles"
);

const missingDevtoolsMetadataResult = runCheck(missingDevtoolsMetadataRoot);
if (
  missingDevtoolsMetadataResult.status === 0 ||
  !missingDevtoolsMetadataResult.stderr.includes("packages/create-aio-plugin/src/devtools.ts") ||
  !missingDevtoolsMetadataResult.stderr.includes("PLUGIN_INVALID_MAIN")
) {
  throw new Error(
    `expected devtools metadata failure, got status ${missingDevtoolsMetadataResult.status}\n${missingDevtoolsMetadataResult.stderr}`
  );
}

const partialDevtoolsMetadataRoot = makeRoot("partial-devtools-metadata");
writeJson(partialDevtoolsMetadataRoot, "docs/plugins/plugin-api-v1-contract.json", {
  apiVersion: "1.0.0",
  defaultHookTimeoutMs: 150,
  defaultFailurePolicy: "fail-open",
  activeHooks: ["gateway.request.afterBodyRead"],
  reservedHooks: ["gateway.response.headers"],
  activeMutationFields: ["requestBody", "headers"],
  configSchemaTypes: ["object"],
  activePermissions: ["request.meta.read", "request.body.read"],
  reservedPermissions: ["network.fetch"],
  hookMatrix: {
    "gateway.request.afterBodyRead": {
      phase: "after request body read and before upstream provider send",
      kind: "request",
      status: "active",
      defaultFailurePolicy: "fail-open",
      timeoutMs: 150,
      reservedHeaderPolicy: "block-gateway-owned",
      readPermissions: ["request.meta.read", "request.body.read"],
      writePermissions: [],
      permissionDependencies: {},
      mutationFields: ["requestBody", "headers"],
      contextFields: ["traceId", "request.body"],
    },
  },
  communityRuntimes: ["extensionHost"],
  unsupportedLegacyRuntimes: ["wasm", "process", "native"],
});
writePassingScaffold(partialDevtoolsMetadataRoot);
writeFileSync(
  join(partialDevtoolsMetadataRoot, "src-tauri/src/gateway/plugins/contract.rs"),
  [
    "gateway.request.afterBodyRead gateway.response.headers",
    "request.meta.read request.body.read network.fetch",
  ].join("\n")
);
writeFileSync(
  join(partialDevtoolsMetadataRoot, "src-tauri/src/domain/plugins.rs"),
  [
    "extensionHost wasm process native privacyFilter",
    "crate::gateway::plugins::contract::is_active_hook",
    "crate::gateway::plugins::contract::is_reserved_hook",
    "crate::gateway::plugins::contract::hook_contract",
    "pub fn is_active_gateway_hook(hook: &str) -> bool {",
    '  hook == "gateway.request.afterBodyRead"',
    "}",
    'pub fn is_reserved_gateway_hook(hook: &str) -> bool { hook == "gateway.response.headers" }',
    "fn permission_risk(permission: &str) { request.meta.read; request.body.read; network.fetch; }",
    "PLUGIN_RESERVED_HOOK",
  ].join("\n")
);
writeFileSync(
  join(partialDevtoolsMetadataRoot, "docs/plugin-manifest-v1.md"),
  "gateway.request.afterBodyRead gateway.response.headers request.meta.read request.body.read network.fetch"
);
writeFileSync(
  join(partialDevtoolsMetadataRoot, "docs/plugins/reference/permissions.md"),
  "request.meta.read request.body.read network.fetch"
);
writeFileSync(
  join(partialDevtoolsMetadataRoot, "packages/create-aio-plugin/src/devtools.ts"),
  [
    "doctorPluginFiles validatePluginFilesStrict",
    "dist/extension.js gateway.hooks contributes.gatewayHooks",
    "PLUGIN_UNSUPPORTED_LEGACY_RUNTIME PLUGIN_REPLAY_UNSUPPORTED",
  ].join("\n")
);

const partialDevtoolsMetadataResult = runCheck(partialDevtoolsMetadataRoot);
if (
  partialDevtoolsMetadataResult.status === 0 ||
  !partialDevtoolsMetadataResult.stderr.includes(
    "packages/create-aio-plugin/src/devtools.ts is missing Extension Host developer tool package shape packPluginBytes"
  ) ||
  !partialDevtoolsMetadataResult.stderr.includes(
    "packages/create-aio-plugin/src/devtools.ts is missing Extension Host developer tool package shape PLUGIN_INVALID_MAIN"
  )
) {
  throw new Error(
    `expected full devtools metadata failure, got status ${partialDevtoolsMetadataResult.status}\n${partialDevtoolsMetadataResult.stderr}`
  );
}

const globalPermissionDependencyRoot = makeRoot("global-permission-dependency");
writeJson(globalPermissionDependencyRoot, "docs/plugins/plugin-api-v1-contract.json", {
  apiVersion: "1.0.0",
  defaultHookTimeoutMs: 150,
  defaultFailurePolicy: "fail-open",
  activeHooks: ["gateway.request.afterBodyRead", "gateway.request.beforeSend"],
  reservedHooks: ["gateway.response.headers"],
  activeMutationFields: ["requestBody"],
  configSchemaTypes: ["object"],
  activePermissions: ["request.body.read", "request.body.write"],
  reservedPermissions: ["network.fetch"],
  hookMatrix: {
    "gateway.request.afterBodyRead": {
      phase: "after request body read and before upstream provider send",
      kind: "request",
      status: "active",
      defaultFailurePolicy: "fail-open",
      timeoutMs: 150,
      reservedHeaderPolicy: "block-gateway-owned",
      readPermissions: ["request.body.read"],
      writePermissions: ["request.body.write"],
      permissionDependencies: {
        "request.body.write": ["request.body.read"],
      },
      mutationFields: ["requestBody"],
      contextFields: ["traceId", "request.body"],
    },
    "gateway.request.beforeSend": {
      phase: "after provider resolution and before upstream provider send",
      kind: "request",
      status: "active",
      defaultFailurePolicy: "fail-open",
      timeoutMs: 150,
      reservedHeaderPolicy: "block-gateway-owned",
      readPermissions: ["request.body.read"],
      writePermissions: ["request.body.write"],
      permissionDependencies: {},
      mutationFields: ["requestBody"],
      contextFields: ["traceId", "request.body"],
    },
  },
  communityRuntimes: ["extensionHost"],
  unsupportedLegacyRuntimes: ["wasm", "process", "native"],
});
writePassingScaffold(globalPermissionDependencyRoot);
writeFileSync(
  join(globalPermissionDependencyRoot, "packages/plugin-sdk/src/index.ts"),
  [
    "export type PluginPermission = 'request.body.read' | 'request.body.write' | 'network.fetch';",
    "export type ActiveGatewayHookName = 'gateway.request.afterBodyRead' | 'gateway.request.beforeSend';",
    "export type ReservedGatewayHookName = 'gateway.response.headers';",
    "export type GatewayHookName = ActiveGatewayHookName | ReservedGatewayHookName;",
    "const runtimeTokens = 'extensionHost wasm';",
    "const activeMutationField = 'requestBody';",
    "function validateManifest(manifest: { permissions: PluginPermission[] }) {",
    "  return validatePermissionSet(manifest.permissions);",
    "}",
    "function validatePermissionSet(permissions: PluginPermission[]) {",
    "  const set = new Set(permissions);",
    "  if (set.has('request.body.write') && !set.has('request.body.read')) {",
    "    return 'request.body.write requires request.body.read';",
    "  }",
    "  return null;",
    "}",
  ].join("\n")
);

const globalPermissionDependencyResult = runCheck(globalPermissionDependencyRoot);
if (
  globalPermissionDependencyResult.status === 0 ||
  !globalPermissionDependencyResult.stderr.includes(
    "packages/plugin-sdk/src/index.ts is missing Extension Host SDK contract export type ExtensionRuntime"
  ) ||
  !globalPermissionDependencyResult.stderr.includes(
    "packages/plugin-sdk/src/index.ts is missing validateCapabilityDependencies body"
  )
) {
  throw new Error(
    `expected capability dependency failure, got status ${globalPermissionDependencyResult.status}\n${globalPermissionDependencyResult.stderr}`
  );
}
