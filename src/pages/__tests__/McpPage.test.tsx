import { render, screen } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import type { ReactElement } from "react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { McpPage } from "../McpPage";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { useSettingsQuery } from "../../query/settings";
import { useWorkspacesListQuery } from "../../query/workspaces";
import { createTestAppSettings } from "../../test/fixtures/settings";

vi.mock("../mcp/McpServersView", () => ({
  McpServersView: () => <div data-testid="mcp-servers-view" />,
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

describe("pages/McpPage", () => {
  it("renders MCP view when active workspace exists", () => {
    setTauriRuntime();
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createTestAppSettings({ cli_priority_order: ["gemini", "codex", "claude"] }),
    } as any);

    vi.mocked(useWorkspacesListQuery).mockReturnValue({
      data: { active_id: 123, items: [] },
      isFetching: false,
      error: null,
    } as any);

    renderWithProviders(<McpPage />);
    expect(screen.getByTestId("mcp-servers-view")).toBeInTheDocument();
  });
});
