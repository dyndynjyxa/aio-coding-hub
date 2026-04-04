import { describe, expect, it } from "vitest";
import {
  DEFAULT_CLI_PRIORITY_ORDER,
  getOrderedClis,
  normalizeCliPriorityOrder,
  pickDefaultCliByPriority,
} from "../cliPriorityOrder";

describe("services/cliPriorityOrder", () => {
  it("normalizes invalid, duplicate, and missing CLI keys", () => {
    expect(normalizeCliPriorityOrder(["codex", "unknown", "codex", "claude"] as unknown[])).toEqual(
      ["codex", "claude", "gemini"]
    );
  });

  it("returns the default order when input is missing", () => {
    expect(normalizeCliPriorityOrder(undefined)).toEqual(DEFAULT_CLI_PRIORITY_ORDER);
  });

  it("filters ordered CLI items by allowed subset", () => {
    expect(
      getOrderedClis(["gemini", "claude", "codex"], ["claude", "codex"]).map((cli) => cli.key)
    ).toEqual(["claude", "codex"]);
  });

  it("picks the highest-priority CLI inside the allowed subset", () => {
    expect(pickDefaultCliByPriority(["gemini", "codex", "claude"], ["claude", "codex"])).toBe(
      "codex"
    );
  });
});
