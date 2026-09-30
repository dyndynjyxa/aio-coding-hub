import { normalizeClaudeModelMapping, type ClaudeModelMapping } from "./claudeModelMapping";
import {
  modelRedirectFromClaudeModelMapping,
  normalizeModelRedirect,
  type ModelRedirect,
} from "./modelRedirect";

export type ParsedRequestLogSpecialSetting = {
  type?: string;
  reason?: string;
} & Record<string, unknown>;

export const CODEX_SYSTEM_REQUEST_SPECIAL_SETTING = {
  type: "codex_system_request",
  threadSource: "system",
} as const;

export function parseRequestLogSpecialSettings(
  specialSettingsJson: string | null | undefined
): ParsedRequestLogSpecialSetting[] {
  if (!specialSettingsJson) return [];

  try {
    const parsed = JSON.parse(specialSettingsJson) as unknown;
    if (Array.isArray(parsed)) {
      return parsed.filter(isParsedRequestLogSpecialSetting);
    }
    return isParsedRequestLogSpecialSetting(parsed) ? [parsed] : [];
  } catch {
    return [];
  }
}

function isParsedRequestLogSpecialSetting(value: unknown): value is ParsedRequestLogSpecialSetting {
  return typeof value === "object" && value !== null;
}

function parsedSettingString(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function parsedSettingNumber(value: unknown): number {
  return typeof value === "number" && Number.isFinite(value) ? value : Number.NaN;
}

function parsedSettingBoolean(value: unknown): boolean {
  return typeof value === "boolean" ? value : false;
}

export function resolveClaudeModelMappingFromSpecialSettings(
  specialSettingsJson: string | null | undefined,
  finalProviderId?: number | null
): ClaudeModelMapping | null {
  return resolveClaudeModelMappingFromParsedSettings(
    parseRequestLogSpecialSettings(specialSettingsJson),
    finalProviderId
  );
}

function resolveClaudeModelMappingFromParsedSettings(
  settings: ParsedRequestLogSpecialSetting[],
  finalProviderId?: number | null
): ClaudeModelMapping | null {
  const mappings = settings
    .map((setting) => {
      if (setting.type !== "claude_model_mapping") return null;
      return normalizeClaudeModelMapping({
        requestedModel: parsedSettingString(setting.requestedModel),
        effectiveModel: parsedSettingString(setting.effectiveModel),
        mappingKind: parsedSettingString(setting.mappingKind),
        providerId: parsedSettingNumber(setting.providerId),
        providerName: parsedSettingString(setting.providerName),
        applied: parsedSettingBoolean(setting.applied),
      });
    })
    .filter((mapping): mapping is ClaudeModelMapping => mapping !== null);

  if (mappings.length === 0) return null;

  if (finalProviderId != null) {
    const finalProviderMapping = mappings
      .slice()
      .reverse()
      .find((mapping) => mapping.providerId === finalProviderId);
    if (finalProviderMapping) return finalProviderMapping;
  }

  return mappings[mappings.length - 1] ?? null;
}

export function resolveModelRedirectFromSpecialSettings(
  specialSettingsJson: string | null | undefined,
  finalProviderId?: number | null
): ModelRedirect | null {
  const settings = parseRequestLogSpecialSettings(specialSettingsJson);
  const redirects = settings
    .filter((setting) => setting.type === "model_redirect")
    .map(normalizeModelRedirect)
    .filter((redirect): redirect is ModelRedirect => redirect !== null);

  if (redirects.length > 0) {
    if (finalProviderId != null) {
      const finalProviderRedirect = redirects
        .slice()
        .reverse()
        .find((redirect) => redirect.providerId === finalProviderId);
      if (finalProviderRedirect) return finalProviderRedirect;
    }
    return redirects[redirects.length - 1] ?? null;
  }

  // Reuse the already-parsed settings; re-parsing the JSON here would double
  // the work on every request-log card render.
  return modelRedirectFromClaudeModelMapping(
    resolveClaudeModelMappingFromParsedSettings(settings, finalProviderId)
  );
}

export function hasCodexSystemRequestSpecialSetting(
  specialSettingsJson: string | null | undefined
): boolean {
  return parseRequestLogSpecialSettings(specialSettingsJson).some(
    (setting) =>
      setting.type === CODEX_SYSTEM_REQUEST_SPECIAL_SETTING.type &&
      setting.threadSource === CODEX_SYSTEM_REQUEST_SPECIAL_SETTING.threadSource
  );
}

export type CodexResponsesTransportRecord = {
  handshakeStatus: number | null;
  eventStatus: number | null;
  clientTransport: "http" | "responses_ws" | null;
  upstreamTransport: "http" | "responses_ws" | null;
  transportAction: string | null;
  failureClass: string | null;
  outputCommitted: boolean | null;
  recoveryFromTraceId: string | null;
  providerId: number | null;
  reasonCode: string | null;
  terminal: "completed" | "incomplete" | "failed" | null;
};

export function resolveCodexResponsesTransportRecords(
  specialSettingsJson: string | null | undefined
): CodexResponsesTransportRecord[] {
  return parseRequestLogSpecialSettings(specialSettingsJson)
    .filter((setting) => setting.type === "codex_responses_transport")
    .map((setting) => ({
      handshakeStatus: setting.handshake_status === 101 ? 101 : null,
      eventStatus:
        setting.status_source === "responses_event" &&
        typeof setting.event_status === "number" &&
        Number.isInteger(setting.event_status) &&
        setting.event_status >= 400 &&
        setting.event_status <= 599
          ? setting.event_status
          : null,
      clientTransport:
        setting.client_transport === "http" || setting.client_transport === "responses_ws"
          ? setting.client_transport
          : null,
      upstreamTransport:
        setting.upstream_transport === "http" || setting.upstream_transport === "responses_ws"
          ? setting.upstream_transport
          : null,
      transportAction: parsedSettingString(setting.transport_action).trim() || null,
      failureClass: parsedSettingString(setting.failure_class).trim() || null,
      outputCommitted:
        typeof setting.output_committed === "boolean" ? setting.output_committed : null,
      recoveryFromTraceId: parsedSettingString(setting.recovery_from_trace_id).trim() || null,
      providerId:
        typeof setting.providerId === "number" &&
        Number.isSafeInteger(setting.providerId) &&
        setting.providerId > 0
          ? setting.providerId
          : null,
      reasonCode: parsedSettingString(setting.reason_code).trim() || null,
      terminal:
        setting.terminal === "completed" ||
        setting.terminal === "incomplete" ||
        setting.terminal === "failed"
          ? setting.terminal
          : null,
    }));
}
