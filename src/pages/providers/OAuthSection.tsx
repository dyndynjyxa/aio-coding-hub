import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { Button } from "../../ui/Button";
import { formatUnixSeconds } from "../../utils/formatters";
import type { UseProviderEditorFormReturn } from "./useProviderEditorForm";

export function OAuthSection(props: { form: UseProviderEditorFormReturn }) {
  const {
    register,
    saving,
    oauthStatus,
    oauthLoading,
    handleOAuthLogin,
    handleOAuthRefresh,
    handleOAuthDisconnect,
  } = props.form;

  return (
    <>
      <FormField label="名称">
        <Input placeholder="default" {...register("name")} />
      </FormField>

      <FormField label="OAuth 连接">
        <div className="rounded-md border border-slate-200 bg-slate-50 p-3 dark:border-slate-700 dark:bg-slate-800/50">
          {oauthLoading ? (
            <div className="flex items-center gap-2 text-sm text-slate-500">
              <span className="animate-spin">⏳</span>
              <span>处理中...</span>
            </div>
          ) : oauthStatus?.connected ? (
            <div className="space-y-2">
              {oauthStatus.email && (
                <p className="text-sm text-slate-700 dark:text-slate-300">
                  <span className="font-medium">账号：</span>
                  {oauthStatus.email}
                </p>
              )}
              {oauthStatus.expires_at && (
                <p className="text-xs text-slate-500 dark:text-slate-400">
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
            <div className="space-y-2">
              <p className="text-sm text-slate-500 dark:text-slate-400">未连接 OAuth</p>
              <Button
                onClick={handleOAuthLogin}
                variant="primary"
                disabled={saving || oauthLoading}
              >
                OAuth 登录
              </Button>
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
