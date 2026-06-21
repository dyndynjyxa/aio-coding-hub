# 插件发布

插件包格式是 `.aio-plugin`。它本质上是一个 zip archive，`plugin.json` 必须位于压缩包根目录，或唯一顶层目录内。

发布检查清单：

- 校验 `plugin.json`。
- 控制 package size 和 entry count。
- 对 package bytes 计算 `sha256`。
- 通过可信 index 发布时，用 Ed25519 签名 release metadata。
- 对 breaking update 写清 rollback 说明。

当前实现支持本地/离线包导入、受约束的远程 `.aio-plugin` 下载、checksum/signature verification、更新时的 permission delta 检查、已撤销插件 quarantine，以及 rollback snapshots。

远程包安装刻意保持窄能力：

- 下载 URL 必须是无凭据的 `https://` 或 `file://`。
- artifact path 必须以 `.aio-plugin` 结尾。
- 包在解压前会受到大小限制。
- remote 和 GitHub release install 必须提供 checksum。
- 如果同时提供 signature 和 trusted public key，宿主会校验 Ed25519 signature。

开发者工具输出 base64 编码的 Ed25519 signature。Public key 是原始 32-byte Ed25519 public key 的 base64 编码，和宿主 verifier 输入保持一致。
