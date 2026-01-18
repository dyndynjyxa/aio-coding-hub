import { useEffect, useMemo, useState } from "react";
import { toast } from "sonner";
import type { AppSettings, GatewayListenMode } from "../../services/settings";
import { gatewayStart, gatewayStop } from "../../services/gateway";
import { wslHostAddressGet } from "../../services/wsl";
import { logToConsole } from "../../services/consoleLog";
import { useGatewayMeta } from "../../hooks/useGatewayMeta";
import { Card } from "../../ui/Card";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { SettingsRow } from "../../ui/SettingsRow";
import { cn } from "../../utils/cn";
import { AlertTriangle, Network } from "lucide-react";

function parseCustomAddress(input: string, fallbackPort: number) {
  const raw = input.trim();
  if (!raw) return { host: "0.0.0.0", port: fallbackPort, overridden: false };
  if (raw.includes("://") || raw.includes("/")) return null;

  if (raw.startsWith("[")) {
    const idx = raw.indexOf("]");
    if (idx < 0) return null;
    const host = raw.slice(1, idx).trim();
    if (!host) return null;
    const tail = raw.slice(idx + 1).trim();
    if (!tail) return { host, port: fallbackPort, overridden: false };
    if (!tail.startsWith(":")) return null;
    const portRaw = tail.slice(1).trim();
    const port = Number(portRaw);
    if (!Number.isFinite(port) || port <= 0 || port > 65535) return null;
    return { host, port: Math.floor(port), overridden: true };
  }

  const parts = raw.split(":");
  if (parts.length === 2) {
    const host = parts[0].trim();
    const portRaw = parts[1].trim();
    const port = Number(portRaw);
    if (!host) return null;
    if (!Number.isFinite(port) || port <= 0 || port > 65535) return null;
    return { host, port: Math.floor(port), overridden: true };
  }

  if (parts.length > 2) {
    // IPv6 without bracket is ambiguous; require [addr]:port.
    return null;
  }

  return { host: raw, port: fallbackPort, overridden: false };
}

function validateCustomAddress(input: string) {
  const parsed = parseCustomAddress(input, 37123);
  if (!parsed) return "自定义地址仅支持 host 或 host:port（IPv6 请使用 [addr]:port）";
  if (parsed.overridden && parsed.port < 1024) return "端口必须 >= 1024";
  return null;
}

export type NetworkSettingsCardProps = {
  available: boolean;
  saving: boolean;
  settings: AppSettings;
  onPersistSettings: (patch: Partial<AppSettings>) => Promise<AppSettings | null>;
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
  const [wslHost, setWslHost] = useState<string | null>(null);

  useEffect(() => {
    setListenMode(settings.gateway_listen_mode);
  }, [settings.gateway_listen_mode]);

  useEffect(() => {
    setCustomAddress(settings.gateway_custom_listen_address);
  }, [settings.gateway_custom_listen_address]);

  useEffect(() => {
    if (!available) return;
    if (listenMode !== "wsl_auto") return;
    let cancelled = false;
    wslHostAddressGet()
      .then((ip) => {
        if (cancelled) return;
        setWslHost(ip ?? null);
      })
      .catch(() => {
        if (cancelled) return;
        setWslHost(null);
      });
    return () => {
      cancelled = true;
    };
  }, [available, listenMode]);

  const currentListenAddress = useMemo(() => {
    if (gateway?.running && gateway.listen_addr) return gateway.listen_addr;

    const port = settings.preferred_port;
    if (listenMode === "localhost") return `127.0.0.1:${port}`;
    if (listenMode === "lan") return `0.0.0.0:${port}`;
    if (listenMode === "wsl_auto") return `${wslHost ?? "127.0.0.1"}:${port}`;
    const parsed = parseCustomAddress(customAddress, port);
    if (!parsed) return "（自定义地址格式无效）";
    return `${parsed.host}:${parsed.port}`;
  }, [
    gateway?.listen_addr,
    gateway?.running,
    listenMode,
    customAddress,
    settings.preferred_port,
    wslHost,
  ]);

  async function restartGatewayIfRunning(preferredPort: number) {
    if (!gateway?.running) return;

    const stopped = await gatewayStop();
    if (!stopped) {
      toast("自动重启失败：无法停止网关");
      return;
    }

    const started = await gatewayStart(preferredPort);
    if (!started) {
      toast("自动重启失败：无法启动网关");
      return;
    }

    if (started.port && started.port !== preferredPort) {
      toast(`端口被占用，已切换到 ${started.port}`);
    } else {
      toast("网关已重启");
    }
  }

  async function commitListenMode(next: GatewayListenMode) {
    if (!available) return;
    setListenMode(next);

    try {
      const updated = await onPersistSettings({ gateway_listen_mode: next });
      if (!updated) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      logToConsole("info", "更新监听模式", { next, running: gateway?.running ?? false });
      await restartGatewayIfRunning(updated.preferred_port);
    } catch (err) {
      logToConsole("error", "更新监听模式失败", { error: String(err), next });
      toast("更新监听模式失败：请稍后重试");
      setListenMode(settings.gateway_listen_mode);
    }
  }

  async function commitCustomAddress() {
    if (!available) return;
    const err = validateCustomAddress(customAddress);
    if (err) {
      toast(err);
      setCustomAddress(settings.gateway_custom_listen_address);
      return;
    }

    try {
      const updated = await onPersistSettings({ gateway_custom_listen_address: customAddress });
      if (!updated) {
        toast("仅在 Tauri Desktop 环境可用");
        setCustomAddress(settings.gateway_custom_listen_address);
        return;
      }

      logToConsole("info", "更新自定义监听地址", {
        address: customAddress,
        running: gateway?.running ?? false,
      });
      await restartGatewayIfRunning(updated.preferred_port);
    } catch (err) {
      logToConsole("error", "更新自定义监听地址失败", {
        error: String(err),
        address: customAddress,
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
        <div className="mb-4 border-b border-slate-100 pb-4">
          <h2 className="text-lg font-semibold text-slate-900 flex items-center gap-2">
            <Network className="h-5 w-5 text-blue-500" />
            网络设置
          </h2>
          <p className="mt-1 text-sm text-slate-500">
            配置网关监听模式与地址（对齐 code-switch-r：仅本地 / WSL 自动检测 / 局域网 /
            自定义地址）。
          </p>
        </div>

        {!available ? (
          <div className="text-sm text-slate-600 bg-slate-50 p-4 rounded-lg">
            仅在 Tauri Desktop 环境可用
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
                  "font-mono text-xs text-slate-700 bg-slate-50 px-3 py-2 rounded border border-slate-100 break-all",
                  !gateway?.running ? "opacity-80" : null
                )}
              >
                {currentListenAddress}
              </div>
            </SettingsRow>

            {listenMode === "lan" ? (
              <div className="mt-3 rounded-lg bg-amber-50 p-3 text-sm text-amber-800 border border-amber-100 flex items-start gap-2">
                <AlertTriangle className="h-4 w-4 mt-0.5 shrink-0" />
                <div>
                  <div className="font-medium">安全提示</div>
                  <div className="text-xs mt-0.5 text-amber-700">
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
