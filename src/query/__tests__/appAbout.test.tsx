import { renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { appAboutGet } from "../../services/app/appAbout";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { isAppAboutAvailable, useAppAboutQuery } from "../appAbout";

vi.mock("../../services/app/appAbout", async () => {
  const actual = await vi.importActual<typeof import("../../services/app/appAbout")>(
    "../../services/app/appAbout"
  );
  return {
    ...actual,
    appAboutGet: vi.fn(),
  };
});

describe("query/appAbout", () => {
  it("fetches about info when tauri runtime is available", async () => {
    setTauriRuntime();

    vi.mocked(appAboutGet).mockResolvedValue({
      os: "windows",
      arch: "x64",
      profile: "release",
      app_version: "0.1.0",
      bundle_type: null,
      run_mode: "installed",
    });

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useAppAboutQuery(), { wrapper });
    await waitFor(() => {
      expect(result.current.isSuccess).toBe(true);
    });

    expect(appAboutGet).toHaveBeenCalledTimes(1);
    expect(result.current.data?.os).toBe("windows");
  });

  it("enters error state when appAboutGet rejects", async () => {
    setTauriRuntime();

    vi.mocked(appAboutGet).mockRejectedValue(new Error("about boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useAppAboutQuery(), { wrapper });
    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("respects options.enabled=false", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useAppAboutQuery({ enabled: false }), { wrapper });
    await Promise.resolve();

    expect(appAboutGet).not.toHaveBeenCalled();
  });

  it("isAppAboutAvailable maps nullability to availability", () => {
    expect(isAppAboutAvailable(null)).toBe(false);
    expect(
      isAppAboutAvailable({
        os: "windows",
        arch: "x64",
        profile: "release",
        app_version: "0.1.0",
        bundle_type: null,
        run_mode: "installed",
      })
    ).toBe(true);
  });
});
