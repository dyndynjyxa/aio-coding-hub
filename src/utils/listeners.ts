export function emitListenerSnapshot<TListener>(
  listeners: ReadonlySet<TListener>,
  notify: (listener: TListener) => void,
  onError?: (error: unknown) => void
) {
  for (const listener of Array.from(listeners)) {
    if (!listeners.has(listener)) continue;
    try {
      notify(listener);
    } catch (error) {
      try {
        onError?.(error);
      } catch {}
    }
  }
}
