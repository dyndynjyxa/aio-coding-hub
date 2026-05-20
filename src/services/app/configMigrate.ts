import { commands, type ConfigImportResult } from "../../generated/bindings";
import { invokeGeneratedIpc, type GeneratedCommandResult } from "../generatedIpc";
import { createRiskyIpcConfirm } from "../ipcConfirm";

export type { ConfigImportResult } from "../../generated/bindings";

export function normalizeConfigMigrateFilePath(filePath: string): string {
  const normalized = filePath.trim();
  if (!normalized) {
    throw new Error("SEC_INVALID_INPUT: filePath is required");
  }
  return normalized;
}

export async function configExport(filePath: string) {
  const normalizedFilePath = normalizeConfigMigrateFilePath(filePath);

  return invokeGeneratedIpc<boolean>({
    title: "导出配置失败",
    cmd: "config_export",
    args: { filePath: normalizedFilePath },
    invoke: () =>
      commands.configExport(normalizedFilePath) as Promise<GeneratedCommandResult<boolean>>,
  });
}

export async function configImport(filePath: string) {
  const normalizedFilePath = normalizeConfigMigrateFilePath(filePath);
  const confirm = createRiskyIpcConfirm("config_import", normalizedFilePath);
  return invokeGeneratedIpc<ConfigImportResult>({
    title: "导入配置失败",
    cmd: "config_import",
    args: { filePath: normalizedFilePath, confirm },
    invoke: () =>
      commands.configImport(normalizedFilePath, confirm) as Promise<
        GeneratedCommandResult<ConfigImportResult>
      >,
  });
}
