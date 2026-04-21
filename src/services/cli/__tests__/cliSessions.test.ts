import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import {
  cliSessionsProjectsList,
  cliSessionsSessionsList,
  cliSessionsMessagesGet,
  cliSessionsSessionDelete,
  escapeShellArg,
} from "../cliSessions";

beforeEach(() => {
  vi.restoreAllMocks();
  vi.spyOn(commands, "cliSessionsProjectsList").mockResolvedValue({ status: "ok", data: [] } as any);
  vi.spyOn(commands, "cliSessionsSessionsList").mockResolvedValue({ status: "ok", data: [] } as any);
  vi.spyOn(commands, "cliSessionsMessagesGet").mockResolvedValue({
    status: "ok",
    data: { messages: [], total: 0, page: 0, page_size: 50, has_more: false },
  } as any);
  vi.spyOn(commands, "cliSessionsSessionDelete").mockResolvedValue({ status: "ok", data: [] } as any);
});

describe("services/cli/cliSessions", () => {
  describe("escapeShellArg", () => {
    it("wraps normal string in single quotes (Unix)", () => {
      expect(escapeShellArg("hello")).toBe("'hello'");
    });

    it("handles empty string (Unix)", () => {
      expect(escapeShellArg("")).toBe("''");
    });

    it("escapes single quotes in string (Unix)", () => {
      expect(escapeShellArg("it's")).toBe("'it'\\''s'");
    });

    it("handles Windows platform", () => {
      const originalUA = navigator.userAgent;
      Object.defineProperty(navigator, "userAgent", {
        value: "Mozilla/5.0 (Windows NT 10.0; Win64; x64)",
        configurable: true,
      });

      expect(escapeShellArg("hello")).toBe('"hello"');
      expect(escapeShellArg("")).toBe('""');
      expect(escapeShellArg('say "hi"')).toBe('"say ""hi"""');

      Object.defineProperty(navigator, "userAgent", {
        value: originalUA,
        configurable: true,
      });
    });
  });

  describe("cliSessionsProjectsList", () => {
    it("calls generated command with correct args", async () => {
      await cliSessionsProjectsList("claude");
      expect(commands.cliSessionsProjectsList).toHaveBeenCalledWith("claude", null);
    });
  });

  describe("cliSessionsSessionsList", () => {
    it("calls generated command with correct args", async () => {
      await cliSessionsSessionsList("codex", "proj-1");
      expect(commands.cliSessionsSessionsList).toHaveBeenCalledWith("codex", "proj-1", null);
    });
  });

  describe("cliSessionsMessagesGet", () => {
    it("calls generated command with correct args", async () => {
      await cliSessionsMessagesGet({
        source: "claude",
        file_path: "/path/to/file.json",
        page: 0,
        page_size: 50,
        from_end: true,
      });
      expect(commands.cliSessionsMessagesGet).toHaveBeenCalledWith(
        "claude",
        "/path/to/file.json",
        0,
        50,
        true,
        null
      );
    });
  });

  describe("cliSessionsSessionDelete", () => {
    it("calls generated command with correct args", async () => {
      await cliSessionsSessionDelete({
        source: "claude",
        file_paths: ["/f1.json", "/f2.json"],
      });
      expect(commands.cliSessionsSessionDelete).toHaveBeenCalledWith(
        "claude",
        ["/f1.json", "/f2.json"],
        null
      );
    });

    it("passes wsl_distro when provided", async () => {
      await cliSessionsSessionDelete({
        source: "codex",
        file_paths: ["/f.json"],
        wsl_distro: "Ubuntu",
      });
      expect(commands.cliSessionsSessionDelete).toHaveBeenCalledWith(
        "codex",
        ["/f.json"],
        "Ubuntu"
      );
    });
  });
});
