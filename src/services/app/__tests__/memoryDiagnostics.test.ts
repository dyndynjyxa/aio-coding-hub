import { beforeEach, describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { queryClient } from "../../../query/queryClient";
import { logToConsole } from "../../consoleLog";
import { appMemoryDiagnosticsGet, collectAppMemoryDiagnostics } from "../memoryDiagnostics";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      appMemoryDiagnosticsGet: vi.fn(),
    },
  };
});

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return {
    ...actual,
    logToConsole: vi.fn(),
  };
});

const backendSnapshot = {
  generated_at_unix: 1,
  app_data_dir: "/tmp/aio",
  db: {
    path: "/tmp/aio/app.db",
    exists: true,
    db_bytes: 10,
    wal_bytes: 2,
    shm_bytes: 1,
  },
  prompt_stats: {
    count: 0,
    total_content_len: 0,
    max_content_len: 0,
    top_items: [],
  },
  cli_sessions: [],
};

describe("services/app/memoryDiagnostics", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    queryClient.clear();
    vi.mocked(commands.appMemoryDiagnosticsGet).mockResolvedValue({
      status: "ok",
      data: backendSnapshot,
    } as any);
  });

  it("reads backend diagnostics through the generated IPC command", async () => {
    await expect(appMemoryDiagnosticsGet()).resolves.toEqual(backendSnapshot);

    expect(commands.appMemoryDiagnosticsGet).toHaveBeenCalledWith();
  });

  it("collects frontend query diagnostics and logs the report", async () => {
    const circular: Record<string, unknown> = { label: "root" };
    circular.self = circular;
    queryClient.setQueryData(["providers", { cli: "claude" }], {
      rows: [circular, true, 7, BigInt(9), null],
    });
    queryClient.setQueryData([123, "unknown group"], "small");

    const report = await collectAppMemoryDiagnostics();

    expect(report.backend).toEqual(backendSnapshot);
    expect(report.frontend.query_count).toBe(2);
    expect(report.frontend.query_estimated_bytes).toBeGreaterThan(0);
    expect(report.frontend.query_groups).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ key: "providers", count: 1 }),
        expect.objectContaining({ key: "unknown", count: 1 }),
      ])
    );
    expect(report.frontend.top_queries).toHaveLength(2);
    expect(report.frontend.top_queries[0]).toEqual(
      expect.objectContaining({
        query_key: expect.stringContaining("providers"),
        estimated_bytes: expect.any(Number),
        observers: 0,
      })
    );
    expect(logToConsole).toHaveBeenCalledWith("info", expect.any(String), report);
  });
});
