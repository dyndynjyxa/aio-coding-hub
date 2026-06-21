# 插件开发总指南

这份指南把 AIO Coding Hub 插件从开发到发布的主线串起来。需要查字段定义时看 [Manifest v1 完整规范](../plugin-manifest-v1.md)，需要查某个运行时细节时再跳到 [API 参考](./reference/README.md) 或 [运行时说明](./runtime/README.md)。

## 适合做成插件的能力

插件适合扩展本地网关链路中可被明确 hook 的行为，例如：

- 在请求发往上游模型前改写、追加或脱敏 prompt。
- 在响应返回给 CLI 前做有边界的检查、替换、阻断或告警。
- 在请求日志持久化前做不可逆脱敏。
- 通过 `configSchema` 给用户提供可视化配置项，比如开关、选择器、文本输入和复选组。

插件不适合接管 CLI 内部编辑器、读取宿主数据库连接、直接操作 WebView，或把第三方 native 动态库加载进 Rust 主进程。

## 推荐开发路径

1. 明确插件目标和目标 hook：请求前处理通常用 `gateway.request.afterBodyRead` 或 `gateway.request.beforeSend`，日志脱敏用 `log.beforePersist`。
2. 优先判断能否用 `declarativeRules` 表达。正则匹配、替换、阻断、告警和追加消息都应先走规则运行时。
3. 编写最小 `plugin.json`：只声明必要 runtime、hooks、permissions、hostCompatibility 和 configSchema。
4. 准备 fixture：至少覆盖 Claude `messages[].content[].text` 和 Codex/OpenAI Responses `input[].content[].text` / `input_text` 形态。
5. 使用 `pnpm create-aio-plugin validate` 做 manifest 与规则校验。
6. 使用 `pnpm create-aio-plugin replay` 在本地 fixture 上验证行为。
7. 使用 `pnpm create-aio-plugin pack` 打包为 `.aio-plugin`。
8. 在 Plugins 页面本地导入，授权权限，启用插件，检查审计日志。
9. 发布前计算 `sha256`，可信索引分发时补 Ed25519 签名。

## 10 分钟快速开始

先 scaffold 一个声明式规则插件：

```bash
pnpm --filter create-aio-plugin test
pnpm create-aio-plugin acme.redactor rule
```

生成目录后，先校验 `plugin.json` 和规则文件：

```bash
pnpm create-aio-plugin validate ./acme.redactor
```

添加 Claude 和 Codex request shapes 作为 replay fixtures。Claude fixture 示例：

```json
{
  "request": {
    "body": "{\"messages\":[{\"role\":\"user\",\"content\":\"SECRET_TOKEN\"}]}"
  }
}
```

Codex/OpenAI Responses fixture 示例：

```json
{
  "request": {
    "body": "{\"input\":[{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"SECRET_TOKEN\"}]}]}"
  }
}
```

在本地回放两个 fixtures：

```bash
pnpm create-aio-plugin replay ./acme.redactor ./fixtures/claude-request.json gateway.request.afterBodyRead
pnpm create-aio-plugin replay ./acme.redactor ./fixtures/codex-request.json gateway.request.afterBodyRead
```

打包插件并从 Plugins 页面本地安装：

```bash
pnpm create-aio-plugin pack ./acme.redactor
```

在 Plugins 页面选择本地包 `acme.redactor.aio-plugin`，确认 `request.body.read` 和 `request.body.write` permissions 后启用插件。命中请求后，插件详情面板应展示 hook completion 或 block/failure events，且不应存储 sensitive payload text。

SDK 检查命令：

```bash
pnpm --filter @aio-coding-hub/plugin-sdk typecheck
pnpm plugin-wasm-sdk:test
```

## 插件目录结构

声明式规则插件的最小结构：

```text
acme.redactor/
  plugin.json
  rules/
    main.json
  fixtures/
    claude-request.json
    codex-request.json
```

WASM 插件会额外包含 `plugin.wasm` 或 Rust 工程目录。`rules`、fixture 和源码目录可以按项目需要组织，但 `plugin.json` 必须位于包根目录，或 `.aio-plugin` 解压后的唯一顶层目录内。

## 最小声明式规则插件

一个声明式规则插件至少需要 `plugin.json` 和规则文件。规则插件通常适合请求体替换、日志脱敏、告警和阻断，不需要执行任意 JavaScript/TypeScript。

## `plugin.json` 核心字段

最小 manifest 示例：

```json
{
  "id": "acme.redactor",
  "name": "Acme Redactor",
  "version": "0.1.0",
  "apiVersion": "1.0.0",
  "runtime": {
    "kind": "declarativeRules",
    "rules": ["rules/main.json"]
  },
  "hooks": [
    {
      "name": "gateway.request.afterBodyRead",
      "priority": 50,
      "failurePolicy": "fail-open"
    }
  ],
  "permissions": ["request.body.read", "request.body.write"],
  "hostCompatibility": {
    "app": ">=0.56.0 <1.0.0",
    "pluginApi": "^1.0.0",
    "platforms": ["macos", "windows", "linux"]
  }
}
```

关键规则：

- `id` 使用 `publisher.plugin-name`，不要使用 `official.*`，该命名空间只属于宿主内置插件。
- `version` 使用 SemVer。
- `apiVersion` 表示插件 API 契约版本，不等于应用版本。
- `runtime.kind` 对社区插件优先使用 `declarativeRules`。
- `hooks` 决定插件在哪些阶段被调用。
- `permissions` 决定插件能看到和修改哪些上下文。
- `hostCompatibility` 决定应用版本、插件 API 版本和平台兼容性。

## 运行时选择

`declarativeRules` 是默认社区运行时。它适合 JSONPath 子集选择、正则匹配、替换、阻断、告警和 `appendMessage`。

WASM 适合需要确定性代码逻辑的插件，例如复杂校验、状态较小的解析器或规则运行时无法表达的转换。WASM 执行受宿主策略控制，未启用时会返回 `PLUGIN_RUNTIME_DISABLED`。

进程运行时仍是 PoC，默认关闭，也没有默认 marketplace enablement。它只用于未来无法放进 WASM ABI 的隔离进程场景。

`native` 只用于内置官方引擎，例如 `official.privacy-filter` 的 `native:privacyFilter`。第三方包不能声明 host-native engine。

## Hooks 与请求形态

常用 hook：

- `gateway.request.afterBodyRead`：读取请求体后、发往上游前，适合隐私过滤、prompt 改写和请求体检查。
- `gateway.request.beforeSend`：完成 provider 解析和最终协议整理后、真正发送上游前，适合必须保证最终 upstream body/header 被修改的插件。
- `gateway.response.chunk`：处理流式响应 chunk，只能看到当前 chunk 和有界滑动窗口。
- `gateway.response.after`：处理非流式完整响应体。
- `log.beforePersist`：日志入库前脱敏。

Claude 和 Codex/OpenAI Responses 的请求结构不同。插件应避免只适配一种结构：

```json
{
  "messages": [
    { "role": "user", "content": [{ "type": "text", "text": "hello claude" }] }
  ]
}
```

```json
{
  "input": [
    {
      "type": "message",
      "role": "user",
      "content": [{ "type": "input_text", "text": "hello codex" }]
    }
  ]
}
```

宿主会提供 `request.normalizedMessages`，用于给插件一个 provider-neutral 的消息视图。需要修改原始请求体时，仍应返回 `requestBody`。

## 权限最小化

只申请插件真实需要的权限：

- 只读请求元信息：`request.meta.read`。
- 读取请求体：`request.body.read`。
- 修改请求体：`request.body.write`。
- 读取或修改流式响应：`stream.inspect` / `stream.modify`。
- 日志脱敏：`log.redact`。

高风险和 critical 权限需要更明确的用户授权。插件升级新增权限后，宿主必须要求重新授权，不能静默继承。

## 配置表单

插件通过 `configSchema` 暴露用户配置。宿主负责渲染低代码表单并在保存前后校验。

示例：

```json
{
  "type": "object",
  "required": ["enabled", "mode"],
  "properties": {
    "enabled": {
      "type": "boolean",
      "title": "启用处理",
      "description": "关闭后插件不会修改请求内容。",
      "default": true,
      "x-aio-ui": { "widget": "switch", "order": 10 }
    },
    "mode": {
      "type": "string",
      "title": "处理模式",
      "default": "balanced",
      "enum": ["balanced", "strict"],
      "x-aio-ui": {
        "widget": "select",
        "enumLabels": {
          "balanced": "平衡",
          "strict": "严格"
        },
        "order": 20
      }
    }
  }
}
```

优先使用 JSON Schema 标准字段 `title`、`description`、`default`、`enum`、`required`。需要界面提示时再使用 `x-aio-ui`。

## 本地校验与回放

常用命令：

```bash
pnpm create-aio-plugin validate ./acme.redactor
pnpm create-aio-plugin replay ./acme.redactor ./fixtures/claude-request.json gateway.request.afterBodyRead
pnpm create-aio-plugin replay ./acme.redactor ./fixtures/codex-request.json gateway.request.afterBodyRead
pnpm create-aio-plugin pack ./acme.redactor
```

验收一个插件时，至少确认：

- `plugin.json` 能通过校验。
- 所有规则文件路径合法，没有 `..` 或绝对路径。
- Claude fixture 和 Codex/OpenAI Responses fixture 都能得到预期结果。
- 未授权的读写不会生效。
- 打包后的 `.aio-plugin` 能在 Plugins 页面导入、授权并启用。

## 发布与升级

本地分发可以直接使用 `.aio-plugin`。远程分发必须提供可信的下载地址和 `sha256`。通过可信索引发布时，建议提供 Ed25519 签名和对应公钥。

升级时要特别检查：

- 新版本是否仍满足 `hostCompatibility`。
- 是否新增 permissions。新增权限必须重新授权。
- 是否改变 `configSchema`。需要通过 `configVersion` 和默认值保证旧配置可迁移。
- 回滚是否能恢复旧版本、旧配置快照、旧权限和启用状态。

## 官方 Privacy Filter 示例

`official.privacy-filter` 是当前唯一内置官方插件。它对齐 [packyme/privacy-filter](https://github.com/packyme/privacy-filter) 的核心脱敏能力，用于请求发往上游前和日志持久化前的不可逆脱敏。完整说明见 [Privacy Filter 示例](./examples/privacy-filter.md)。

这个示例说明了三件事：

- 插件页面可以完全根据 `configSchema` 和 `x-aio-ui` 渲染配置，而不是写专用页面。
- 官方 `native` 引擎应少而聚焦，避免扩大宿主维护面。
- 社区版同类插件应优先从 `declarativeRules` 或 WASM 起步，而不是请求第三方 `native` 能力。

## 常见排障

- 插件无法启用：先看 `hostCompatibility`、runtime policy、manifest 校验错误和权限是否已授权。
- WASM 插件提示 `PLUGIN_RUNTIME_DISABLED`：说明宿主策略尚未启用 WASM 执行。
- 请求没有被改写：检查 hook 是否选对，是否同时拥有读写权限，fixture 是否覆盖实际 provider 请求结构。
- 只能看到原始本地请求：隐私过滤保护的是 gateway-to-upstream body 和持久化日志；client-to-gateway 的本地入站请求在 hook 前仍是原文。
- 日志仍有敏感值：确认插件声明并授权 `log.redact`，且启用了 `log.beforePersist`。
