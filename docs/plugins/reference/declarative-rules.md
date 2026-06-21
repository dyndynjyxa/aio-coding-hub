# 声明式规则运行时

`declarativeRules` 是社区插件当前优先使用的运行时。它允许插件在不向宿主注入任意代码的前提下，检查和转换 request bodies、response bodies、stream chunks 和 log messages。

## Manifest 运行时声明

在 `plugin.json` 中声明规则文件：

```json
{
  "runtime": {
    "kind": "declarativeRules",
    "rules": ["rules/main.json"]
  }
}
```

规则路径相对于插件根目录。路径不能包含 `..`，也不能使用绝对路径前缀。

## 规则文件结构

```json
{
  "rules": [
    {
      "id": "redact-api-key",
      "hook": "gateway.request.afterBodyRead",
      "target": {
        "field": "request.body",
        "jsonPath": "$.messages[*].content"
      },
      "match": {
        "regex": "sk-[A-Za-z0-9_-]{20,}",
        "caseSensitive": true
      },
      "action": {
        "kind": "replace",
        "replacement": "[REDACTED]"
      }
    }
  ]
}
```

每条规则包含：

- `id`：稳定的规则标识，用于诊断。
- `hook`：[Hooks](./hooks.md) 中的一个 hook 名。
- `target`：规则扫描的位置。
- `match.regex`：Rust `regex` pattern。
- `match.caseSensitive`：可选，默认 `true`。
- `action`：命中后的动作。
- `when`：可选的运行时过滤条件。

## Targets 目标字段

支持的 `target.field` 值：

- `request.body`
- `response.body`
- `stream.chunk`
- `log.message`

`request.body` 和 `response.body` 可以通过 `jsonPath` 指向 JSON payload 内部的字符串字段。

支持的 JSONPath 子集：

- `$`
- `.key`
- `[*]`

示例：

- `$.input`
- `$.prompt`
- `$.messages[*].content`
- `$.choices[*].message.content`

暂不支持 quoted keys、filters、recursive descent、numeric indexes 和任意 JSONPath 表达式。

## Actions 动作

### replace

替换所选文本中的所有 regex 命中。

```json
{
  "kind": "replace",
  "replacement": "[REDACTED]"
}
```

Capture groups 使用 Rust regex replacement syntax：

```json
{
  "kind": "replace",
  "replacement": "$1[SECRET]"
}
```

### block

当 pipeline 和 hook 允许阻断时，停止当前 request、response 或 stream processing。

```json
{
  "kind": "block",
  "reason": "Dangerous output blocked by plugin."
}
```

### warn

记录 warning reason，但不修改 target。

```json
{
  "kind": "warn",
  "message": "Suspicious content detected."
}
```

### appendMessage

向 chat-style request bodies 追加 `system` 或 `developer` message。

```json
{
  "kind": "appendMessage",
  "role": "system",
  "content": "Clarify intent and preserve user constraints."
}
```

## 条件规则

使用 `when` 根据 CLI、model 或 config value 限制规则生效范围：

```json
{
  "when": {
    "cliKeys": ["codex", "claude"],
    "models": ["gpt-4.1"],
    "configEquals": {
      "redactBeforeUpstream": true
    }
  }
}
```

提供的条件必须全部匹配。

## 权限

规则仍然需要 manifest 中声明匹配的 permissions：

- 读取 request body：`request.body.read`
- 修改 request body：`request.body.write`
- 读取 response body：`response.body.read`
- 修改 response body：`response.body.write`
- 读取 stream chunks：`stream.inspect`
- 修改 stream chunks：`stream.modify`
- 日志脱敏：`log.redact`

宿主会在规则执行前裁剪 hook context，并在规则执行后拒绝未授权 mutation。

## 运行时限制

- 最大 regex pattern length：4 KiB。
- 最大 compiled regex size：2 MiB。
- 每个 runtime 最多规则数：256。
- Hook execution 受 gateway plugin timeout 约束。
- 当 target 无法解析为 JSON syntax 时，会跳过 invalid JSON targets。

## 本地 Replay 兼容性

`create-aio-plugin replay` 为本地 fixtures 实现宿主支持的 v1.1 declarative rule subset。它刻意保持确定性，不执行 WASM、process plugins、network calls 或 host-only native engines。

Replay 支持社区规则运行时相同的 v1.1 rule actions：`replace`、`block`、`warn` 和 `appendMessage`。对 request body rewrites，它支持 raw text targets，也支持文档化 JSONPath 子集，例如 `$.messages[*].content`、`$.input[*].content[*].text` 和 `$.input`。

## 适合场景

- 通过追加 instructions 做 prompt optimization。
- API key、token、email 和 log redaction。
- 阻断已知危险命令模式的 safety checks。
- 轻量 response warnings。

## 不适合场景

当插件需要以下能力时，应使用 WASM 或未来的隔离进程运行时：

- entropy scoring。
- Luhn validation。
- external API calls。
- model-based classification。
- filesystem access。
- complex stateful analysis。
