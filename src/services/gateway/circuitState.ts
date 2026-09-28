// 熔断状态字符串归一化的唯一入口：大小写统一只发生在这里。
// 后端契约为大写（Rust `CircuitState::as_str()`），历史 attempts_json 可能残留小写。

export type CircuitState = "CLOSED" | "OPEN" | "HALF_OPEN";

/** 严格解析：无法识别时返回 null（供需要区分“未知”的事件/通知路径使用）。 */
export function parseCircuitState(raw: string | null | undefined): CircuitState | null {
  const upper = raw?.toUpperCase();
  if (upper === "CLOSED" || upper === "OPEN" || upper === "HALF_OPEN") return upper;
  return null;
}

/** 展示层归一化：未知值回退 CLOSED。 */
export function normalizeCircuitState(raw: string | null | undefined): CircuitState {
  return parseCircuitState(raw) ?? "CLOSED";
}
