import { useCallback, useState } from "react";
import { toast } from "sonner";
import { Button } from "../../ui/Button";
import { Card } from "../../ui/Card";
import { Input } from "../../ui/Input";
import { Switch } from "../../ui/Switch";
import { Cloud, Eye, EyeOff, Info } from "lucide-react";
import {
  webdavTest,
  webdavUploadSync,
  webdavDownloadSync,
  type WebDavConfig,
} from "../../services/app/webdav";

const WEBDAV_CONFIG_STORAGE_KEY = "aio-coding-hub:webdav-config";

function loadWebDavConfig(): WebDavConfig {
  try {
    const raw = localStorage.getItem(WEBDAV_CONFIG_STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch {
    // ignore
  }
  return { url: "", username: "", password: "", encryption_password: null };
}

function saveWebDavConfig(config: WebDavConfig) {
  localStorage.setItem(WEBDAV_CONFIG_STORAGE_KEY, JSON.stringify(config));
}

type SyncDataKey = "accounts" | "providers" | "prompts" | "settings";

export function WebDavSyncSection() {
  const [config, setConfig] = useState<WebDavConfig>(loadWebDavConfig);
  const [showPassword, setShowPassword] = useState(false);
  const [showEncPassword, setShowEncPassword] = useState(false);
  const [encryptionEnabled, setEncryptionEnabled] = useState(
    () => !!loadWebDavConfig().encryption_password
  );
  const [encPassword, setEncPassword] = useState(
    () => loadWebDavConfig().encryption_password ?? ""
  );
  const [testing, setTesting] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [downloading, setDownloading] = useState(false);

  const [syncData, setSyncData] = useState<Record<SyncDataKey, boolean>>({
    accounts: true,
    providers: true,
    prompts: true,
    settings: true,
  });

  const updateField = useCallback((field: keyof WebDavConfig, value: string) => {
    setConfig((prev) => ({ ...prev, [field]: value }));
  }, []);

  const getEffectiveConfig = useCallback((): WebDavConfig => {
    return {
      ...config,
      encryption_password: encryptionEnabled && encPassword ? encPassword : null,
    };
  }, [config, encryptionEnabled, encPassword]);

  const handleSaveConfig = useCallback(() => {
    const effective = getEffectiveConfig();
    saveWebDavConfig(effective);
    toast.success("WebDAV 配置已保存");
  }, [getEffectiveConfig]);

  const handleTest = useCallback(async () => {
    setTesting(true);
    try {
      const result = await webdavTest(getEffectiveConfig());
      if (result.success) {
        toast.success(result.message);
      } else {
        toast.error(result.message);
      }
    } catch (error) {
      toast.error(`测试失败：${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setTesting(false);
    }
  }, [getEffectiveConfig]);

  const handleUpload = useCallback(async () => {
    setUploading(true);
    try {
      const result = await webdavUploadSync(getEffectiveConfig());
      if (result.success) {
        toast.success(`上传成功（${(result.bytes_uploaded / 1024).toFixed(1)} KB）`);
      } else {
        toast.error(result.message);
      }
    } catch (error) {
      toast.error(`上传失败：${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setUploading(false);
    }
  }, [getEffectiveConfig]);

  const handleDownload = useCallback(async () => {
    setDownloading(true);
    try {
      const result = await webdavDownloadSync(getEffectiveConfig());
      toast.success(
        `同步成功：${result.providers_imported} 个供应商，${result.workspaces_imported} 个工作区`
      );
    } catch (error) {
      toast.error(`下载失败：${error instanceof Error ? error.message : String(error)}`);
    } finally {
      setDownloading(false);
    }
  }, [getEffectiveConfig]);

  return (
    <Card className="p-5">
      <div className="mb-1 flex items-center gap-2">
        <Cloud className="h-4 w-4 text-blue-600 dark:text-blue-400" />
        <h3 className="text-sm font-semibold text-foreground">WebDAV 同步</h3>
      </div>
      <p className="mb-5 text-xs leading-relaxed text-muted-foreground">
        配置 WebDAV 以同步共享数据。部分设备本地设置不会上传，也不会被远端覆盖。
      </p>

      {/* 同步范围说明 */}
      <div className="mb-5 rounded-lg border border-blue-200 bg-blue-50/60 px-3 py-2.5 dark:border-blue-800 dark:bg-blue-950/30">
        <div className="flex items-start gap-2">
          <Info className="mt-0.5 h-3.5 w-3.5 shrink-0 text-blue-600 dark:text-blue-400" />
          <p className="text-[11px] leading-relaxed text-blue-800 dark:text-blue-300">
            <span className="font-medium">同步范围说明：</span>
            WebDAV 主要用于同步共享数据，不等同于完整设备备份。当前设备的 WebDAV
            连接/加密设置、同步数据选择、以及那些自动同步 WebDAV 配置不会通过 WebDAV
            上传，也不会被或替覆盖。如需工作区本地设置，请使用手动导出/导入。
          </p>
        </div>
      </div>

      {/* WebDAV 配置表单 */}
      <div className="space-y-4">
        <div>
          <label className="mb-1.5 block text-xs font-medium text-foreground">WebDAV 地址</label>
          <Input
            placeholder="https://"
            value={config.url}
            onChange={(e) => updateField("url", e.target.value)}
          />
        </div>

        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <div>
            <label className="mb-1.5 block text-xs font-medium text-foreground">用户名</label>
            <Input
              placeholder="用户名"
              value={config.username}
              onChange={(e) => updateField("username", e.target.value)}
            />
          </div>
          <div>
            <label className="mb-1.5 block text-xs font-medium text-foreground">密码</label>
            <div className="relative">
              <Input
                type={showPassword ? "text" : "password"}
                placeholder="••••••••••"
                value={config.password}
                onChange={(e) => updateField("password", e.target.value)}
                className="pr-9"
              />
              <button
                type="button"
                className="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                onClick={() => setShowPassword(!showPassword)}
                aria-label={showPassword ? "隐藏密码" : "显示密码"}
              >
                {showPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
              </button>
            </div>
          </div>
        </div>

        {/* 同步数据选择 */}
        <div>
          <label className="mb-1 block text-xs font-medium text-foreground">同步数据</label>
          <p className="mb-2.5 text-[11px] leading-relaxed text-muted-foreground">
            选择需要通过 WebDAV 同步的共享数据类型。例如 WebDAV
            配置、同步选择和账号会自动创新等设备本地信息不会被覆盖在当前设备。
          </p>
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
            {(
              [
                { key: "accounts", label: "账号" },
                { key: "providers", label: "供应商" },
                { key: "prompts", label: "提示词" },
                { key: "settings", label: "偏好设置" },
              ] as const
            ).map(({ key, label }) => (
              <label
                key={key}
                className="flex cursor-pointer items-center gap-2 rounded-md border border-input px-2.5 py-1.5 text-xs transition hover:bg-secondary"
              >
                <input
                  type="checkbox"
                  checked={syncData[key]}
                  onChange={(e) => setSyncData((prev) => ({ ...prev, [key]: e.target.checked }))}
                  className="h-3.5 w-3.5 rounded border-gray-300 text-blue-600 focus:ring-blue-500"
                />
                <span className="text-foreground">{label}</span>
              </label>
            ))}
          </div>
        </div>

        {/* 加密设置 */}
        <div className="rounded-lg border border-input px-4 py-3">
          <div className="flex items-center justify-between">
            <div>
              <div className="text-xs font-medium text-foreground">WebDAV 数据加密</div>
              <div className="mt-0.5 text-[11px] text-muted-foreground">
                使用密码加密上传到 WebDAV 的同步数据。恢复时需要同一密码。
              </div>
            </div>
            <Switch checked={encryptionEnabled} onCheckedChange={setEncryptionEnabled} />
          </div>

          {encryptionEnabled && (
            <div className="mt-3 border-t border-input pt-3">
              <label className="mb-1.5 block text-xs font-medium text-foreground">加密密码</label>
              <div className="relative">
                <Input
                  type={showEncPassword ? "text" : "password"}
                  placeholder="请输入密码"
                  value={encPassword}
                  onChange={(e) => setEncPassword(e.target.value)}
                  className="pr-9"
                />
                <button
                  type="button"
                  className="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                  onClick={() => setShowEncPassword(!showEncPassword)}
                  aria-label={showEncPassword ? "隐藏密码" : "显示密码"}
                >
                  {showEncPassword ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                </button>
              </div>
              <p className="mt-1 text-[11px] text-muted-foreground">
                用于加密上传到 WebDAV 的同步数据，以及解密从 WebDAV 下载的数据。
              </p>
            </div>
          )}
        </div>

        {/* 操作按钮 */}
        <div className="grid grid-cols-4 gap-2">
          <Button variant="primary" size="sm" onClick={handleSaveConfig} className="w-full">
            保存配置
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => void handleTest()}
            disabled={testing || !config.url}
            className="w-full"
          >
            {testing ? "测试中…" : "测试连接"}
          </Button>
          <Button
            variant="warning"
            size="sm"
            onClick={() => void handleUpload()}
            disabled={uploading || !config.url}
            className="w-full"
          >
            {uploading ? "上传中…" : "上传同步数据"}
          </Button>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => void handleDownload()}
            disabled={downloading || !config.url}
            className="w-full"
          >
            {downloading ? "下载中…" : "下载并导入共享数据"}
          </Button>
        </div>
      </div>
    </Card>
  );
}
