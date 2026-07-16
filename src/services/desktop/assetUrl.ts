// Usage: Tauri asset 协议 URL 适配器：本地文件绝对路径 → webview 可加载的 asset URL
// （scope 由 Rust 在启动/更改存储目录时动态授权）。原始 @tauri-apps/api 导入仅允许
// 存在于专用适配器（见 desktopBridge.contract.test.ts）。

export { convertFileSrc as convertDesktopFileSrc } from "@tauri-apps/api/core";
