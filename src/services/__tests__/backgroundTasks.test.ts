import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  emitBackgroundTaskVisibilityTrigger,
  registerBackgroundTask,
  resetBackgroundTaskSchedulerForTests,
  runBackgroundTask,
  setBackgroundTaskSchedulerForeground,
  startBackgroundTaskScheduler,
} from "../backgroundTasks";
import { backgroundTaskVisibilityTriggers } from "../../constants/backgroundTaskContracts";

vi.mock("../consoleLog", () => ({ logToConsole: vi.fn() }));

describe("services/backgroundTasks", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    resetBackgroundTaskSchedulerForTests();
  });

  it("runs startup tasks and foreground intervals independently", async () => {
    const proxyRun = vi.fn().mockResolvedValue(undefined);
    const updateRun = vi.fn().mockResolvedValue(undefined);

    registerBackgroundTask({
      taskId: "proxy",
      intervalMs: 15_000,
      runOnAppStart: true,
      foregroundOnly: true,
      run: proxyRun,
    });
    registerBackgroundTask({
      taskId: "update",
      intervalMs: 300_000,
      runOnAppStart: true,
      foregroundOnly: true,
      run: updateRun,
    });

    startBackgroundTaskScheduler();
    await Promise.resolve();

    expect(proxyRun).toHaveBeenCalledTimes(1);
    expect(updateRun).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(15_000);
    expect(proxyRun).toHaveBeenCalledTimes(2);
    expect(updateRun).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(285_000);
    expect(updateRun).toHaveBeenCalledTimes(2);
  });

  it("pauses foreground tasks when app is hidden and resumes when visible", async () => {
    const run = vi.fn().mockResolvedValue(undefined);
    registerBackgroundTask({
      taskId: "proxy",
      intervalMs: 15_000,
      runOnAppStart: false,
      foregroundOnly: true,
      run,
    });

    startBackgroundTaskScheduler();
    setBackgroundTaskSchedulerForeground(false);
    await vi.advanceTimersByTimeAsync(30_000);
    expect(run).not.toHaveBeenCalled();

    setBackgroundTaskSchedulerForeground(true);
    await vi.advanceTimersByTimeAsync(15_000);
    expect(run).toHaveBeenCalledTimes(1);
  });

  it("runs triggered tasks immediately and prevents concurrent re-entry", async () => {
    let resolveRun: (() => void) | undefined;
    const run = vi.fn().mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          resolveRun = resolve;
        })
    );

    registerBackgroundTask({
      taskId: "proxy",
      intervalMs: null,
      runOnAppStart: false,
      foregroundOnly: true,
      visibilityTriggers: [backgroundTaskVisibilityTriggers.homeOverviewVisible],
      run,
    });

    startBackgroundTaskScheduler();
    const firstTrigger = emitBackgroundTaskVisibilityTrigger(
      backgroundTaskVisibilityTriggers.homeOverviewVisible
    );
    const secondTrigger = emitBackgroundTaskVisibilityTrigger(
      backgroundTaskVisibilityTriggers.homeOverviewVisible
    );
    await Promise.resolve();
    expect(run).toHaveBeenCalledTimes(1);

    resolveRun?.();
    await firstTrigger;
    await secondTrigger;

    const thirdTrigger = emitBackgroundTaskVisibilityTrigger(
      backgroundTaskVisibilityTriggers.homeOverviewVisible
    );
    await Promise.resolve();
    expect(run).toHaveBeenCalledTimes(2);

    resolveRun?.();
    await thirdTrigger;
  });

  it("queues one manual rerun while a task is already running", async () => {
    let resolveRun: (() => void) | undefined;
    const run = vi.fn().mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          resolveRun = resolve;
        })
    );

    registerBackgroundTask({
      taskId: "update",
      intervalMs: null,
      runOnAppStart: false,
      foregroundOnly: true,
      run,
    });

    startBackgroundTaskScheduler();

    const firstRun = runBackgroundTask("update", { trigger: "interval" });
    await Promise.resolve();
    expect(run).toHaveBeenCalledTimes(1);

    const queuedManualRun = runBackgroundTask("update", {
      trigger: "manual",
      payload: { source: "settings" },
    });
    await Promise.resolve();
    expect(run).toHaveBeenCalledTimes(1);

    resolveRun?.();
    await vi.waitFor(() => {
      expect(run).toHaveBeenCalledTimes(2);
    });

    resolveRun?.();
    await firstRun;
    await queuedManualRun;
  });
});
