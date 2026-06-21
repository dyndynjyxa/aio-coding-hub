export type PluginPermissionRisk = "low" | "medium" | "high" | "critical";

export type ActiveGatewayHookName =
  | "gateway.request.afterBodyRead"
  | "gateway.request.beforeSend"
  | "gateway.response.chunk"
  | "gateway.response.after"
  | "gateway.error"
  | "log.beforePersist";

export type ReservedGatewayHookName =
  | "gateway.request.received"
  | "gateway.request.beforeProviderResolution"
  | "gateway.response.headers";

export type GatewayHookName = ActiveGatewayHookName | ReservedGatewayHookName;

export type PluginPermission =
  | "request.meta.read"
  | "request.header.read"
  | "request.header.readSensitive"
  | "request.header.write"
  | "request.body.read"
  | "request.body.write"
  | "response.header.read"
  | "response.header.write"
  | "response.body.read"
  | "response.body.write"
  | "stream.inspect"
  | "stream.modify"
  | "log.redact"
  | "plugin.storage"
  | "network.fetch"
  | "file.read"
  | "file.write"
  | "secret.read";

export type PluginRuntime =
  | { kind: "declarativeRules"; rules: string[] }
  | { kind: "wasm"; abiVersion: string; memoryLimitBytes?: number };

export type PluginHook = {
  name: GatewayHookName;
  priority?: number;
  failurePolicy?: "fail-open" | "fail-closed";
};

export type PluginHostCompatibility = {
  app: string;
  pluginApi: string;
  platforms?: string[];
};

export type JsonValue =
  | null
  | boolean
  | number
  | string
  | JsonValue[]
  | { [key: string]: JsonValue | undefined };

export type PluginManifest = {
  id: string;
  name: string;
  version: string;
  apiVersion: string;
  runtime: PluginRuntime;
  hooks: PluginHook[];
  permissions: PluginPermission[];
  hostCompatibility: PluginHostCompatibility;
  entry?: string;
  configSchema?: JsonValue;
  configVersion?: number;
  description?: string;
  author?: JsonValue;
  homepage?: string;
  repository?: JsonValue;
  license?: string;
  checksum?: string;
  signature?: string;
  category?: string;
};

export type GatewayNormalizedMessage = {
  role: string;
  text: string;
  source: string;
};

export type GatewayVisibleRequestContext = {
  cliKey?: string;
  method?: string;
  path?: string;
  query?: string;
  headers?: Record<string, JsonValue>;
  body?: string;
  normalizedMessages?: GatewayNormalizedMessage[];
  requestedModel?: string;
};

export type GatewayVisibleResponseContext = {
  status?: number;
  headers?: Record<string, JsonValue>;
  body?: string;
};

export type GatewayVisibleStreamContext = {
  sequence?: number;
  chunk?: string;
};

export type GatewayVisibleLogContext = {
  message?: string;
};

export type GatewayVisibleHookContext = {
  request?: GatewayVisibleRequestContext;
  response?: GatewayVisibleResponseContext;
  stream?: GatewayVisibleStreamContext;
  log?: GatewayVisibleLogContext;
};

export type PluginHookContext = {
  hook: GatewayHookName;
  traceId?: string;
  config: JsonValue;
  context: GatewayVisibleHookContext;
};

export type PluginHookResult =
  | { action: "pass"; audit?: JsonValue[] }
  | { action: "warn"; message: string; audit?: JsonValue[] }
  | { action: "block"; reason: string; audit?: JsonValue[] }
  | {
      action: "replace";
      requestBody?: string;
      responseBody?: string;
      streamChunk?: string;
      logMessage?: string;
      headers?: Record<string, string>;
      audit?: JsonValue[];
    };

export type ValidationResult =
  | { ok: true }
  | { ok: false; error: { code: string; message: string } };

const PERMISSION_RISKS: Record<PluginPermission, PluginPermissionRisk> = {
  "request.meta.read": "low",
  "request.header.read": "medium",
  "request.header.readSensitive": "high",
  "request.header.write": "high",
  "request.body.read": "high",
  "request.body.write": "high",
  "response.header.read": "low",
  "response.header.write": "medium",
  "response.body.read": "high",
  "response.body.write": "high",
  "stream.inspect": "high",
  "stream.modify": "high",
  "log.redact": "medium",
  "plugin.storage": "medium",
  "network.fetch": "high",
  "file.read": "high",
  "file.write": "high",
  "secret.read": "critical",
};

const KNOWN_HOOKS = new Set<GatewayHookName>([
  "gateway.request.received",
  "gateway.request.afterBodyRead",
  "gateway.request.beforeProviderResolution",
  "gateway.request.beforeSend",
  "gateway.response.headers",
  "gateway.response.chunk",
  "gateway.response.after",
  "gateway.error",
  "log.beforePersist",
]);

const RESERVED_HOOKS = new Set<GatewayHookName>([
  "gateway.request.received",
  "gateway.request.beforeProviderResolution",
  "gateway.response.headers",
]);

const KNOWN_PERMISSIONS = new Set<PluginPermission>(
  Object.keys(PERMISSION_RISKS) as PluginPermission[]
);

const RESERVED_PERMISSIONS = new Set<PluginPermission>([
  "plugin.storage",
  "network.fetch",
  "file.read",
  "file.write",
  "secret.read",
]);

export function permissionRisk(permission: PluginPermission): PluginPermissionRisk {
  return PERMISSION_RISKS[permission];
}

export function validateManifest(manifest: PluginManifest): ValidationResult {
  if (!/^[a-z0-9][a-z0-9-]*(\.[a-z0-9][a-z0-9-]*)+$/.test(manifest.id)) {
    return invalid("PLUGIN_INVALID_ID", "plugin id must look like publisher.plugin-name");
  }
  if (!isSemver(manifest.version) || !isSemver(manifest.apiVersion)) {
    return invalid("PLUGIN_INVALID_VERSION", "version and apiVersion must be SemVer");
  }
  if (manifest.runtime.kind === "declarativeRules" && manifest.runtime.rules.length === 0) {
    return invalid("PLUGIN_INVALID_RUNTIME", "declarativeRules runtime requires rules");
  }
  if (manifest.runtime.kind === "wasm" && !isSemver(manifest.runtime.abiVersion)) {
    return invalid("PLUGIN_INVALID_RUNTIME", "wasm runtime requires SemVer abiVersion");
  }
  if (manifest.runtime.kind === "wasm" && semverMajor(manifest.runtime.abiVersion) !== 1) {
    return invalid("PLUGIN_UNSUPPORTED_WASM_ABI", "wasm abiVersion must be compatible with v1");
  }
  if (!isSimpleCompatibilityRange(manifest.hostCompatibility.app)) {
    return invalid(
      "PLUGIN_INVALID_HOST_COMPATIBILITY",
      "hostCompatibility.app must be a non-empty simple SemVer range"
    );
  }
  if (!supportsPluginApiV1(manifest.hostCompatibility.pluginApi)) {
    return invalid(
      "PLUGIN_UNSUPPORTED_PLUGIN_API",
      "hostCompatibility.pluginApi must support plugin API v1"
    );
  }
  if (manifest.hooks.length === 0) {
    return invalid("PLUGIN_MISSING_HOOKS", "plugin must declare at least one hook");
  }
  for (const hook of manifest.hooks) {
    if (RESERVED_HOOKS.has(hook.name)) {
      return invalid(
        "PLUGIN_RESERVED_HOOK",
        `hook is reserved for a future host integration and is not active in plugin API v1: ${hook.name}`
      );
    }
    if (!KNOWN_HOOKS.has(hook.name)) {
      return invalid("PLUGIN_UNKNOWN_HOOK", `unknown hook: ${hook.name}`);
    }
  }
  for (const permission of manifest.permissions) {
    if (RESERVED_PERMISSIONS.has(permission)) {
      return invalid(
        "PLUGIN_RESERVED_PERMISSION",
        `permission is reserved for a future host-mediated API and is not active in plugin API v1: ${permission}`
      );
    }
    if (!KNOWN_PERMISSIONS.has(permission)) {
      return invalid("PLUGIN_UNKNOWN_PERMISSION", `unknown permission: ${permission}`);
    }
  }
  const permissionSetError = validatePermissionSet(manifest.permissions);
  if (permissionSetError) return permissionSetError;
  const permissionScopeError = validatePermissionScope(manifest);
  if (permissionScopeError) return permissionScopeError;
  return { ok: true };
}

function invalid(code: string, message: string): ValidationResult {
  return { ok: false, error: { code, message } };
}

function isSemver(value: string): boolean {
  return /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$/.test(value);
}

function semverMajor(value: string): number | null {
  const major = /^(\d+)\./.exec(value)?.[1];
  return major == null ? null : Number.parseInt(major, 10);
}

function isSimpleCompatibilityRange(value: string): boolean {
  const trimmed = value.trim();
  if (!trimmed) return false;
  return /^(?:[<>=^~]*\s*\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?|\d+\.x\.x)(?:\s+(?:[<>=^~]*\s*\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?|\d+\.x\.x))*$/.test(
    trimmed
  );
}

function supportsPluginApiV1(value: string): boolean {
  const trimmed = value.trim();
  return trimmed === "^1.0.0" || trimmed === "1.x.x" || trimmed === ">=1.0.0 <2.0.0";
}

function validatePermissionSet(permissions: PluginPermission[]): ValidationResult | null {
  const set = new Set(permissions);
  if (set.has("request.body.write") && !set.has("request.body.read")) {
    return invalid(
      "PLUGIN_INVALID_PERMISSION_SET",
      "request.body.write requires request.body.read"
    );
  }
  if (set.has("response.body.write") && !set.has("response.body.read")) {
    return invalid(
      "PLUGIN_INVALID_PERMISSION_SET",
      "response.body.write requires response.body.read"
    );
  }
  if (set.has("stream.modify") && !set.has("stream.inspect")) {
    return invalid("PLUGIN_INVALID_PERMISSION_SET", "stream.modify requires stream.inspect");
  }
  return null;
}

function hookAllowsPermission(hookName: GatewayHookName, permission: PluginPermission): boolean {
  if (
    permission === "request.meta.read" ||
    permission === "request.header.read" ||
    permission === "request.header.readSensitive" ||
    permission === "request.header.write" ||
    permission === "request.body.read" ||
    permission === "request.body.write"
  ) {
    return hookName === "gateway.request.afterBodyRead" || hookName === "gateway.request.beforeSend";
  }
  if (
    permission === "response.header.read" ||
    permission === "response.header.write" ||
    permission === "response.body.read" ||
    permission === "response.body.write"
  ) {
    return hookName === "gateway.response.after" || hookName === "gateway.error";
  }
  if (permission === "stream.inspect" || permission === "stream.modify") {
    return hookName === "gateway.response.chunk";
  }
  if (permission === "log.redact") return hookName === "log.beforePersist";
  return false;
}

function validatePermissionScope(manifest: PluginManifest): ValidationResult | null {
  for (const permission of manifest.permissions) {
    if (RESERVED_PERMISSIONS.has(permission)) continue;
    if (!manifest.hooks.some((hook) => hookAllowsPermission(hook.name, permission))) {
      return invalid(
        "PLUGIN_PERMISSION_SCOPE_MISMATCH",
        `permission ${permission} does not apply to any declared hook`
      );
    }
  }
  return null;
}
