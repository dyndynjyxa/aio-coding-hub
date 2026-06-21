import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it } from "vitest";
import { PluginsPage } from "../pages/PluginsPage";
import { createTestQueryClient } from "../test/utils/reactQuery";

function renderPluginsPage() {
  return render(
    <QueryClientProvider client={createTestQueryClient()}>
      <MemoryRouter>
        <PluginsPage />
      </MemoryRouter>
    </QueryClientProvider>
  );
}

describe("plugins e2e smoke", () => {
  it("installs and displays the official Privacy Filter through the desktop IPC bridge", async () => {
    renderPluginsPage();

    fireEvent.click(await screen.findByRole("button", { name: /Privacy Filter/ }));

    await waitFor(() => {
      expect(screen.getAllByText("Privacy Filter").length).toBeGreaterThan(0);
    });
    expect(screen.getAllByText("official.privacy-filter").length).toBeGreaterThan(0);
    expect(await screen.findByText("gateway.request.afterBodyRead")).toBeInTheDocument();
    expect(await screen.findByText("log.beforePersist")).toBeInTheDocument();
    expect(await screen.findByText("Plugin installed")).toBeInTheDocument();
  });
});
