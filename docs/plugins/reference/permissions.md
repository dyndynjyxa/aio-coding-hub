# 插件权限

Permissions 必须显式声明，并按风险分级。宿主在调用插件前会按权限裁剪 hook context；插件返回未授权写入时，宿主会拒绝该 mutation。

常用权限：

- `request.meta.read`：低风险，读取方法、路径、CLI key、trace ID 等元信息。
- `request.header.read`：中风险，读取非敏感请求头。
- `request.header.readSensitive`：高风险，读取 `Authorization`、`Cookie` 等敏感请求头。
- `request.header.write`：高风险，修改请求头。
- `request.body.read`：高风险，读取请求体。
- `request.body.write`：高风险，修改请求体。
- `response.header.read`：低风险，读取响应头。
- `response.header.write`：中风险，修改返回给 CLI 的安全响应头。
- `response.body.read`：高风险，读取有大小预算保护的完整非流式响应体。
- `response.body.write`：高风险，修改非流式响应体。
- `stream.inspect`：高风险，读取流式响应 chunk 和 sliding window。
- `stream.modify`：高风险，替换或阻断流式响应 chunk。
- `log.redact`：中风险，在日志持久化前脱敏。

为未来 host-mediated APIs 保留的权限：

- `plugin.storage`：中风险，使用隔离插件存储。
- `network.fetch`：高风险，通过宿主代理发起网络请求。
- `file.read`：高风险，通过宿主代理读取文件。
- `file.write`：高风险，通过宿主代理写入文件。
- `secret.read`：critical 风险，读取宿主管理的密钥。

Reserved permissions 在宿主实现对应 API 前会被 manifest 校验拒绝。

高风险和 critical 权限需要清晰的用户授权文案。插件升级新增权限时，必须重新授权，插件才能带着这些新能力启用。
