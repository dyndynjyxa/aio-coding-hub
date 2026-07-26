import { describe, expect, it } from "vitest";
import { GatewayErrorDescriptions } from "../gatewayErrorCodes";

describe("constants/gatewayErrorCodes", () => {
  it("GW_UPSTREAM_TIMEOUT suggestion names the first-byte timeout settings path (R4)", () => {
    const suggestion = GatewayErrorDescriptions.GW_UPSTREAM_TIMEOUT.suggestion;
    expect(suggestion).toContain("首字节超时");
    expect(suggestion).toContain("设置 → 通用");
    expect(suggestion).toContain("0=禁用");
  });
});
