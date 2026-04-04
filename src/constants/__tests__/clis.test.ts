import { describe, expect, it } from "vitest";
import {
  CLIS,
  CLI_FILTER_ITEMS,
  CLI_FILTER_SHORT_ITEMS,
  CLI_SHORT_ITEMS,
  cliBadgeTone,
  cliBadgeToneStatic,
  cliFromKeyOrDefault,
  cliLongLabel,
  cliShortLabel,
  enabledFlagForCli,
  isCliKey,
} from "../clis";

describe("constants/clis", () => {
  it("exports filter items derived from CLIS", () => {
    expect(CLIS.map((cli) => cli.key)).toEqual(["claude", "codex", "gemini"]);
    expect(CLI_FILTER_ITEMS[0]).toEqual({ key: "all", label: "全部" });
    expect(CLI_FILTER_ITEMS.map((item) => item.key)).toContain("claude");
    expect(CLI_SHORT_ITEMS).toEqual([
      { key: "claude", label: "Claude Code" },
      { key: "codex", label: "Codex" },
      { key: "gemini", label: "Gemini" },
    ]);
    expect(CLI_FILTER_SHORT_ITEMS[0]).toEqual({ key: "all", label: "全部" });
    expect(CLI_FILTER_SHORT_ITEMS.slice(1)).toEqual(CLI_SHORT_ITEMS);
  });

  it("handles key and label helpers", () => {
    expect(isCliKey("claude")).toBe(true);
    expect(isCliKey("not-a-cli")).toBe(false);
    expect(isCliKey(123)).toBe(false);

    expect(cliLongLabel("codex")).toBe("Codex");
    expect(cliLongLabel("unknown-cli")).toBe("unknown-cli");

    expect(cliFromKeyOrDefault(null).key).toBe("claude");
    expect(cliFromKeyOrDefault("not-a-cli").key).toBe("claude");
    expect(cliFromKeyOrDefault("gemini").key).toBe("gemini");

    const row: any = { enabled_claude: true, enabled_codex: false, enabled_gemini: true };
    expect(enabledFlagForCli(row, "claude" as any)).toBe(true);
    expect(enabledFlagForCli(row, "codex" as any)).toBe(false);

    expect(cliShortLabel("claude")).toBe("Claude Code");
    expect(cliShortLabel("codex")).toBe("Codex");
    expect(cliShortLabel("gemini")).toBe("Gemini");
    expect(cliShortLabel("other")).toBe("other");

    expect(cliBadgeTone("claude")).toContain("bg-slate-100");
    expect(cliBadgeTone("claude")).toContain("group-hover:bg-white");
    expect(cliBadgeTone("codex")).toContain("bg-slate-100");
    expect(cliBadgeTone("gemini")).toContain("bg-slate-100");
    expect(cliBadgeTone("unknown")).toContain("bg-slate-100");
    expect(cliBadgeTone("unknown")).not.toContain("group-hover");

    expect(cliBadgeToneStatic("claude")).toContain("bg-slate-100");
    expect(cliBadgeToneStatic("claude")).not.toContain("group-hover");
    expect(cliBadgeToneStatic("unknown")).toContain("bg-slate-100");
  });
});
