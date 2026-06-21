# 进程插件运行时 PoC

## 目标

process runtime PoC 探索通过 child process 和 JSON-RPC over stdio 实现插件隔离。它只是设计和生命周期基础：disabled by default，并且 no marketplace enablement by default。

这个运行时不是 WASM runtime 的替代品。它服务于未来无法放进 WASM ABI、但仍需要与 Rust main process 和 Tauri WebView 隔离的插件。

## 边界

process runtime 会把插件 executable 作为 child process 运行，并满足：

- stdin 和 stdout 只用于 JSON-RPC over stdio。
- stderr 只作为 bounded diagnostics 捕获。
- 不继承 app stdin。
- 不能直接访问 Tauri WebView。
- 不能直接访问 app SQLite connections。
- 宿主不会隐式授予 network 或 filesystem 权限。

未来任何 filesystem、network 或 secret access 都必须通过显式 host-mediated APIs。M5 不暴露这些 API。

## JSON-RPC over stdio 协议

每个 request 是一个 newline-delimited JSON-RPC 2.0 object：

```json
{"jsonrpc":"2.0","id":1,"method":"plugin.handleHook","params":{"hook":"gateway.request.afterBodyRead","context":{}}}
```

每个 response 是一个 newline-delimited JSON-RPC 2.0 object：

```json
{"jsonrpc":"2.0","id":1,"result":{"action":"pass"}}
```

宿主会拒绝：

- malformed JSON。
- mismatched IDs。
- 超过 configured byte limit 的 response。
- plugin-side JSON-RPC errors。
- hook timeout 后输出的内容。

## 生命周期

process lifecycle 有四个有边界阶段：

1. 在 start timeout 内 spawn child。
2. 发送 hook request，并在 hook timeout 内等待一个 response。
3. 只在 idle recycle 过期前保持进程 warm。
4. 在 timeout、crash、protocol error 或 idle recycle 时 kill 并 reap child。

初始 PoC 每个 test session 启动一个 process，并且只在它保持 healthy 且 idle time 低于配置阈值时复用。

## 必需限制

- start timeout 默认 500 ms。
- hook timeout 默认 300 ms。
- idle recycle 默认 30 seconds。
- request 和 response lines 分别限制在 256 KiB。
- stderr diagnostics 有边界，不会无上限 stream 到 UI。

## 安全策略

- The runtime is disabled by default。
- There is no marketplace enablement by default。
- 使用前，host policy 必须显式把 process plugins 标记为 experimental。
- crash isolation 必须保证 child exit 不会导致 app crash。
- Timeouts 必须 kill child process，并记录一条英文 diagnostic message。
- 宿主必须把每个 protocol error 视为 runtime failure。

## M5 验收测试

M5 backend tests 覆盖：

- 合法 child process 能启动并返回 JSON-RPC hook result。
- startup 期间 sleep 的 child 会触发 start timeout。
- hook handling 期间 sleep 的 child 会触发 hook timeout 并被 kill。
- 提前退出的 child 会被报告为 crash isolation，而不是 host crash。
- healthy idle child 会在 idle recycle 后被回收。
