import { commands, type ClearRequestLogsResult, type DbDiskUsage } from "../../generated/bindings";
import {
  invokeGeneratedIpc,
  mapGeneratedCommandResponse,
  type GeneratedCommandResult,
} from "../generatedIpc";
import { createRiskyIpcConfirm } from "../ipcConfirm";

export type { ClearRequestLogsResult, DbDiskUsage };

function requireNonNegativeSafeInteger(value: number, label: string): number {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new Error(`IPC_INVALID_RESULT: ${label} must be a non-negative safe integer`);
  }
  return value;
}

function toDbDiskUsage(value: DbDiskUsage): DbDiskUsage {
  const dbBytes = requireNonNegativeSafeInteger(value.db_bytes, "db_disk_usage.db_bytes");
  const walBytes = requireNonNegativeSafeInteger(value.wal_bytes, "db_disk_usage.wal_bytes");
  const shmBytes = requireNonNegativeSafeInteger(value.shm_bytes, "db_disk_usage.shm_bytes");
  const totalBytes = requireNonNegativeSafeInteger(value.total_bytes, "db_disk_usage.total_bytes");
  const expectedTotal = dbBytes + walBytes + shmBytes;
  if (!Number.isSafeInteger(expectedTotal) || totalBytes !== expectedTotal) {
    throw new Error("IPC_INVALID_RESULT: db_disk_usage.total_bytes mismatch");
  }

  return {
    db_bytes: dbBytes,
    wal_bytes: walBytes,
    shm_bytes: shmBytes,
    total_bytes: totalBytes,
  };
}

export function isClearRequestLogsResult(
  value: ClearRequestLogsResult | null | undefined
): value is ClearRequestLogsResult {
  return (
    !!value &&
    Number.isSafeInteger(value.request_logs_deleted) &&
    value.request_logs_deleted >= 0 &&
    Number.isSafeInteger(value.request_attempt_logs_deleted) &&
    value.request_attempt_logs_deleted >= 0
  );
}

function toClearRequestLogsResult(value: ClearRequestLogsResult): ClearRequestLogsResult {
  if (!isClearRequestLogsResult(value)) {
    throw new Error("IPC_INVALID_RESULT: clear request logs counts must be non-negative integers");
  }
  return value;
}

export async function dbDiskUsageGet() {
  return invokeGeneratedIpc<DbDiskUsage>({
    title: "读取数据库磁盘用量失败",
    cmd: "db_disk_usage_get",
    invoke: async () =>
      mapGeneratedCommandResponse(
        (await commands.dbDiskUsageGet()) as GeneratedCommandResult<DbDiskUsage>,
        toDbDiskUsage
      ),
  });
}

export async function requestLogsClearAll() {
  return invokeGeneratedIpc<ClearRequestLogsResult>({
    title: "清空请求日志失败",
    cmd: "request_logs_clear_all",
    invoke: async () =>
      mapGeneratedCommandResponse(
        (await commands.requestLogsClearAll()) as GeneratedCommandResult<ClearRequestLogsResult>,
        toClearRequestLogsResult
      ),
  });
}

export async function appDataReset() {
  const confirm = createRiskyIpcConfirm("app_data_reset", "app_data");
  return invokeGeneratedIpc<boolean>({
    title: "重置应用数据失败",
    cmd: "app_data_reset",
    args: { confirm },
    invoke: () => commands.appDataReset(confirm) as Promise<GeneratedCommandResult<boolean>>,
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
