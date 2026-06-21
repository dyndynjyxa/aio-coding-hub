# 插件 Manifest

插件 manifest 文件名是 `plugin.json`，遵循 [Manifest v1 完整规范](../../plugin-manifest-v1.md)。

必填字段：

- `id`：带发布者命名空间的 ID，例如 `publisher.plugin-name`。
- `name`：展示给用户的插件名称。
- `version`：使用 SemVer 的插件版本。
- `apiVersion`：使用 SemVer 的插件 API 版本。
- `runtime`：社区插件优先使用 `declarativeRules`；`wasm` 受宿主策略控制；`native` 仅保留给内置官方插件。
- `hooks`：插件注册的 hook 声明。
- `permissions`：插件请求的权限名称。
- `hostCompatibility`：应用版本和插件 API 兼容性约束。

`official.*` 命名空间只保留给内置官方插件。本地包、marketplace 包和 GitHub 包必须使用自己的发布者命名空间。

运行时示例：

```json
{ "kind": "declarativeRules", "rules": ["rules/main.json"] }
```

```json
{ "kind": "wasm", "abiVersion": "1.0.0", "memoryLimitBytes": 16777216 }
```

仅官方内置插件可用的 native 运行时示例：

```json
{ "kind": "native", "engine": "privacyFilter" }
```

只有从官方源安装的内置官方插件可以使用 `native`。

`hostCompatibility` 必须包含 `app` 和 `pluginApi`；`platforms` 可以限制支持的操作系统。

`configSchema` 可以包含标准 JSON Schema 展示字段和 AIO `x-aio-ui` 元数据。详见 [Config Schema](./config-schema.md)。

plugin API v1 的 active hooks 见 [Hooks](./hooks.md)。Reserved hooks 和 reserved permissions 只是为未来兼容命名保留；在宿主真正实现前，manifest 校验会拒绝它们。
