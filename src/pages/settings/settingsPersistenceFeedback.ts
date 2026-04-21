import { toast } from "sonner";
import type { AppAboutInfo } from "../../services/app/appAbout";
import { logToConsole } from "../../services/consoleLog";
import type { GatewayStatus } from "../../services/gateway/gateway";
import type { SettingsMutationResult } from "../../query/settings";
import type { PersistKey, PersistedSettings } from "./settingsPersistenceModel";

type PresentSettingsMutationFeedbackInput = {
  before: PersistedSettings;
  desired: PersistedSettings;
  after: PersistedSettings;
  settledKeys: PersistKey[];
  result: SettingsMutationResult;
  gateway: GatewayStatus | null;
  about: AppAboutInfo | null;
};

export function presentSettingsMutationFeedback(
  input: PresentSettingsMutationFeedbackInput
) {
  const { before, desired, after, settledKeys, result, gateway, about } = input;
  const runtime = result.runtime;
  const portSettled = settledKeys.includes("preferred_port");

  logToConsole("info", "更新设置", {
    changed: settledKeys,
    settings: after,
  });

  try {
    const circuitSettled =
      settledKeys.includes("circuit_breaker_failure_threshold") ||
      settledKeys.includes("circuit_breaker_open_duration_minutes");
    if (circuitSettled && gateway?.running && !portSettled) {
      toast("熔断参数已即时生效");
    }

    if (settledKeys.includes("auto_start")) {
      if (after.auto_start !== desired.auto_start) {
        toast("开机自启设置失败，已回退");
      } else if (after.auto_start && about?.run_mode === "portable") {
        toast("portable 模式开启自启：移动应用位置可能导致自启失效");
      }
    }

    if (!portSettled) {
      return;
    }

    logToConsole("info", "端口变更，运行态已由后端接管", {
      from: before.preferred_port,
      to: after.preferred_port,
      gateway_rebound: runtime.gateway_rebound,
      cli_proxy_synced: runtime.cli_proxy_synced,
      gateway_status: runtime.gateway_status,
    });

    if (runtime.gateway_status.port && runtime.gateway_status.port !== after.preferred_port) {
      toast(`端口被占用，已切换到 ${runtime.gateway_status.port}`);
    } else if (runtime.gateway_rebound) {
      toast("网关已按新配置重绑");
    }

    if (runtime.cli_proxy_synced) {
      toast("CLI 代理配置已同步");
    }
  } catch (err) {
    logToConsole("error", "设置已保存，但后续动作失败", {
      error: String(err),
      changed: settledKeys,
    });
    toast("设置已保存，但后续动作失败：请检查网关和 CLI 代理状态");
  }
}
