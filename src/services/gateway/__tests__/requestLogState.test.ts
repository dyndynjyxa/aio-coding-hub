import { describe, expect, it } from "vitest";
import { isRequestSignalComplete, isRequestSignalLogRefreshTrigger } from "../requestLogState";

describe("services/gateway/requestLogState", () => {
  it("uses start and complete request signals to refresh request logs", () => {
    expect(isRequestSignalComplete({ phase: "start" })).toBe(false);
    expect(isRequestSignalComplete({ phase: "complete" })).toBe(true);
    expect(isRequestSignalLogRefreshTrigger({ phase: "start" })).toBe(true);
    expect(isRequestSignalLogRefreshTrigger({ phase: "complete" })).toBe(true);
    expect(isRequestSignalLogRefreshTrigger({ phase: "other" })).toBe(false);
  });
});
