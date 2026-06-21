# WASM 插件运行时设计

## 目标

WASM runtime 是 AIO Coding Hub 面向社区 code-plugin 的 policy-gated runtime。它用于把插件逻辑放在 Rust main-process trust boundary 之外执行，同时保留 deterministic resource limits、permission trimming、auditability 和 cross-platform behavior。`declarativeRules` 是默认社区运行时；WASM 只用于宿主策略启用后，确实需要 code execution 的插件。

WASM packages are installable only when host policy enables execution。在 compatibility tests、signing policy 和 host allowlist 都到位前，该运行时不会允许任意 marketplace execution。`plugin.wasm` artifacts 会由 `create-aio-plugin pack` 作为 binary files 打包。

## vNext 宿主策略

在 vNext 中，WASM manifests 是 compatibility contract 的一部分，但 gateway execution 受策略控制。除非 host policy 显式设置 `wasm_enabled = true`，否则 `runtime.kind = "wasm"` 的插件不能启用；否则 gateway 返回 `PLUGIN_RUNTIME_DISABLED`。

WASM enablement is rejected while host policy disables execution。插件仍可作为 ABI artifact 被打包和校验，但在 host policy 显式允许 WASM execution 前，用户不能在 gateway 中启用它。

## WASM ABI v1

WASM ABI v1 contract 刻意保持很窄：

- guest module 导出一个名为 `aio_plugin_handle` 的 guest entrypoint。
- host 向 guest memory 写入一个 UTF-8 JSON request。
- guest 返回一个编码为 `u64` 的 UTF-8 JSON response pointer/length pair。
- response 必须是与现有 gateway plugin pipeline 兼容的 hook result。
- host only passes permission-trimmed JSON，绝不传递 internal Rust references、database handles、provider secrets 或 WebView state。

Rust 插件作者应使用 `packages/plugin-wasm-sdk` 中的 `aio-plugin-wasm-sdk` 获取这些 ABI shapes 和 `aio_plugin_entrypoint!` macro。

初始 JSON envelope：

```json
{
  "abiVersion": "1.0.0",
  "pluginId": "publisher.plugin-name",
  "hook": "gateway.request.afterBodyRead",
  "traceId": "optional-trace-id",
  "config": {},
  "context": {}
}
```

guest response envelope：

```json
{
  "action": "replace",
  "requestBody": "{\"messages\":[]}",
  "headers": {
    "x-plugin-redacted": "1"
  },
  "audit": []
}
```

只有当 hook 和 granted permissions 允许时，`action` 才可以是 `pass`、`replace`、`block` 或 `warn`。Replacement fields 使用与 host 相同的 active gateway envelope：`requestBody`、`responseBody`、`streamChunk`、`logMessage` 和 `headers`。Legacy `contextPatch` output 在 vNext 中会被拒绝。

## Guest Entrypoint 入口

guest entrypoint signature：

```wat
(func (export "aio_plugin_handle") (param i32 i32) (result i64))
```

两个参数分别是 request JSON 的 pointer 和 byte length。返回值把 response pointer 和 byte length 打包在一起：

```text
return = (ptr << 32) | len
```

host 要求导出名为 `memory` 的 linear memory。ABI v1 中，host 不会传递 filesystem、network、environment variables、wall-clock access、process spawning 或 random data 的 host functions。

## memory/time/filesystem/network 限制

M5 强制执行这些默认限制：

- Maximum input JSON bytes：256 KiB。
- Maximum output JSON bytes：256 KiB。
- Default guest memory limit：16 MiB，除非 manifest 提供更低限制。
- Default hook timeout：继承 gateway hook timeout，并受 runtime 上限约束。
- 每条 Wasmtime instruction 消耗 fuel，fuel 耗尽的 module 会被终止。
- no WASI filesystem imports are provided。
- no network imports are provided。
- no environment variable imports are provided。
- no host clock import is provided。

host 绝不会把 app data、plugin data、logs、cache 或 user directories mount 到 WASM。未来任何 storage API 都必须是专用、permission-gated 的 host function，并带有 size limits 和 audit logs。

## 执行模型

M5 中 host 会为每次 hook call 创建 fresh Wasmtime store。这比 pooling 慢，但在 foundation phase 更简单、更稳。等 deterministic reset semantics 经过测试后，可以再加入 pooling。

每次执行：

1. 校验 manifest runtime kind 是 `wasm`。
2. 从已安装插件目录读取 module。
3. 在没有 WASI imports 的情况下 compile 和 instantiate module。
4. 把 permission-trimmed JSON envelope 写入 exported memory。
5. 执行 `aio_plugin_handle`。
6. 读取并 bounds-check response JSON。
7. 把 timeout、trap、bad pointer、malformed JSON 和 missing export 转成 structured runtime failures。

## 安全要求

- host only passes permission-trimmed JSON。
- 除非已授权 `request.header.readSensitive` 且 hook 允许，否则插件不能读取 sensitive headers。
- 除非已授权对应 write/modify permission，否则插件不能写 body、headers 或 stream chunks。
- 插件不能访问文件，因为 no WASI filesystem imports are available。
- 插件不能访问网络，因为 no network imports are available。
- fuel-based termination 是 dead-loop protection 的强制要求。
- 所有 runtime failures 都必须能通过英文 diagnostic messages 审计。

## 失败策略

WASM runtime failures 会隔离在当前 hook invocation 内：

- Missing export：runtime failure，plugin result 视为 hook error。
- Trap 或 fuel exhaustion：runtime failure，plugin result 视为 hook error。
- Oversized input 或 output：runtime failure，plugin result 视为 hook error。
- Malformed output JSON：runtime failure，plugin result 视为 hook error。

gateway pipeline 仍根据 hook policy 决定 fail-open 或 fail-closed。runtime 自身绝不会静默忽略错误。

## M5 验收测试

M5 backend tests 覆盖：

- 合法 WASM module 可以 echo 一个小 hook response。
- 导入 WASI filesystem APIs 的 module 会在 instantiation 被拒绝。
- dead-loop module 会因 fuel exhaustion 终止，而不是阻塞 host。

SDK 和示例检查命令：

```bash
pnpm plugin-wasm-sdk:test
```
