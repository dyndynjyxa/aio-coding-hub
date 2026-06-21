# 官方示例插件

官方 catalog 会刻意保持很小。`official.privacy-filter` 是当前唯一 bundled official plugin。

这样可以让 trusted host surface 保持收敛，同时继续通过 `declarativeRules`、WASM 和默认关闭的进程运行时 PoC 提供开放扩展能力。

## 当前官方 ID

- `official.privacy-filter`

用户可以在 Plugins 页面通过官方插件安装入口安装它。

## Privacy Filter

ID: `official.privacy-filter`

Runtime: `native:privacyFilter`

它对齐 [packyme/privacy-filter](https://github.com/packyme/privacy-filter) 的核心 redaction behavior。

它展示了 prompts 和 request logs 的 pre-upstream privacy filtering。

它也展示了 schema-driven configuration UI。宿主会根据 `configSchema` 和 `x-aio-ui` metadata 渲染开关、选择器和 sensitive-type checkbox group，不需要为它写 host-side plugin-specific page component。

Hooks：

- `gateway.request.afterBodyRead`
- `gateway.request.beforeSend`
- `log.beforePersist`

Permissions：

- `request.body.read`
- `request.body.write`
- `log.redact`

行为：

- 脱敏 emails、Chinese mobile phone numbers、Chinese ID card patterns、Luhn-valid bank cards 和 IPv4 addresses。
- 从 `rules/gitleaks.toml` 加载 upstream gitleaks-style rule set。
- 脱敏 known vendor secrets、contextual passwords/API keys 和 high-entropy secret candidates。
- 使用 span merging 和 false-positive mitigation 处理 SSH command targets、paths、URLs、hashes、UUIDs、template variables、common placeholders 和 business ID assignments。

Provider request shapes：

`official.privacy-filter` 会按 `redactionScopes` 选择请求处理范围，并只脱敏协议白名单里的文本字段。默认范围包含系统/开发者指令、用户输入、工具返回结果，以及 legacy `prompt` / raw text bodies。Codex/OpenAI Responses payloads 会处理 `instructions`、`input` string、`input[].content[].text` 和 `function_call_output.output`；Claude-style payloads 会处理 `system`、`messages[].content[].text(type=text)` 和 `tool_result.content`；OpenAI-compatible chat payloads 会处理 `messages[].content` 和 role `tool` 的 content。工具定义、tool schema、tool call arguments、metadata、reasoning/thinking blocks、file/image IDs、URLs 和 base64 data 会保持原样。

Gateway boundary note：Privacy Filter 会接收原始 client-to-gateway body，因为 gateway 必须先看到 prompt 才能脱敏。它的保护保证是：当插件启用并选中匹配策略和处理范围后，gateway-to-upstream provider request body 中的白名单字段和 persisted request logs 会被脱敏。日志脱敏由 `redactLogs` 和 `sensitiveTypes` 控制，不受 request `redactionScopes` 影响。如果你检查 hook 执行前的本地 client request，仍可能看到原始输入。

重要限制：

和 upstream 一样，Privacy Filter 是 irreversible redaction。它不会在 upstream processing 后把原始敏感值恢复到模型响应中。

## 官方风格示例清单

一个 official-style example 必须包含：

- 一个 minimal manifest。
- 一个 Claude messages fixture。
- 一个 Codex/OpenAI Responses input fixture。
- 一个 local replay command。
- 一个 package command。
- 精确列出它请求的 permissions。
- 简短说明哪些行为是 intentionally unsupported。

社区示例应优先使用 `declarativeRules`。只有当行为需要确定性代码执行且规则运行时无法表达时，才考虑 WASM。WASM examples 可以展示 ABI packaging，但 gateway execution 在宿主启用前仍受策略限制。

## 已移除的内置示例

早期草案包含 built-in prompt optimizer、safety detector 和 generic redactor examples。它们不再作为官方插件内置。

类似行为应实现为社区插件：

- Prompt rewriting：在 `gateway.request.afterBodyRead` 上使用 `declarativeRules`。
- Response safety checks：在 `gateway.response.after` 或 `gateway.response.chunk` 上使用 `declarativeRules`。
- Generic log redaction：在 `log.beforePersist` 上使用 `declarativeRules`；规则运行时表达力不够时再考虑 WASM。

## 代码位置

官方插件 fixture 存放在宿主仓库：

```text
src-tauri/resources/plugins/official/privacy-filter/
```

宿主在这里注册它：

```text
src-tauri/src/app/plugins/official.rs
```

在 plugin API v1 稳定前，该 fixture 会继续保留在本仓库。API 稳定后，SDK、scaffolder 和 community examples 可以迁移到独立仓库。
