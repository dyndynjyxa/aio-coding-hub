import { logToConsole } from "./consoleLog";

export type BackgroundTaskTrigger = "startup" | "interval" | "visibility" | "manual";

export type BackgroundTaskRunContext<TPayload = unknown> = {
  trigger: BackgroundTaskTrigger;
  payload?: TPayload;
};

export type BackgroundTaskDefinition = {
  taskId: string;
  run: (context: BackgroundTaskRunContext) => Promise<void> | void;
  intervalMs: number | null;
  runOnAppStart?: boolean;
  foregroundOnly?: boolean;
  visibilityTriggers?: string[];
};

type RegisteredTask = BackgroundTaskDefinition & {
  runningPromise: Promise<void> | null;
  pendingPromise: Promise<void> | null;
  pendingContext: BackgroundTaskRunContext | null;
  intervalHandle: ReturnType<typeof setInterval> | null;
};

const tasks = new Map<string, RegisteredTask>();
let started = false;
let foregroundActive = true;

function clearTaskInterval(task: RegisteredTask) {
  if (task.intervalHandle == null) return;
  clearInterval(task.intervalHandle);
  task.intervalHandle = null;
}

function shouldScheduleTask(task: RegisteredTask) {
  if (!started) return false;
  if (task.intervalMs == null || task.intervalMs <= 0) return false;
  if (task.foregroundOnly && !foregroundActive) return false;
  return true;
}

function scheduleTaskInterval(task: RegisteredTask) {
  clearTaskInterval(task);
  if (!shouldScheduleTask(task)) return;
  task.intervalHandle = setInterval(() => {
    void runBackgroundTask(task.taskId, { trigger: "interval" });
  }, task.intervalMs ?? undefined);
}

function rescheduleAllTasks() {
  for (const task of tasks.values()) {
    scheduleTaskInterval(task);
  }
}

export function registerBackgroundTask(definition: BackgroundTaskDefinition) {
  const existing = tasks.get(definition.taskId);
  if (existing) {
    clearTaskInterval(existing);
  }

  const task: RegisteredTask = {
    ...definition,
    runningPromise: existing?.runningPromise ?? null,
    pendingPromise: existing?.pendingPromise ?? null,
    pendingContext: existing?.pendingContext ?? null,
    intervalHandle: null,
  };

  tasks.set(definition.taskId, task);
  scheduleTaskInterval(task);

  if (started && definition.runOnAppStart) {
    void runBackgroundTask(definition.taskId, { trigger: "startup" });
  }

  return () => {
    const current = tasks.get(definition.taskId);
    if (!current) return;
    clearTaskInterval(current);
    tasks.delete(definition.taskId);
  };
}

export function startBackgroundTaskScheduler() {
  if (started) return;
  started = true;
  rescheduleAllTasks();
  for (const task of tasks.values()) {
    if (task.runOnAppStart) {
      void runBackgroundTask(task.taskId, { trigger: "startup" });
    }
  }
}

export function setBackgroundTaskSchedulerForeground(active: boolean) {
  foregroundActive = active;
  rescheduleAllTasks();
}

function shouldReplacePendingContext(
  current: BackgroundTaskRunContext | null,
  next: BackgroundTaskRunContext
) {
  if (!current) return true;
  if (current.trigger === "manual" && next.trigger !== "manual") return false;
  if (next.trigger === "manual") return true;
  return true;
}

export async function runBackgroundTask(
  taskId: string,
  context: BackgroundTaskRunContext = { trigger: "manual" }
) {
  const task = tasks.get(taskId);
  if (!task) return;
  if (task.runningPromise) {
    if (context.trigger !== "manual") {
      return task.runningPromise;
    }
    if (shouldReplacePendingContext(task.pendingContext, context)) {
      task.pendingContext = context;
    }
    if (!task.pendingPromise) {
      task.pendingPromise = task.runningPromise
        .catch(() => {})
        .then(async () => {
          const current = tasks.get(taskId);
          const nextContext = current?.pendingContext ?? null;
          if (current) {
            current.pendingContext = null;
          }
          if (!nextContext) return;
          await runBackgroundTask(taskId, nextContext);
        })
        .finally(() => {
          const current = tasks.get(taskId);
          if (current) {
            current.pendingPromise = null;
          }
        });
    }
    return task.pendingPromise;
  }

  const promise = Promise.resolve()
    .then(() => task.run(context))
    .catch((error) => {
      logToConsole("warn", "后台任务执行失败", {
        taskId,
        error: String(error),
      });
    })
    .finally(() => {
      const current = tasks.get(taskId);
      if (current) {
        current.runningPromise = null;
      }
    });

  task.runningPromise = promise;
  return promise;
}

export async function emitBackgroundTaskVisibilityTrigger(triggerId: string) {
  const runs: Array<Promise<void> | undefined> = [];
  for (const task of tasks.values()) {
    if (task.visibilityTriggers?.includes(triggerId)) {
      runs.push(runBackgroundTask(task.taskId, { trigger: "visibility" }));
    }
  }
  await Promise.all(runs);
}

export function resetBackgroundTaskSchedulerForTests() {
  for (const task of tasks.values()) {
    clearTaskInterval(task);
  }
  tasks.clear();
  started = false;
  foregroundActive = true;
}
