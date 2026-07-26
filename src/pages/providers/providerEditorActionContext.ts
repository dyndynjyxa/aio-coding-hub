import type {
  ClaudeModels,
  CliKey,
  ProviderOAuthDeviceCodeStartResult,
  ProviderOAuthStatusResult,
  ProviderExtensionValuesInput,
  ProviderUpsertInput,
  ProviderSummary,
} from "../../services/providers/providers";
import type { ProviderEditorDialogFormInput } from "../../schemas/providerEditorDialog";
import type { BaseUrlRow, ProviderBaseUrlMode } from "./types";

/** Provider identity and lifecycle */
export type ProviderActionContext = {
  mode: "create" | "edit";
  cliKey: CliKey;
  editingProviderId: number | null;
  editProvider: ProviderSummary | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onSaved: (cliKey: CliKey) => void;
};

/** OAuth status payload shared by auth-related fields */
export type OAuthStatusValue = ProviderOAuthStatusResult | null;

/** Authentication and bridge state */
export type AuthActionContext = {
  authMode: "api_key" | "oauth" | "cx2cc";
  oauthStatus: OAuthStatusValue;
  setOauthStatus: (v: OAuthStatusValue) => void;
  refreshOauthStatus: (providerId?: number | null) => Promise<OAuthStatusValue>;
  /** 将 OAuth 状态直接写入 React Query 缓存，保持编辑器快照与本地状态一致。 */
  writeOauthStatusCache: (status: OAuthStatusValue, providerId?: number | null) => void;
  oauthLoading: boolean;
  setOauthLoading: (v: boolean) => void;
  oauthDeviceFlow: ProviderOAuthDeviceCodeStartResult | null;
  setOauthDeviceFlow: (v: ProviderOAuthDeviceCodeStartResult | null) => void;
  oauthDevicePolling: boolean;
  setOauthDevicePolling: (v: boolean) => void;
  oauthDeviceError: string | null;
  setOauthDeviceError: (v: string | null) => void;
  cx2ccSourceValue: string;
  isCodexGatewaySource: boolean;
  sourceProviderId: number | null;
  selectedCx2ccSourceProvider: ProviderSummary | null;
};

/** Form data and UI state */
export type FormActionContext = {
  saving: boolean;
  setSaving: (v: boolean) => void;
  copyingApiKey: boolean;
  setCopyingApiKey: (v: boolean) => void;
  baseUrlMode: ProviderBaseUrlMode;
  baseUrlRows: BaseUrlRow[];
  tags: string[];
  claudeModels: ClaudeModels;
  streamIdleTimeoutSeconds: string;
  apiKeyConfigured: boolean;
  apiKeyValue: string;
  form: {
    getValues: () => ProviderEditorDialogFormInput;
    setValue: (
      name: keyof ProviderEditorDialogFormInput,
      value: string | boolean,
      options?: { shouldDirty?: boolean; shouldTouch?: boolean; shouldValidate?: boolean }
    ) => void;
  };
};

export type ProviderEditorPayloadContext = {
  mode: "create" | "edit";
  cliKey: CliKey;
  editingProviderId: number | null;
  authMode: "api_key" | "oauth" | "cx2cc";
  baseUrlMode: ProviderBaseUrlMode;
  baseUrlRows: BaseUrlRow[];
  tags: string[];
  claudeModels: ClaudeModels;
  streamIdleTimeoutSeconds: string;
  apiKeyConfigured: boolean;
  isCodexGatewaySource: boolean;
  sourceProviderId: number | null;
  selectedCx2ccSourceProvider: ProviderSummary | null;
  formValues: ProviderEditorDialogFormInput;
  extensionValues?: ProviderExtensionValuesInput[] | null;
};

export type ProviderEditorPayloadBuildError =
  | {
      kind: "schema";
      issues: Array<{ path: Array<PropertyKey>; message: string }>;
    }
  | {
      kind: "message";
      message: string;
    };

export type ProviderEditorPayloadBuildSuccess = {
  payload: ProviderUpsertInput;
  parsedName: string;
};

export type CopyApiKeyActionContext = ProviderActionContext &
  Pick<
    FormActionContext,
    "copyingApiKey" | "setCopyingApiKey" | "apiKeyConfigured" | "apiKeyValue"
  >;

export type SaveActionContext = ProviderActionContext &
  ProviderEditorPayloadContext &
  Pick<FormActionContext, "saving" | "setSaving" | "form"> &
  Pick<AuthActionContext, "oauthStatus" | "setOauthStatus" | "refreshOauthStatus"> & {
    persistProvider: (input: ProviderUpsertInput) => Promise<ProviderSummary>;
  };

export type OAuthActionContext = ProviderActionContext &
  ProviderEditorPayloadContext &
  Pick<FormActionContext, "form"> &
  Pick<
    AuthActionContext,
    | "oauthStatus"
    | "setOauthStatus"
    | "refreshOauthStatus"
    | "writeOauthStatusCache"
    | "setOauthLoading"
    | "oauthDeviceFlow"
    | "setOauthDeviceFlow"
    | "oauthDevicePolling"
    | "setOauthDevicePolling"
    | "oauthDeviceError"
    | "setOauthDeviceError"
  > & {
    persistProvider: (input: ProviderUpsertInput) => Promise<ProviderSummary>;
    removeProvider: (providerId: number) => Promise<boolean>;
    beginOAuthLoginAttempt: () => number;
    isOAuthLoginAttemptCurrent: (attemptId: number) => boolean;
    cancelOAuthDeviceFlow: (flowId: string) => void;
    setActiveOAuthDeviceFlow: (attemptId: number, flowId: string) => void;
    clearActiveOAuthDeviceFlow: (flowId: string) => void;
  };
