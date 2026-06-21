// Usage: Frontend IPC wrappers for community plugin management.

import {
  commands,
  type JsonValue,
  type PluginAuditLog,
  type PluginDetail,
  type PluginInstallSource,
  type PluginManifest,
  type PluginMarketListing,
  type PluginPermissionRisk,
  type PluginRuntime,
  type PluginStatus,
  type PluginSummary,
} from "../generated/bindings";
import { invokeGeneratedIpc } from "./generatedIpc";

export type {
  JsonValue,
  PluginAuditLog,
  PluginDetail,
  PluginInstallSource,
  PluginManifest,
  PluginMarketListing,
  PluginPermissionRisk,
  PluginRuntime,
  PluginStatus,
  PluginSummary,
};

const PLUGIN_AUDIT_LOG_DEFAULT_LIMIT = 50;
const PLUGIN_AUDIT_LOG_MAX_LIMIT = 500;

function normalizeRequiredText(label: string, value: string): string {
  const normalized = value.trim();
  if (!normalized) {
    throw new Error(`SEC_INVALID_INPUT: ${label} is required`);
  }
  return normalized;
}

export function normalizePluginId(pluginId: string): string {
  return normalizeRequiredText("pluginId", pluginId);
}

export function normalizePluginFilePath(filePath: string): string {
  return normalizeRequiredText("filePath", filePath);
}

function clampAuditLimit(limit: number | null | undefined): number {
  if (limit == null || !Number.isFinite(limit)) return PLUGIN_AUDIT_LOG_DEFAULT_LIMIT;
  return Math.min(PLUGIN_AUDIT_LOG_MAX_LIMIT, Math.max(1, Math.trunc(limit)));
}

function normalizePermissions(permissions: readonly string[]): string[] {
  const out: string[] = [];
  const seen = new Set<string>();
  for (const raw of permissions) {
    const permission = raw.trim();
    if (!permission || seen.has(permission)) continue;
    seen.add(permission);
    out.push(permission);
  }
  return out;
}

export async function pluginList() {
  return invokeGeneratedIpc<PluginSummary[]>({
    title: "读取插件列表失败",
    cmd: "plugin_list",
    invoke: async () => commands.pluginList(),
  });
}

export async function pluginGet(pluginId: string) {
  const normalizedPluginId = normalizePluginId(pluginId);

  return invokeGeneratedIpc<PluginDetail>({
    title: "读取插件详情失败",
    cmd: "plugin_get",
    args: { pluginId: normalizedPluginId },
    invoke: async () => commands.pluginGet({ pluginId: normalizedPluginId }),
  });
}

export async function pluginInstallFromFile(filePath: string) {
  const normalizedFilePath = normalizePluginFilePath(filePath);

  return invokeGeneratedIpc<PluginDetail>({
    title: "导入插件失败",
    cmd: "plugin_install_from_file",
    args: { filePath: normalizedFilePath },
    invoke: async () => commands.pluginInstallFromFile({ filePath: normalizedFilePath }),
  });
}

export async function pluginUpdateFromFile(filePath: string) {
  const normalizedFilePath = normalizePluginFilePath(filePath);

  return invokeGeneratedIpc<PluginDetail>({
    title: "更新插件失败",
    cmd: "plugin_update_from_file",
    args: { filePath: normalizedFilePath },
    invoke: async () => commands.pluginUpdateFromFile({ filePath: normalizedFilePath }),
  });
}

export async function pluginInstallRemote(input: {
  pluginId: string;
  downloadUrl: string;
  checksum: string;
  signature?: string | null;
  publicKey?: string | null;
  source?: "market" | "github_release" | null;
}) {
  const pluginId = normalizePluginId(input.pluginId);
  const downloadUrl = normalizeRequiredText("downloadUrl", input.downloadUrl);
  const checksum = normalizeRequiredText("checksum", input.checksum);
  const signature =
    input.signature == null ? null : normalizeRequiredText("signature", input.signature);
  const publicKey =
    input.publicKey == null ? null : normalizeRequiredText("publicKey", input.publicKey);
  const source = input.source ?? null;

  return invokeGeneratedIpc<PluginDetail>({
    title: "远程安装插件失败",
    cmd: "plugin_install_remote",
    args: { pluginId, downloadUrl, checksum, signature, publicKey, source },
    invoke: async () =>
      commands.pluginInstallRemote({
        pluginId,
        downloadUrl,
        checksum,
        signature,
        publicKey,
        source,
      }),
  });
}

export async function pluginRollback(pluginId: string, version: string) {
  const normalizedPluginId = normalizePluginId(pluginId);
  const normalizedVersion = normalizeRequiredText("version", version);

  return invokeGeneratedIpc<PluginDetail>({
    title: "回滚插件失败",
    cmd: "plugin_rollback",
    args: { pluginId: normalizedPluginId, version: normalizedVersion },
    invoke: async () =>
      commands.pluginRollback({ pluginId: normalizedPluginId, version: normalizedVersion }),
  });
}

export async function pluginParseMarketIndex(
  indexJson: string,
  indexUrl?: string | null,
  signature?: string | null
) {
  const normalizedIndexJson = normalizeRequiredText("indexJson", indexJson);
  const normalizedIndexUrl = indexUrl == null ? null : normalizeRequiredText("indexUrl", indexUrl);
  const normalizedSignature =
    signature == null ? null : normalizeRequiredText("signature", signature);

  return invokeGeneratedIpc<PluginMarketListing[]>({
    title: "解析插件市场索引失败",
    cmd: "plugin_parse_market_index",
    args: {
      indexJson: normalizedIndexJson,
      indexUrl: normalizedIndexUrl,
      signature: normalizedSignature,
    },
    invoke: async () =>
      commands.pluginParseMarketIndex({
        indexJson: normalizedIndexJson,
        indexUrl: normalizedIndexUrl,
        signature: normalizedSignature,
      }),
  });
}

export async function pluginInstallOfficial(pluginId: string) {
  const normalizedPluginId = normalizePluginId(pluginId);

  return invokeGeneratedIpc<PluginDetail>({
    title: "安装官方插件失败",
    cmd: "plugin_install_official",
    args: { pluginId: normalizedPluginId },
    invoke: async () => commands.pluginInstallOfficial({ pluginId: normalizedPluginId }),
  });
}

export async function pluginQuarantineRevoked(pluginId: string) {
  const normalizedPluginId = normalizePluginId(pluginId);

  return invokeGeneratedIpc<PluginDetail>({
    title: "隔离撤销插件失败",
    cmd: "plugin_quarantine_revoked",
    args: { pluginId: normalizedPluginId },
    invoke: async () => commands.pluginQuarantineRevoked({ pluginId: normalizedPluginId }),
  });
}

export async function pluginEnable(pluginId: string) {
  const normalizedPluginId = normalizePluginId(pluginId);

  return invokeGeneratedIpc<PluginDetail>({
    title: "启用插件失败",
    cmd: "plugin_enable",
    args: { pluginId: normalizedPluginId },
    invoke: async () => commands.pluginEnable({ pluginId: normalizedPluginId }),
  });
}

export async function pluginDisable(pluginId: string) {
  const normalizedPluginId = normalizePluginId(pluginId);

  return invokeGeneratedIpc<PluginDetail>({
    title: "禁用插件失败",
    cmd: "plugin_disable",
    args: { pluginId: normalizedPluginId },
    invoke: async () => commands.pluginDisable({ pluginId: normalizedPluginId }),
  });
}

export async function pluginUninstall(pluginId: string) {
  const normalizedPluginId = normalizePluginId(pluginId);

  return invokeGeneratedIpc<PluginDetail>({
    title: "卸载插件失败",
    cmd: "plugin_uninstall",
    args: { pluginId: normalizedPluginId },
    invoke: async () => commands.pluginUninstall({ pluginId: normalizedPluginId }),
  });
}

export async function pluginSaveConfig(pluginId: string, config: JsonValue) {
  const normalizedPluginId = normalizePluginId(pluginId);

  return invokeGeneratedIpc<PluginDetail>({
    title: "保存插件配置失败",
    cmd: "plugin_save_config",
    args: { pluginId: normalizedPluginId, config },
    invoke: async () => commands.pluginSaveConfig({ pluginId: normalizedPluginId, config }),
  });
}

export async function pluginGrantPermissions(pluginId: string, permissions: readonly string[]) {
  const normalizedPluginId = normalizePluginId(pluginId);
  const normalizedPermissions = normalizePermissions(permissions);

  return invokeGeneratedIpc<PluginDetail>({
    title: "授权插件权限失败",
    cmd: "plugin_grant_permissions",
    args: { pluginId: normalizedPluginId, permissions: normalizedPermissions },
    invoke: async () =>
      commands.pluginGrantPermissions({
        pluginId: normalizedPluginId,
        permissions: normalizedPermissions,
      }),
  });
}

export async function pluginRevokePermission(pluginId: string, permission: string) {
  const normalizedPluginId = normalizePluginId(pluginId);
  const normalizedPermission = normalizeRequiredText("permission", permission);

  return invokeGeneratedIpc<PluginDetail>({
    title: "撤销插件权限失败",
    cmd: "plugin_revoke_permission",
    args: { pluginId: normalizedPluginId, permission: normalizedPermission },
    invoke: async () =>
      commands.pluginRevokePermission({
        pluginId: normalizedPluginId,
        permission: normalizedPermission,
      }),
  });
}

export async function pluginListAuditLogs(input: {
  pluginId?: string | null;
  limit?: number | null;
}) {
  const pluginId = input.pluginId == null ? null : normalizePluginId(input.pluginId);
  const limit = clampAuditLimit(input.limit);

  return invokeGeneratedIpc<PluginAuditLog[]>({
    title: "读取插件审计日志失败",
    cmd: "plugin_list_audit_logs",
    args: { pluginId, limit },
    invoke: async () => commands.pluginListAuditLogs({ pluginId, limit }),
  });
}
