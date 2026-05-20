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

  it("does not reschedule foreground intervals when foreground state is unchanged", () => {
    const setIntervalSpy = vi.spyOn(globalThis, "setInterval");
    const clearIntervalSpy = vi.spyOn(globalThis, "clearInterval");
    const run = vi.fn().mockResolvedValue(undefined);

    registerBackgroundTask({
      taskId: "proxy",
      intervalMs: 15_000,
      runOnAppStart: false,
      foregroundOnly: true,
      run,
    });

    startBackgroundTaskScheduler();
    expect(setIntervalSpy).toHaveBeenCalledTimes(1);

    setBackgroundTaskSchedulerForeground(true);
    expect(setIntervalSpy).toHaveBeenCalledTimes(1);
    expect(clearIntervalSpy).not.toHaveBeenCalled();

    setBackgroundTaskSchedulerForeground(false);
    expect(clearIntervalSpy).toHaveBeenCalledTimes(1);

    setBackgroundTaskSchedulerForeground(false);
    expect(setIntervalSpy).toHaveBeenCalledTimes(1);
    expect(clearIntervalSpy).toHaveBeenCalledTimes(1);

    setBackgroundTaskSchedulerForeground(true);
    expect(setIntervalSpy).toHaveBeenCalledTimes(2);

    setBackgroundTaskSchedulerForeground(true);
    expect(setIntervalSpy).toHaveBeenCalledTimes(2);
    expect(clearIntervalSpy).toHaveBeenCalledTimes(1);

    setIntervalSpy.mockRestore();
    clearIntervalSpy.mockRestore();
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

  it("does not let a stale unregister callback remove a replacement task", async () => {
    const firstRun = vi.fn().mockResolvedValue(undefined);
    const secondRun = vi.fn().mockResolvedValue(undefined);

    const unregisterFirst = registerBackgroundTask({
      taskId: "refresh",
      intervalMs: null,
      runOnAppStart: true,
      run: firstRun,
    });

    registerBackgroundTask({
      taskId: "refresh",
      intervalMs: null,
      runOnAppStart: true,
      run: secondRun,
    });

    unregisterFirst();
    startBackgroundTaskScheduler();
    await Promise.resolve();

    expect(firstRun).not.toHaveBeenCalled();
    expect(secondRun).toHaveBeenCalledTimes(1);
  });

  it("does not let a stale running task finalizer clear a newly registered task run", async () => {
    let resolveOldRun: (() => void) | undefined;
    const oldRun = vi.fn().mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          resolveOldRun = resolve;
        })
    );
    const newRunResolvers: Array<() => void> = [];
    const newRun = vi.fn().mockImplementation(
      () =>
        new Promise<void>((resolve) => {
          newRunResolvers.push(resolve);
        })
    );

    const unregisterOld = registerBackgroundTask({
      taskId: "refresh",
      intervalMs: null,
      runOnAppStart: false,
      run: oldRun,
    });

    const oldRunPromise = runBackgroundTask("refresh");
    await Promise.resolve();
    expect(oldRun).toHaveBeenCalledTimes(1);

    unregisterOld();
    registerBackgroundTask({
      taskId: "refresh",
      intervalMs: null,
      runOnAppStart: false,
      run: newRun,
    });

    const firstNewRun = runBackgroundTask("refresh");
    await Promise.resolve();
    expect(newRun).toHaveBeenCalledTimes(1);

    resolveOldRun?.();
    await oldRunPromise;

    const queuedNewRun = runBackgroundTask("refresh");
    await Promise.resolve();
    expect(newRun).toHaveBeenCalledTimes(1);

    newRunResolvers.shift()?.();
    await vi.waitFor(() => {
      expect(newRun).toHaveBeenCalledTimes(2);
    });

    newRunResolvers.shift()?.();
    await firstNewRun;
    await queuedNewRun;
  });
});
