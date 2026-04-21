import {
  commands,
  type FrontendErrorReportInput,
} from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type { FrontendErrorReportInput };

export async function appFrontendErrorReport(input: FrontendErrorReportInput) {
  return invokeGeneratedIpc<boolean>({
    title: "上报前端异常失败",
    cmd: "app_frontend_error_report",
    args: { input },
    invoke: () =>
      commands.appFrontendErrorReport(input) as Promise<GeneratedCommandResult<boolean>>,
  });
}
