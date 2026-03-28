import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CliManagerGeminiTab } from "../GeminiTab";

vi.mock("../../CliVersionBadge", () => ({
  CliVersionBadge: ({ cliKey }: { cliKey: string }) => <div>version-badge-{cliKey}</div>,
}));

function createGeminiInfo(overrides: Partial<any> = {}) {
  return {
    found: true,
    version: "1.2.3",
    executable_path: "/bin/gemini",
    resolved_via: "PATH",
    shell: "/bin/zsh",
    error: null,
    ...overrides,
  };
}

function createGeminiConfig(overrides: Partial<any> = {}) {
  return {
    configDir: "/home/user/.gemini",
    configPath: "/home/user/.gemini/settings.json",
    exists: true,
    modelName: "gemini-2.5-pro",
    modelMaxSessionTurns: -1,
    modelCompressionThreshold: 0.7,
    defaultApprovalMode: "plan",
    enableAutoUpdate: true,
    enableNotifications: false,
    vimMode: true,
    retryFetchErrors: true,
    maxAttempts: 5,
    uiTheme: "dark",
    uiHideBanner: true,
    uiHideTips: false,
    uiShowLineNumbers: true,
    uiInlineThinkingMode: "full",
    usageStatisticsEnabled: false,
    sessionRetentionEnabled: true,
    sessionRetentionMaxAge: "30d",
    planModelRouting: false,
    securityAuthSelectedType: "gemini-api-key",
    ...overrides,
  };
}

describe("components/cli-manager/tabs/GeminiTab", () => {
  it("renders installed state, badge, and refresh action", () => {
    const refresh = vi.fn();
    render(
      <CliManagerGeminiTab
        geminiAvailable="available"
        geminiLoading={false}
        geminiInfo={createGeminiInfo()}
        geminiConfigLoading={false}
        geminiConfigSaving={false}
        geminiConfig={createGeminiConfig()}
        refreshGeminiInfo={refresh}
        persistGeminiConfig={vi.fn()}
      />
    );

    expect(screen.getByText("已安装 1.2.3")).toBeInTheDocument();
    expect(screen.getByText("version-badge-gemini")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "刷新状态" }));
    expect(refresh).toHaveBeenCalled();
  });

  it("persists config changes through inputs, selects, and switches", () => {
    const persistGeminiConfig = vi.fn();

    render(
      <CliManagerGeminiTab
        geminiAvailable="available"
        geminiLoading={false}
        geminiInfo={createGeminiInfo()}
        geminiConfigLoading={false}
        geminiConfigSaving={false}
        geminiConfig={createGeminiConfig()}
        refreshGeminiInfo={vi.fn()}
        persistGeminiConfig={persistGeminiConfig}
      />
    );

    const modelItem = screen.getByText("默认模型 (model.name)").parentElement?.parentElement;
    expect(modelItem).toBeTruthy();
    const modelInput = within(modelItem as HTMLElement).getByRole("textbox");
    fireEvent.change(modelInput, { target: { value: " gemini-2.5-flash " } });
    fireEvent.blur(modelInput);
    expect(persistGeminiConfig).toHaveBeenCalledWith({ modelName: "gemini-2.5-flash" });

    const approvalItem = screen.getByText("审批模式 (general.defaultApprovalMode)").parentElement
      ?.parentElement;
    expect(approvalItem).toBeTruthy();
    fireEvent.change(within(approvalItem as HTMLElement).getByRole("combobox"), {
      target: { value: "auto_edit" },
    });
    expect(persistGeminiConfig).toHaveBeenCalledWith({ defaultApprovalMode: "auto_edit" });

    const maxAttemptsItem = screen.getByText("最大尝试次数 (general.maxAttempts)").parentElement
      ?.parentElement;
    expect(maxAttemptsItem).toBeTruthy();
    const maxAttemptsInput = within(maxAttemptsItem as HTMLElement).getByRole("spinbutton");
    fireEvent.change(maxAttemptsInput, { target: { value: "9" } });
    fireEvent.blur(maxAttemptsInput);
    expect(persistGeminiConfig).toHaveBeenCalledWith({ maxAttempts: 9 });

    const hideBannerItem = screen.getByText("隐藏 Banner (ui.hideBanner)").parentElement
      ?.parentElement;
    expect(hideBannerItem).toBeTruthy();
    fireEvent.click(within(hideBannerItem as HTMLElement).getByRole("switch"));
    expect(persistGeminiConfig).toHaveBeenCalledWith({ uiHideBanner: false });

    const statsItem = screen.getByText("使用统计 (privacy.usageStatisticsEnabled)").parentElement
      ?.parentElement;
    expect(statsItem).toBeTruthy();
    fireEvent.click(within(statsItem as HTMLElement).getByRole("switch"));
    expect(persistGeminiConfig).toHaveBeenCalledWith({ usageStatisticsEnabled: true });
  });

  it("renders unavailable and error states", () => {
    const { rerender } = render(
      <CliManagerGeminiTab
        geminiAvailable="unavailable"
        geminiLoading={false}
        geminiInfo={null}
        geminiConfigLoading={false}
        geminiConfigSaving={false}
        geminiConfig={null}
        refreshGeminiInfo={vi.fn()}
        persistGeminiConfig={vi.fn()}
      />
    );
    expect(screen.getByText("数据不可用")).toBeInTheDocument();

    rerender(
      <CliManagerGeminiTab
        geminiAvailable="available"
        geminiLoading={false}
        geminiInfo={createGeminiInfo({
          found: false,
          version: null,
          executable_path: null,
          error: "boom",
        })}
        geminiConfigLoading={false}
        geminiConfigSaving={false}
        geminiConfig={null}
        refreshGeminiInfo={vi.fn()}
        persistGeminiConfig={vi.fn()}
      />
    );
    expect(screen.getByText("检测失败：")).toBeInTheDocument();
    expect(screen.getByText("boom")).toBeInTheDocument();
  });
});
