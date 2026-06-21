# 插件安全与隔离

插件系统围绕最小权限和运行时隔离设计。默认 vNext hook timeout: 150 ms。

核心规则：

- no arbitrary JavaScript：任意 JavaScript 不会在 Rust 主进程中执行。
- no arbitrary JavaScript：任意 JavaScript 不会在 Tauri WebView 中执行。
- WASM 不提供 WASI filesystem 或 network imports。
- Process runtime PoC 默认关闭。
- Hook 失败必须记录审计事件。
- 高风险 hook 可以使用 fail-closed 策略。
- 重复 runtime failure 可以让插件进入 `quarantined` 状态。

未签名离线包会受到限制。除非未来明确的可信策略允许，否则 high 和 critical 权限会被拒绝。
