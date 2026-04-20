import { commands } from "../../generated/bindings";
import type { AppSettings } from "./settings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export async function settingsCircuitBreakerNoticeSet(enable: boolean) {
  const update = {
    enableCircuitBreakerNotice: enable,
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
