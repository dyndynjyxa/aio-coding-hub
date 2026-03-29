import { render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import { SettingsDialogs } from "./SettingsDialogs";

function wrapper({ children }: { children: React.ReactNode }) {
  return <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>;
}

describe("SettingsDialogs", () => {
  it("renders config import confirmation summary and warnings", () => {
    render(
      <SettingsDialogs
        modelPriceAliasesDialogOpen={false}
        setModelPriceAliasesDialogOpen={vi.fn()}
        clearRequestLogsDialogOpen={false}
        setClearRequestLogsDialogOpen={vi.fn()}
        clearingRequestLogs={false}
        setClearingRequestLogs={vi.fn()}
        clearRequestLogs={vi.fn(async () => {})}
        resetAllDialogOpen={false}
        setResetAllDialogOpen={vi.fn()}
        resettingAll={false}
        setResettingAll={vi.fn()}
        resetAllData={vi.fn(async () => {})}
        configImportDialogOpen
        setConfigImportDialogOpen={vi.fn()}
        importingConfig={false}
        setImportingConfig={vi.fn()}
        pendingConfigBundle={{
          schema_version: 2,
          exported_at: "2026-03-27T00:00:00Z",
          app_version: "0.33.6",
          settings: "{}",
          providers: [{ id: 1 }],
          sort_modes: [{ name: "Balanced" }],
          sort_mode_active: { claude: "Balanced" },
          workspaces: [
            {
              name: "Default",
              prompts: [
                { name: "default", content: "foo", enabled: true },
                { name: "review", content: "bar", enabled: false },
              ],
            },
            { name: "Work", prompt: { name: "legacy", content: "baz", enabled: true } },
          ],
          mcp_servers: [{ server_key: "fs" }, { server_key: "git" }],
          skill_repos: [{ git_url: "https://example.com/repo.git" }],
          installed_skills: [{ skill_key: "review" }, { skill_key: "debug" }],
          local_skills: [{ cli_key: "codex", dir_name: "local-a" }],
        }}
        confirmConfigImport={vi.fn(async () => {})}
      />,
      { wrapper }
    );

    expect(screen.getByText("确认导入配置")).toBeInTheDocument();
    expect(screen.getByText(/API Key 等敏感信息/)).toBeInTheDocument();
    expect(screen.getByText(/导入将覆盖当前所有配置/)).toBeInTheDocument();
    expect(screen.getByText(/Providers：1/)).toBeInTheDocument();
    expect(screen.getByText(/Sort Modes：1/)).toBeInTheDocument();
    expect(screen.getByText(/Workspaces：2/)).toBeInTheDocument();
    expect(screen.getByText(/Prompts：3/)).toBeInTheDocument();
    expect(screen.getByText(/MCP Servers：2/)).toBeInTheDocument();
    expect(screen.getByText(/Skill Repos：1/)).toBeInTheDocument();
    expect(screen.getByText(/Installed Skills：2/)).toBeInTheDocument();
    expect(screen.getByText(/Local Skills：1/)).toBeInTheDocument();
  });
});
