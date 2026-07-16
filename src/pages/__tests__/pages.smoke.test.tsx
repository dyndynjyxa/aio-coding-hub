import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { QueryClientProvider } from "@tanstack/react-query";
import { MemoryRouter } from "react-router-dom";
import type { ReactElement } from "react";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import { CliManagerPage } from "../CliManagerPage";
import { ConsolePage } from "../ConsolePage";
import { HomePage } from "../HomePage";
import { ImageGenPage } from "../ImageGenPage";
import { LogsPage } from "../LogsPage";
import { McpPage } from "../McpPage";
import { PromptsPage } from "../PromptsPage";
import { ProvidersPage } from "../ProvidersPage";
import { SkillsMarketPage } from "../SkillsMarketPage";
import { SkillsPage } from "../SkillsPage";
import { SettingsPage } from "../SettingsPage";
import { UsagePage } from "../UsagePage";
import { WorkspacesPage } from "../WorkspacesPage";

function renderPage(element: ReactElement) {
  const client = createTestQueryClient();
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter>{element}</MemoryRouter>
    </QueryClientProvider>
  );
}

describe("pages (smoke)", () => {
  it("renders HomePage", () => {
    renderPage(<HomePage />);
    expect(screen.getByRole("heading", { level: 1, name: "首页" })).toBeInTheDocument();
  });

  it("renders ImageGenPage", () => {
    renderPage(<ImageGenPage />);
    expect(screen.getByRole("heading", { level: 1, name: "生图" })).toBeInTheDocument();
  });

  it("renders ProvidersPage", () => {
    renderPage(<ProvidersPage />);
    expect(screen.getByRole("heading", { level: 1, name: "供应商" })).toBeInTheDocument();
  });

  it("renders WorkspacesPage", () => {
    renderPage(<WorkspacesPage />);
    expect(screen.getByRole("heading", { level: 1, name: "工作区" })).toBeInTheDocument();
  });

  it("renders PromptsPage", () => {
    renderPage(<PromptsPage />);
    expect(screen.getByRole("heading", { level: 1, name: "提示词" })).toBeInTheDocument();
  });

  it("renders McpPage", () => {
    renderPage(<McpPage />);
    expect(screen.getByRole("heading", { level: 1, name: "MCP" })).toBeInTheDocument();
  });

  it("renders SkillsPage", () => {
    renderPage(<SkillsPage />);
    expect(screen.getByRole("heading", { level: 1, name: "Skill" })).toBeInTheDocument();
  });

  it("renders SkillsMarketPage", () => {
    renderPage(<SkillsMarketPage />);
    expect(screen.getByRole("heading", { level: 1, name: "Skill 市场" })).toBeInTheDocument();
  });

  it("renders UsagePage", () => {
    renderPage(<UsagePage />);
    expect(screen.getByRole("heading", { level: 1, name: "用量分析" })).toBeInTheDocument();
  });

  it("renders ConsolePage", () => {
    renderPage(<ConsolePage />);
    expect(screen.getByRole("heading", { level: 1, name: "控制台" })).toBeInTheDocument();
  });

  it("renders LogsPage", () => {
    renderPage(<LogsPage />);
    expect(screen.getByRole("heading", { level: 1, name: "代理记录" })).toBeInTheDocument();
  });

  it("renders CliManagerPage", () => {
    renderPage(<CliManagerPage />);
    expect(screen.getByRole("heading", { level: 1, name: "CLI 管理" })).toBeInTheDocument();
  });

  it("renders SettingsPage", () => {
    renderPage(<SettingsPage />);
    expect(screen.getByRole("heading", { level: 1, name: "设置" })).toBeInTheDocument();
  });
});
