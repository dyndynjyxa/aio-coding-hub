# 插件架构审计

本文记录将官方 catalog 收敛到 `official.privacy-filter` 后，当前插件系统架构的审计结论。

## 决策

只保留 `official.privacy-filter` 作为 bundled official plugin。

移除之前官方 catalog 中的 built-in prompt optimizer、safety detector 和 generic redactor examples。它们仍然是有效扩展场景，但应通过 `declarativeRules`、WASM 或未来隔离进程运行时作为社区插件实现。

## 架构依据

成熟插件系统通常保持小而可信的 host core，并暴露稳定 extension points，而不是不断累积 host-owned examples：

- VS Code uses manifest-declared [contribution points](https://code.visualstudio.com/api/references/contribution-points) and [activation events](https://code.visualstudio.com/api/references/activation-events).
- Chrome extensions require manifest-declared [permissions](https://developer.chrome.com/docs/extensions/develop/concepts/declare-permissions) and use constrained background [service workers](https://developer.chrome.com/docs/extensions/develop/migrate/to-service-workers).
- IDE plugin platforms expose explicit [extension points](https://plugins.jetbrains.com/docs/intellij/plugin-extension-points.html) and versioned compatibility contracts.

AIO Coding Hub 采用同样形态：

- `plugin.json` 声明 ID、runtime、hooks、permissions、config schema 和 host compatibility。
- Hooks 是明确的 gateway/log extension points，带有 bounded timeouts 和 permission-trimmed contexts。
- 社区代码执行不会进入 Rust main process 和 WebView。
- `native` 只保留给 built-in official engines。第三方包不能声明 host-native engines。

## 信任边界

当前 host trust boundary：

- Trusted：Rust host、gateway pipeline、database、packaged official native privacy engine。
- Semi-trusted：signed marketplace metadata 和 package checksums。
- Untrusted by default：local packages、marketplace packages、GitHub release packages、rule files、WASM bytecode、process runtime binaries。

`official.*` namespace 必须继续由宿主拥有。本地、marketplace 和 GitHub 包必须使用类似 `acme.plugin-name` 的 publisher namespace。

## 扩展模型

推荐 runtime 选择顺序：

1. `declarativeRules`：用于 JSON path selection、regex detection、replacement、warning、blocking 和 message append behavior。
2. WASM：用于需要 rule files 之外逻辑的 deterministic code plugins。
3. Managed process runtime：只用于未来无法适配 WASM 的场景，并且默认没有 marketplace enablement。

不要开放第三方 `native` 插件，除非先补齐独立 signed binary policy、ABI stability story、crash isolation model、upgrade story 和 platform-specific security review。

## 性能与稳定性建议

保持 hot path 可预测：

- 按 priority 顺序执行 hooks，并使用固定 timeout budgets。
- 在暴露给插件前，对 request 和 response bodies 做大小边界控制。
- Stream hooks 保持 chunk-based，并提供 sliding-window context，而不是缓冲完整 stream。
- 按 plugin ID、version 和 runtime key 缓存 parsed rule/native engine state。
- 对非安全增强使用 fail open；只对用户明确启用的 security/privacy gates 使用 fail closed。
- 记录 runtime failures 和 circuit-open skips，避免坏插件持续拖慢 gateway。
- official native engines 要少而聚焦，控制 host startup、binary size 和维护风险。

## v1.1 Performance Budgets

- Empty plugin pipeline request hook：不应有 allocation-heavy runtime dispatch，在维护者笔记本 performance smoke 上低于 25 microseconds。
- One noop declarative plugin request hook：在维护者笔记本 performance smoke 上低于 250 microseconds。
- 没有 `gateway.response.chunk` plugins 时：direct stream pass-through path 必须保持 active。
- One declarative rule plugin：parsed rule runtime 必须在首次执行后缓存。
- Privacy Filter：compiled detector 必须按 plugin ID、version、installed directory 和 runtime key 缓存。

## 当前形态

Bundled official plugin：

- `official.privacy-filter`：与 `packyme/privacy-filter` 对齐的 native host engine，用于 irreversible pre-upstream privacy redaction 和 log redaction。

开放给社区的能力：

- Declarative prompt helpers。
- Declarative response safety checks。
- Declarative 或 WASM log redactors。
- WASM examples 和 SDK contracts。
- 默认关闭的 Process runtime proof-of-concept documentation。

## 后续审计点

在把 plugin API v1 标记为 stable 前：

- 确认 hook names 和 permission names 已足够稳定，可以进入 semantic versioning。
- 补充 WASM enablement 和 package signing 的 marketplace policy。
- 把 official examples 保留为文档中的 community patterns，而不是 bundled host plugins。
- 增加 plugin hook overhead 和 Privacy Filter 在大型但允许 payload 上 redaction latency 的 benchmarks。
- 增加 telemetry-safe counters，记录 plugin timeouts、skips 和 quarantines，但不记录 sensitive payloads。

## v1.1 加固决策

- Plugin API v1.1 使用 `plugin-api-v1-contract.json` 作为 source of truth。
- Provider-neutral request context 通过 `request.normalizedMessages` 提供。
- 当 host policy disables execution 时，WASM enablement remains rejected。
- Plugin refresh 时会清理 runtime caches。
- Plugin hot-path performance smoke tests 是 release readiness 的一部分。
- `create-aio-plugin replay` 与受支持的 declarative rule subset 保持一致。
