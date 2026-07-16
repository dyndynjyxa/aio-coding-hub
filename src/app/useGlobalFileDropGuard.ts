// Usage: tauri.conf.json 关闭 dragDropEnabled 后整个 webview 接收标准 HTML5 拖放事件；
// window 级兜底 preventDefault，防止文件拖到非落区时 webview 导航离开应用。App 根挂载一次。

import { useEffect } from "react";

export function useGlobalFileDropGuard() {
  useEffect(() => {
    const prevent = (event: DragEvent) => {
      event.preventDefault();
    };
    window.addEventListener("dragover", prevent);
    window.addEventListener("drop", prevent);
    return () => {
      window.removeEventListener("dragover", prevent);
      window.removeEventListener("drop", prevent);
    };
  }, []);
}
