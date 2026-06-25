# Association Audit 官方插件

`official.association-audit` 是默认关闭的被动关联审计插件。它不修改网关响应，不阻断请求，只在后台对每条请求的输入和输出做一次独立的安全抽查，结果写入插件审计日志。

## 它做什么

插件把请求原文和响应原文（经基础脱敏后）打包发给一个指定的审计模型，由该模型判断：

- 模型输出是否植根于用户的原始请求（association score 低说明模型跑偏了）
- 是否出现未授权的脚本/回调/凭据/网络/持久化内容（由 signals 逐项标记）
- 是否发生角色或策略漂移（模型偏离了它应该扮演的角色）

正常的诊断工具调用不构成风险。审计模型只输出顾问信号，不做阻断/重试/改写决策。

## 快速上手

### 1. 准备工作：确认有一个可用的 Provider

Audit 调用的 provider 必须同时满足以下条件（详见 [Provider 选择条件](#provider-选择条件)）：

- 已启用、鉴权方式为 API Key、已填写 base URL 和 API key
- CLI key 为 `claude` 或 `codex`
- 不是桥接 / OAuth provider

在 AIO Coding Hub 的 **Providers 页面** 确认你有一个满足条件的 provider，记下它的数字 ID。

### 2. 安装插件

在 **Plugins 页面 > 官方插件** 中找到 `Association Audit`，点击安装。

安装完成后插件状态为 **Disabled**，mode 为 **off**，不会执行任何操作。

### 3. 配置

在插件详情页修改配置：

| 字段 | 说明 | 推荐值 |
|------|------|--------|
| `mode` | 审计模式 | `prefiltered` |
| `providerId` | 步骤 1 中确认的 provider 数字 ID | 必填 |
| `model` | provider 能识别的原始模型名，如 `claude-sonnet-4-20250514` | 必填 |
| `sampleRate` | 采样模式下的百分比（1-100） | 默认 10 |
| `timeoutSeconds` | 单次审计调用超时（1-60） | 默认 8 |
| `maxInputChars` | 发给审计模型的请求体最大字符数 | 默认 6000 |
| `maxOutputChars` | 发给审计模型的响应体最大字符数 | 默认 12000 |

`providerId` 和 `model` 必须都填才能发起审计调用，否则每条 trace 都会记录 `not_configured`。

### 4. 启用

配置完成后，在插件页面点击 **启用**。插件随即进入网关 hook 管线，开始对每条请求被动工作。

### 5. 查看结果

正常使用网关一段时间后，打开任意 **请求日志详情**：

- **摘要 tab**：`Plugin Audit Logs` 卡片列出该 trace 的审计记录，含风险等级标签（颜色编码）和详情摘要
- **原始 tab**：展开 `plugin_audit_logs` 查看完整 JSON

## 审计模式

| mode | 行为 | 适用场景 |
|------|------|----------|
| `off` | 不执行审计 | 默认状态，或不希望产生审计开销时 |
| `sampled` | 按 `sampleRate` 百分比稳定采样 | 只想抽查部分请求时 |
| `prefiltered` | 先本地过滤，有信号才调模型（推荐） | 平衡成本和覆盖率 |
| `all` | 所有有响应的 trace 都审计 | 需要完整覆盖时 |

### prefilter 过滤逻辑

当 mode 为 `prefiltered` 时，插件先做本地判断，满足任一条件才会发起模型调用：

- 响应被截断（超过 `maxOutputChars` 或流式累积上限）
- 响应体过短（< 120 字符）→ 直接跳过
- 响应中包含以下可疑标记：`curl`、`powershell`、`cmd.exe`、`bash -c`、`chmod`、`rm -rf`、`http://`、`api_key`、`bearer`、`webhook`、`ssh-rsa`、`BEGIN PRIVATE KEY` 等
- 请求和响应的 token overlap 低于 8%，且响应超过 400 字符（说明模型可能显著偏离用户输入）

## 审计原理

### 发给审计模型的数据

插件把每次请求打包为如下结构，并做了基础 secret redaction（Bearer token、api_key=xxx、私钥、长 token 等替换为 `[REDACTED_*]`）：

```json
{
  "schemaVersion": 1,
  "traceId": "trace-xxx",
  "mode": "prefiltered",
  "request": {
    "cliKey": "claude",
    "method": "POST",
    "path": "/v1/messages",
    "requestedModel": "claude-sonnet-4-20250514",
    "body": "{\"messages\":[{\"role\":\"user\",\"content\":\"请帮我...\"}]}",
    "truncated": false
  },
  "response": {
    "status": 200,
    "body": "{\"content\":[{\"type\":\"text\",\"text\":\"好的...\"}]}",
    "truncated": false,
    "events": [
      { "index": 0, "kind": "message_start", "text": "..." },
      { "index": 1, "kind": "content_block_delta", "text": "..." }
    ]
  },
  "task": {
    "kind": "association_audit",
    "instructions": "Return only the requested JSON object."
  }
}
```

系统指令明确告诉审计模型需要判断的风险维度：低关联性、未授权可执行/脚本/回调/凭据/网络/持久化内容、角色或策略漂移、不相关的推广/厂商内容。正常诊断工具调用不构成风险。

### 输出归一化

审计模型的原始 JSON 输出会被归一化处理：

- `association_score` 钳位到 `[0.0, 1.0]`
- `overall_risk` 标准化到 `none` / `low` / `medium` / `high` / `critical` / `unknown`
- signal 的 `code` 做蛇形命名（如 `"Ungrounded Script"` → `"ungrounded_script"`）
- `confidence` 钳位到 `[0.0, 1.0]`
- `reason` 和 `evidence` 字段再做一轮 secret redaction
- signals 最多保留 12 条，每条 reason 限 900 字符、evidence 限 700 字符

## 结果解读

每条审计记录在请求日志详情中展示。常见 event type：

| event_type | 含义 | details 中的关键字段 |
|---|---|---|
| `association.audit.completed` | 审计完成 | `result.association_score`、`result.overall_risk`、`result.signals[]` |
| `association.audit.skipped` | 被采样或 prefilter 跳过 | `reason`（`sampled_out` 或 `prefilter_no_signal`） |
| `association.audit.not_configured` | provider/model 未配置或不可用 | `reason`（具体原因） |
| `association.audit.failed` | 模型调用失败 | `error`（错误信息） |
| `association.audit.invalid_response` | 审计模型返回非 JSON 或格式错误 | `error` |

### 实战解读示例

一次 `completed` 审计记录的关键字段：

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
      ],
      "insufficient_context": false
    }
  }
}
```

解读：

- `association_score = 0.32`：请求和响应之间的语义关联偏低，模型输出可能偏离了用户意图
- `overall_risk = "medium"`：存在中等级别的风险信号
- `signals[0]`：在第 3 个输出事件中检测到一个高风险的工具调用，安装与用户请求无关的包，置信度 85%
- evidence 中的敏感值已被脱敏

## Provider 选择条件

配置的 provider 必须满足以下全部条件：

1. **已启用** -- provider 状态为 `enabled`
2. **API Key 鉴权** -- `auth_mode` 为 `api_key`，OAuth provider 不被支持
3. **非桥接 provider** -- 不能有 `source_provider_id` 或 `bridge_type`
4. **CLI key 为 claude 或 codex** -- 目前只适配了这两种 CLI key 对应的协议格式
5. **API key 已配置** -- provider 的 API key 字段非空
6. **base URL 已填写** -- 至少一个 `base_urls` 非空

### 例子

**可以：**

- 直接填了 API key + `https://api.anthropic.com` 的 Anthropic provider（`cli_key = "claude"`）
- 通过 OneAPI 等转发服务暴露的 OpenAI 兼容 endpoint，cli_key 被映射为 `codex`，使用 API key 鉴权

**不行：**

- OAuth 登录态的 Codex provider（`auth_mode = "oauth"` → 被 `model.invoke` 拒绝）
- 通过 bridge 暴露的 provider（`source_provider_id` 或 `bridge_type` 非空 → `bridged_provider_unsupported`）
- Provider 的 CLI key 是 `gemini`、`openai` 或其他非 `claude`/`codex` 的值（→ `provider_cli_unsupported`）
- API key 未填写（→ `provider_api_key_missing`）
- base URL 为空（→ `provider_base_url_missing`）

## 模型映射的影响

AIO Coding Hub 的 provider 模型映射功能（如 `claude_models` 别名、`cx2cc` bridge）**不影响** association audit。

`model.invoke` 绕过了网关的主请求路由层，直接把用户配置的 `model` 字符串作为请求体中的 `"model"` 字段发送给 provider。用户在插件配置里填的模型名就是最终发给 provider 的模型名，需要填 provider 能直接识别的原始名称，别名不会生效。

## 边界与限制

- 不阻断、重试、改写或路由任何请求 -- 纯被动顾问
- 不暴露 provider credentials 给插件代码 -- 由宿主通过 `model.invoke` 代管
- 不把结果写入旧的 `association_audit_json` 专用字段 -- 全部走 `plugin_audit_logs`
- 不替代隐私过滤 -- 敏感字段脱敏由 Privacy Filter 或其他 redaction 插件在更早边界处理
- 审计调用在后台异步执行，不阻塞主请求 -- 如果网关关闭前审计未完成，该 trace 不会产生审计记录

## 常见排障

**"安装了插件但请求日志里没有审计记录"**

- 检查插件状态是否为 Enabled
- 检查 `mode` 是否设为 `off`（off 时不执行任何操作）
- 如果是 `sampled` 模式，只有 `sampleRate` 比例的 trace 会被选中
- 如果是 `prefiltered` 模式，本地过滤可能跳过了所有 trace

**"每条 trace 都是 not_configured"**

- 检查 `providerId` 是否为正数
- 检查 `model` 是否非空
- 检查对应 provider 是否满足 [Provider 选择条件](#provider-选择条件) 的全部六条

**"大量 failed 记录"**

- 查看 `details.error` 确定失败原因
- 常见原因：provider 不可达（`MODEL_INVOKE_REQUEST_FAILED`）、超时（`MODEL_INVOKE_TIMEOUT`）、凭据过期（`MODEL_INVOKE_PROVIDER_CREDENTIAL_FAILED`）

**"审计结果看起来不对"**

- 审计判断由配置的模型做出，结果质量取决于该模型的能力
- 尝试更换更大能力的模型（如从 haiku 换成 sonnet）
- 尝试调整 `maxInputChars` / `maxOutputChars`，确保输入/输出没有被过度截断

**"provider 填了但提示 unsupported"**

- 确认 provider 的 CLI key 是 `claude` 或 `codex`
- 确认 provider 不是 OAuth 也不是桥接 provider
- 如果是 Gemini 等第三方 provider，目前不被支持
