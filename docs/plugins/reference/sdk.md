# 插件 SDK

`@aio-coding-hub/plugin-sdk` 提供插件 manifest、hooks、permissions、runtimes 和 validation helpers 的共享 TypeScript 契约。

`aio-plugin-wasm-sdk` 提供与之匹配的 Rust/WASM ABI contracts，用于编译为 WebAssembly 的代码插件。

SDK 面向这些场景：

- 插件作者编写 `plugin.json`。
- scaffold 和 packaging tools。
- marketplace/index validation。
- examples 和 compatibility tests。

## 包位置

本仓库中的 SDK 位于：

```text
packages/plugin-sdk
packages/plugin-wasm-sdk
```

运行 SDK 检查：

```bash
pnpm --filter @aio-coding-hub/plugin-sdk typecheck
pnpm plugin-wasm-sdk:test
```

## 主要类型

TypeScript SDK 导出：

- `PluginManifest`
- `PluginRuntime`
- `PluginHook`
- `GatewayHookName`
- `PluginPermission`
- `PluginPermissionRisk`
- `PluginHookContext`
- `PluginHookResult`

同时导出辅助函数：

- `permissionRisk(permission)`
- `validateManifest(manifest)`

`create-aio-plugin` 使用 SDK 做 manifest validation，并针对真实插件目录提供本地开发命令：

```bash
pnpm create-aio-plugin validate ./acme.redactor
pnpm create-aio-plugin replay ./acme.redactor ./fixtures/request.json gateway.request.afterBodyRead
pnpm create-aio-plugin pack ./acme.redactor
```

Rust/WASM SDK 导出：

- `PluginManifest`
- `PluginRuntime`
- `PluginHook`
- `PluginHostCompatibility`
- `HookRequest`
- `HookResult`
- `HookAction`
- `aio_plugin_entrypoint!`
- pointer/length helpers for the ABI return value

## TypeScript 中的最小 Manifest

```ts
import type { PluginManifest } from "@aio-coding-hub/plugin-sdk";
import { validateManifest } from "@aio-coding-hub/plugin-sdk";

const manifest: PluginManifest = {
  id: "acme.redactor",
  name: "Acme Redactor",
  version: "0.1.0",
  apiVersion: "1.0.0",
  runtime: { kind: "declarativeRules", rules: ["rules/main.json"] },
  hooks: [{ name: "gateway.request.afterBodyRead", priority: 50 }],
  permissions: ["request.body.read", "request.body.write"],
  hostCompatibility: {
    app: ">=0.56.0 <1.0.0",
    pluginApi: "^1.0.0",
    platforms: ["macos", "windows", "linux"]
  },
  configSchema: {
    type: "object",
    required: ["enabled"],
    properties: {
      enabled: {
        type: "boolean",
        title: "启用处理",
        description: "关闭后插件不会修改请求内容。",
        default: true,
        "x-aio-ui": { widget: "switch", order: 10 }
      }
    }
  }
};

const result = validateManifest(manifest);
if (!result.ok) {
  throw new Error(`${result.error.code}: ${result.error.message}`);
}
```

## SDK 边界

SDK 是契约包。它不执行插件代码，也不会授予宿主能力。

Rust/WASM SDK 遵循同样边界。它只负责序列化 ABI-compatible JSON、定义 hook result helpers，并提供 `aio_plugin_entrypoint!` macro 用于导出 `aio_plugin_handle`。

`PluginHookResult` 使用与 gateway host 相同的 active mutation envelope：

```ts
const result = {
  action: "replace",
  requestBody: "{\"messages\":[]}",
  headers: { "x-plugin-redacted": "1" }
} satisfies PluginHookResult;
```

替换内容时使用 `requestBody`、`responseBody`、`streamChunk`、`logMessage` 和 `headers`。`contextPatch` 不是 active vNext gateway mutation field。

真正的宿主强制检查仍发生在 Rust application 中：

- manifest compatibility checks。
- permission grants。
- hook context trimming。
- mutation permission enforcement。
- runtime timeout 和 failure policy handling。
- package checksum/signature verification。

## Rust/WASM 示例

仓库内包含一个最小 WASM redactor example：

```text
packages/plugin-wasm-sdk/examples/redactor
```

可以这样测试：

```bash
pnpm plugin-wasm-sdk:test
```

## 版本建议

- `apiVersion` 应与插件 API 主版本保持一致。
- 插件包版本使用 SemVer。
- 如果 SDK 只添加向后兼容的类型，插件 API 主版本可以不变。
- Breaking hook、permission、runtime 或 manifest changes 需要新的插件 API 主版本。
