export function developmentTimeEstimateTooltip(
  fullIdleGapMinutes: number,
  sessionBreakGapMinutes: number
) {
  return `根据请求开始时间和耗时估算：合并并发请求；${fullIdleGapMinutes} 分钟内的间隔计入，${fullIdleGapMinutes}–${sessionBreakGapMinutes} 分钟逐步减少，超过 ${sessionBreakGapMinutes} 分钟不计入。`;
}

export const FOLDER_DEVELOPMENT_TIME_NOTE =
  "文件夹维度按各文件夹独立估算：同一时段并行使用多个文件夹时会分别计入，各文件夹合计可能大于日期维度的预估开发时间。";

export const FULL_IDLE_GAP_TOOLTIP =
  "相邻请求的间隔不超过这个值时，间隔会全部计入预估开发时间；超过后开始逐步减少，直到“停止计入”阈值降为 0。";

export const SESSION_BREAK_GAP_TOOLTIP =
  "相邻请求的间隔达到这个值时，计入时间降为 0；超过这个值的间隔不计入，并视为新的活动会话。";
