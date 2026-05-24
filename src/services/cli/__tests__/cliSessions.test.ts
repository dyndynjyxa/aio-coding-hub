import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import {
  CLI_SESSIONS_DEFAULT_PAGE_SIZE,
  CLI_SESSIONS_MAX_DELETE_PATHS,
  CLI_SESSIONS_MAX_LOOKUP_ITEMS,
  CLI_SESSIONS_MAX_PAGE_SIZE,
  CLI_SESSIONS_MAX_PATH_CHARS,
  CLI_SESSIONS_MAX_TAIL_MESSAGES,
  CLI_SESSIONS_WSL_DISTRO_MAX_CHARS,
  type CliSessionsFolderLookupEntry,
  type CliSessionsPaginatedMessages,
  type CliSessionsProjectSummary,
  type CliSessionsSessionSummary,
  cliSessionsFolderLookupByIds,
  cliSessionsProjectsList,
  cliSessionsSessionsList,
  cliSessionsMessagesGet,
  cliSessionsSessionDelete,
  normalizeCliSessionsDeleteFilePaths,
  normalizeCliSessionsFilePath,
  normalizeCliSessionsFolderLookupItems,
  normalizeCliSessionsPage,
  normalizeCliSessionsPageSize,
  normalizeCliSessionsProjectId,
  normalizeCliSessionsWslDistro,
  validateCliSessionsMessageWindow,
  escapeShellArg,
} from "../cliSessions";

function makeCliSessionsProjectSummary(
  overrides: Partial<CliSessionsProjectSummary> = {}
): CliSessionsProjectSummary {
  return {
    source: "claude",
    id: "proj-1",
    display_path: "/tmp/project",
    short_name: "project",
    session_count: 1,
    last_modified: null,
    model_provider: null,
    wsl_distro: null,
    ...overrides,
  };
}

function makeCliSessionsSessionSummary(
  overrides: Partial<CliSessionsSessionSummary> = {}
): CliSessionsSessionSummary {
  return {
    source: "claude",
    session_id: "sess-1",
    file_path: "/tmp/session.json",
    first_prompt: null,
    message_count: 0,
    created_at: null,
    modified_at: null,
    git_branch: null,
    project_path: null,
    is_sidechain: null,
    cwd: null,
    model_provider: null,
    cli_version: null,
    wsl_distro: null,
    ...overrides,
  };
}

function makeCliSessionsPaginatedMessages(
  overrides: Partial<CliSessionsPaginatedMessages> = {}
): CliSessionsPaginatedMessages {
  return {
    messages: [],
    total: 0,
    page: 0,
    page_size: 50,
    has_more: false,
    ...overrides,
  };
}

function makeCliSessionsFolderLookupEntry(
  overrides: Partial<CliSessionsFolderLookupEntry> = {}
): CliSessionsFolderLookupEntry {
  return {
    source: "claude",
    session_id: "s1",
    folder_name: "project",
    folder_path: "/tmp/project",
    ...overrides,
  };
}

beforeEach(() => {
  vi.restoreAllMocks();
  vi.spyOn(commands, "cliSessionsProjectsList").mockResolvedValue({
    status: "ok",
    data: [makeCliSessionsProjectSummary()],
  });
  vi.spyOn(commands, "cliSessionsSessionsList").mockResolvedValue({
    status: "ok",
    data: [makeCliSessionsSessionSummary()],
  });
  vi.spyOn(commands, "cliSessionsMessagesGet").mockResolvedValue({
    status: "ok",
    data: makeCliSessionsPaginatedMessages(),
  });
  vi.spyOn(commands, "cliSessionsSessionDelete").mockResolvedValue({ status: "ok", data: [] });
  vi.spyOn(commands, "cliSessionsFolderLookupByIds").mockResolvedValue({
    status: "ok",
    data: [makeCliSessionsFolderLookupEntry()],
  });
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

    it("normalizes wslDistro before generated ipc", async () => {
      expect(normalizeCliSessionsWslDistro("  Ubuntu  ")).toBe("Ubuntu");
      expect(normalizeCliSessionsWslDistro("   ")).toBeNull();

      await cliSessionsProjectsList("claude", "  Ubuntu  ");
      await cliSessionsProjectsList("claude", "   ");

      expect(commands.cliSessionsProjectsList).toHaveBeenNthCalledWith(1, "claude", "Ubuntu");
      expect(commands.cliSessionsProjectsList).toHaveBeenNthCalledWith(2, "claude", null);
    });

    it("rejects invalid wslDistro before generated ipc", async () => {
      vi.mocked(commands.cliSessionsProjectsList).mockClear();

      expect(() => normalizeCliSessionsWslDistro("Ubu\nntu")).toThrow("SEC_INVALID_INPUT");
      expect(() =>
        normalizeCliSessionsWslDistro("x".repeat(CLI_SESSIONS_WSL_DISTRO_MAX_CHARS + 1))
      ).toThrow("SEC_INVALID_INPUT");
      await expect(cliSessionsProjectsList("claude", "Ubu\nntu")).rejects.toThrow(
        "SEC_INVALID_INPUT"
      );

      expect(commands.cliSessionsProjectsList).not.toHaveBeenCalled();
    });
  });

  describe("cliSessionsSessionsList", () => {
    it("calls generated command with correct args", async () => {
      await cliSessionsSessionsList("codex", "proj-1");
      expect(commands.cliSessionsSessionsList).toHaveBeenCalledWith("codex", "proj-1", null);
    });

    it("trims projectId before generated ipc", async () => {
      expect(normalizeCliSessionsProjectId("  proj-1  ")).toBe("proj-1");

      await cliSessionsSessionsList("codex", "  proj-1  ");
      expect(commands.cliSessionsSessionsList).toHaveBeenCalledWith("codex", "proj-1", null);
    });

    it("normalizes wslDistro before session-list ipc", async () => {
      await cliSessionsSessionsList("codex", "proj-1", "  Ubuntu  ");

      expect(commands.cliSessionsSessionsList).toHaveBeenCalledWith("codex", "proj-1", "Ubuntu");
    });

    it("rejects empty projectId before generated ipc", async () => {
      vi.mocked(commands.cliSessionsSessionsList).mockClear();

      await expect(cliSessionsSessionsList("codex", "   ")).rejects.toThrow("SEC_INVALID_INPUT");
      await expect(
        cliSessionsSessionsList("codex", "x".repeat(CLI_SESSIONS_MAX_PATH_CHARS + 1))
      ).rejects.toThrow("projectId is too long");

      expect(commands.cliSessionsSessionsList).not.toHaveBeenCalled();
    });
  });

  describe("cliSessionsMessagesGet", () => {
    it("calls generated command with correct args", async () => {
      await cliSessionsMessagesGet({
        source: "claude",
        filePath: "/path/to/file.json",
        page: 0,
        pageSize: 50,
        fromEnd: true,
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

    it("normalizes page and pageSize before generated ipc", async () => {
      expect(normalizeCliSessionsFilePath("  /path/to/file.json  ")).toBe("/path/to/file.json");
      expect(normalizeCliSessionsPage(0)).toBe(0);
      expect(normalizeCliSessionsPageSize(0)).toBe(CLI_SESSIONS_DEFAULT_PAGE_SIZE);
      expect(normalizeCliSessionsPageSize(999)).toBe(CLI_SESSIONS_MAX_PAGE_SIZE);
      expect(() =>
        validateCliSessionsMessageWindow(40, CLI_SESSIONS_DEFAULT_PAGE_SIZE, true)
      ).toThrow(`max ${CLI_SESSIONS_MAX_TAIL_MESSAGES}`);
      expect(() =>
        validateCliSessionsMessageWindow(40, CLI_SESSIONS_DEFAULT_PAGE_SIZE, false)
      ).not.toThrow();

      await cliSessionsMessagesGet({
        source: "claude",
        filePath: "  /path/to/file.json  ",
        page: 0,
        pageSize: 0,
        fromEnd: true,
      });
      await cliSessionsMessagesGet({
        source: "claude",
        filePath: "/path/to/file.json",
        page: 1,
        pageSize: 999,
        fromEnd: false,
      });

      expect(commands.cliSessionsMessagesGet).toHaveBeenNthCalledWith(
        1,
        "claude",
        "/path/to/file.json",
        0,
        CLI_SESSIONS_DEFAULT_PAGE_SIZE,
        true,
        null
      );
      expect(commands.cliSessionsMessagesGet).toHaveBeenNthCalledWith(
        2,
        "claude",
        "/path/to/file.json",
        1,
        CLI_SESSIONS_MAX_PAGE_SIZE,
        false,
        null
      );
    });

    it("normalizes wslDistro before message ipc", async () => {
      await cliSessionsMessagesGet({
        source: "claude",
        filePath: "/path/to/file.json",
        page: 0,
        pageSize: 50,
        fromEnd: true,
        wslDistro: "  Ubuntu  ",
      });

      expect(commands.cliSessionsMessagesGet).toHaveBeenCalledWith(
        "claude",
        "/path/to/file.json",
        0,
        50,
        true,
        "Ubuntu"
      );
    });

    it("rejects invalid message pagination before generated ipc", async () => {
      vi.mocked(commands.cliSessionsMessagesGet).mockClear();

      await expect(
        cliSessionsMessagesGet({
          source: "claude",
          filePath: "/path/to/file.json",
          page: -1,
          pageSize: 50,
          fromEnd: true,
        })
      ).rejects.toThrow("SEC_INVALID_INPUT");
      await expect(
        cliSessionsMessagesGet({
          source: "claude",
          filePath: "/path/to/file.json",
          page: 0,
          pageSize: 1.5,
          fromEnd: true,
        })
      ).rejects.toThrow("SEC_INVALID_INPUT");
      await expect(
        cliSessionsMessagesGet({
          source: "claude",
          filePath: "   ",
          page: 0,
          pageSize: 50,
          fromEnd: true,
        })
      ).rejects.toThrow("SEC_INVALID_INPUT");
      await expect(
        cliSessionsMessagesGet({
          source: "claude",
          filePath: "x".repeat(CLI_SESSIONS_MAX_PATH_CHARS + 1),
          page: 0,
          pageSize: 50,
          fromEnd: true,
        })
      ).rejects.toThrow("filePath is too long");
      await expect(
        cliSessionsMessagesGet({
          source: "claude",
          filePath: "/path/to/file.json",
          page: 40,
          pageSize: 50,
          fromEnd: true,
        })
      ).rejects.toThrow("message pagination window is too large");

      expect(commands.cliSessionsMessagesGet).not.toHaveBeenCalled();
    });
  });

  describe("cliSessionsSessionDelete", () => {
    it("calls generated command with correct args", async () => {
      await cliSessionsSessionDelete({
        source: "claude",
        filePaths: ["/f1.json", "/f2.json"],
      });
      expect(commands.cliSessionsSessionDelete).toHaveBeenCalledWith(
        "claude",
        ["/f1.json", "/f2.json"],
        null
      );
    });

    it("normalizes wsl_distro when provided", async () => {
      await cliSessionsSessionDelete({
        source: "codex",
        filePaths: ["/f.json"],
        wslDistro: "  Ubuntu  ",
      });
      expect(commands.cliSessionsSessionDelete).toHaveBeenCalledWith(
        "codex",
        ["/f.json"],
        "Ubuntu"
      );
    });

    it("trims delete file paths and filters empty values before generated ipc", async () => {
      expect(normalizeCliSessionsDeleteFilePaths([" /f1.json ", "   ", "/f2.json"])).toEqual([
        "/f1.json",
        "/f2.json",
      ]);

      await cliSessionsSessionDelete({
        source: "claude",
        filePaths: [" /f1.json ", "   ", "/f2.json"],
      });

      expect(commands.cliSessionsSessionDelete).toHaveBeenCalledWith(
        "claude",
        ["/f1.json", "/f2.json"],
        null
      );
    });

    it("rejects empty and oversized delete batches before generated ipc", async () => {
      vi.mocked(commands.cliSessionsSessionDelete).mockClear();

      await expect(
        cliSessionsSessionDelete({
          source: "claude",
          filePaths: [],
        })
      ).rejects.toThrow("SEC_INVALID_INPUT");
      await expect(
        cliSessionsSessionDelete({
          source: "claude",
          filePaths: ["   "],
        })
      ).rejects.toThrow("SEC_INVALID_INPUT");
      await expect(
        cliSessionsSessionDelete({
          source: "claude",
          filePaths: Array.from(
            { length: CLI_SESSIONS_MAX_DELETE_PATHS + 1 },
            (_, index) => `/f${index}.json`
          ),
        })
      ).rejects.toThrow("filePaths must contain at most");

      expect(commands.cliSessionsSessionDelete).not.toHaveBeenCalled();
    });
  });

  describe("cliSessionsFolderLookupByIds", () => {
    it("passes generated lookup items without any-casts", async () => {
      await cliSessionsFolderLookupByIds([{ source: "claude", session_id: "s1" }], " Ubuntu ");
      expect(commands.cliSessionsFolderLookupByIds).toHaveBeenCalledWith(
        [{ source: "claude", session_id: "s1" }],
        "Ubuntu"
      );
    });

    it("trims lookup session ids and returns empty lookup locally", async () => {
      expect(
        normalizeCliSessionsFolderLookupItems([
          { source: "claude", session_id: " s1 " },
          { source: "codex", session_id: "   " },
        ])
      ).toEqual([{ source: "claude", session_id: "s1" }]);

      await cliSessionsFolderLookupByIds(
        [
          { source: "claude", session_id: " s1 " },
          { source: "codex", session_id: "   " },
        ],
        "Ubuntu"
      );
      expect(commands.cliSessionsFolderLookupByIds).toHaveBeenCalledWith(
        [{ source: "claude", session_id: "s1" }],
        "Ubuntu"
      );

      vi.mocked(commands.cliSessionsFolderLookupByIds).mockClear();
      await expect(
        cliSessionsFolderLookupByIds([{ source: "claude", session_id: "   " }])
      ).resolves.toEqual([]);
      expect(commands.cliSessionsFolderLookupByIds).not.toHaveBeenCalled();
    });

    it("rejects oversized lookup batches before generated ipc", async () => {
      vi.mocked(commands.cliSessionsFolderLookupByIds).mockClear();

      await expect(
        cliSessionsFolderLookupByIds(
          Array.from({ length: CLI_SESSIONS_MAX_LOOKUP_ITEMS + 1 }, (_, index) => ({
            source: "claude",
            session_id: `s${index}`,
          }))
        )
      ).rejects.toThrow("folder lookup items must contain at most");
      await expect(
        cliSessionsFolderLookupByIds([
          { source: "claude", session_id: "x".repeat(CLI_SESSIONS_MAX_PATH_CHARS + 1) },
        ])
      ).rejects.toThrow("sessionId is too long");

      expect(commands.cliSessionsFolderLookupByIds).not.toHaveBeenCalled();
    });
  });
});
