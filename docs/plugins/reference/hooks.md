# 插件 Hooks

Hooks 是网关和日志 pipeline 中稳定的扩展点。Plugin API v1 刻意保持 active surface 小而明确，让社区插件能清楚判断调用时机、权限边界和 mutation 行为。

默认 vNext hook timeout: 150 ms.
默认 vNext failure policy: `fail-open`.

Reserved hooks 在宿主实现对应调用点前，会被 manifest validation 拒绝：

- `gateway.request.received`
- `gateway.request.beforeProviderResolution`
- `gateway.response.headers`

## Hook 矩阵

| Hook | 阶段 | 读权限 | 写权限 | Mutation fields | Context fields |
| --- | --- | --- | --- | --- | --- |
| `gateway.request.afterBodyRead` | 读取 request body 后、发送 upstream provider 前。 | `request.meta.read`, `request.header.read`, `request.header.readSensitive`, `request.body.read` | `request.header.write`, `request.body.write` | `headers`, `requestBody` | `traceId`, `request.cliKey`, `request.method`, `request.path`, `request.query`, `request.headers`, `request.body`, `request.requestedModel`, `request.normalizedMessages` |
| `gateway.request.beforeSend` | provider resolution 后、发送 upstream provider 前。 | `request.meta.read`, `request.header.read`, `request.header.readSensitive`, `request.body.read` | `request.header.write`, `request.body.write` | `headers`, `requestBody` | `traceId`, `request.cliKey`, `request.method`, `request.path`, `request.query`, `request.headers`, `request.body`, `request.requestedModel`, `request.normalizedMessages` |
| `gateway.response.chunk` | 每个有边界的 streaming response chunk。 | `stream.inspect` | `stream.modify` | `streamChunk` | `traceId`, `stream.sequence`, `stream.chunk` |
| `gateway.response.after` | 完整 non-streaming upstream response body 可用后。 | `response.header.read`, `response.body.read` | `response.header.write`, `response.body.write` | `headers`, `responseBody` | `traceId`, `response.status`, `response.headers`, `response.body` |
| `gateway.error` | gateway error response materialization 后、发送前。 | `response.header.read`, `response.body.read` | `response.header.write`, `response.body.write` | `headers`, `responseBody` | `traceId`, `response.status`, `response.headers`, `response.body` |
| `log.beforePersist` | gateway request log persistence 前。 | `log.redact` | `log.redact` | `logMessage` | `traceId`, `log.message` |

## gateway.request.afterBodyRead

- 阶段：读取 request body 后、发送 upstream provider 前。
- 默认超时：150 ms。
- 默认失败策略：`fail-open`。
- 读权限：`request.meta.read`、`request.header.read`、`request.header.readSensitive`、`request.body.read`。
- 写权限：`request.header.write`、`request.body.write`。
- Mutation fields：`headers`、`requestBody`。
- Provider-neutral field：`request.normalizedMessages`。

这个 hook 适合 prompt optimization、privacy filtering 和 request-body checks。只有插件拥有 `request.body.read` 时，宿主才会提供 `request.body` 和 `request.normalizedMessages`。

Claude-style fixture：

```json
{
  "messages": [
    {
      "role": "user",
      "content": [{ "type": "text", "text": "hello claude" }]
    }
  ]
}
```

Codex/OpenAI Responses-style fixture：

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

这两种结构的 normalized context 都会包含类似条目：

```json
{
  "request": {
    "normalizedMessages": [
      { "role": "user", "text": "hello codex", "source": "openai.responses.input_text" }
    ]
  }
}
```

## gateway.request.beforeSend

- 阶段：provider resolution 后、发送 upstream provider 前。
- 默认超时：150 ms。
- 默认失败策略：`fail-open`。
- 读权限：`request.meta.read`、`request.header.read`、`request.header.readSensitive`、`request.body.read`。
- 写权限：`request.header.write`、`request.body.write`。
- Mutation fields：`headers`、`requestBody`。
- Provider-neutral field：`request.normalizedMessages`。

它在当前 attempt 的 provider selection、auth/header preparation、request body sanitizers 和 protocol rectifiers 后执行，紧贴 gateway 向 upstream provider 发送 bytes 前。插件必须保证最终 upstream request-body 或 request-header mutation 时，使用这个 hook。

这个 hook 看到的是 semantic decoded request body content。如果插件修改 body，gateway 会更新最终 upstream body，并按需要移除或重新计算 wire-level length/encoding 语义。未改变的请求会尽量保留原始 passthrough body。

## gateway.response.chunk

- 阶段：每个有边界的 streaming response chunk。
- 默认超时：150 ms。
- 默认失败策略：`fail-open`。
- 读权限：`stream.inspect`。
- 写权限：`stream.modify`。
- Mutation fields：`streamChunk`。
- Context fields：`traceId`、`stream.sequence`、`stream.chunk`。

这个 hook 接收有边界的 streaming chunks，而不是完整响应。需要完整 response bodies 的插件，应在 non-streaming requests 中使用 `gateway.response.after`。

## gateway.response.after

- 阶段：完整 non-streaming upstream response body 可用后。
- 默认超时：150 ms。
- 默认失败策略：`fail-open`。
- 读权限：`response.header.read`、`response.body.read`。
- 写权限：`response.header.write`、`response.body.write`。
- Mutation fields：`headers`、`responseBody`。
- Context fields：`traceId`、`response.status`、`response.headers`、`response.body`。

这个 hook 适合 non-streaming response redaction、warnings 或 response blocking。

## gateway.error

- 阶段：gateway error response materialization 后、发送前。
- 默认超时：150 ms。
- 默认失败策略：`fail-open`。
- 读权限：`response.header.read`、`response.body.read`。
- 写权限：`response.header.write`、`response.body.write`。
- Mutation fields：`headers`、`responseBody`。
- Context fields：`traceId`、`response.status`、`response.headers`、`response.body`。

这个 hook 用于脱敏或改写 gateway-generated error responses，不应处理 provider success responses。

## log.beforePersist

- 阶段：gateway request log persistence 前。
- 默认超时：150 ms。
- 默认失败策略：`fail-open`。
- 读权限：`log.redact`。
- 写权限：`log.redact`。
- Mutation fields：`logMessage`。
- Context fields：`traceId`、`log.message`。

这个 hook 用于 request logs 入队或写入前的不可逆脱敏。
