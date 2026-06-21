import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = dirname(scriptDir);

const requiredDocs = [
  {
    path: "docs/plugin-system-rfc.md",
    phrases: [
      "短期不执行任意 JavaScript/TypeScript",
      "提示词优化只能在网关请求阶段可靠实现",
      "第三方代码不得直接进入主进程或 WebView",
      "Skill 市场",
      "gateway.request.afterBodyRead",
      "Final Gateway Hook Chain",
      "gateway.request.beforeSend (active upstream header/body mutation)",
      "gateway.response.chunk (active SSE chunk inspect/modify/block)",
      "log.beforePersist (active request log redaction before enqueue)",
      "WASM",
    ],
  },
  {
    path: "docs/plugin-manifest-v1.md",
    phrases: [
      "publisher.plugin-name",
      "SemVer",
      "apiVersion",
      "hostCompatibility",
      "gateway.response.chunk",
      "Active hooks in plugin API v1",
      "Reserved hooks for future host integration",
      "Reserved permissions for future host-mediated APIs",
      "request.header.readSensitive",
      "official.privacy-filter",
      "acme.prompt-helper",
      "quarantined",
      "高危权限需要二次授权",
      "插件升级新增权限必须重新授权",
    ],
  },
  {
    path: "docs/plugins/README.md",
    phrases: [
      "插件开发手册",
      "插件开发总指南",
      "按目标查找",
      "插件 API 参考",
      "Privacy Filter 示例",
      "Manifest",
      "Hooks",
      "Permissions",
    ],
  },
  {
    path: "docs/plugins/developer-guide.md",
    phrases: [
      "插件开发总指南",
      "plugin.json",
      "declarativeRules",
      "gateway.request.beforeSend",
      "request.normalizedMessages",
      "configSchema",
      "x-aio-ui",
      "pnpm create-aio-plugin validate",
      "pnpm create-aio-plugin replay",
      "pnpm create-aio-plugin pack",
      ".aio-plugin",
      "official.privacy-filter",
    ],
  },
  {
    path: "docs/plugins/runtime/wasm.md",
    phrases: [
      "WASM ABI v1",
      "WASM packages are installable only when host policy enables execution",
      "PLUGIN_RUNTIME_DISABLED",
      "WASM enablement is rejected while host policy disables execution",
      "guest entrypoint",
      "memory/time/filesystem/network 限制",
      "no WASI filesystem imports",
      "fuel-based termination",
      "host only passes permission-trimmed JSON",
    ],
  },
  {
    path: "docs/plugins/runtime/process-poc.md",
    phrases: [
      "JSON-RPC over stdio",
      "disabled by default",
      "start timeout",
      "hook timeout",
      "crash isolation",
      "idle recycle",
      "no marketplace enablement by default",
    ],
  },
  {
    path: "docs/plugins/developer-guide.md",
    phrases: [
      "create-aio-plugin",
      "pnpm create-aio-plugin",
      "pnpm create-aio-plugin validate",
      "pnpm create-aio-plugin replay",
      "pnpm create-aio-plugin pack",
      "从 Plugins 页面本地安装",
      "Claude 和 Codex request shapes",
      "@aio-coding-hub/plugin-sdk",
      "plugin.json",
      "最小声明式规则插件",
      "声明式规则",
    ],
  },
  {
    path: "docs/plugins/reference/sdk.md",
    phrases: [
      "@aio-coding-hub/plugin-sdk",
      "PluginManifest",
      "validateManifest",
      "permissionRisk",
      "SDK 边界",
    ],
  },
  {
    path: "docs/plugins/reference/declarative-rules.md",
    phrases: [
      "declarativeRules",
      "规则文件结构",
      "request.body",
      "log.message",
      "appendMessage",
      "运行时限制",
    ],
  },
  {
    path: "docs/plugins/examples/privacy-filter.md",
    phrases: [
      "official.privacy-filter",
      "packyme/privacy-filter",
      "已移除的内置示例",
    ],
  },
  {
    path: "docs/plugins/architecture/audit.md",
    phrases: [
      "official.privacy-filter",
      "declarativeRules",
      "WASM",
      "native",
      "信任边界",
      "性能与稳定性建议",
    ],
  },
  {
    path: "docs/plugins/reference/manifest.md",
    phrases: ["apiVersion", "hostCompatibility", "declarativeRules", "wasm"],
  },
  {
    path: "docs/plugins/reference/hooks.md",
    phrases: [
      "gateway.request.afterBodyRead",
      "gateway.response.chunk",
      "log.beforePersist",
      "默认 vNext hook timeout: 150 ms",
    ],
  },
  {
    path: "docs/plugins/reference/permissions.md",
    phrases: ["request.body.read", "secret.read", "critical", "重新授权"],
  },
  {
    path: "docs/plugins/reference/config-schema.md",
    phrases: [
      "string",
      "number",
      "boolean",
      "password",
      "enum is supported as a keyword",
      "vNext does not provide host-managed secret storage",
    ],
  },
  {
    path: "docs/plugins/architecture/security.md",
    phrases: [
      "fail-closed",
      "quarantined",
      "no arbitrary JavaScript",
      "默认 vNext hook timeout: 150 ms",
    ],
  },
  {
    path: "docs/plugins/runtime/streaming.md",
    phrases: ["sliding window", "gateway.response.chunk", "stream.modify"],
  },
  {
    path: "docs/plugins/reference/publishing.md",
    phrases: [".aio-plugin", "sha256", "Ed25519", "rollback"],
  },
  {
    path: "docs/plugins/reference/compatibility.md",
    phrases: ["SemVer", "pluginApi", "platforms", "WASM ABI"],
  },
];

const failures = [];

for (const doc of requiredDocs) {
  const fullPath = join(repoRoot, doc.path);
  if (!existsSync(fullPath)) {
    failures.push(`${doc.path}: missing required document`);
    continue;
  }

  const text = readFileSync(fullPath, "utf8");
  for (const phrase of doc.phrases) {
    if (!text.includes(phrase)) {
      failures.push(`${doc.path}: missing required phrase "${phrase}"`);
    }
  }
}

if (failures.length > 0) {
  console.error("Plugin system documentation contract failed:");
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}
