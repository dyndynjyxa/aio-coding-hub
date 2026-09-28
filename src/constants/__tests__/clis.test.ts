import { describe, expect, it } from "vitest";
import {
  CLIS,
  CLI_KEYS,
  CLI_REGISTRY,
  CLI_FILTER_ITEMS,
  CLI_FILTER_SHORT_ITEMS,
  CLI_SHORT_ITEMS,
  cliBadgeTone,
  cliBadgeToneStatic,
  cliFilterItemsWith,
  cliFromKeyOrDefault,
  cliKeysWith,
  cliLongLabel,
  cliShortLabel,
  enabledFlagForCli,
  isCliKey,
} from "../clis";

describe("constants/clis", () => {
  it("exports filter items derived from CLIS", () => {
    expect(CLI_KEYS).toEqual(["claude", "codex", "gemini", "grok"]);
    expect(CLIS.map((cli) => cli.key)).toEqual(CLI_KEYS);
    expect(CLI_FILTER_ITEMS[0]).toEqual({ key: "all", label: "全部" });
    expect(CLI_FILTER_ITEMS.map((item) => item.key)).toContain("claude");
    expect(CLI_SHORT_ITEMS).toEqual([
      { key: "claude", label: "Claude" },
      { key: "codex", label: "Codex" },
      { key: "gemini", label: "Gemini" },
      { key: "grok", label: "Grok" },
    ]);
    expect(CLI_FILTER_SHORT_ITEMS[0]).toEqual({ key: "all", label: "全部" });
    expect(CLI_FILTER_SHORT_ITEMS.slice(1)).toEqual(CLI_SHORT_ITEMS);
  });

  it("keeps Grok inside supported capabilities and outside excluded capabilities", () => {
    const grok = CLI_REGISTRY.find((cli) => cli.key === "grok");
    expect(grok?.capabilities).toEqual({
      gateway: true,
      provider: true,
      logs: true,
      usage: true,
      pricing: true,
      cliProxy: true,
      cliManager: true,
      mcp: true,
      skills: true,
      prompts: true,
      workspaces: true,
      wsl: false,
      managedUpdate: false,
      providerPluginTarget: false,
    });
    const legacyCapabilities = {
      gateway: true,
      provider: true,
      logs: true,
      usage: true,
      pricing: true,
      cliProxy: true,
      cliManager: true,
      mcp: true,
      skills: true,
      prompts: true,
      workspaces: true,
      wsl: true,
      managedUpdate: true,
      providerPluginTarget: true,
    };
    for (const cliKey of ["claude", "codex", "gemini"] as const) {
      expect(CLI_REGISTRY.find((cli) => cli.key === cliKey)?.capabilities).toEqual(
        legacyCapabilities
      );
    }
    expect(cliKeysWith("provider")).toEqual(["claude", "codex", "gemini", "grok"]);
    expect(cliKeysWith("wsl")).toEqual(["claude", "codex", "gemini"]);
    expect(cliFilterItemsWith("usage").map((item) => item.key)).toEqual([
      "all",
      "claude",
      "codex",
      "gemini",
      "grok",
    ]);
  });

  it("handles key and label helpers", () => {
    expect(isCliKey("claude")).toBe(true);
    expect(isCliKey("grok")).toBe(true);
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
    if (false) {
      // @ts-expect-error Grok uses workspace relations, not the legacy enabled_* columns.
      enabledFlagForCli(row, "grok");
    }

    expect(cliShortLabel("claude")).toBe("Claude");
    expect(cliShortLabel("codex")).toBe("Codex");
    expect(cliShortLabel("gemini")).toBe("Gemini");
    expect(cliShortLabel("grok")).toBe("Grok");
    expect(cliShortLabel("other")).toBe("other");

    expect(cliBadgeTone("claude")).toContain("bg-slate-100");
    expect(cliBadgeTone("claude")).toContain("border-slate-200/90");
    expect(cliBadgeTone("claude")).toContain("group-hover:bg-white");
    expect(cliBadgeTone("codex")).toContain("bg-slate-100");
    expect(cliBadgeTone("gemini")).toContain("bg-slate-100");
    expect(cliBadgeTone("unknown")).toContain("bg-slate-100");
    expect(cliBadgeTone("unknown")).not.toContain("group-hover");

    expect(cliBadgeToneStatic("claude")).toContain("bg-slate-100");
    expect(cliBadgeToneStatic("claude")).toContain("border-slate-200/90");
    expect(cliBadgeToneStatic("claude")).not.toContain("group-hover");
    expect(cliBadgeToneStatic("unknown")).toContain("bg-slate-100");
  });
});
