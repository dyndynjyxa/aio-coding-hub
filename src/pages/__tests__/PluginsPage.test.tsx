import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import type { ReactElement } from "react";
import { MemoryRouter } from "react-router-dom";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { PluginsPage } from "../PluginsPage";
import type { PluginDetail, PluginSummary } from "../../services/plugins";
import { openDesktopSinglePath } from "../../services/desktop/dialog";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import {
  usePluginDisableMutation,
  usePluginEnableMutation,
  usePluginGrantPermissionsMutation,
  usePluginInstallFromFileMutation,
  usePluginInstallOfficialMutation,
  usePluginQuery,
  usePluginRollbackMutation,
  usePluginSaveConfigMutation,
  usePluginUpdateFromFileMutation,
  usePluginsListQuery,
  usePluginUninstallMutation,
} from "../../query/plugins";

vi.mock("sonner", () => {
  const toast = Object.assign(vi.fn(), {
    loading: vi.fn(),
    success: vi.fn(),
    error: vi.fn(),
  });
  return { toast };
});

vi.mock("../../services/desktop/dialog", async () => {
  const actual = await vi.importActual<typeof import("../../services/desktop/dialog")>(
    "../../services/desktop/dialog"
  );
  return { ...actual, openDesktopSinglePath: vi.fn() };
});

vi.mock("../../query/plugins", async () => {
  const actual = await vi.importActual<typeof import("../../query/plugins")>("../../query/plugins");
  return {
    ...actual,
    usePluginsListQuery: vi.fn(),
    usePluginQuery: vi.fn(),
    usePluginInstallFromFileMutation: vi.fn(),
    usePluginInstallOfficialMutation: vi.fn(),
    usePluginUpdateFromFileMutation: vi.fn(),
    usePluginRollbackMutation: vi.fn(),
    usePluginEnableMutation: vi.fn(),
    usePluginGrantPermissionsMutation: vi.fn(),
    usePluginDisableMutation: vi.fn(),
    usePluginUninstallMutation: vi.fn(),
    usePluginSaveConfigMutation: vi.fn(),
  };
});

function summary(overrides: Partial<PluginSummary> = {}): PluginSummary {
  return {
    id: 1,
    plugin_id: "community.prompt-helper",
    name: "Community Prompt Helper",
    current_version: "1.0.0",
    status: "disabled",
    runtime: "declarativeRules",
    permission_risk: "high",
    update_available: false,
    last_error: null,
    created_at: 10,
    updated_at: 20,
    ...overrides,
  };
}

function detail(overrides: Partial<PluginDetail> = {}): PluginDetail {
  const baseSummary = summary();
  return {
    summary: baseSummary,
    manifest: {
      id: baseSummary.plugin_id,
      name: baseSummary.name,
      version: "1.0.0",
      apiVersion: "1.0.0",
      runtime: { kind: "declarativeRules", rules: ["rules/main.json"] },
      hooks: [{ name: "gateway.request.afterBodyRead", priority: 100, failurePolicy: "fail-open" }],
      permissions: ["request.body.read", "request.body.write"],
      hostCompatibility: {
        app: ">=0.56.0 <1.0.0",
        pluginApi: "^1.0.0",
        platforms: ["macos", "windows", "linux"],
      },
      configSchema: {
        type: "object",
        required: ["mode"],
        properties: {
          mode: { type: "string", enum: ["append_instruction", "rewrite_system_message"] },
        },
      },
    },
    install_source: "local",
    installed_dir: null,
    config: { mode: "append_instruction" },
    granted_permissions: ["request.body.read"],
    pending_permissions: ["request.body.write"],
    audit_logs: [
      {
        id: 1,
        plugin_id: baseSummary.plugin_id,
        trace_id: "trace-1",
        event_type: "plugin.installed",
        risk_level: "low",
        message: "Plugin installed",
        details: {},
        created_at: 30,
      },
    ],
    runtime_failures: [],
    ...overrides,
  };
}

function mutation(overrides: Record<string, unknown> = {}) {
  return {
    mutateAsync: vi.fn().mockResolvedValue(detail()),
    isPending: false,
    ...overrides,
  };
}

function renderWithProviders(element: ReactElement) {
  const client = createTestQueryClient();
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter>{element}</MemoryRouter>
    </QueryClientProvider>
  );
}

describe("pages/PluginsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(usePluginInstallFromFileMutation).mockReturnValue(mutation() as any);
    vi.mocked(usePluginInstallOfficialMutation).mockReturnValue(mutation() as any);
    vi.mocked(usePluginUpdateFromFileMutation).mockReturnValue(mutation() as any);
    vi.mocked(usePluginRollbackMutation).mockReturnValue(mutation() as any);
    vi.mocked(usePluginEnableMutation).mockReturnValue(mutation() as any);
    vi.mocked(usePluginGrantPermissionsMutation).mockReturnValue(mutation() as any);
    vi.mocked(usePluginDisableMutation).mockReturnValue(mutation() as any);
    vi.mocked(usePluginUninstallMutation).mockReturnValue(mutation() as any);
    vi.mocked(usePluginSaveConfigMutation).mockReturnValue(mutation() as any);
    vi.mocked(usePluginQuery).mockReturnValue({
      data: detail(),
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);
  });

  it("renders list fields and plugin detail permissions", () => {
    vi.mocked(usePluginsListQuery).mockReturnValue({
      data: [summary({ update_available: true, last_error: "Last failure" })],
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);

    renderWithProviders(<PluginsPage />);

    expect(screen.getAllByText("Community Prompt Helper").length).toBeGreaterThan(0);
    expect(screen.getAllByText("community.prompt-helper").length).toBeGreaterThan(0);
    expect(screen.getAllByText("规则插件").length).toBeGreaterThan(0);
    expect(screen.getByText("高风险")).toBeInTheDocument();
    expect(screen.getByText("可更新")).toBeInTheDocument();
    expect(screen.getByText("Last failure")).toBeInTheDocument();
    expect(screen.getByText("gateway.request.afterBodyRead")).toBeInTheDocument();
    expect(screen.getByText("request.body.write")).toBeInTheDocument();
    expect(screen.getByText("待允许")).toBeInTheDocument();
    expect(screen.getByText("Plugin installed")).toBeInTheDocument();
  });

  it("presents plugin value, data access, settings, and developer metadata in that order", () => {
    vi.mocked(usePluginsListQuery).mockReturnValue({
      data: [summary()],
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);

    renderWithProviders(<PluginsPage />);

    expect(screen.getByText("这个插件会做什么")).toBeInTheDocument();
    expect(screen.getByText("数据访问")).toBeInTheDocument();
    expect(screen.getByText("设置")).toBeInTheDocument();
    expect(screen.getByText("开发者信息")).toBeInTheDocument();
    expect(screen.getByText("读取你发送给模型的内容")).toBeInTheDocument();
  });

  it("disables plugin actions while config save is pending", () => {
    vi.mocked(usePluginsListQuery).mockReturnValue({
      data: [summary({ status: "disabled", update_available: true })],
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(usePluginSaveConfigMutation).mockReturnValue(mutation({ isPending: true }) as any);

    renderWithProviders(<PluginsPage />);

    expect(screen.getByRole("button", { name: /启用/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: /卸载/ })).toBeDisabled();
    expect(screen.getByRole("button", { name: /授权待审批权限/ })).toBeDisabled();
  });

  it("uses the generic schema form for official plugin configuration", () => {
    vi.mocked(usePluginsListQuery).mockReturnValue({
      data: [
        summary({
          plugin_id: "official.privacy-filter",
          name: "Privacy Filter",
          runtime: "native:privacyFilter",
        }),
      ],
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(usePluginQuery).mockReturnValue({
      data: detail({
        summary: summary({
          plugin_id: "official.privacy-filter",
          name: "Privacy Filter",
          runtime: "native:privacyFilter",
        }),
        manifest: {
          ...detail().manifest,
          id: "official.privacy-filter",
          name: "Privacy Filter",
          runtime: { kind: "native", engine: "privacyFilter" },
          permissions: ["request.body.read", "request.body.write", "log.redact"],
          configSchema: {
            type: "object",
            "x-aio-ui": {
              sections: [
                {
                  id: "content",
                  title: "检测策略",
                  description:
                    "这里展示的是可配置的策略大类；密钥类检测由打包的 200+ Gitleaks 规则、上下文规则和熵检测共同支撑。",
                  order: 10,
                },
              ],
            },
            properties: {
              sensitiveTypes: {
                type: "array",
                title: "策略大类",
                description:
                  "这些不是全部底层规则。密钥相关选项会控制打包的 200+ Gitleaks 规则以及上下文/熵检测结果是否生效。",
                items: {
                  type: "string",
                  enum: ["email", "cn_phone"],
                  "x-aio-ui": {
                    enumLabels: { email: "邮箱地址", cn_phone: "中国手机号" },
                  },
                },
                "x-aio-ui": { section: "content", widget: "checkboxGroup", order: 10 },
              },
            },
          },
        },
        config: { sensitiveTypes: ["email", "cn_phone"] },
        granted_permissions: ["request.body.read", "request.body.write", "log.redact"],
      }),
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);

    renderWithProviders(<PluginsPage />);

    expect(screen.getByText("检测策略")).toBeInTheDocument();
    expect(screen.getAllByText(/200\+ Gitleaks/).length).toBeGreaterThanOrEqual(2);
    expect(screen.getByLabelText("邮箱地址")).toBeChecked();
    expect(screen.queryByLabelText("sensitiveTypes")).not.toBeInTheDocument();
  });

  it("shows empty and error states", () => {
    vi.mocked(usePluginsListQuery).mockReturnValueOnce({
      data: [],
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);

    const { rerender } = renderWithProviders(<PluginsPage />);
    expect(screen.getByText("还没有安装插件")).toBeInTheDocument();

    vi.mocked(usePluginsListQuery).mockReturnValueOnce({
      data: null,
      isLoading: false,
      isFetching: false,
      error: new Error("boom"),
    } as any);
    rerender(
      <QueryClientProvider client={createTestQueryClient()}>
        <MemoryRouter>
          <PluginsPage />
        </MemoryRouter>
      </QueryClientProvider>
    );
    expect(screen.getByText(/插件列表加载失败/)).toBeInTheDocument();
  });

  it("wires import and enable actions", async () => {
    const importMutation = mutation();
    const installOfficialMutation = mutation();
    const enableMutation = mutation();
    vi.mocked(usePluginInstallFromFileMutation).mockReturnValue(importMutation as any);
    vi.mocked(usePluginInstallOfficialMutation).mockReturnValue(installOfficialMutation as any);
    vi.mocked(usePluginEnableMutation).mockReturnValue(enableMutation as any);
    vi.mocked(usePluginsListQuery).mockReturnValue({
      data: [summary()],
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(openDesktopSinglePath).mockResolvedValue("/tmp/plugin.json");

    renderWithProviders(<PluginsPage />);
    fireEvent.click(screen.getByRole("button", { name: "导入 .aio-plugin" }));
    expect(screen.getByRole("button", { name: /Privacy Filter/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Safety Detector/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Prompt Optimizer/ })).not.toBeInTheDocument();
    expect(
      screen.queryByRole("button", { name: /Sensitive Data Redactor/ })
    ).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Privacy Filter/ }));
    fireEvent.click(screen.getByRole("button", { name: "启用" }));

    await waitFor(() => {
      expect(importMutation.mutateAsync).toHaveBeenCalledWith("/tmp/plugin.json");
      expect(installOfficialMutation.mutateAsync).toHaveBeenCalledWith("official.privacy-filter");
      expect(enableMutation.mutateAsync).toHaveBeenCalledWith("community.prompt-helper");
      expect(toast.success).toHaveBeenCalled();
    });
  });

  it("approves pending plugin permissions from the detail panel", async () => {
    const grantMutation = mutation();
    vi.mocked(usePluginGrantPermissionsMutation).mockReturnValue(grantMutation as any);
    vi.mocked(usePluginsListQuery).mockReturnValue({
      data: [summary()],
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);

    renderWithProviders(<PluginsPage />);
    fireEvent.click(screen.getByRole("button", { name: "授权待审批权限" }));

    await waitFor(() => {
      expect(grantMutation.mutateAsync).toHaveBeenCalledWith({
        pluginId: "community.prompt-helper",
        permissions: ["request.body.write"],
      });
      expect(toast.success).toHaveBeenCalledWith("授权权限成功");
    });
  });

  it("keeps the pending permission action visible when enable fails", async () => {
    const enableMutation = mutation({
      mutateAsync: vi
        .fn()
        .mockRejectedValue(new Error("PLUGIN_PERMISSION_REQUIRED: request.body.write")),
    });
    vi.mocked(usePluginEnableMutation).mockReturnValue(enableMutation as any);
    vi.mocked(usePluginsListQuery).mockReturnValue({
      data: [summary()],
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);

    renderWithProviders(<PluginsPage />);
    fireEvent.click(screen.getByRole("button", { name: "启用" }));

    await waitFor(() => {
      expect(enableMutation.mutateAsync).toHaveBeenCalledWith("community.prompt-helper");
      expect(toast.error).toHaveBeenCalledWith(
        "启用插件失败（code PLUGIN_PERMISSION_REQUIRED）：request.body.write"
      );
    });
    expect(screen.getByRole("button", { name: "授权待审批权限" })).toBeInTheDocument();
  });

  it("shows package risk labels and wires update/rollback actions", async () => {
    const updateMutation = mutation();
    const rollbackMutation = mutation();
    vi.mocked(usePluginUpdateFromFileMutation).mockReturnValue(updateMutation as any);
    vi.mocked(usePluginRollbackMutation).mockReturnValue(rollbackMutation as any);
    vi.mocked(usePluginsListQuery).mockReturnValue({
      data: [
        summary({
          plugin_id: "community.redactor",
          name: "Community Redactor",
          status: "update_available",
          update_available: true,
          permission_risk: "critical",
        }),
        summary({
          plugin_id: "community.revoked",
          name: "Revoked Plugin",
          status: "quarantined",
          update_available: false,
          last_error: "revoked by market",
        }),
      ],
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(usePluginQuery).mockReturnValue({
      data: detail({
        summary: summary({
          plugin_id: "community.redactor",
          name: "Community Redactor",
          current_version: "1.1.0",
          status: "update_available",
          permission_risk: "critical",
          update_available: true,
        }),
        install_source: "offline",
        audit_logs: [
          {
            id: 2,
            plugin_id: "community.redactor",
            trace_id: null,
            event_type: "plugin.installed",
            risk_level: "high",
            message: "Local plugin package installed",
            details: { unsigned: true, fromVersion: "1.0.0" },
            created_at: 40,
          },
        ],
      }),
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);
    vi.mocked(openDesktopSinglePath).mockResolvedValue("/tmp/community-redactor-1.1.0.aio-plugin");

    renderWithProviders(<PluginsPage />);
    fireEvent.click(screen.getAllByText("Community Redactor")[0]);
    fireEvent.click(screen.getByRole("button", { name: "更新" }));
    fireEvent.click(screen.getByRole("button", { name: "回滚 1.0.0" }));

    await waitFor(() => {
      expect(screen.getAllByText("未签名").length).toBeGreaterThan(0);
      expect(screen.getByText("已隔离")).toBeInTheDocument();
      expect(screen.getByText("revoked by market")).toBeInTheDocument();
      expect(updateMutation.mutateAsync).toHaveBeenCalledWith(
        "/tmp/community-redactor-1.1.0.aio-plugin"
      );
      expect(rollbackMutation.mutateAsync).toHaveBeenCalledWith({
        pluginId: "community.redactor",
        version: "1.0.0",
      });
    });
  });

  it("does not offer enable action for quarantined or uninstalled plugins", () => {
    const enableMutation = mutation();
    vi.mocked(usePluginEnableMutation).mockReturnValue(enableMutation as any);
    vi.mocked(usePluginsListQuery).mockReturnValue({
      data: [
        summary({
          plugin_id: "community.revoked",
          name: "Revoked Plugin",
          status: "quarantined",
          last_error: "revoked by market",
        }),
        summary({
          plugin_id: "community.removed",
          name: "Removed Plugin",
          status: "uninstalled",
        }),
      ],
      isLoading: false,
      isFetching: false,
      error: null,
    } as any);

    renderWithProviders(<PluginsPage />);

    expect(screen.getByText("已隔离")).toBeInTheDocument();
    expect(screen.getByText("已卸载")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "启用" })).not.toBeInTheDocument();
    expect(enableMutation.mutateAsync).not.toHaveBeenCalled();
  });
});
