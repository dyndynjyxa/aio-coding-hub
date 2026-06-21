# 插件兼容性

插件兼容性使用 SemVer 描述。宿主安装、启用和升级插件时，会同时检查插件版本、插件 API 版本、应用版本、平台和运行时 ABI。

Manifest 中的关键字段：

- `version`：插件自身发布版本。
- `apiVersion`：该 manifest 使用的插件 API 版本。
- `hostCompatibility.app`：兼容的 AIO Coding Hub 应用版本范围。
- `hostCompatibility.pluginApi`：兼容的 pluginApi 版本范围。
- `hostCompatibility.platforms`：可选的平台 allowlist。

WASM 插件还需要声明 WASM ABI 版本：

```json
{ "kind": "wasm", "abiVersion": "1.0.0" }
```

宿主会拒绝不支持的主版本。未来插件 API 变更必须保持向后兼容；无法兼容时，需要提升主版本并让旧插件继续按旧契约运行或被明确标记为不兼容。
