export const gatewayEventNames = {
  status: "gateway:status",
  requestSignal: "gateway:request_signal",
  requestStart: "gateway:request_start",
  attempt: "gateway:attempt",
  request: "gateway:request",
  log: "gateway:log",
  circuit: "gateway:circuit",
} as const;

export type GatewayEventName = (typeof gatewayEventNames)[keyof typeof gatewayEventNames];

/**
 * Character caps applied to gateway event payload fields, mirroring the Rust
 * emitter's EVENT_*_MAX_CHARS (src-tauri/src/gateway/events.rs). ID_MAX_LENGTH
 * is a frontend-only validation bound (the Rust side does not truncate ids).
 * Kept in sync by src/constants/__tests__/crossLayerContracts.test.ts.
 */
export const GATEWAY_EVENT_TEXT_LIMITS = {
  ID_MAX_LENGTH: 256,
  METHOD_MAX_LENGTH: 32,
  STATE_MAX_LENGTH: 64,
  SHORT_TEXT_MAX_LENGTH: 512,
  PATH_MAX_LENGTH: 2048,
  QUERY_MAX_LENGTH: 4096,
  URL_MAX_LENGTH: 2048,
} as const;
