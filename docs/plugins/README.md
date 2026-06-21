# AIO Coding Hub 插件开发手册

本目录是 AIO Coding Hub 插件系统的中文入口。这里不再平铺所有专题文档；新开发者先读主线指南，需要细节时再进入参考目录。

插件可以扩展本地网关、请求和响应 hook、日志脱敏，以及由界面管理的配置表单。社区插件优先使用 `declarativeRules`；`native` 只保留给宿主内置官方插件。

## 先读什么

- [插件开发总指南](./developer-guide.md)：唯一主线入口，从创建插件到本地回放、配置表单、打包和发布。
- [Privacy Filter 示例](./examples/privacy-filter.md)：查看官方示例插件如何组织 manifest、配置和隐私过滤边界。
- [插件 API 参考](./reference/README.md)：查 `plugin.json`、hooks、permissions、config schema、SDK 和发布规则。

## 按目标查找

| 我想做什么 | 阅读 |
| --- | --- |
| 开发第一个插件 | [插件开发总指南](./developer-guide.md) |
| 给插件加配置项 | [Config Schema](./reference/config-schema.md) |
| 处理 Claude/Codex 请求结构 | [插件开发总指南：Hooks 与请求形态](./developer-guide.md#hooks-与请求形态) |
| 查 hook 触发时机 | [Hooks](./reference/hooks.md) |
| 查权限和风险等级 | [Permissions](./reference/permissions.md) |
| 写声明式规则 | [Declarative Rules](./reference/declarative-rules.md) |
| 打包发布 `.aio-plugin` | [Publishing](./reference/publishing.md) |
| 理解 WASM 限制 | [WASM 运行时](./runtime/wasm.md) |
| 理解架构和边界 | [插件架构说明](./architecture/README.md) |

## 目录结构

- `developer-guide.md`：开发者主线手册。
- `examples/`：官方示例和推荐社区插件形态。
- `reference/`：稳定 API 契约和工具链说明。
- `runtime/`：WASM、进程运行时、流式响应等执行模型。
- `architecture/`：维护者视角的安全、隔离、性能和稳定性说明。
- `plugin-api-v1-contract.json`：机器可读的插件 API v1 契约。

## 推荐开发顺序

1. 如果插件不需要执行代码，先选择 `declarativeRules`。
2. 编写 `plugin.json`，只声明必需的 hooks 和 permissions。
3. 添加聚焦的规则文件、fixture，或 WASM 入口代码。
4. 使用 `create-aio-plugin` 校验真实插件目录。
5. 在导入桌面应用前，用 replay fixture 覆盖 Claude 和 Codex/OpenAI Responses 请求形态。
6. 本地行为稳定后再打包 `.aio-plugin`，需要可信分发时再补签名。

## 当前稳定性说明

- 不支持任意 JavaScript 或 TypeScript 插件直接运行。
- WASM 和进程运行时文档描述的是隔离契约；是否允许执行仍由宿主策略控制。
- Manifest 校验只接受已激活 hooks 和 permissions；保留项仅用于未来兼容命名。
- 当前只有 `official.privacy-filter` 是内置官方 `native` 插件。社区扩展应使用 `declarativeRules`、WASM，或未来隔离进程运行时。
