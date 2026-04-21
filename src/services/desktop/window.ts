import { commands } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type DesktopTheme = "light" | "dark" | "system";

export async function setDesktopWindowTheme(theme: DesktopTheme) {
  return invokeGeneratedIpc<boolean>({
    title: "同步窗口主题失败",
    cmd: "desktop_window_set_theme",
    args: { theme },
    invoke: () =>
      commands.desktopWindowSetTheme(theme) as Promise<GeneratedCommandResult<boolean>>,
  });
}
