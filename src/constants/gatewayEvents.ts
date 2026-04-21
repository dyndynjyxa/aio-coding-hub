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
