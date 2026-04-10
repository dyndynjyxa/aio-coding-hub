import { invokeService } from "../invokeServiceCommand";
import type { AppSettings } from "./settings";

export async function settingsCircuitBreakerNoticeSet(enable: boolean) {
  return invokeService<AppSettings>("保存熔断提示设置失败", "settings_circuit_breaker_notice_set", {
    enableCircuitBreakerNotice: enable,
  });
}
