import { fireEvent, render, screen } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import type { ReactElement } from "react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { ProvidersPage } from "../ProvidersPage";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import { useProvidersListQuery } from "../../query/providers";
import { useSettingsQuery } from "../../query/settings";
import { createTestAppSettings } from "../../test/fixtures/settings";

vi.mock("../providers/ProvidersView", () => ({
  ProvidersView: ({ activeCli }: any) => (
    <div data-testid="providers-view">providers:{activeCli}</div>
  ),
}));

vi.mock("../providers/SortModesView", () => ({
  SortModesView: ({ activeCli }: any) => (
    <div data-testid="sort-modes-view">sort-modes:{activeCli}</div>
  ),
}));

vi.mock("../../query/providers", async () => {
  const actual =
    await vi.importActual<typeof import("../../query/providers")>("../../query/providers");
  return { ...actual, useProvidersListQuery: vi.fn() };
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

describe("pages/ProvidersPage", () => {
  it("uses top tabs to switch CLI providers view and sort modes view", () => {
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createTestAppSettings({ cli_priority_order: ["codex", "claude", "gemini"] }),
    } as any);
    vi.mocked(useProvidersListQuery).mockReturnValue({
      data: [],
      isFetching: false,
    } as any);

    renderWithProviders(<ProvidersPage />);

    expect(screen.getByRole("heading", { level: 1, name: "供应商" })).toBeInTheDocument();
    expect(screen.getByTestId("providers-view")).toBeInTheDocument();
    expect(screen.getByText("providers:codex")).toBeInTheDocument();

    expect(screen.getAllByRole("tab").map((tab) => tab.textContent)).toEqual([
      "Codex",
      "Claude Code",
      "Gemini",
      "排序模板",
    ]);

    fireEvent.click(screen.getByRole("tab", { name: "Claude Code" }));

    expect(screen.getByRole("heading", { level: 1, name: "供应商" })).toBeInTheDocument();
    expect(screen.getByText("providers:claude")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "排序模板" }));

    expect(screen.getByRole("heading", { level: 1, name: "排序模板" })).toBeInTheDocument();
    expect(screen.getByTestId("sort-modes-view")).toBeInTheDocument();
    expect(screen.getByText("sort-modes:claude")).toBeInTheDocument();
  });
});
