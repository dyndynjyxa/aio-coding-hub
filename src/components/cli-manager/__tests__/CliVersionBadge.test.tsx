import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { CliVersionBadge } from "../CliVersionBadge";
import { cliCheckLatestVersion, cliUpdateCli } from "../../../services/cli/cliUpdate";
import { toast } from "sonner";

vi.mock("../../../services/cli/cliUpdate", () => ({
  cliCheckLatestVersion: vi.fn(),
  cliUpdateCli: vi.fn(),
}));

vi.mock("sonner", () => ({
  toast: Object.assign(vi.fn(), { success: vi.fn(), error: vi.fn() }),
}));

describe("components/cli-manager/CliVersionBadge", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows up-to-date state", async () => {
    vi.mocked(cliCheckLatestVersion).mockResolvedValue({
      cliKey: "gemini",
      npmPackage: "@google/gemini-cli",
      installedVersion: "1.0.0",
      latestVersion: "1.0.0",
      updateAvailable: false,
      error: null,
    });

    render(<CliVersionBadge cliKey="gemini" installedVersion="1.0.0" />);

    await waitFor(() => {
      expect(screen.getByText("已是最新")).toBeInTheDocument();
    });
  });

  it("shows update action and runs update flow", async () => {
    vi.mocked(cliCheckLatestVersion)
      .mockResolvedValueOnce({
        cliKey: "codex",
        npmPackage: "@openai/codex",
        installedVersion: "1.0.0",
        latestVersion: "1.1.0",
        updateAvailable: true,
        error: null,
      })
      .mockResolvedValueOnce({
        cliKey: "codex",
        npmPackage: "@openai/codex",
        installedVersion: "1.1.0",
        latestVersion: "1.1.0",
        updateAvailable: false,
        error: null,
      });
    vi.mocked(cliUpdateCli).mockResolvedValue({
      cliKey: "codex",
      success: true,
      output: "updated",
      error: null,
    });

    const confirmSpy = vi.spyOn(window, "confirm").mockReturnValue(true);

    render(<CliVersionBadge cliKey="codex" installedVersion="1.0.0" />);

    await waitFor(() => {
      expect(screen.getByText("最新: v1.1.0")).toBeInTheDocument();
    });

    fireEvent.click(screen.getByRole("button", { name: "更新" }));

    await waitFor(() => {
      expect(cliUpdateCli).toHaveBeenCalledWith("codex");
    });
    expect(toast.success).toHaveBeenCalled();
    confirmSpy.mockRestore();
  });

  it("reruns latest-version check when refresh token changes", async () => {
    vi.mocked(cliCheckLatestVersion).mockResolvedValue({
      cliKey: "gemini",
      npmPackage: "@google/gemini-cli",
      installedVersion: "1.0.0",
      latestVersion: "1.0.0",
      updateAvailable: false,
      error: null,
    });

    const { rerender } = render(
      <CliVersionBadge cliKey="gemini" installedVersion="1.0.0" refreshToken={0} />
    );

    await waitFor(() => {
      expect(cliCheckLatestVersion).toHaveBeenCalledTimes(1);
    });

    rerender(<CliVersionBadge cliKey="gemini" installedVersion="1.0.0" refreshToken={1} />);

    await waitFor(() => {
      expect(cliCheckLatestVersion).toHaveBeenCalledTimes(2);
    });
  });
});
