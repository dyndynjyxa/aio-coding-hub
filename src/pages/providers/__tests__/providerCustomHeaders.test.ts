import { describe, expect, it } from "vitest";
import {
  isProtectedCustomHeaderName,
  isValidCustomHeaderName,
  normalizeCustomHeaders,
  validateCustomHeaders,
} from "../providerCustomHeaders";

describe("providerCustomHeaders", () => {
  it("trims names/values and drops rows with empty names", () => {
    expect(
      normalizeCustomHeaders([
        { name: "  X-User-Id  ", value: "  42  " },
        { name: "   ", value: "ignored" },
      ])
    ).toEqual([{ name: "X-User-Id", value: "42" }]);
  });

  it("dedupes by case-insensitive name keeping the last occurrence", () => {
    expect(
      normalizeCustomHeaders([
        { name: "X-User-Id", value: "first" },
        { name: "x-user-id", value: "second" },
      ])
    ).toEqual([{ name: "x-user-id", value: "second" }]);
  });

  it("accepts RFC 7230 token names and rejects names with separators", () => {
    expect(isValidCustomHeaderName("X-Tenant-Id")).toBe(true);
    expect(isValidCustomHeaderName("X User Id")).toBe(false);
    expect(isValidCustomHeaderName("bad:name")).toBe(false);
  });

  it("flags protected headers managed by the gateway", () => {
    expect(isProtectedCustomHeaderName("Host")).toBe(true);
    expect(isProtectedCustomHeaderName("content-length")).toBe(true);
    expect(isProtectedCustomHeaderName("X-Domain")).toBe(false);
  });

  it("validates rows and returns the first error message", () => {
    expect(validateCustomHeaders([{ name: "", value: "" }])).toBeNull();
    expect(validateCustomHeaders([{ name: "", value: "v" }])).toBe("请求头名称不能为空");
    expect(validateCustomHeaders([{ name: "bad name", value: "v" }])).toContain("无效");
    expect(validateCustomHeaders([{ name: "Host", value: "v" }])).toContain("网关管理");
    expect(validateCustomHeaders([{ name: "X-Domain", value: "v" }])).toBeNull();
  });
});
