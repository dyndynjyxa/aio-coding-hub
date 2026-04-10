import { invokeServiceCommand } from "../invokeServiceCommand";

export type AppAboutInfo = {
  os: string;
  arch: string;
  profile: string;
  app_version: string;
  bundle_type: string | null;
  run_mode: string;
};

export async function appAboutGet() {
  return invokeServiceCommand<AppAboutInfo>({
    title: "读取应用信息失败",
    cmd: "app_about_get",
  });
}
