# 插件 Manifest v1

`plugin.json` 是插件与 AIO Coding Hub 之间稳定的 package contract。Manifest v1 优先支持声明式规则插件；当 host policy 启用时支持 WASM code plugins；同时保留少量 official-only native engines。

## 1. 必填字段

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `id` | string | 全局唯一插件 ID。 |
| `name` | string | 展示给用户的名称。 |
| `version` | string | 插件版本，使用 SemVer。 |
| `apiVersion` | string | 插件 API 版本，例如 `1.0.0`。 |
| `runtime` | object | 运行时声明。 |
| `hooks` | array | Hook 注册信息。 |
| `permissions` | array | 请求的权限。 |
| `hostCompatibility` | object | 支持的 AIO Coding Hub 宿主版本范围。 |

## 2. 可选字段

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `entry` | string | 运行时 artifact path，例如 `plugin.wasm`；声明式规则不需要该字段。 |
| `configSchema` | object | 用于用户配置的 JSON Schema subset。 |
| `configVersion` | integer | 配置 schema 版本。 |
| `description` | string | 展示给用户的简短摘要。 |
| `author` | string or object | 作者元数据。 |
| `homepage` | string | 项目主页 URL。 |
| `repository` | string or object | 源码仓库元数据。 |
| `license` | string | 尽量使用 SPDX license expression。 |
| `checksum` | string | Package checksum。 |
| `signature` | string | Package signature。 |
| `category` | string | `security`、`productivity`、`redaction` 或 `utility`。 |

## 3. ID 与版本规则

Plugin IDs 使用 `publisher.plugin-name` 格式。

- publisher 和 name segment 必须是 lowercase ASCII。
- 每个 segment 可以包含字母、数字和 hyphen。
- 使用 dots 分隔 namespace segments。
- Path separators、`..`、spaces、shell metacharacters 和 empty segments 都是非法的。
- `official.privacy-filter` 是唯一 bundled official plugin ID。
- `official.*` namespace 只能通过 built-in official plugin source 安装；local、marketplace 和 GitHub packages 必须使用自己的 publisher namespace。

Versions 必须遵循 SemVer。Pre-release versions 可用于本地开发和 unsigned packages；marketplace stable releases 应使用 release versions。

`apiVersion` 独立于 app version。宿主可以在同一 major API 内添加 backward-compatible fields。Breaking changes 需要新的 major API。

## 4. Runtime

Runtime v1 支持社区声明式规则：

```json
{
  "kind": "declarativeRules",
  "rules": ["rules/main.json"]
}
```

WASM runtime：

```json
{
  "kind": "wasm",
  "abiVersion": "1.0.0",
  "memoryLimitBytes": 16777216
}
```

WASM packages are installable only when host policy enables execution。未启用 WASM execution 的宿主必须拒绝或禁用 WASM plugins，不能把它们路由到其他 runtime。

短期 validation 必须拒绝 arbitrary JavaScript/TypeScript、Node.js、Deno、native dynamic libraries 和 WebView code。

Official-only native runtime：

```json
{
  "kind": "native",
  "engine": "privacyFilter"
}
```

`native` 只保留给从 built-in official source 安装的 built-in official plugins。第三方包不能声明 host-native engines。

## 5. Host Compatibility

`hostCompatibility` 约束插件安装和启用：

```json
{
  "app": ">=0.56.0 <1.0.0",
  "pluginApi": "^1.0.0",
  "platforms": ["macos", "windows", "linux"]
}
```

不兼容插件会被标记为 `incompatible`，不会进入 hook pipeline。

## 6. Hook v1

Active hooks in plugin API v1 是当前已经接入 gateway 或 log pipeline 的 hooks。Reserved hooks for future host integration 会被记录下来以稳定命名；但在宿主实现对应调用点前，manifest validation 会用 `PLUGIN_RESERVED_HOOK` 拒绝它们。

| Hook | 触发时机 | 可修改内容 | 默认超时 | 默认失败策略 | 匹配权限 |
| --- | --- | --- | --- | --- | --- |
| `gateway.request.afterBodyRead` | Body reader 完成 allowed body buffering 后 | JSON body、raw body metadata | 200 ms | fail-open | `request.body.read`, `request.body.write` |
| `gateway.request.beforeSend` | reqwest 发送 upstream request 前 | headers 和 body | 300 ms | fail-open 或 security fail-closed | `request.header.write`, `request.body.write` |
| `gateway.response.chunk` | CLI output 前的 stream chunk | chunk pass、replace、block、warn | 20 ms | security fail-closed、non-security fail-open | `stream.inspect`, `stream.modify` |
| `gateway.response.after` | 大小预算内的完整 non-stream response | body pass、replace、block、warn | 300 ms | security fail-closed、non-security fail-open | `response.body.read`, `response.body.write` |
| `gateway.error` | 观察到 host 或 upstream error 后 | 不隐藏 host error | 100 ms | fail-open | `request.meta.read` |
| `log.beforePersist` | Request 或 audit log 持久化前 | redacted log fields | 100 ms | fail-closed-to-host-redaction | `log.redact` |

Streaming hooks 接收 bounded chunks 和固定大小 sliding window，不会接收无限制完整响应。

Reserved hooks：

- `gateway.request.received`
- `gateway.request.beforeProviderResolution`
- `gateway.response.headers`

## 7. Permission v1

Reserved permissions for future host-mediated APIs 会被记录下来以稳定命名；但在这些 API 存在前，manifest validation 会用 `PLUGIN_RESERVED_PERMISSION` 拒绝它们。

| Permission | Risk | 说明 |
| --- | --- | --- |
| `request.meta.read` | low | 读取 method、path、CLI key、trace ID、provider hints。 |
| `request.header.read` | medium | 读取非敏感 request headers。 |
| `request.header.readSensitive` | high | 读取 `Authorization` 和 `Cookie` 等 sensitive request headers。 |
| `request.header.write` | high | 修改 request headers。 |
| `request.body.read` | high | 读取 request body。 |
| `request.body.write` | high | 修改 request body。 |
| `response.header.read` | low | 读取 response headers。 |
| `response.header.write` | medium | 修改返回给 CLI 的 safe response headers。 |
| `response.body.read` | high | 在预算内读取完整 non-stream response body。 |
| `response.body.write` | high | 修改 non-stream response body。 |
| `stream.inspect` | high | 检查 streamed chunks 和 sliding window。 |
| `stream.modify` | high | 替换或阻断 streamed chunks。 |
| `log.redact` | medium | 持久化前脱敏 log fields。 |

Reserved permissions：

| Permission | Risk | Future host-mediated API |
| --- | --- | --- |
| `plugin.storage` | medium | 使用隔离 plugin storage。 |
| `network.fetch` | high | 发起 host-mediated network requests。 |
| `file.read` | high | 读取 host-mediated files。 |
| `file.write` | high | 写入 host-mediated files。 |
| `secret.read` | critical | 读取 host-managed secrets。 |

高危权限需要二次授权。Critical permissions require second confirmation and stronger UI copy。

插件升级新增权限必须重新授权。The host must keep the plugin disabled or partially disabled until the new permissions are approved。

## 8. Hook 与 Permission 兼容性

Validation 会拒绝：

- Unknown hook names。
- Reserved hook names。
- Unknown permissions。
- Reserved permissions。
- 为不能修改的 hooks 请求 write permissions。
- 没有 `request.header.readSensitive` 却读取 sensitive header。
- 没有匹配 body read/write permission 却写 body。
- 没有 `stream.modify` 却执行 `stream.modify` actions。
- 在 host 提供对应 API 前请求 `network.fetch`、`file.read`、`file.write` 或 `secret.read`。

## 9. Config Schema 子集

受支持的 `configSchema` subset 包括：

- `string`
- `number`
- `integer`
- `boolean`
- `enum`
- `array`
- `object`
- `password`

插件不能提供 custom GUI code。宿主负责渲染表单、保存前校验，并在启用前再次校验。Sensitive values 不会以 plaintext 返回前端。

## 10. 状态机

状态：

- `available`
- `installed`
- `enabled`
- `disabled`
- `update_available`
- `incompatible`
- `quarantined`
- `uninstalled`

允许的状态转换：

| From | To | Trigger |
| --- | --- | --- |
| `available` | `installed` | 用户安装 package 或 market plugin。 |
| `installed` | `enabled` | 用户授权 required permissions 且配置有效。 |
| `installed` | `disabled` | 用户安装但不启用。 |
| `enabled` | `disabled` | 用户禁用插件。 |
| `disabled` | `enabled` | 用户在校验通过后启用插件。 |
| `enabled` | `update_available` | Market 发现新的兼容版本。 |
| `disabled` | `update_available` | Market 发现新的兼容版本。 |
| `update_available` | `enabled` | 更新成功且 permissions 仍有效。 |
| `update_available` | `disabled` | 更新成功但需要新的 permission approval。 |
| `installed` | `incompatible` | Host/API/platform version 不兼容。 |
| `enabled` | `quarantined` | 重复 crash、timeout、signature failure 或 revoked market status。 |
| `disabled` | `quarantined` | Signature failure 或 revoked market status。 |
| `quarantined` | `disabled` | 用户确认并在校验后恢复。 |
| any active state | `uninstalled` | 用户卸载插件。 |

Upgrade failure 会恢复 previous version、config snapshot、permissions 和 enabled state。Signature failure 会让插件进入 `quarantined`。Runtime crash 和 repeated timeout 可以让 enabled plugin 进入 `quarantined`。

## 11. Manifest 示例：社区 Prompt Helper

```json
{
  "id": "acme.prompt-helper",
  "name": "Prompt Helper",
  "version": "1.0.0",
  "apiVersion": "1.0.0",
  "runtime": {
    "kind": "declarativeRules",
    "rules": ["rules/main.json"]
  },
  "hooks": [
    {
      "name": "gateway.request.afterBodyRead",
      "priority": 100,
      "failurePolicy": "fail-open"
    }
  ],
  "permissions": ["request.body.read", "request.body.write"],
  "hostCompatibility": {
    "app": ">=0.56.0 <1.0.0",
    "pluginApi": "^1.0.0",
    "platforms": ["macos", "windows", "linux"]
  },
  "configSchema": {
    "type": "object",
    "required": ["mode"],
    "properties": {
      "mode": {
        "type": "string",
        "enum": ["append_instruction", "prepend_context"]
      },
      "onlyModels": {
        "type": "array",
        "items": { "type": "string" }
      },
      "onlyClis": {
        "type": "array",
        "items": { "type": "string", "enum": ["claude", "codex", "gemini"] }
      }
    }
  }
}
```

## 12. Manifest 示例：Privacy Filter

```json
{
  "id": "official.privacy-filter",
  "name": "Privacy Filter",
  "version": "1.0.0",
  "apiVersion": "1.0.0",
  "category": "privacy",
  "description": "Official native privacy filter aligned with packyme/privacy-filter for pre-upstream prompt and log redaction.",
  "homepage": "https://github.com/packyme/privacy-filter",
  "repository": {
    "type": "git",
    "url": "https://github.com/packyme/privacy-filter.git"
  },
  "license": "MIT",
  "runtime": {
    "kind": "native",
    "engine": "privacyFilter"
  },
  "hooks": [
    {
      "name": "gateway.request.afterBodyRead",
      "priority": 5,
      "failurePolicy": "fail-closed"
    },
    {
      "name": "log.beforePersist",
      "priority": 1,
      "failurePolicy": "fail-closed"
    }
  ],
  "permissions": ["request.body.read", "request.body.write", "log.redact"],
  "hostCompatibility": {
    "app": ">=0.56.0 <1.0.0",
    "pluginApi": "^1.0.0",
    "platforms": ["macos", "windows", "linux"]
  },
  "configSchema": {
    "type": "object",
    "required": ["redactBeforeUpstream", "redactLogs", "profile"],
    "properties": {
      "redactBeforeUpstream": {
        "type": "boolean"
      },
      "redactLogs": {
        "type": "boolean"
      },
      "profile": {
        "type": "string",
        "enum": ["balanced"]
      }
    }
  }
}
```
