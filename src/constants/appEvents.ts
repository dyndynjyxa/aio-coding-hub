export const appEventNames = {
  heartbeat: "app:heartbeat",
  notice: "notice:notify",
  providerCodexCatalog: "providers:codex_catalog",
  startupStatus: "app:startup_status",
} as const;

export type AppEventName = (typeof appEventNames)[keyof typeof appEventNames];
