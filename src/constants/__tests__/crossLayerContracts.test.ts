import { describe, expect, it } from "vitest";
import { appEventNames } from "../appEvents";
import { gatewayEventNames } from "../gatewayEvents";
import { GatewayErrorCodes } from "../gatewayErrorCodes";
import { HOME_USAGE_PERIOD_VALUES } from "../homeUsagePeriods";
import bindingsSource from "../../generated/bindings.ts?raw";
import heartbeatSource from "../../../src-tauri/src/app/heartbeat_watchdog.rs?raw";
import noticeSource from "../../../src-tauri/src/app/notice.rs?raw";
import startupStateSource from "../../../src-tauri/src/app/startup_state.rs?raw";
import gatewayEventsSource from "../../../src-tauri/src/gateway/events.rs?raw";
import gatewayErrorCodeSource from "../../../src-tauri/src/gateway/proxy/error_code.rs?raw";

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
