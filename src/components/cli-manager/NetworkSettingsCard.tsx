import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { AppSettings, GatewayListenMode } from "../../services/settings/settings";
import { logToConsole } from "../../services/consoleLog";
import {
  formatHostPort,
  parseCustomListenAddress,
  validateGatewayCustomListenAddress,
} from "../../services/settings/settingsValidation";
import { useGatewayMeta } from "../../hooks/useGatewayMeta";
import { useWslHostAddressQuery } from "../../query/wsl";
import { Card } from "../../ui/Card";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { SettingsRow } from "../../ui/SettingsRow";
import { cn } from "../../utils/cn";
import { AlertTriangle, Network } from "lucide-react";

export type NetworkSettingsCardProps = {
  available: boolean;
  saving: boolean;
  settings: AppSettings;
  onPersistSettings: (
    patch: Partial<AppSettings> & { upstream_proxy_password?: never }
  ) => Promise<AppSettings | null>;
};

export function NetworkSettingsCard({
  available,
  saving,
  settings,
  onPersistSettings,
}: NetworkSettingsCardProps) {
  const gatewayMeta = useGatewayMeta();
  const gateway = gatewayMeta.gateway;

  const [listenMode, setListenMode] = useState<GatewayListenMode>(settings.gateway_listen_mode);
  const [customAddress, setCustomAddress] = useState<string>(
    settings.gateway_custom_listen_address
  );
  const wslHostQuery = useWslHostAddressQuery({
    enabled: available && listenMode === "wsl_auto",
  });
  const wslHost = wslHostQuery.data ?? null;

  useEffect(() => {
    setListenMode(settings.gateway_listen_mode);
  }, [settings.gateway_listen_mode]);

  useEffect(() => {
    setCustomAddress(settings.gateway_custom_listen_address);
  }, [settings.gateway_custom_listen_address]);

  const currentListenAddress = useMemo(() => {
    if (gateway?.running && gateway.listen_addr) return gateway.listen_addr;

    const port = settings.preferred_port;
    if (listenMode === "localhost") return `127.0.0.1:${port}`;
    if (listenMode === "lan") return `0.0.0.0:${port}`;
    if (listenMode === "wsl_auto") return `${wslHost ?? "127.0.0.1"}:${port}`;
    const parsed = parseCustomListenAddress(customAddress);
    if (!parsed) return "（自定义地址格式无效）";
    return formatHostPort(parsed.host, parsed.port ?? port);
  }, [
    gateway?.listen_addr,
    gateway?.running,
    listenMode,
    customAddress,
    settings.preferred_port,
    wslHost,
  ]);

  async function commitListenMode(next: GatewayListenMode) {
    if (!available) return;
    setListenMode(next);

    try {
      const updated = await onPersistSettings({ gateway_listen_mode: next });
      if (!updated) {
        return;
      }

      logToConsole("info", "更新监听模式", { next, running: gateway?.running ?? false });
      toast("监听模式已保存");
    } catch (err) {
      logToConsole("error", "更新监听模式失败", { error: String(err), next });
      toast("更新监听模式失败：请稍后重试");
      setListenMode(settings.gateway_listen_mode);
    }
  }

  async function commitCustomAddress() {
    if (!available) return;
    const trimmed = customAddress.trim();
    const err = validateGatewayCustomListenAddress(trimmed);
    if (err) {
      toast(err);
      setCustomAddress(settings.gateway_custom_listen_address);
      return;
    }

    try {
      const updated = await onPersistSettings({ gateway_custom_listen_address: trimmed });
      if (!updated) {
        setCustomAddress(settings.gateway_custom_listen_address);
        return;
      }

      logToConsole("info", "更新自定义监听地址", {
        address: trimmed,
        running: gateway?.running ?? false,
      });
      toast("自定义监听地址已保存");
    } catch (err) {
      logToConsole("error", "更新自定义监听地址失败", {
        error: String(err),
        address: trimmed,
      });
      toast("更新自定义监听地址失败：请稍后重试");
      setCustomAddress(settings.gateway_custom_listen_address);
    }
  }

  return (
    <Card className="md:col-span-2 relative overflow-hidden">
      <div className="absolute top-0 right-0 p-4 opacity-5">
        <Network className="h-32 w-32" />
      </div>

      <div className="relative z-10">
        <div className="mb-4 border-b border-border pb-4">
          <h2 className="text-lg font-semibold text-foreground flex items-center gap-2">
            <Network className="h-5 w-5 text-blue-500" />
            网络设置
          </h2>
        </div>

        {!available ? (
          <div className="text-sm font-medium text-secondary-foreground dark:text-foreground bg-secondary p-4 rounded-lg">
            数据不可用
          </div>
        ) : (
          <div className="space-y-1">
            <SettingsRow label="监听模式">
              <Select
                value={listenMode}
                onChange={(e) => void commitListenMode(e.currentTarget.value as GatewayListenMode)}
                disabled={saving}
                className="w-56"
              >
                <option value="localhost">仅本地 (127.0.0.1)</option>
                <option value="wsl_auto">WSL 自动检测</option>
                <option value="lan">局域网 (0.0.0.0)</option>
                <option value="custom">自定义地址</option>
              </Select>
            </SettingsRow>

            {listenMode === "custom" ? (
              <SettingsRow label="自定义地址">
                <Input
                  value={customAddress}
                  placeholder="0.0.0.0 或 0.0.0.0:37123"
                  onChange={(e) => setCustomAddress(e.currentTarget.value)}
                  onBlur={() => void commitCustomAddress()}
                  disabled={saving}
                  className="font-mono"
                />
              </SettingsRow>
            ) : null}

            <SettingsRow label="当前监听地址">
              <div
                className={cn(
                  "font-mono text-xs text-secondary-foreground bg-secondary px-3 py-2 rounded border border-border break-all",
                  !gateway?.running ? "opacity-80" : null
                )}
              >
                {currentListenAddress}
              </div>
            </SettingsRow>

            {listenMode === "lan" ? (
              <div className="mt-3 rounded-lg bg-amber-50 dark:bg-amber-900/30 p-3 text-sm text-amber-800 dark:text-amber-400 border border-amber-100 dark:border-amber-800 flex items-start gap-2">
                <AlertTriangle className="h-4 w-4 mt-0.5 shrink-0" />
                <div>
                  <div className="font-medium">安全提示</div>
                  <div className="text-xs mt-0.5 text-amber-700 dark:text-amber-400">
                    局域网模式会将网关暴露在本机网络接口上。请确保防火墙与访问控制策略符合你的安全要求。
                  </div>
                </div>
              </div>
            ) : null}
          </div>
        )}
      </div>
    </Card>
  );
}
