import { describe, expect, it, vi } from "vitest";
import { emitListenerSnapshot } from "../listeners";

describe("utils/listeners", () => {
  it("notifies a snapshot so listener mutations do not affect the active emit", () => {
    const listeners = new Set<() => void>();
    const first = vi.fn(() => {
      listeners.delete(second);
    });
    const second = vi.fn();
    listeners.add(first);
    listeners.add(second);

    emitListenerSnapshot(listeners, (listener) => listener());

    expect(first).toHaveBeenCalledTimes(1);
    expect(second).not.toHaveBeenCalled();
  });

  it("continues after listener errors and reports them", () => {
    const expectedError = new Error("listener failed");
    const onError = vi.fn();
    const afterError = vi.fn();
    const listeners = new Set<() => void>([
      () => {
        throw expectedError;
      },
      afterError,
    ]);

    emitListenerSnapshot(listeners, (listener) => listener(), onError);

    expect(onError).toHaveBeenCalledWith(expectedError);
    expect(afterError).toHaveBeenCalledTimes(1);
  });

  it("does not let an error handler failure stop later listeners", () => {
    const afterError = vi.fn();
    const listeners = new Set<() => void>([
      () => {
        throw new Error("listener failed");
      },
      afterError,
    ]);

    emitListenerSnapshot(
      listeners,
      (listener) => listener(),
      () => {
        throw new Error("error handler failed");
      }
    );

    expect(afterError).toHaveBeenCalledTimes(1);
  });
});
