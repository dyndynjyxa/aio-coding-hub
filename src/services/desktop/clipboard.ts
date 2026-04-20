import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export async function writeDesktopClipboardText(text: string) {
  return invokeGeneratedIpc<boolean>({
    title: "复制到剪贴板失败",
    cmd: "desktop_clipboard_write_text",
    args: { text },
    invoke: () =>
      commands.desktopClipboardWriteText(text) as Promise<GeneratedCommandResult<boolean>>,
  });
}
