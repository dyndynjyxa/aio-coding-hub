import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import type { ReactElement } from "react";
import { MemoryRouter } from "react-router-dom";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { SkillsMarketPage } from "../SkillsMarketPage";
import {
  useSkillInstallToLocalMutation,
  useSkillRepoDeleteMutation,
  useSkillRepoUpsertMutation,
  useSkillReposListQuery,
  useSkillsDiscoverAvailableMutation,
  useSkillsDiscoverAvailableQuery,
  useSkillsInstalledListQuery,
  useSkillsLocalListQuery,
} from "../../query/skills";
import { useWorkspacesListQuery } from "../../query/workspaces";
import { useSettingsQuery } from "../../query/settings";
import { logToConsole } from "../../services/consoleLog";
import { createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { createTestAppSettings } from "../../test/fixtures/settings";

const navigateMock = vi.fn();

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../services/consoleLog", () => ({ logToConsole: vi.fn() }));

vi.mock("react-router-dom", async () => {
  const actual = await vi.importActual<typeof import("react-router-dom")>("react-router-dom");
  return { ...actual, useNavigate: () => navigateMock };
});

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

vi.mock("../../query/skills", async () => {
  const actual = await vi.importActual<typeof import("../../query/skills")>("../../query/skills");
  return {
    ...actual,
    useSkillReposListQuery: vi.fn(),
    useSkillsInstalledListQuery: vi.fn(),
    useSkillsLocalListQuery: vi.fn(),
    useSkillsDiscoverAvailableQuery: vi.fn(),
    useSkillsDiscoverAvailableMutation: vi.fn(),
    useSkillRepoUpsertMutation: vi.fn(),
    useSkillRepoDeleteMutation: vi.fn(),
    useSkillInstallToLocalMutation: vi.fn(),
  };
});

function renderWithProviders(element: ReactElement) {
  const client = createTestQueryClient();
  return render(
    <QueryClientProvider client={client}>
      <MemoryRouter>{element}</MemoryRouter>
    </QueryClientProvider>
  );
}

function mockCommonState() {
  localStorage.clear();
  vi.mocked(useSettingsQuery).mockReturnValue({
    data: createTestAppSettings(),
  } as any);
  vi.mocked(useSkillReposListQuery).mockReturnValue({
    data: [
      {
        id: 1,
        git_url: "https://github.com/acme/repo-one",
        branch: "main",
        enabled: true,
        created_at: 1,
        updated_at: 2,
      },
      {
        id: 2,
        git_url: "https://github.com/acme/repo-two",
        branch: "main",
        enabled: true,
        created_at: 1,
        updated_at: 3,
      },
    ],
    isLoading: false,
  } as any);
  vi.mocked(useWorkspacesListQuery).mockReturnValue({
    data: { active_id: 7 },
    isLoading: false,
  } as any);
  vi.mocked(useSkillsInstalledListQuery).mockReturnValue({
    data: [],
    isLoading: false,
  } as any);
  vi.mocked(useSkillsLocalListQuery).mockReturnValue({
    data: [],
    isLoading: false,
  } as any);
  vi.mocked(useSkillsDiscoverAvailableMutation).mockReturnValue({
    isPending: false,
    mutateAsync: vi.fn().mockResolvedValue([]),
  } as any);
  vi.mocked(useSkillRepoUpsertMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
  vi.mocked(useSkillRepoDeleteMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
  vi.mocked(useSkillInstallToLocalMutation).mockReturnValue({ mutateAsync: vi.fn() } as any);
}

describe("pages/SkillsMarketPage", () => {
  it("validates and saves repo in the repo dialog", async () => {
    setTauriRuntime();
    navigateMock.mockClear();
    mockCommonState();

    const upsert = { mutateAsync: vi.fn().mockResolvedValue({ id: 9 }), isPending: false };
    vi.mocked(useSkillRepoUpsertMutation).mockReturnValue(upsert as any);
    vi.mocked(useSkillsDiscoverAvailableQuery).mockReturnValue({
      data: [],
      isFetching: false,
    } as any);

    renderWithProviders(<SkillsMarketPage />);

    fireEvent.click(screen.getByRole("button", { name: "管理仓库" }));
    expect(screen.getByText("Skill 仓库")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "添加仓库" }));
    expect(toast).toHaveBeenCalledWith("请填写 Git URL");

    fireEvent.change(screen.getByPlaceholderText("https://github.com/owner/repo"), {
      target: { value: "https://github.com/acme/new-repo" },
    });
    fireEvent.change(screen.getByPlaceholderText("auto / main / master"), {
      target: { value: "main" },
    });
    fireEvent.click(screen.getByRole("button", { name: "添加仓库" }));

    await waitFor(() => {
      expect(upsert.mutateAsync).toHaveBeenCalledWith({
        repoId: null,
        gitUrl: "https://github.com/acme/new-repo",
        branch: "main",
        enabled: true,
      });
    });
    expect(toast).toHaveBeenCalledWith("仓库已添加");
  });

  it("groups skills by repo and installs a single skill to current CLI", async () => {
    setTauriRuntime();
    navigateMock.mockClear();
    mockCommonState();

    vi.mocked(useSkillsDiscoverAvailableQuery).mockReturnValue({
      data: [
        {
          name: "Alpha",
          description: "Alpha desc",
          source_git_url: "https://github.com/acme/repo-one",
          source_branch: "main",
          source_subdir: "skills/alpha",
          installed: false,
        },
        {
          name: "Gamma",
          description: "Gamma desc",
          source_git_url: "https://github.com/acme/repo-two",
          source_branch: "main",
          source_subdir: "skills/gamma",
          installed: false,
        },
      ],
      isFetching: false,
    } as any);

    const discover = { isPending: false, mutateAsync: vi.fn().mockResolvedValue([{ name: "x" }]) };
    vi.mocked(useSkillsDiscoverAvailableMutation).mockReturnValue(discover as any);

    const install = {
      mutateAsync: vi.fn().mockResolvedValue({
        dir_name: "alpha",
        path: "/tmp/alpha",
        name: "Alpha",
        description: "Alpha desc",
        source_git_url: "https://github.com/acme/repo-one",
        source_branch: "main",
        source_subdir: "skills/alpha",
      }),
    };
    vi.mocked(useSkillInstallToLocalMutation).mockReturnValue(install as any);

    renderWithProviders(<SkillsMarketPage />);

    expect(screen.getAllByText("acme/repo-one")[0]).toBeInTheDocument();
    expect(screen.getAllByText("acme/repo-two")[0]).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "刷新发现" }));
    await waitFor(() => expect(discover.mutateAsync).toHaveBeenCalledWith(true));
    expect(logToConsole).toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "安装到 Claude Code" }));
    await waitFor(() => {
      expect(install.mutateAsync).toHaveBeenCalledWith({
        gitUrl: "https://github.com/acme/repo-one",
        branch: "main",
        sourceSubdir: "skills/alpha",
      });
    });
    expect(toast).toHaveBeenCalledWith("已安装到 Claude Code");
  });

  it("falls back to the global CLI priority when localStorage is missing", () => {
    setTauriRuntime();
    navigateMock.mockClear();
    mockCommonState();
    vi.mocked(useSettingsQuery).mockReturnValue({
      data: createTestAppSettings({ cli_priority_order: ["gemini", "codex", "claude"] }),
    } as any);
    vi.mocked(useSkillsDiscoverAvailableQuery).mockReturnValue({
      data: [],
      isFetching: false,
    } as any);

    renderWithProviders(<SkillsMarketPage />);
    expect(useWorkspacesListQuery).toHaveBeenCalledWith("gemini");
  });

  it("keeps the market list area as a flex scroll region for expanded repo content", () => {
    setTauriRuntime();
    navigateMock.mockClear();
    mockCommonState();

    vi.mocked(useSkillsDiscoverAvailableQuery).mockReturnValue({
      data: [
        {
          name: "Alpha",
          description: "Alpha desc",
          source_git_url: "https://github.com/acme/repo-one",
          source_branch: "main",
          source_subdir: "skills/alpha",
          installed: false,
        },
        {
          name: "Beta",
          description: "Beta desc",
          source_git_url: "https://github.com/acme/repo-one",
          source_branch: "main",
          source_subdir: "skills/beta",
          installed: false,
        },
      ],
      isFetching: false,
    } as any);

    renderWithProviders(<SkillsMarketPage />);

    expect(screen.getByTestId("skills-market-list-card")).toHaveClass("flex", "flex-col", "flex-1");
    expect(screen.getByTestId("skills-market-scroll-region")).toHaveClass(
      "flex-1",
      "overflow-y-auto"
    );
  });

  it("supports installing a whole repo and navigating to generic skills when needed", async () => {
    setTauriRuntime();
    navigateMock.mockClear();
    mockCommonState();

    vi.mocked(useSkillsInstalledListQuery).mockReturnValue({
      data: [
        {
          id: 101,
          skill_key: "beta",
          name: "Beta",
          description: "",
          source_git_url: "https://github.com/acme/repo-two",
          source_branch: "main",
          source_subdir: "skills/beta",
          enabled: false,
          created_at: 1,
          updated_at: 1,
        },
      ],
      isLoading: false,
    } as any);
    vi.mocked(useSkillsLocalListQuery).mockReturnValue({
      data: [
        {
          dir_name: "gamma",
          path: "/tmp/gamma",
          name: "Gamma",
          description: "",
          source_git_url: "https://github.com/acme/repo-one",
          source_branch: "main",
          source_subdir: "skills/gamma",
        },
      ],
      isLoading: false,
    } as any);
    vi.mocked(useSkillsDiscoverAvailableQuery).mockReturnValue({
      data: [
        {
          name: "Alpha",
          description: "",
          source_git_url: "https://github.com/acme/repo-one",
          source_branch: "main",
          source_subdir: "skills/alpha",
          installed: false,
        },
        {
          name: "Gamma",
          description: "",
          source_git_url: "https://github.com/acme/repo-one",
          source_branch: "main",
          source_subdir: "skills/gamma",
          installed: false,
        },
        {
          name: "Beta",
          description: "",
          source_git_url: "https://github.com/acme/repo-two",
          source_branch: "main",
          source_subdir: "skills/beta",
          installed: true,
        },
      ],
      isFetching: false,
    } as any);

    const install = {
      mutateAsync: vi.fn().mockResolvedValueOnce({
        dir_name: "alpha",
        path: "/tmp/alpha",
        name: "Alpha",
        description: "",
        source_git_url: "https://github.com/acme/repo-one",
        source_branch: "main",
        source_subdir: "skills/alpha",
      }),
    };
    vi.mocked(useSkillInstallToLocalMutation).mockReturnValue(install as any);

    renderWithProviders(<SkillsMarketPage />);

    fireEvent.click(screen.getByRole("button", { name: "安装本仓库全部技能" }));
    await waitFor(() => expect(install.mutateAsync).toHaveBeenCalledTimes(1));
    expect(install.mutateAsync).toHaveBeenCalledWith({
      gitUrl: "https://github.com/acme/repo-one",
      branch: "main",
      sourceSubdir: "skills/alpha",
    });
    expect(toast).toHaveBeenCalledWith("已安装 1 个技能到 Claude Code");

    fireEvent.click(screen.getByRole("switch"));
    const repoTwoSection = screen.getAllByText("acme/repo-two")[0]?.closest("section");
    expect(repoTwoSection).not.toBeNull();
    fireEvent.click(within(repoTwoSection as HTMLElement).getByRole("button", { name: "展开" }));
    fireEvent.click(
      within(repoTwoSection as HTMLElement).getByRole("button", { name: "去通用技能" })
    );
    expect(navigateMock).toHaveBeenCalledWith("/skills");
  });

  it("ignores stale installed/local cache when the active workspace is missing", () => {
    setTauriRuntime();
    navigateMock.mockClear();
    mockCommonState();

    vi.mocked(useWorkspacesListQuery).mockReturnValue({
      data: { active_id: null },
      isLoading: false,
    } as any);
    vi.mocked(useSkillsInstalledListQuery).mockReturnValue({
      data: [
        {
          id: 101,
          skill_key: "beta",
          name: "Beta",
          description: "",
          source_git_url: "https://github.com/acme/repo-two",
          source_branch: "main",
          source_subdir: "skills/beta",
          enabled: true,
          created_at: 1,
          updated_at: 1,
        },
      ],
      isLoading: false,
    } as any);
    vi.mocked(useSkillsLocalListQuery).mockReturnValue({
      data: [
        {
          dir_name: "gamma",
          path: "/tmp/gamma",
          name: "Gamma",
          description: "",
          source_git_url: "https://github.com/acme/repo-one",
          source_branch: "main",
          source_subdir: "skills/gamma",
        },
      ],
      isLoading: false,
    } as any);
    vi.mocked(useSkillsDiscoverAvailableQuery).mockReturnValue({
      data: [
        {
          name: "Alpha",
          description: "",
          source_git_url: "https://github.com/acme/repo-one",
          source_branch: "main",
          source_subdir: "skills/alpha",
          installed: false,
        },
        {
          name: "Gamma",
          description: "",
          source_git_url: "https://github.com/acme/repo-one",
          source_branch: "main",
          source_subdir: "skills/gamma",
          installed: false,
        },
        {
          name: "Beta",
          description: "",
          source_git_url: "https://github.com/acme/repo-two",
          source_branch: "main",
          source_subdir: "skills/beta",
          installed: true,
        },
      ],
      isFetching: false,
    } as any);

    renderWithProviders(<SkillsMarketPage />);

    expect(screen.getByText(/当前还没有激活的 workspace/)).toBeInTheDocument();
    expect(screen.queryByText("本机 1")).not.toBeInTheDocument();
    expect(screen.queryByText("通用已启用 1")).not.toBeInTheDocument();

    const repoTwoSection = screen.getAllByText("acme/repo-two")[0]?.closest("section");
    expect(repoTwoSection).not.toBeNull();
    fireEvent.click(within(repoTwoSection as HTMLElement).getByRole("button", { name: "展开" }));
    expect(
      within(repoTwoSection as HTMLElement).getByRole("button", { name: "安装到 Claude Code" })
    ).toBeInTheDocument();
    expect(
      within(repoTwoSection as HTMLElement).queryByRole("button", { name: "去通用技能" })
    ).not.toBeInTheDocument();
  });
});
