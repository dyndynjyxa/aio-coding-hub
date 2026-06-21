# 插件 API 参考

这里放插件作者需要查的稳定契约。日常开发先读 [插件开发总指南](../developer-guide.md)，遇到字段、hook、权限或打包规则不确定时，再回到本目录查询。

## 必读契约

- [Manifest](./manifest.md)：`plugin.json` 必填字段、运行时声明、命名空间和兼容性入口。
- [Hooks](./hooks.md)：网关与日志 hook 的触发时机、上下文字段、超时和可修改字段。
- [Permissions](./permissions.md)：权限名称、风险等级、授权和重新授权规则。
- [Config Schema](./config-schema.md)：配置表单 schema、`x-aio-ui` 和低代码渲染规则。
- [Declarative Rules](./declarative-rules.md)：规则文件结构、target、action、条件和运行时限制。

## 工具与发布

- [SDK](./sdk.md)：`@aio-coding-hub/plugin-sdk` 和 `aio-plugin-wasm-sdk` 的边界与示例。
- [Publishing](./publishing.md)：`.aio-plugin`、`sha256`、Ed25519 签名、远程安装和 rollback。
- [Compatibility](./compatibility.md)：SemVer、`pluginApi`、platforms 和 WASM ABI 兼容规则。

## 规范来源

- [Manifest v1 完整规范](../../plugin-manifest-v1.md)：规范性 manifest 文档。
- [plugin-api-v1-contract.json](../plugin-api-v1-contract.json)：hook、permission 和 runtime 的机器可读契约。
