# 插件运行时说明

这里解释插件运行时如何执行，以及当前哪些能力已经开放。普通社区插件优先使用 `declarativeRules`；只有确实需要代码执行时，才阅读 WASM 或进程运行时说明。

- [WASM 运行时](./wasm.md)：WASM ABI v1、`PLUGIN_RUNTIME_DISABLED`、资源限制和失败策略。
- [进程运行时 PoC](./process-poc.md)：默认关闭的 JSON-RPC over stdio 进程隔离设计。
- [流式响应插件](./streaming.md)：`gateway.response.chunk`、sliding window 和 `stream.modify` 的边界。

声明式规则属于插件作者最常用的社区运行时，契约文档在 [Declarative Rules](../reference/declarative-rules.md)。
