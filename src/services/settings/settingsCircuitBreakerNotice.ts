import { commands } from "../../generated/bindings";
import type { AppSettings } from "./settings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";
import { normalizeBooleanSetting } from "./settingsPrimitiveValidation";

export async function settingsCircuitBreakerNoticeSet(enable: boolean) {
  const normalizedEnable = normalizeBooleanSetting(enable, "enableCircuitBreakerNotice");
  const update = {
    enableCircuitBreakerNotice: normalizedEnable,
  };

  return invokeGeneratedIpc<AppSettings>({
    title: "保存熔断提示设置失败",
    cmd: "settings_circuit_breaker_notice_set",
    args: { update },
    invoke: () =>
      commands.settingsCircuitBreakerNoticeSet(update) as Promise<
        GeneratedCommandResult<AppSettings>
      >,
  });
}
