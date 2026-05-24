import { render, screen } from "@testing-library/react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { AppLayout } from "../AppLayout";

vi.mock("../../components/UpdateDialog", () => ({
  UpdateDialog: () => <div data-testid="update-dialog">update-dialog</div>,
}));

vi.mock("../../ui/Sidebar", () => ({
  Sidebar: () => <aside data-testid="sidebar">sidebar</aside>,
}));

describe("layout/AppLayout", () => {
  it("renders sidebar, main content area (Outlet), and UpdateDialog", () => {
    render(
      <MemoryRouter>
        <AppLayout />
      </MemoryRouter>
    );

    expect(screen.getByTestId("sidebar")).toBeInTheDocument();
    expect(screen.getByTestId("update-dialog")).toBeInTheDocument();
    expect(document.querySelector("[data-tauri-drag-region]")).toBeInTheDocument();
  });
});
