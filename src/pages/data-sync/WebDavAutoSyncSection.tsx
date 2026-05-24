import { useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { Switch } from "../../ui/Switch";
import { RefreshCw } from "lucide-react";
import { webdavDownloadSync, type WebDavConfig } from "../../services/app/webdav";

const AUTO_SYNC_STORAGE_KEY = "aio-coding-hub:webdav-auto-sync";
const WEBDAV_CONFIG_STORAGE_KEY = "aio-coding-hub:webdav-config";

type AutoSyncConfig = {
  enabled: boolean;
  intervalSeconds: number;
  strategy: "smart_merge" | "remote_overwrite" | "local_overwrite";
};

function loadAutoSyncConfig(): AutoSyncConfig {
  try {
    const raw = localStorage.getItem(AUTO_SYNC_STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch {
    // ignore
  }
  return { enabled: false, intervalSeconds: 3600, strategy: "smart_merge" };
}

function saveAutoSyncConfig(config: AutoSyncConfig) {
  localStorage.setItem(AUTO_SYNC_STORAGE_KEY, JSON.stringify(config));
}

function loadWebDavConfig(): WebDavConfig {
  try {
    const raw = localStorage.getItem(WEBDAV_CONFIG_STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch {
    // ignore
  }
  return { url: "", username: "", password: "", encryption_password: null };
}

export function WebDavAutoSyncSection() {
  const [config, setConfig] = useState<AutoSyncConfig>(loadAutoSyncConfig);
  const [syncing, setSyncing] = useState(false);
  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const handleSave = useCallback(() => {
    saveAutoSyncConfig(config);
    toast.success("自动同步设置已保存");
  }, [config]);

  const handleSyncNow = useCallback(async () => {
    const webdavConfig = loadWebDavConfig();
    if (!webdavConfig.url) {
      toast.error("请先配置 WebDAV 连接");
      return;
    }
    setSyncing(true);
    try {
      const result = await webdavDownloadSync(webdavConfig);
      toast.success(
        `同步成功：${result.providers_imported} 个供应商，${result.workspaces_imported} 个工作区`
      );
    } catch (error) {
      toast.error(`同步失败：${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setSyncing(false);
    }
  }, []);

  // Auto-sync timer
  useEffect(() => {
    if (intervalRef.current) {
      clearInterval(intervalRef.current);
      intervalRef.current = null;
    }

    if (config.enabled && config.intervalSeconds > 0) {
      intervalRef.current = setInterval(() => {
        void handleSyncNow();
      }, config.intervalSeconds * 1000);
    }

    return () => {
      if (intervalRef.current) {
        clearInterval(intervalRef.current);
      }
    };
  }, [config.enabled, config.intervalSeconds, handleSyncNow]);

  return (
    <Card className="p-5">
      <div className="mb-1 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <RefreshCw className="h-4 w-4 text-blue-600 dark:text-blue-400" />
          <h3 className="text-sm font-semibold text-foreground">WebDAV 自动同步</h3>
        </div>
        <span className="rounded-full bg-amber-100 px-2 py-0.5 text-[10px] font-medium text-amber-800 dark:bg-amber-900/30 dark:text-amber-300">
          实验性
        </span>
      </div>
      <p className="mb-5 text-xs leading-relaxed text-muted-foreground">
        配置自动同步共享数据。设备本地设置仍保留在当前设备。
      </p>

      <div className="space-y-4">
        {/* 启用开关 */}
        <div className="flex items-center justify-between">
          <div>
            <div className="text-xs font-medium text-foreground">启用自动同步</div>
            <div className="mt-0.5 text-[11px] text-muted-foreground">
              {config.enabled ? "已启用" : "已停用"}
              {" — "}开启后将按设定间隔自动从 WebDAV 拉取共享数据。
            </div>
          </div>
          <Switch
            checked={config.enabled}
            onCheckedChange={(checked) => setConfig((prev) => ({ ...prev, enabled: checked }))}
          />
        </div>

        {/* 同步间隔 */}
        <div>
          <label className="mb-1.5 block text-xs font-medium text-foreground">同步间隔（秒）</label>
          <Input
            type="number"
            min={60}
            step={60}
            value={config.intervalSeconds}
            onChange={(e) =>
              setConfig((prev) => ({
                ...prev,
                intervalSeconds: Math.max(60, parseInt(e.target.value) || 3600),
              }))
            }
          />
          <p className="mt-1 text-[11px] text-muted-foreground">
            约 {Math.round(config.intervalSeconds / 60)} 分钟
          </p>
        </div>

        {/* 同步策略 */}
        <div>
          <label className="mb-1.5 block text-xs font-medium text-foreground">同步策略</label>
          <Select
            value={config.strategy}
            onChange={(e) =>
              setConfig((prev) => ({
                ...prev,
                strategy: e.target.value as AutoSyncConfig["strategy"],
              }))
            }
          >
            <option value="smart_merge">智能合并 - 根据时间戳合并本地和远程数据</option>
            <option value="remote_overwrite">远程覆盖 - 远程数据覆盖本地</option>
            <option value="local_overwrite">本地优先 - 保留本地数据，仅补充远程新增</option>
          </Select>
          <p className="mt-1 text-[11px] text-muted-foreground">选择数据冲突的处理方式</p>
        </div>

        {/* 操作按钮 */}
        <div className="grid grid-cols-2 gap-3">
          <Button variant="primary" size="sm" onClick={handleSave} className="w-full">
            保存设置
          </Button>
          <Button
            variant="primary"
            size="sm"
            onClick={() => void handleSyncNow()}
            disabled={syncing}
            className="w-full"
          >
            {syncing ? "同步中…" : "立即同步"}
          </Button>
        </div>
      </div>
    </Card>
  );
}
