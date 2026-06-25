# Association Audit 官方插件

`official.association-audit` 是默认关闭的 bundled official plugin。它把 PR 级被动关联审计能力迁移到官方插件边界内：通过插件配置选择 provider/model，经 `model.invoke` 发起宿主托管的有界模型调用，并把结果写入 `plugin_audit_logs`。

## Manifest

ID: `official.association-audit`

Runtime: `native:associationAudit`

Hooks：

- `gateway.request.afterBodyRead`
- `gateway.response.chunk`
- `gateway.response.after`
- `gateway.error`
- `log.beforePersist`

Permissions：

- `request.meta.read`
- `request.body.read`
- `response.body.read`
- `stream.inspect`
- `log.redact`
- `model.invoke`

## 配置

默认配置保持关闭：

```json
{
  "mode": "off",
  "providerId": null,
  "model": "",
  "sampleRate": 10,
  "timeoutSeconds": 8,
  "maxInputChars": 6000,
  "maxOutputChars": 12000
}
```

`mode` 支持：

- `off`：不执行审计。
- `sampled`：按 `sampleRate` 对 trace 做稳定采样。
- `prefiltered`：先用本地启发式过滤，只有截断、可疑输出或低 token overlap 时才调用模型。
- `all`：对有响应内容的 trace 都尝试审计。

`providerId` 和 `model` 都必须配置后才会调用模型。插件不读取 API key；宿主用 `model.invoke` 负责 provider credential lookup、超时、请求/响应大小上限和错误归一化。

## 记录位置

插件不会新增 request log 专用列，也不会修改主响应。审计结果通过现有插件审计日志表写入，并在 request log detail 的 `plugin_audit_logs` 中按 trace 展示。

常见 event type：

- `association.audit.completed`
- `association.audit.skipped`
- `association.audit.not_configured`
- `association.audit.failed`
- `association.audit.invalid_response`

`details.result` 保存规范化结果，包括 `association_score`、`overall_risk`、`signals`、`insufficient_context` 和 `notes`。输入、输出和 evidence 会在入审计包和持久化前做基础 secret redaction。

## 边界

Association Audit 是被动审计插件：

- 不阻断、重试、改写或路由请求。
- 不暴露 provider credentials 给插件代码。
- 不把结果写入旧的 `association_audit_json` 专用字段。
- 不替代隐私过滤；敏感字段仍应由 Privacy Filter 或其他 redaction 插件在更早边界处理。

## 代码位置

```text
src-tauri/resources/plugins/official/association-audit/
src-tauri/src/app/plugins/association_audit.rs
```
