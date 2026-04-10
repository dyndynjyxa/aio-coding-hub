import { invokeService } from "../invokeServiceCommand";

export type DbDiskUsage = {
  db_bytes: number;
  wal_bytes: number;
  shm_bytes: number;
  total_bytes: number;
};

export type ClearRequestLogsResult = {
  request_logs_deleted: number;
  request_attempt_logs_deleted: number;
};

export async function dbDiskUsageGet() {
  return invokeService<DbDiskUsage>("读取数据库磁盘用量失败", "db_disk_usage_get");
}

export async function requestLogsClearAll() {
  return invokeService<ClearRequestLogsResult>("清空请求日志失败", "request_logs_clear_all");
}

export async function appDataReset() {
  return invokeService<boolean>("重置应用数据失败", "app_data_reset");
}

export async function appDataDirGet() {
  return invokeService<string>("读取应用数据目录失败", "app_data_dir_get");
}

export async function appExit() {
  return invokeService<boolean>("退出应用失败", "app_exit");
}

export async function appRestart() {
  return invokeService<boolean>("重启应用失败", "app_restart");
}
