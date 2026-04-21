import {
  commands,
  type ClearRequestLogsResult,
  type DbDiskUsage,
} from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";

export type { ClearRequestLogsResult, DbDiskUsage };

export async function dbDiskUsageGet() {
  return invokeGeneratedIpc<DbDiskUsage>({
    title: "读取数据库磁盘用量失败",
    cmd: "db_disk_usage_get",
    invoke: () => commands.dbDiskUsageGet() as Promise<GeneratedCommandResult<DbDiskUsage>>,
  });
}

export async function requestLogsClearAll() {
  return invokeGeneratedIpc<ClearRequestLogsResult>({
    title: "清空请求日志失败",
    cmd: "request_logs_clear_all",
    invoke: () =>
      commands.requestLogsClearAll() as Promise<GeneratedCommandResult<ClearRequestLogsResult>>,
  });
}

export async function appDataReset() {
  return invokeGeneratedIpc<boolean>({
    title: "重置应用数据失败",
    cmd: "app_data_reset",
    invoke: () => commands.appDataReset() as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function appDataDirGet() {
  return invokeGeneratedIpc<string>({
    title: "读取应用数据目录失败",
    cmd: "app_data_dir_get",
    invoke: () => commands.appDataDirGet() as Promise<GeneratedCommandResult<string>>,
  });
}

export async function appExit() {
  return invokeGeneratedIpc<boolean>({
    title: "退出应用失败",
    cmd: "app_exit",
    invoke: () => commands.appExit() as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function appRestart() {
  return invokeGeneratedIpc<boolean>({
    title: "重启应用失败",
    cmd: "app_restart",
    invoke: () => commands.appRestart() as Promise<GeneratedCommandResult<boolean>>,
  });
}
