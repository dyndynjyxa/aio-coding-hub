import { commands } from "../../generated/bindings";
import { normalizeClipboardText } from "../clipboardText";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export async function writeDesktopClipboardText(text: string) {
  const normalizedText = normalizeClipboardText(text);

  return invokeGeneratedIpc<boolean>({
    title: "复制到剪贴板失败",
    cmd: "desktop_clipboard_write_text",
    args: { text: normalizedText },
    invoke: () =>
      commands.desktopClipboardWriteText(normalizedText) as Promise<
        GeneratedCommandResult<boolean>
      >,
  });
}
