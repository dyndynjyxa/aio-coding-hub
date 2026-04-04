import { render, screen, waitFor } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import type { ReactElement } from "react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { PromptsPage } from "../PromptsPage";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { logToConsole } from "../../services/consoleLog";
import { useSettingsQuery } from "../../query/settings";
import { useWorkspacesListQuery } from "../../query/workspaces";
import { createTestAppSettings } from "../../test/fixtures/settings";

vi.mock("sonner", () => {
  const toast = Object.assign(vi.fn(), {
    loading: vi.fn(),
    success: vi.fn(),
    error: vi.fn(),
  });
  return { toast };
});

vi.mock("../../services/consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../services/consoleLog")>(
    "../../services/consoleLog"
  );
  return { ...actual, logToConsole: vi.fn() };
});

vi.mock("../prompts/PromptsView", () => ({
  PromptsView: () => <div data-testid="prompts-view" />,
}));

vi.mock("../../query/workspaces", async () => {
  const actual =
    await vi.importActual<typeof import("../../query/workspaces")>("../../query/workspaces");
  return { ...actual, useWorkspacesListQuery: vi.fn() };
});

vi.mock("../../query/settings", async () => {
  const actual =
    await vi.importActual<typeof import("../../query/settings")>("../../query/settings");
  return { ...actual, useSettingsQuery: vi.fn() };
});

function renderWithProviders(element: ReactElement) {
  const client = createTestQueryClient();
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter>{element}</MemoryRouter>
    </QueryClientProvider>
  );
}

describe("pages/PromptsPage", () => {
  it("shows missing workspace hint when active workspace is null", () => {
    setTauriRuntime();
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createTestAppSettings({ cli_priority_order: ["codex", "claude", "gemini"] }),
    } as any);

    vi.mocked(useWorkspacesListQuery).mockReturnValue({
      data: { active_id: null, items: [] },
      isFetching: false,
      error: null,
    } as any);

    renderWithProviders(<PromptsPage />);
    expect(screen.getByText(/未找到 Codex 的当前工作区/)).toBeInTheDocument();
  });

  it("logs and toasts when workspaces query errors", async () => {
    setTauriRuntime();
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createTestAppSettings({ cli_priority_order: ["gemini", "claude", "codex"] }),
    } as any);

    vi.mocked(useWorkspacesListQuery).mockReturnValue({
      data: { active_id: null, items: [] },
      isFetching: false,
      error: new Error("boom"),
    } as any);

    renderWithProviders(<PromptsPage />);

    await waitFor(() => {
      expect(logToConsole).toHaveBeenCalled();
      expect(toast).toHaveBeenCalledWith("加载失败：请查看控制台日志");
    });
  });
});
