import { FormField } from "../../ui/FormField";
import { Input } from "../../ui/Input";
import { Select } from "../../ui/Select";
import { TagsField } from "./TagsField";
import { CX2CC_GLOBAL_SOURCE_VALUE, CX2CC_PROXY_TOKEN } from "./providerEditorUtils";
import type { UseProviderEditorFormReturn } from "./useProviderEditorForm";

export function Cx2ccSection(props: { form: UseProviderEditorFormReturn }) {
  const {
    register,
    saving,
    tags,
    setTags,
    tagInput,
    setTagInput,
    cx2ccSourceValue,
    setCx2ccSourceValue,
    isCodexGatewaySource,
    selectedCx2ccSourceProvider,
    codexGatewayBaseUrl,
    cx2ccFallbackModels,
    codexProviders,
  } = props.form;

  return (
    <>
      <div className="grid gap-3 sm:grid-cols-2">
        <FormField label="名称">
          <Input placeholder="default" {...register("name")} />
        </FormField>

        <TagsField
          tags={tags}
          setTags={setTags}
          tagInput={tagInput}
          setTagInput={setTagInput}
          saving={saving}
        />
      </div>

      <FormField label="备注">
        <Input placeholder="可选备注信息" disabled={saving} {...register("note")} />
      </FormField>

      <FormField label="源 Codex 来源">
        <Select
          value={cx2ccSourceValue}
          onChange={(e) => {
            setCx2ccSourceValue(e.target.value);
          }}
          disabled={saving}
          className="w-full"
        >
          <option value="">请选择 Codex 来源…</option>
          <option value={CX2CC_GLOBAL_SOURCE_VALUE}>
            当前 AIO 服务 Codex 网关（跟随当前分流）
          </option>
          {codexProviders
            .filter((p) => p.enabled && p.source_provider_id == null && p.bridge_type == null)
            .map((p) => (
              <option key={p.id} value={p.id}>
                {p.name} ({p.auth_mode === "oauth" ? "OAuth" : "API Key"})
              </option>
            ))}
        </Select>
        {isCodexGatewaySource ? (
          <Cx2ccGatewaySourceInfo
            codexGatewayBaseUrl={codexGatewayBaseUrl}
            cx2ccFallbackModels={cx2ccFallbackModels}
          />
        ) : selectedCx2ccSourceProvider ? (
          <Cx2ccProviderSourceInfo
            provider={selectedCx2ccSourceProvider}
            cx2ccFallbackModels={cx2ccFallbackModels}
          />
        ) : null}
      </FormField>
    </>
  );
}

function Cx2ccGatewaySourceInfo(props: {
  codexGatewayBaseUrl: string;
  cx2ccFallbackModels: { main: string; haiku: string; sonnet: string; opus: string } | null;
}) {
  const { codexGatewayBaseUrl, cx2ccFallbackModels } = props;

  return (
    <div className="rounded-md border border-slate-200 bg-slate-50 px-3 py-2.5 text-xs text-slate-500 dark:border-slate-700 dark:bg-slate-800/50 dark:text-slate-400">
      <p>
        已选择
        <span className="mx-1 font-medium text-slate-700 dark:text-slate-200">
          当前 AIO 服务 Codex 网关
        </span>
      </p>
      <div className="mt-1 flex flex-wrap gap-x-3 gap-y-1 text-[11px] leading-5">
        <span>
          认证：
          <span className="ml-1 text-slate-700 dark:text-slate-200">App Token</span>
        </span>
        <span>
          价格倍率：
          <span className="ml-1 font-mono text-slate-700 dark:text-slate-200">免费</span>
        </span>
        <span className="min-w-0 max-w-full truncate" title={codexGatewayBaseUrl}>
          Base URL：
          <span className="ml-1 font-mono text-slate-700 dark:text-slate-200">
            {codexGatewayBaseUrl}
          </span>
        </span>
        <span>
          Token：
          <span className="ml-1 font-mono text-slate-700 dark:text-slate-200">
            {CX2CC_PROXY_TOKEN}
          </span>
        </span>
      </div>
      <p className="mt-1 text-[11px] leading-5">
        说明：转译后的请求会进入当前 AIO 服务 Codex 网关，再按当前 Codex 分流继续路由。
      </p>
      <Cx2ccFallbackModelsInfo cx2ccFallbackModels={cx2ccFallbackModels} />
    </div>
  );
}

function Cx2ccProviderSourceInfo(props: {
  provider: { name: string; auth_mode: string; cost_multiplier: number; base_urls: string[] };
  cx2ccFallbackModels: { main: string; haiku: string; sonnet: string; opus: string } | null;
}) {
  const { provider, cx2ccFallbackModels } = props;

  return (
    <div className="rounded-md border border-slate-200 bg-slate-50 px-3 py-2.5 text-xs text-slate-500 dark:border-slate-700 dark:bg-slate-800/50 dark:text-slate-400">
      <p>
        已选择
        <span className="mx-1 font-medium text-slate-700 dark:text-slate-200">
          {provider.name}
        </span>
      </p>
      <div className="mt-1 flex flex-wrap gap-x-3 gap-y-1 text-[11px] leading-5">
        <span>
          认证：
          <span className="ml-1 text-slate-700 dark:text-slate-200">
            {provider.auth_mode === "oauth" ? "OAuth" : "API Key"}
          </span>
        </span>
        <span>
          价格倍率：
          <span className="ml-1 font-mono text-slate-700 dark:text-slate-200">
            x{provider.cost_multiplier.toFixed(2)}
          </span>
        </span>
        <span
          className="min-w-0 max-w-full truncate"
          title={provider.base_urls[0] ?? "跟随网关默认路由"}
        >
          Base URL：
          <span className="ml-1 font-mono text-slate-700 dark:text-slate-200">
            {provider.base_urls[0] ?? "跟随网关默认路由"}
          </span>
        </span>
      </div>
      <Cx2ccFallbackModelsInfo cx2ccFallbackModels={cx2ccFallbackModels} />
    </div>
  );
}

function Cx2ccFallbackModelsInfo(props: {
  cx2ccFallbackModels: { main: string; haiku: string; sonnet: string; opus: string } | null;
}) {
  const { cx2ccFallbackModels } = props;

  return (
    <p className="mt-1 text-[11px] leading-5">
      默认模型映射： 主模型
      <span className="mx-1 font-mono text-slate-700 dark:text-slate-200">
        {cx2ccFallbackModels?.main ?? "全局默认值"}
      </span>
      / Haiku
      <span className="mx-1 font-mono text-slate-700 dark:text-slate-200">
        {cx2ccFallbackModels?.haiku ?? "全局默认值"}
      </span>
      / Sonnet
      <span className="mx-1 font-mono text-slate-700 dark:text-slate-200">
        {cx2ccFallbackModels?.sonnet ?? "全局默认值"}
      </span>
      / Opus
      <span className="mx-1 font-mono text-slate-700 dark:text-slate-200">
        {cx2ccFallbackModels?.opus ?? "全局默认值"}
      </span>
    </p>
  );
}
