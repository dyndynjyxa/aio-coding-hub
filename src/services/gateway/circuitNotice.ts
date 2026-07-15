/**
 * 熔断通知（前端渲染）
 *
 * `gateway:circuit` 状态跃迁（prev != next）且熔断提示开关开启时，在前端组装
 * 通知文案并通过 `noticeSend` 走现有系统通知发送路径（notice_send → notice:notify）。
 *
 * 正文行结构对齐已删除的 Rust `build_circuit_notice`（src-tauri/src/gateway/events.rs）；
 * 触发失败短标签唯一来源为 `constants/gatewayErrorCodes`——其映射覆盖全部错误码，
 * 比旧 Rust `short_label_zh` 的 10 码子集更宽（原始码仍在括号中展示），属有意改进。
 */

import { GatewayErrorCodes, getGatewayErrorShortLabel } from "../../constants/gatewayErrorCodes";
import { logToConsole } from "../consoleLog";
import { noticeSend, type NoticeLevel } from "../notification/notice";
import { parseCircuitState, type CircuitState } from "./circuitState";
import type { GatewayCircuitEvent } from "./gatewayEvents";

// 兜底默认值与 Rust 侧一致：
// src-tauri/src/infra/settings/defaults.rs `DEFAULT_ENABLE_CIRCUIT_BREAKER_NOTICE = false`
// （crossLayerContracts.test.ts 有跨层断言守护，两侧漂移会翻红。）
export const DEFAULT_ENABLE_CIRCUIT_BREAKER_NOTICE = false;

// settings 快照到达前（applySettingsRuntimeSnapshot）保持 Rust 默认值。
let enabled = DEFAULT_ENABLE_CIRCUIT_BREAKER_NOTICE;

export function setCircuitBreakerNoticeEnabled(value: boolean) {
  enabled = value === true;
}

export function getCircuitBreakerNoticeEnabled(): boolean {
  return enabled;
}

function stateText(state: CircuitState): string {
  if (state === "OPEN") return "熔断";
  if (state === "HALF_OPEN") return "半开";
  return "正常";
}

function reasonText(reason: string): string {
  switch (reason) {
    case "FAILURE_THRESHOLD_REACHED":
      return "失败次数达到阈值";
    case "OPEN_EXPIRED":
      return "熔断到期，进入半开试探";
    case "HALF_OPEN_SUCCESS":
      return "半开试探成功，恢复正常";
    case "HALF_OPEN_FAILURE":
      return "半开试探失败，重新熔断";
    default:
      return reason;
  }
}

export type CircuitNoticeContent = {
  level: NoticeLevel;
  title: string;
  lines: string[];
};

/**
 * 组装熔断通知内容（纯函数）。非跃迁（prev == next）或状态无法识别时返回 null。
 */
export function buildCircuitNoticeContent(event: GatewayCircuitEvent): CircuitNoticeContent | null {
  const prev = parseCircuitState(event.prev_state);
  const next = parseCircuitState(event.next_state);
  if (prev == null || next == null || prev === next) return null;

  const provider = event.provider_name;
  const level: NoticeLevel =
    next === "OPEN" ? "warning" : next === "HALF_OPEN" ? "info" : "success";
  const title =
    next === "OPEN"
      ? `熔断触发：${provider}`
      : next === "HALF_OPEN"
        ? `熔断试探：${provider}`
        : `熔断恢复：${provider}`;

  const lines: string[] = [
    `CLI：${event.cli_key}`,
    `Provider：${provider} (id=${event.provider_id})`,
    `Base URL：${event.base_url}`,
    `状态：${stateText(prev)} → ${stateText(next)}`,
    `失败：${event.failure_count} / ${event.failure_threshold}`,
    `原因：${reasonText(event.reason)}（${event.reason}）`,
  ];

  // 触发失败归因仅在 →熔断 跃迁时渲染（其余状态正文与旧版一致）；
  // 旧后端事件缺失该字段时优雅降级为不渲染。
  const triggerCode = event.trigger_error_code ?? null;
  if (next === "OPEN" && triggerCode) {
    lines.push(`触发失败：${getGatewayErrorShortLabel(triggerCode)}（${triggerCode}）`);
    const timeoutSecs = event.first_byte_timeout_secs ?? null;
    if (triggerCode === GatewayErrorCodes.UPSTREAM_TIMEOUT && timeoutSecs != null) {
      lines.push(
        `首字节超时配置：${timeoutSecs} 秒；若上游响应慢属预期，可调大：设置 → 通用 → 首字节超时（0=禁用）`
      );
    }
  }

  if (event.open_until != null) {
    const remainingSecs = event.open_until - event.ts;
    if (remainingSecs > 0) {
      const remainingMinutes = Math.ceil(remainingSecs / 60);
      lines.push(`熔断至：${event.open_until}（约 ${remainingMinutes} 分钟后）`);
    } else {
      lines.push(`熔断至：${event.open_until}（已到期）`);
    }
  } else {
    lines.push("熔断至：—");
  }

  lines.push(`Trace：${event.trace_id}`);

  return { level, title, lines };
}

/**
 * 开关开启且事件为状态跃迁时发送系统通知；否则静默返回。
 * 发送失败只记录日志，不向调用方抛错。
 */
export async function maybeSendCircuitBreakerNotice(event: GatewayCircuitEvent): Promise<void> {
  if (!enabled) return;

  const content = buildCircuitNoticeContent(event);
  if (!content) return;

  try {
    await noticeSend({
      level: content.level,
      title: content.title,
      body: content.lines.join("\n"),
    });
  } catch (err) {
    logToConsole("warn", "发送熔断通知失败", { error: String(err), title: content.title });
  }
}
