# Codex 独立搜索兼容与验收（issue #378）

## 问题与实现边界

`POST alpha/search` 使用独立的搜索协议。原网关对所有 Codex JSON 请求执行 Session ID 补全，向搜索体注入 `prompt_cache_key`，导致上游拒绝未知参数；本地会话提取又忽略搜索体的 `id`，可能合并不同搜索会话或随输入变化漂移。

修复复用现有网关链路，不新增设置、协议框架或认证分支：

- 统一识别 Codex POST 的转发路径 `/alpha/search`、`/v1/alpha/search`、`/codex/alpha/search`、`/v1/codex/alpha/search`，支持尾斜杠。query 按原规则保留。
- 搜索跳过 Session ID 补全。本地绑定只读取搜索体字符串 `id`，复用现有会话 ID 清理与供应商选择逻辑；缺失、空白或非字符串 `id` 不使用请求头或输入指纹作为回退。不会修改发送给上游的 `id`。
- 每次发送在供应商自定义头和 `beforeSend` 插件之后、请求体重新编码之前，移除顶层 `prompt_cache_key`、`prompt_cache_retention`、`store`。
- 移除 `openai-beta`、`session_id`、`x-session-id`、`conversation_id`、`x-codex-beta-features`、`x-codex-turn-state`、`x-openai-internal-codex-responses-lite`；保留认证、账号身份、UA、originator、version 和 `x-codex-turn-metadata`。
- OAuth 与 API Key 共用以上规则，不受 Session ID 补全开关影响。OAuth 继续使用现有 ChatGPT 路径转换和认证流程。
- 其他搜索字段、嵌套同名字段、合法插件修改保持原语义；无需清理的请求体保持原始字节，gzip 沿用现有解码/重编码机制。
- 仅实际发生清理时记录 `codex_alpha_search_compat`，包含字段名、头名、供应商 ID，不记录移除的值。

普通 Responses、CX2CC、供应商资格检查、强制路由、熔断、失败重试及计费继续沿用原逻辑。本次不引入 PAT fallback、搜索能力探测或新的错误健康度策略。

参考 [issue #378](https://github.com/dyndynjyxa/aio-coding-hub/issues/378) 和 [sub2api 的独立搜索实现](https://github.com/Wei-Shaw/sub2api/blob/9a62841fd124d026cf3694fcf9b79e98addcdbdc/backend/internal/service/openai_alpha_search.go)。

## 自动验收

| 核心场景 | 验收标准 | 覆盖位置 |
| --- | --- | --- |
| 开关开/关、路径别名、gzip | 搜索无 Responses 注入；query、未知字段、响应中的 `encrypted_output`/`output`/`results` 保留 | `routes/tests/alpha_search.rs` |
| 自定义头与插件 | 最终上游请求清理已知不兼容字段和头；合法插件改动及身份信息保留；审计不含被移除的值 | `routes/tests/alpha_search.rs` |
| 会话绑定 | commands-only 请求和同一 `id` 的后续查询使用已绑定供应商；不同 `id` 独立选择 | `routes/tests/alpha_search.rs` |
| 非法/缺失 id | 无指纹回退或错误绑定；合法 id 使用现有规范化规则 | `proxy/handler/mod.rs` |
| OAuth | 现有路径转换不会调用 Responses 请求体转换；搜索清理保留 OAuth 认证和账号身份头 | `prepare/codex_chatgpt.rs` |
| 协议边界 | 仅 Codex POST 搜索匹配；清理幂等；未污染、非对象或无法解析的体不被重写；嵌套字段保留 | `proxy/codex_alpha_search.rs` |
| 错误与相邻功能 | 上游普通 400 保持错误且不增加修复重试；Responses 会话补全和缓存字段保留；现有 CX2CC、会话、插件用例通过 | gateway 测试集 |
| 前端设置 | 文案说明搜索例外；开关能从两种状态准确持久化；页面和 service/query 既有成功/失败流程通过 | GeneralTab、CliManagerPage、settings 测试 |

```sh
pnpm tauri:test --lib gateway::
pnpm tauri:check
pnpm tauri:clippy
pnpm exec vitest run src/components/cli-manager/tabs/__tests__/GeneralTab.test.tsx src/pages/__tests__/CliManagerPage.test.tsx src/query/__tests__/settings.test.tsx src/services/settings/__tests__/settingsCodexSessionIdCompletion.service.test.ts
pnpm typecheck
pnpm lint
pnpm check:spec-links
```

本次本地验证：gateway 测试 961 项通过、4 项忽略；前端上述 4 个测试文件共 54 项通过；`tauri:check`、`tauri:clippy`（全 targets，warnings 视为错误）、`typecheck`、`lint`、`check:spec-links` 和 `git diff --check` 均通过。新增用例先复现搜索请求被补入 `session_id` 头，修复后通过。

本机 Rust 验证需指定 `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX15.4.sdk`，避开当前默认 SDK 与链接器的架构不兼容问题。这是验证环境设置，不属于产品改动。

## 真实客户端验收（待执行）

自动测试的 API Key 用例经过真实网关路由和本地 HTTP 上游；OAuth 用例验证路径/身份/请求体处理，不代表已用真实账号完成联网搜索。

1. 分别使用实际 Codex CLI 和 Desktop 内置 CLI，记录各自版本，配置 AIO 的 Codex OAuth 供应商后发起网络搜索。
2. 在同一会话执行后续打开/查找操作，确认搜索结果可用、会话 ID 稳定且沿用可用的已绑定供应商；另一会话不误复用此绑定。
3. 分别打开和关闭“Codex Session ID 补全”，两种状态下都不得因 AIO 注入 `prompt_cache_key` 出现 unknown_parameter。
4. 使用支持独立搜索的 API Key 中转重复验证；普通 Responses 对话、工具调用及 CX2CC 请求正常。
5. 上游没有搜索权限或本身不支持该端点时，应显示原有错误，不将此次兼容修复理解为新增上游能力。
