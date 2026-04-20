import { commands } from "../../generated/bindings";
import type { AppSettings } from "./settings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export async function settingsCodexSessionIdCompletionSet(enable: boolean) {
  const update = {
    enableCodexSessionIdCompletion: enable,
  };

  return invokeGeneratedIpc<AppSettings>({
    title: "保存 Codex Session ID 补全设置失败",
    cmd: "settings_codex_session_id_completion_set",
    args: { update },
    invoke: () =>
      commands.settingsCodexSessionIdCompletionSet(update) as Promise<
        GeneratedCommandResult<AppSettings>
      >,
  });
}
