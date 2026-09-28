import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";
import { formatUnixSeconds } from "../../utils/formatters";
import type { UseProviderEditorFormReturn } from "./useProviderEditorForm";

export function OAuthSection(props: { form: UseProviderEditorFormReturn }) {
  const {
    register,
    saving,
    cliKey,
    oauthStatus,
    oauthLoading,
    oauthDeviceFlow,
    oauthDevicePolling,
    oauthDeviceError,
    handleOAuthLogin,
    handleOAuthDeviceLogin,
    handleOAuthRefresh,
    handleOAuthDisconnect,
  } = props.form;

  return (
    <>
      <FormField label="名称">
        <Input placeholder="default" {...register("name")} />
      </FormField>

      <FormField label="OAuth 连接">
        <div className="rounded-md border border-border bg-secondary p-3 dark:border-border dark:bg-secondary/50">
          {oauthLoading && !oauthDeviceFlow ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <span className="animate-spin">⏳</span>
              <span>处理中...</span>
            </div>
          ) : oauthStatus?.connected ? (
            <div className="space-y-2">
              {oauthStatus.email && (
                <p className="text-sm text-secondary-foreground">
                  <span className="font-medium">账号：</span>
                  {oauthStatus.email}
                </p>
              )}
              {oauthStatus.expires_at && (
                <p className="text-xs text-muted-foreground">
                  <span className="font-medium">到期：</span>
                  {formatUnixSeconds(oauthStatus.expires_at)}
                </p>
              )}
              <div className="flex items-center gap-2">
                <Button
                  onClick={handleOAuthRefresh}
                  variant="secondary"
                  disabled={saving || oauthLoading}
                >
                  刷新 Token
                </Button>
                <Button
                  onClick={handleOAuthDisconnect}
                  variant="secondary"
                  disabled={saving || oauthLoading}
                >
                  断开连接
                </Button>
              </div>
            </div>
          ) : (
            <div className="space-y-3">
              <p className="text-sm text-muted-foreground">未连接 OAuth</p>
              <div className="flex flex-wrap items-center gap-2">
                <Button
                  onClick={handleOAuthLogin}
                  variant="primary"
                  disabled={saving || oauthLoading}
                >
                  OAuth 登录
                </Button>
                {cliKey === "codex" || cliKey === "grok" ? (
                  <Button
                    onClick={handleOAuthDeviceLogin}
                    variant="secondary"
                    disabled={saving || (oauthLoading && !oauthDevicePolling)}
                  >
                    设备码登录
                  </Button>
                ) : null}
              </div>
              {cliKey === "codex" || cliKey === "grok" ? (
                <p className="text-xs leading-relaxed text-muted-foreground">
                  若当前环境下 localhost
                  回调不稳定，可改用设备码登录，在浏览器中输入验证码完成授权。
                </p>
              ) : null}
              {oauthDeviceFlow ? (
                <div className="rounded-md border border-border bg-card p-3 text-sm text-card-foreground">
                  <p>
                    <span className="font-medium">验证码：</span>
                    <code className="ml-2 rounded bg-muted px-2 py-1 font-mono">
                      {oauthDeviceFlow.user_code}
                    </code>
                  </p>
                  <p className="mt-2 text-xs leading-relaxed text-muted-foreground">
                    请在浏览器中打开 {oauthDeviceFlow.verification_uri}
                    ，输入上面的验证码后返回本窗口等待完成。
                  </p>
                  {oauthDevicePolling ? (
                    <p className="mt-2 text-xs text-muted-foreground">等待授权中...</p>
                  ) : null}
                  {oauthDeviceError ? (
                    <p className="mt-2 text-xs text-destructive">{oauthDeviceError}</p>
                  ) : null}
                </div>
              ) : null}
            </div>
          )}
        </div>
      </FormField>

      <FormField label="价格倍率">
        <Input
          type="number"
          min="0.0001"
          step="0.01"
          placeholder="1.0"
          {...register("cost_multiplier")}
        />
      </FormField>
    </>
  );
}
