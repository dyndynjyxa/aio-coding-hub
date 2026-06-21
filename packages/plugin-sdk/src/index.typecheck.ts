import {
  type GatewayHookName,
  type PluginHookResult,
  type PluginManifest,
  type PluginPermission,
  type PluginRuntime,
  permissionRisk,
  validateManifest,
} from "./index";

const manifest: PluginManifest = {
  id: "acme.redactor",
  name: "Redactor",
  version: "1.0.0",
  apiVersion: "1.0.0",
  runtime: { kind: "declarativeRules", rules: ["rules/main.json"] },
  hooks: [{ name: "gateway.request.afterBodyRead", priority: 10 }],
  permissions: ["request.body.read", "log.redact"],
  hostCompatibility: { app: ">=0.56.0 <1.0.0", pluginApi: "^1.0.0" },
};

const runtime: PluginRuntime = manifest.runtime;
const hook: GatewayHookName = manifest.hooks[0].name;
const permission: PluginPermission = "request.body.read";

if (runtime.kind !== "declarativeRules") {
  throw new Error("unexpected runtime");
}

if (hook !== "gateway.request.afterBodyRead") {
  throw new Error("unexpected hook");
}

if (permissionRisk(permission) !== "high") {
  throw new Error("unexpected risk");
}

const result = validateManifest(manifest);
if (!result.ok) {
  throw new Error(result.error.message);
}

const reservedHookManifest: PluginManifest = {
  ...manifest,
  hooks: [{ name: "gateway.request.received" }],
  permissions: ["request.meta.read"],
};
const reservedHookResult = validateManifest(reservedHookManifest);
if (reservedHookResult.ok || reservedHookResult.error.code !== "PLUGIN_RESERVED_HOOK") {
  throw new Error("reserved hook should be rejected by SDK validation");
}

const reservedPermissionManifest: PluginManifest = {
  ...manifest,
  permissions: ["request.body.read", "network.fetch"],
};
const reservedPermissionResult = validateManifest(reservedPermissionManifest);
if (
  reservedPermissionResult.ok ||
  reservedPermissionResult.error.code !== "PLUGIN_RESERVED_PERMISSION"
) {
  throw new Error("reserved permission should be rejected by SDK validation");
}

const replaceRequestResult: PluginHookResult = {
  action: "replace",
  requestBody: "{\"messages\":[]}",
};

const replaceResponseHeadersResult: PluginHookResult = {
  action: "replace",
  headers: { "x-plugin-redacted": "1" },
  responseBody: "{\"ok\":true}",
};

if (replaceRequestResult.action !== "replace" || !replaceResponseHeadersResult.headers) {
  throw new Error("host mutation hook results should be representable");
}
