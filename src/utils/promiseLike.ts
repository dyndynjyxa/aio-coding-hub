export type MaybePromiseLike<T = void> = void | PromiseLike<T>;

function isPromiseLike(value: unknown): value is PromiseLike<unknown> {
  if ((typeof value !== "object" && typeof value !== "function") || value === null) return false;
  return typeof (value as { then?: unknown }).then === "function";
}

export function observePromiseLikeRejection(value: unknown, onRejected: (error: unknown) => void) {
  if (!isPromiseLike(value)) return;
  void Promise.resolve(value).catch(onRejected);
}
