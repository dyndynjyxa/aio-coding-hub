import { describe, expect, it } from "vitest";
import { AppErrorCodes } from "../appErrorCodes";
import { appEventNames } from "../appEvents";
import { DEFAULT_GATEWAY_PORT } from "../gateway";
import { GATEWAY_EVENT_TEXT_LIMITS, gatewayEventNames } from "../gatewayEvents";
import { GatewayErrorCodes } from "../gatewayErrorCodes";
import { HOME_USAGE_PERIOD_VALUES } from "../homeUsagePeriods";
import { MAX_MODEL_NAME_LEN } from "../../schemas/providerEditorDialog";
import { DEFAULT_ENABLE_CIRCUIT_BREAKER_NOTICE } from "../../services/gateway/circuitNotice";
import { CODEX_SYSTEM_REQUEST_SPECIAL_SETTING } from "../../services/gateway/requestLogSpecialSettings";
import { MAX_ATTEMPTS_PER_TRACE } from "../../services/gateway/traceLimits";
import { SETTINGS_VALIDATION_LIMITS } from "../../services/settings/settingsValidation";
import { getSettingsState, resetMswState } from "../../test/msw/state";
import bindingsSource from "../../generated/bindings.ts?raw";
import heartbeatSource from "../../../src-tauri/src/app/heartbeat_watchdog.rs?raw";
import noticeSource from "../../../src-tauri/src/app/notice.rs?raw";
import settingsServiceSource from "../../../src-tauri/src/app/settings_service.rs?raw";
import startupStateSource from "../../../src-tauri/src/app/startup_state.rs?raw";
import promptsSource from "../../../src-tauri/src/domain/prompts.rs?raw";
import providersValidationSource from "../../../src-tauri/src/domain/providers/validation.rs?raw";
import workspacesSource from "../../../src-tauri/src/domain/workspaces.rs?raw";
import providersTypesSource from "../../../src-tauri/src/domain/providers/types.rs?raw";
import gatewayEventsSource from "../../../src-tauri/src/gateway/events.rs?raw";
import codexRequestClassifierSource from "../../../src-tauri/src/gateway/proxy/handler/middleware/codex_request_classifier.rs?raw";
import gatewayErrorCodeSource from "../../../src-tauri/src/gateway/proxy/error_code.rs?raw";
import settingsDefaultsSource from "../../../src-tauri/src/infra/settings/defaults.rs?raw";
import settingsPersistenceSource from "../../../src-tauri/src/infra/settings/persistence.rs?raw";

function extractRustStringConst(source: string, constName: string) {
  const match = source.match(new RegExp(`const\\s+${constName}:\\s*&str\\s*=\\s*"([^"]+)"`));
  expect(match, `missing Rust const ${constName}`).toBeTruthy();
  return match?.[1] ?? "";
}

function extractBindingsUnionLiterals(source: string, typeName: string) {
  const match = source.match(new RegExp(`export type ${typeName} = (.+)$`, "m"));
  expect(match, `missing generated type ${typeName}`).toBeTruthy();
  return Array.from((match?.[1] ?? "").matchAll(/"([^"]+)"/g), (part) => part[1]);
}

function extractRustNumericConst(source: string, constName: string) {
  const match = source.match(
    new RegExp(`const\\s+${constName}:\\s*(?:u16|u32|u64|usize)\\s*=\\s*([0-9_*\\s]+);`)
  );
  expect(match, `missing Rust numeric const ${constName}`).toBeTruthy();
  const expression = (match?.[1] ?? "").replace(/_/g, "");
  return expression
    .split("*")
    .map((part: string) => Number.parseInt(part.trim(), 10))
    .reduce((product: number, factor: number) => product * factor, 1);
}

function extractRustBoolConst(source: string, constName: string) {
  const match = source.match(new RegExp(`const\\s+${constName}:\\s*bool\\s*=\\s*(true|false);`));
  expect(match, `missing Rust bool const ${constName}`).toBeTruthy();
  return match?.[1] === "true";
}

function extractRustGatewayErrorCodes(source: string) {
  return Array.from(
    new Set(
      Array.from(source.matchAll(/"((?:GW|CLI_PROXY)_[A-Z0-9_]+)"/g), (match) => match[1]).filter(
        (value) => value !== "GW_UNKNOWN"
      )
    )
  );
}

describe("cross-layer contracts", () => {
  it("keeps app event names aligned with Rust emitters", () => {
    expect(extractRustStringConst(heartbeatSource, "HEARTBEAT_EVENT_NAME")).toBe(
      appEventNames.heartbeat
    );
    expect(extractRustStringConst(noticeSource, "NOTICE_EVENT_NAME")).toBe(appEventNames.notice);
    expect(extractRustStringConst(startupStateSource, "APP_STARTUP_STATUS_EVENT_NAME")).toBe(
      appEventNames.startupStatus
    );
  });

  it("keeps gateway event names aligned with Rust emitters", () => {
    expect(extractRustStringConst(gatewayEventsSource, "GATEWAY_STATUS_EVENT_NAME")).toBe(
      gatewayEventNames.status
    );
    expect(extractRustStringConst(gatewayEventsSource, "GATEWAY_REQUEST_START_EVENT_NAME")).toBe(
      gatewayEventNames.requestStart
    );
    expect(extractRustStringConst(gatewayEventsSource, "GATEWAY_ATTEMPT_EVENT_NAME")).toBe(
      gatewayEventNames.attempt
    );
    expect(extractRustStringConst(gatewayEventsSource, "GATEWAY_REQUEST_EVENT_NAME")).toBe(
      gatewayEventNames.request
    );
    expect(extractRustStringConst(gatewayEventsSource, "GATEWAY_REQUEST_SIGNAL_EVENT_NAME")).toBe(
      gatewayEventNames.requestSignal
    );
    expect(extractRustStringConst(gatewayEventsSource, "GATEWAY_LOG_EVENT_NAME")).toBe(
      gatewayEventNames.log
    );
    expect(extractRustStringConst(gatewayEventsSource, "GATEWAY_CIRCUIT_EVENT_NAME")).toBe(
      gatewayEventNames.circuit
    );
  });

  it("keeps gateway error codes aligned with Rust definitions", () => {
    expect(extractRustGatewayErrorCodes(gatewayErrorCodeSource)).toEqual(
      Object.values(GatewayErrorCodes)
    );
  });

  it("keeps the Codex system request marker aligned with Rust", () => {
    expect(
      extractRustStringConst(codexRequestClassifierSource, "CODEX_SYSTEM_REQUEST_SETTING_TYPE")
    ).toBe(CODEX_SYSTEM_REQUEST_SPECIAL_SETTING.type);
    expect(
      extractRustStringConst(codexRequestClassifierSource, "CODEX_SYSTEM_REQUEST_THREAD_SOURCE")
    ).toBe(CODEX_SYSTEM_REQUEST_SPECIAL_SETTING.threadSource);
    expect(codexRequestClassifierSource).toContain(
      '"threadSource": CODEX_SYSTEM_REQUEST_THREAD_SOURCE'
    );
  });

  it("keeps gateway event truncation limits aligned with the Rust emitter", () => {
    const pairs = [
      ["EVENT_METHOD_MAX_CHARS", GATEWAY_EVENT_TEXT_LIMITS.METHOD_MAX_LENGTH],
      ["EVENT_STATE_MAX_CHARS", GATEWAY_EVENT_TEXT_LIMITS.STATE_MAX_LENGTH],
      ["EVENT_SHORT_TEXT_MAX_CHARS", GATEWAY_EVENT_TEXT_LIMITS.SHORT_TEXT_MAX_LENGTH],
      ["EVENT_PATH_MAX_CHARS", GATEWAY_EVENT_TEXT_LIMITS.PATH_MAX_LENGTH],
      ["EVENT_QUERY_MAX_CHARS", GATEWAY_EVENT_TEXT_LIMITS.QUERY_MAX_LENGTH],
      ["EVENT_URL_MAX_CHARS", GATEWAY_EVENT_TEXT_LIMITS.URL_MAX_LENGTH],
    ] as const;
    for (const [rustName, frontendValue] of pairs) {
      expect(extractRustNumericConst(gatewayEventsSource, rustName), rustName).toBe(frontendValue);
    }
    // ID_MAX_LENGTH is a frontend-only validation bound (Rust never truncates ids).
    expect(GATEWAY_EVENT_TEXT_LIMITS.ID_MAX_LENGTH).toBe(256);
    // The emitter trims the attempts array to the same cap the trace store keeps.
    expect(extractRustNumericConst(gatewayEventsSource, "REQUEST_EVENT_MAX_ATTEMPTS")).toBe(
      MAX_ATTEMPTS_PER_TRACE
    );
  });

  it("keeps gateway event payloads free of skip_serializing_if outside known exemptions", () => {
    // The shared-fixture contract tests compare serde_json values, so a field
    // skipped when None would silently evade both sides while the frontend
    // normalizers never learn about it. Options must serialize as explicit null.
    // Exemptions: circuit attribution fields are omitted when None by explicit
    // space-constraint design (attempts_json must gain zero bytes on success
    // paths); both sides pin the omission with dedicated tests (Rust key-set
    // assertions in failover_loop/tests.rs, absence handling in attemptsJson).
    const exemptFields = ["circuit_recover_at_unix", "circuit_trigger_error_code"];
    const skippedFields = Array.from(
      gatewayEventsSource.matchAll(
        /#\[serde\(skip_serializing_if[^\]]*\)\]\s*(?:pub(?:\([^)]*\))?\s+)?(\w+):/g
      ),
      (match) => match[1]
    );
    expect(skippedFields.sort()).toEqual(exemptFields.sort());
  });

  it("keeps the MSW settings mock aligned with Rust defaults for drift-prone fields", () => {
    // The MSW mock mirrors ~60 backend defaults by hand; these are the fields
    // that have actually drifted historically (schema bumps and default flips).
    resetMswState();
    const settings = getSettingsState();
    expect(settings.schema_version).toBe(
      extractRustNumericConst(settingsDefaultsSource, "SCHEMA_VERSION")
    );
    expect(settings.upstream_stream_idle_timeout_seconds).toBe(
      extractRustNumericConst(
        settingsDefaultsSource,
        "DEFAULT_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS"
      )
    );
    expect(settings.enable_billing_header_rectifier).toBe(
      extractRustBoolConst(settingsDefaultsSource, "DEFAULT_ENABLE_BILLING_HEADER_RECTIFIER")
    );
  });

  it("keeps app error codes emitted by their owning Rust modules", () => {
    // Frontend matches these codes (never message text); each must stay present
    // in the Rust module that owns the emission.
    const owners: Record<keyof typeof AppErrorCodes, string[]> = {
      PROMPT_NAME_REQUIRED: [promptsSource],
      PROMPT_NAME_CONFLICT: [promptsSource],
      SETTINGS_RECOVERY_REQUIRED: [settingsServiceSource, settingsPersistenceSource],
      DB_CONSTRAINT: [workspacesSource],
      SEC_INVALID_INPUT: [providersValidationSource],
    };

    for (const [code, sources] of Object.entries(owners)) {
      const literal = AppErrorCodes[code as keyof typeof AppErrorCodes];
      for (const source of sources) {
        expect(
          source.includes(`"${literal}"`) || source.includes(`${literal}: `),
          `Rust owner module no longer emits ${literal}`
        ).toBe(true);
      }
    }
  });

  it("keeps settings validation limits aligned with Rust defaults", () => {
    const limits = SETTINGS_VALIDATION_LIMITS;
    // Limits with a Rust const of the same name in infra/settings/defaults.rs.
    const rustBackedLimits = [
      "MAX_UPDATE_RELEASES_URL_LEN",
      "MAX_UPSTREAM_PROXY_URL_LEN",
      "MAX_UPSTREAM_PROXY_USERNAME_LEN",
      "MAX_UPSTREAM_PROXY_PASSWORD_LEN",
      "MAX_CX2CC_MODEL_NAME_LEN",
      "MAX_CX2CC_OPTIONAL_FIELD_LEN",
      "MAX_LOG_RETENTION_DAYS",
      "MAX_REQUEST_LOG_RETENTION_DAYS",
      "MAX_PROVIDER_COOLDOWN_SECONDS",
      "MAX_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS",
      "MAX_UPSTREAM_FIRST_BYTE_TIMEOUT_SECONDS",
      "MIN_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS",
      "MAX_UPSTREAM_STREAM_IDLE_TIMEOUT_SECONDS",
      "MAX_UPSTREAM_REQUEST_TIMEOUT_NON_STREAMING_SECONDS",
      "MAX_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER",
      "MAX_FAILOVER_MAX_PROVIDERS_TO_TRY",
      "MAX_FAILOVER_TOTAL_ATTEMPTS",
      "MAX_CIRCUIT_BREAKER_FAILURE_THRESHOLD",
      "MAX_CIRCUIT_BREAKER_OPEN_DURATION_MINUTES",
    ] as const;
    for (const name of rustBackedLimits) {
      expect(extractRustNumericConst(settingsDefaultsSource, name), name).toBe(limits[name]);
    }

    // Frontend-only minimums mirroring hardcoded backend checks.
    expect(limits.MIN_PREFERRED_PORT, "persistence.rs: preferred_port < 1024").toBe(1024);
    expect(settingsPersistenceSource).toContain("preferred_port < 1024");
    expect(limits.MAX_PREFERRED_PORT, "u16::MAX").toBe(65535);
    expect(limits.MIN_LOG_RETENTION_DAYS, "persistence.rs: log_retention_days == 0").toBe(1);
    expect(settingsPersistenceSource).toContain("log_retention_days == 0");
    // MIN_*=1 limits anchor the validate_bounds error message text so that
    // relaxing/removing the Rust check turns this test red.
    const minOnePairs = [
      [
        limits.MIN_PROVIDER_BASE_URL_PING_CACHE_TTL_SECONDS,
        "provider_base_url_ping_cache_ttl_seconds must be >= 1",
      ],
      [
        limits.MIN_FAILOVER_MAX_ATTEMPTS_PER_PROVIDER,
        "failover_max_attempts_per_provider must be >= 1",
      ],
      [limits.MIN_FAILOVER_MAX_PROVIDERS_TO_TRY, "failover_max_providers_to_try must be >= 1"],
      [
        limits.MIN_CIRCUIT_BREAKER_FAILURE_THRESHOLD,
        "circuit_breaker_failure_threshold must be >= 1",
      ],
      [
        limits.MIN_CIRCUIT_BREAKER_OPEN_DURATION_MINUTES,
        "circuit_breaker_open_duration_minutes must be >= 1",
      ],
    ] as const;
    for (const [frontendValue, rustAnchor] of minOnePairs) {
      expect(frontendValue, `persistence.rs: ${rustAnchor}`).toBe(1);
      expect(settingsPersistenceSource, rustAnchor).toContain(rustAnchor);
    }
  });

  it("keeps the circuit-breaker notice fallback default aligned with Rust", () => {
    // circuitNotice.ts uses this before the settings snapshot arrives; a Rust
    // default flip without a frontend update must turn this red.
    expect(
      extractRustBoolConst(settingsDefaultsSource, "DEFAULT_ENABLE_CIRCUIT_BREAKER_NOTICE")
    ).toBe(DEFAULT_ENABLE_CIRCUIT_BREAKER_NOTICE);
  });

  it("keeps the default gateway port aligned with Rust", () => {
    // Guards all three former hardcode sites: Rust DEFAULT_GATEWAY_PORT plus
    // the shared frontend const consumed by useGatewayMeta and
    // settingsPersistenceModel. Editing either side's 37123 turns this red.
    expect(extractRustNumericConst(settingsDefaultsSource, "DEFAULT_GATEWAY_PORT")).toBe(
      DEFAULT_GATEWAY_PORT
    );
  });

  it("keeps the provider model-name length limit aligned with Rust", () => {
    expect(extractRustNumericConst(providersTypesSource, "MAX_MODEL_NAME_LEN")).toBe(
      MAX_MODEL_NAME_LEN
    );
  });

  it("keeps generated HomeUsagePeriod literals aligned with shared frontend values", () => {
    expect(extractBindingsUnionLiterals(bindingsSource, "HomeUsagePeriod")).toEqual([
      ...HOME_USAGE_PERIOD_VALUES,
    ]);
  });

  it("keeps request detail events gated behind the summary signal path", () => {
    expect(gatewayEventsSource).toContain("emit_request_signal(");
    expect(gatewayEventsSource).toContain("if !should_emit_gateway_detail_event(app) {");
    expect(gatewayEventsSource).toMatch(
      /emit_request_signal\([\s\S]+?if !should_emit_gateway_detail_event\(app\) \{\s+return;\s+\}/
    );
  });

  it("keeps secret-safe upstream proxy fields in the generated settings contract", () => {
    expect(bindingsSource).toContain("codex_oauth_compatible_proxy_mode");
    expect(bindingsSource).toContain("codexOauthCompatibleProxyMode");
    expect(bindingsSource).toContain("upstream_proxy_enabled");
    expect(bindingsSource).toContain("upstream_proxy_url");
    expect(bindingsSource).toContain("upstream_proxy_username");
    expect(bindingsSource).toContain("upstream_proxy_password_configured");
    expect(bindingsSource).toContain("upstreamProxyEnabled");
    expect(bindingsSource).toContain("upstreamProxyUrl");
    expect(bindingsSource).toContain("upstreamProxyUsername");
    expect(bindingsSource).toContain("upstreamProxyPassword: SensitiveStringUpdate | null");
    expect(bindingsSource).toContain("export type SettingsMutationResult");
  });
});
