import { describe, expect, it } from "vitest";
import {
  compactWhitespace,
  formatActionFailureToast,
  formatUnknownError,
  normalizeErrorWithCode,
  parseErrorCodeMessage,
} from "../errors";

describe("utils/errors", () => {
  it("formatUnknownError handles common inputs", () => {
    expect(formatUnknownError("x")).toBe("x");
    expect(formatUnknownError(new Error("boom"))).toBe("boom");
    expect(formatUnknownError({ message: " m " })).toBe(" m ");
    expect(formatUnknownError({ x: 1 })).toContain('"x":1');
  });

  it("formatUnknownError handles circular objects and broken toString", () => {
    const circular: any = {};
    circular.self = circular;
    expect(formatUnknownError(circular)).toContain('"self":"[Circular]"');

    const broken: any = {
      toString() {
        throw new Error("boom");
      },
    };
    broken.self = broken;
    expect(formatUnknownError(broken)).toContain('"toString":"[Function]"');
  });

  it("formatUnknownError bounds large strings and structured error objects", () => {
    const long = `ERR_CODE: ${"x".repeat(5000)}`;
    const formatted = formatUnknownError(long);
    expect(formatted.length).toBeLessThan(4200);
    expect(formatted).toContain("[Truncated");

    const structured = formatUnknownError({
      items: Array.from({ length: 40 }, (_, index) => ({
        label: `item-${index}`,
        text: "y".repeat(1000),
      })),
    });

    expect(structured.length).toBeLessThanOrEqual(4140);
    expect(structured).toContain("[Truncated");
    expect(structured).not.toContain("item-39");

    const wide = formatUnknownError(
      Object.fromEntries(
        Array.from({ length: 40 }, (_, index) => [`k${String(index).padStart(2, "0")}`, index])
      )
    );
    expect(wide).toContain("__truncated__");
    expect(wide).not.toContain("k39");
  });

  it("formatUnknownError tolerates unreadable object properties", () => {
    const err: Record<string, unknown> = {};
    Object.defineProperty(err, "bad", {
      enumerable: true,
      get() {
        throw new Error("nope");
      },
    });

    expect(formatUnknownError(err)).toContain('"bad":"[Unreadable]"');
  });

  it("parseErrorCodeMessage parses code prefix", () => {
    expect(parseErrorCodeMessage("GW_UPSTREAM_TIMEOUT: hello")).toEqual({
      error_code: "GW_UPSTREAM_TIMEOUT",
      message: "hello",
    });
    expect(parseErrorCodeMessage("   ")).toEqual({ error_code: null, message: "未知错误" });
    expect(parseErrorCodeMessage("Error:   ")).toEqual({ error_code: null, message: "未知错误" });
    expect(parseErrorCodeMessage("Error: X:  ")).toEqual({
      error_code: "X",
      message: "X:",
    });
    expect(parseErrorCodeMessage("plain")).toEqual({ error_code: null, message: "plain" });
  });

  it("normalizeErrorWithCode compacts whitespace", () => {
    expect(compactWhitespace("  a\n b   c ")).toBe("a b c");
    // Note: parseErrorCodeMessage does not treat multi-line strings as a coded error.
    expect(normalizeErrorWithCode("X: a b").message).toBe("a b");
  });

  it("formatActionFailureToast includes code when present", () => {
    expect(formatActionFailureToast("保存", "X: msg").toast).toBe("保存失败（code X）：msg");
    expect(formatActionFailureToast("保存", "msg").toast).toBe("保存失败：msg");
  });
});
