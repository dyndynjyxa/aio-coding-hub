// Usage:
// - Canonical app-level (non-gateway) error-code constants mirrored from the Rust backend.
// - Frontend must match backend errors by code (via parseErrorCodeMessage), never by message text.
// - Rust-side presence of each code is guarded by src/constants/__tests__/crossLayerContracts.test.ts.

export const AppErrorCodes = {
  PROMPT_NAME_REQUIRED: "PROMPT_NAME_REQUIRED",
  PROMPT_NAME_CONFLICT: "PROMPT_NAME_CONFLICT",
  SETTINGS_RECOVERY_REQUIRED: "SETTINGS_RECOVERY_REQUIRED",
  DB_CONSTRAINT: "DB_CONSTRAINT",
  SEC_INVALID_INPUT: "SEC_INVALID_INPUT",
} as const;

export type AppErrorCode = (typeof AppErrorCodes)[keyof typeof AppErrorCodes];
