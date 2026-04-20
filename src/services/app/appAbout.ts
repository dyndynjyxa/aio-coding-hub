import { commands, type AppAboutInfo } from "../../generated/bindings";
import { invokeGeneratedIpc } from "../generatedIpc";

export type { AppAboutInfo };

export async function appAboutGet() {
  return invokeGeneratedIpc<AppAboutInfo>({
    title: "读取应用信息失败",
    cmd: "app_about_get",
    invoke: () => commands.appAboutGet(),
  });
}
