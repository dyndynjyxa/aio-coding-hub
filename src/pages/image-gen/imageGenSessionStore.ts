// Usage: 生图会话模块级 store（页面私有）。路由懒加载会卸载页面组件，会话与 objectURL
// 生命周期因此提升到应用会话级；controller 经 useSyncExternalStore 订阅读取。

import { emitListenerSnapshot } from "../../utils/listeners";
import type { ImageGenReferenceImage, ImageGenTask } from "./useImageGenController";

export type ImageGenSessionState = {
  /** 追加序（创建时间序）；展示层经 filterTasks 反转为"新的在前"。 */
  tasks: ImageGenTask[];
  referenceImages: ImageGenReferenceImage[];
  prompt: string;
  /** 是否已从 DB 拉取过首屏历史（模块级记忆，跨路由挂载只拉一次）。 */
  hydrated: boolean;
  /** DB 中是否还有更早的历史任务（上次拉取返回满页）。 */
  hasMore: boolean;
};

type Listener = () => void;

const EMPTY_STATE: ImageGenSessionState = {
  tasks: [],
  referenceImages: [],
  prompt: "",
  hydrated: false,
  hasMore: false,
};

let snapshot: ImageGenSessionState = EMPTY_STATE;
const listeners = new Set<Listener>();

export function getImageGenSession(): ImageGenSessionState {
  return snapshot;
}

export function subscribeImageGenSession(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** 同步函数式更新：并发完成回调各自基于最新快照合并，不互相覆盖。 */
export function updateImageGenSession(
  updater: (prev: ImageGenSessionState) => ImageGenSessionState
) {
  snapshot = updater(snapshot);
  emitListenerSnapshot(listeners, (listener) => listener());
}

// objectURL 生命周期为应用会话级；落盘成功的任务切换为 asset 协议路径并释放 URL，
// 内存驻留仅剩 memory 形态（生成中 / 落盘失败）任务与输入区参考图。
const trackedUrls = new Set<string>();

export function trackImageGenObjectUrl(blob: Blob): string {
  const url = URL.createObjectURL(blob);
  trackedUrls.add(url);
  return url;
}

export function releaseImageGenObjectUrl(url: string) {
  URL.revokeObjectURL(url);
  trackedUrls.delete(url);
}

/** 仅测试用：revoke 全部已登记 URL 并清空会话（模块 store 会跨测试泄漏）。 */
export function resetImageGenSessionForTests() {
  for (const url of trackedUrls) URL.revokeObjectURL(url);
  trackedUrls.clear();
  snapshot = EMPTY_STATE;
  emitListenerSnapshot(listeners, (listener) => listener());
}
