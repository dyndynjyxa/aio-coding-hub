import { invokeService } from "../invokeServiceCommand";
import type { AppSettings } from "./settings";

export async function settingsCodexSessionIdCompletionSet(enable: boolean) {
  return invokeService<AppSettings>(
    "保存 Codex Session ID 补全设置失败",
    "settings_codex_session_id_completion_set",
    {
      enableCodexSessionIdCompletion: enable,
    }
  );
}
