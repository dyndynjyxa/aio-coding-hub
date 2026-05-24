import { invokeTauriOrNull } from "../tauriInvoke";

export type WebDavConfig = {
  url: string;
  username: string;
  password: string;
  encryption_password: string | null;
};

export type WebDavTestResult = {
  success: boolean;
  message: string;
};

export type WebDavUploadResult = {
  success: boolean;
  message: string;
  bytes_uploaded: number;
};

export type WebDavDownloadResult = {
  success: boolean;
  message: string;
  data: string | null;
};

export type ConfigImportResult = {
  providers_imported: number;
  sort_modes_imported: number;
  workspaces_imported: number;
  prompts_imported: number;
  mcp_servers_imported: number;
  skill_repos_imported: number;
  installed_skills_imported: number;
  local_skills_imported: number;
};

export async function webdavTest(config: WebDavConfig): Promise<WebDavTestResult> {
  const result = await invokeTauriOrNull<WebDavTestResult>("webdav_test", { config });
  if (!result) throw new Error("WebDAV 测试连接失败");
  return result;
}

export async function webdavUploadSync(config: WebDavConfig): Promise<WebDavUploadResult> {
  const result = await invokeTauriOrNull<WebDavUploadResult>("webdav_upload_sync", { config });
  if (!result) throw new Error("WebDAV 上传失败");
  return result;
}

export async function webdavDownloadSync(config: WebDavConfig): Promise<ConfigImportResult> {
  const result = await invokeTauriOrNull<ConfigImportResult>("webdav_download_sync", { config });
  if (!result) throw new Error("WebDAV 下载失败");
  return result;
}
