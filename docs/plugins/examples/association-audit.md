# Association Audit / 关联审计

`community.association-audit` 是一个默认关闭的社区被动关联审计插件。它不修改网关响应，不阻断请求，只在后台对每条请求的输入和输出做一次独立的安全抽查，结果写入插件审计日志。

> `community.association-audit` is a default-off community passive audit plugin. It never modifies gateway responses or blocks requests. It performs an independent security spot-check on each request's input and output in the background, writing results to plugin audit logs.

## 做了什么 / What it does

插件把请求原文和响应原文（经基础脱敏后）打包发给一个指定的审计模型，由该模型判断：

- 模型输出是否植根于用户的原始请求（association score 低说明模型跑偏了）
- 是否出现未授权的脚本/回调/凭据/网络/持久化内容（由 signals 逐项标记）
- 是否发生角色或策略漂移（模型偏离了它应该扮演的角色）

正常的诊断工具调用不构成风险。审计模型只输出顾问信号，不做阻断/重试/改写决策。

> The plugin packages the request and response bodies (after basic redaction) and sends them to a designated audit model, which judges:
>
> - Whether the model output is grounded in the user's original request (low association score means the model went off-track)
> - Whether unauthorized script/callback/credential/network/persistence content appears (flagged per signal)
> - Whether role or policy drift occurred
>
> Normal diagnostic tool calls are not risky. The audit model outputs advisory signals only — no blocking, retrying, or rewriting decisions.

## 安装 / Installation

本插件以 `.aio-plugin` 包形式分发。用户从社区渠道下载后，在 AIO Coding Hub 的 **Plugins 页面** 通过"从文件安装"导入。

> This plugin is distributed as a `.aio-plugin` package. Download from community channels, then import via "Install from file" on the **Plugins page** in AIO Coding Hub.

安装完成后插件状态为 **Disabled**，mode 为 **off**，不会执行任何操作。

> After installation the plugin status is **Disabled** with mode **off** — no execution occurs.

## 配置 / Configuration

安装后，在插件详情页修改配置：

> After installation, configure the plugin on its detail page:

| 字段 Field       | 说明 Description                                                        | 推荐值 Recommended |
| ---------------- | ----------------------------------------------------------------------- | ------------------ |
| `mode`           | 审计模式 / Audit mode                                                   | `prefiltered`      |
| `providerId`     | 用于 `model.invoke` 的 provider 数字 ID                                 | 必填 Required      |
| `model`          | provider 能识别的原始模型名 / Raw model name recognized by the provider | 必填 Required      |
| `sampleRate`     | 采样百分比 (1-100) / Sampling percentage                                | 默认 10            |
| `timeoutSeconds` | 审计调用超时 (1-60) / Audit call timeout                                | 默认 8             |
| `maxInputChars`  | 审计包中的请求最大字符数 / Max request chars in audit package           | 默认 6000          |
| `maxOutputChars` | 审计包中的响应最大字符数 / Max response chars in audit package          | 默认 12000         |

`providerId` 和 `model` 必须都填才能发起审计调用。

> Both `providerId` and `model` must be filled for audit calls to execute.

## 审计模式 / Audit Modes

| mode          | 行为 Behavior                                                                                    | 适用场景 Use case                    |
| ------------- | ------------------------------------------------------------------------------------------------ | ------------------------------------ |
| `off`         | 不执行审计 / No audit                                                                            | 默认状态 Default                     |
| `sampled`     | 按 `sampleRate` 稳定采样 / Stable sampling                                                       | 抽查 Sampling                        |
| `prefiltered` | 先本地过滤，有信号才调模型（推荐） / Local filter first, call model only on signal (recommended) | 平衡成本与覆盖 Cost/coverage balance |
| `all`         | 所有有响应的 trace 都审计 / Audit all traces with responses                                      | 完整覆盖 Full coverage               |

### Prefilter 过滤逻辑 / Prefilter Logic

当 mode 为 `prefiltered` 时，满足任一条件才会发起模型调用：

> When mode is `prefiltered`, the model is called only when any of these conditions is met:

- 响应被截断 / Response is truncated
- 响应体过短 (< 120 字符) → 直接跳过 / Response too short → skipped directly
- 响应包含可疑标记：`curl`、`powershell`、`cmd.exe`、`bash -c`、`chmod`、`rm -rf`、`http://`、`api_key`、`bearer`、`webhook`、`ssh-rsa`、`BEGIN PRIVATE KEY` 等 / Response contains suspicious markers
- 请求和响应 token overlap < 8% 且响应 > 400 字符 / Token overlap between request and response below 8% with response > 400 chars

## Provider 选择条件 / Provider Selection Requirements

> The configured provider must satisfy ALL of the following:

1. **已启用** / Enabled
2. **API Key 鉴权** / `auth_mode` is `api_key` (not OAuth)
3. **非桥接** / No `source_provider_id` or `bridge_type`
4. **CLI key 为 claude 或 codex** / Only `claude` or `codex` CLI key supported
5. **API key 已配置** / API key is configured
6. **base URL 已填写** / At least one non-empty base URL

### 可以 / Works

- 直接填了 API key 的 Anthropic provider（`cli_key = "claude"`）/ Direct Anthropic provider with API key
- 通过 OneAPI 转发、使用 API key 鉴权的 OpenAI 兼容 endpoint / OpenAI-compatible endpoint via proxy with API key auth

### 不行 / Won't work

- OAuth provider → `api_key_provider_required`
- Bridge provider → `bridged_provider_unsupported`
- Gemini 等非 claude/codex 的 CLI key → `provider_cli_unsupported`
- API key 未填写 / Empty API key
- base URL 为空 / Empty base URL

## 模型映射 / Model Mapping

`model.invoke` 绕过网关的主请求路由层。用户在插件配置里填的 `model` 字符串会被原样发送。Provider 的 `claude_models` 别名、`cx2cc` bridge 等映射功能不影响审计调用。

> `model.invoke` bypasses the gateway's main request routing layer. The `model` string from plugin config is sent as-is. Provider model aliases (`claude_models`, `cx2cc` bridge, etc.) do not affect audit calls.

## 结果解读 / Reading Results

审计结果在请求日志详情中展示。每条记录包含：

> Audit results appear in request log detail. Each record contains:

| event_type                           | 含义 Meaning                    | 关键字段 Key fields                                                   |
| ------------------------------------ | ------------------------------- | --------------------------------------------------------------------- |
| `association.audit.completed`        | 审计完成 / Completed            | `result.association_score`、`result.overall_risk`、`result.signals[]` |
| `association.audit.skipped`          | 被跳过 / Skipped                | `reason`（`sampled_out` 或 `prefilter_no_signal`）                    |
| `association.audit.not_configured`   | 未配置 / Not configured         | `reason`（具体原因 / specific cause）                                 |
| `association.audit.failed`           | 失败 / Failed                   | `error`（错误信息 / error message）                                   |
| `association.audit.invalid_response` | 响应格式异常 / Invalid response | `error`                                                               |

### 示例解读 / Example

```json
{
  "event_type": "association.audit.completed",
  "risk_level": "medium",
  "details": {
    "result": {
      "association_score": 0.32,
      "overall_risk": "medium",
      "signals": [
        {
          "code": "ungrounded_script",
          "severity": "high",
          "confidence": 0.85,
          "event_index": 3,
          "event_kind": "tool_call",
          "reason": "Writes script that installs packages unrelated to user request",
          "evidence": "pip install [REDACTED_TOKEN] ..."
        }
      ]
    }
  }
}
```

- `association_score = 0.32`：请求和响应之间语义关联偏低 / Low semantic association
- `overall_risk = "medium"`：存在中等风险信号 / Medium-level risk signal present
- signal 标记了一个高风险工具调用，置信度 85% / One high-severity tool call flagged at 85% confidence

## 常见排障 / Troubleshooting

**"没有审计记录" / "No audit records"**

- 插件状态是否为 Enabled / Is the plugin enabled?
- `mode` 是否设为 `off` / Is mode set to `off`?
- `sampled` 模式下只有部分 trace 被选中 / Only a fraction of traces are selected in `sampled` mode
- `prefiltered` 模式下本地过滤可能跳过了所有 trace / Local prefilter may have skipped all traces

**"全是 not_configured" / "All records are not_configured"**

- `providerId` 是否为正数 / Is `providerId` a positive integer?
- `model` 是否非空 / Is `model` non-empty?
- Provider 是否满足全部六条选择条件 / Does the provider meet all six selection requirements?

**"大量 failed" / "Many failed records"**

- 查看 `details.error` / Check `details.error`
- 常见原因：provider 不可达、超时、凭据过期 / Common causes: provider unreachable, timeout, expired credentials

**"provider 填了但提示 unsupported" / "Provider filled but says unsupported"**

- 确认 CLI key 是 `claude` 或 `codex` / Verify CLI key is `claude` or `codex`
- 确认不是 OAuth 也不是桥接 provider / Verify it's not OAuth or bridged

## 边界 / Boundaries

- 不阻断、重试、改写或路由请求 / Does not block, retry, rewrite, or route requests
- 不暴露 provider credentials / Does not expose provider credentials
- 不替代隐私过滤 / Does not replace privacy filtering
- 审计调用在后台异步执行，关网关前未完成的调用不会产生记录 / Audit calls run async in background; incomplete calls on gateway shutdown produce no records

## 代码位置 / Code Locations

```text
plugins/association-audit/plugin.json
src-tauri/src/app/plugins/association_audit.rs
```
