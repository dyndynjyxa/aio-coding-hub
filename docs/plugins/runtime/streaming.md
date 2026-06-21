# 流式响应插件

Streaming plugins 使用 `gateway.response.chunk`。

运行时会提供：

- 当前 chunk 的 bytes 或 text。
- 用于跨 chunk 检测的有界 sliding window。
- trace metadata。
- 已按权限裁剪的 context。

没有 `stream.inspect` 时，插件不能读取 stream 内容。没有 `stream.modify` 时，插件不能替换或阻断 chunk。

流式插件不能假设自己能看到完整响应。它们应只检测有界模式，并根据已授权 permissions 返回 pass、warn、replace 或 block。
