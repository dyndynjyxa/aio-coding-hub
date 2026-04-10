import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type {
  AvailableSkillSummary,
  InstalledSkillSummary,
  LocalSkillSummary,
  SkillRepoSummary,
  SkillsPaths,
} from "../../services/workspace/skills";
import {
  skillImportLocal,
  skillInstallToLocal,
  skillLocalDelete,
  skillReturnToLocal,
  skillsImportLocalBatch,
  skillInstall,
  skillRepoDelete,
  skillRepoUpsert,
  skillReposList,
  skillSetEnabled,
  skillUninstall,
  skillsDiscoverAvailable,
  skillsInstalledList,
  skillsLocalList,
  skillsPathsGet,
} from "../../services/workspace/skills";
import { createQueryWrapper, createTestQueryClient } from "../../test/utils/reactQuery";
import { setTauriRuntime } from "../../test/utils/tauriRuntime";
import { skillsKeys } from "../keys";
import {
  useSkillImportLocalMutation,
  useSkillInstallToLocalMutation,
  useSkillLocalDeleteMutation,
  useSkillReturnToLocalMutation,
  useSkillsImportLocalBatchMutation,
  useSkillInstallMutation,
  useSkillRepoDeleteMutation,
  useSkillRepoUpsertMutation,
  useSkillReposListQuery,
  useSkillSetEnabledMutation,
  useSkillUninstallMutation,
  useSkillsDiscoverAvailableMutation,
  useSkillsDiscoverAvailableQuery,
  useSkillsInstalledListQuery,
  useSkillsLocalListQuery,
  useSkillsPathsQuery,
} from "../skills";

vi.mock("../../services/workspace/skills", async () => {
  const actual = await vi.importActual<typeof import("../../services/workspace/skills")>(
    "../../services/workspace/skills"
  );
  return {
    ...actual,
    skillReposList: vi.fn(),
    skillsInstalledList: vi.fn(),
    skillsLocalList: vi.fn(),
    skillsDiscoverAvailable: vi.fn(),
    skillsPathsGet: vi.fn(),
    skillRepoUpsert: vi.fn(),
    skillRepoDelete: vi.fn(),
    skillInstall: vi.fn(),
    skillInstallToLocal: vi.fn(),
    skillSetEnabled: vi.fn(),
    skillUninstall: vi.fn(),
    skillReturnToLocal: vi.fn(),
    skillLocalDelete: vi.fn(),
    skillImportLocal: vi.fn(),
    skillsImportLocalBatch: vi.fn(),
  };
});

describe("query/skills", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("does not call skillReposList when options.enabled=false", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useSkillReposListQuery({ enabled: false }), { wrapper });
    await Promise.resolve();

    expect(skillReposList).not.toHaveBeenCalled();
  });

  it("calls skillReposList with tauri runtime", async () => {
    setTauriRuntime();
    vi.mocked(skillReposList).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useSkillReposListQuery(), { wrapper });

    await waitFor(() => {
      expect(skillReposList).toHaveBeenCalled();
    });
  });

  it("useSkillReposListQuery enters error state when skillReposList rejects", async () => {
    setTauriRuntime();
    vi.mocked(skillReposList).mockRejectedValue(new Error("skills query boom"));

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillReposListQuery(), { wrapper });

    await waitFor(() => {
      expect(result.current.isError).toBe(true);
    });
  });

  it("useSkillsInstalledListQuery refetch returns null when workspaceId is null", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillsInstalledListQuery(null), { wrapper });
    await act(async () => {
      const res = await result.current.refetch();
      expect(res.data).toBeNull();
    });

    expect(skillsInstalledList).not.toHaveBeenCalled();
  });

  it("useSkillsInstalledListQuery calls skillsInstalledList when workspaceId is set", async () => {
    setTauriRuntime();
    vi.mocked(skillsInstalledList).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useSkillsInstalledListQuery(1), { wrapper });

    await waitFor(() => {
      expect(skillsInstalledList).toHaveBeenCalledWith(1);
    });
  });

  it("useSkillsInstalledListQuery drops stale data when workspaceId becomes null", async () => {
    setTauriRuntime();
    const rows: InstalledSkillSummary[] = [
      {
        id: 1,
        skill_key: "alpha",
        name: "Alpha",
        description: "",
        source_git_url: "https://example.com/acme/repo.git",
        source_branch: "main",
        source_subdir: "skills/alpha",
        enabled: true,
        created_at: 1,
        updated_at: 1,
      },
    ];
    vi.mocked(skillsInstalledList).mockResolvedValue(rows);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result, rerender } = renderHook(
      ({ workspaceId }: { workspaceId: number | null }) => useSkillsInstalledListQuery(workspaceId),
      {
        wrapper,
        initialProps: { workspaceId: 1 as number | null },
      }
    );

    await waitFor(() => {
      expect(result.current.data).toEqual(rows);
    });

    rerender({ workspaceId: null as number | null });

    await waitFor(() => {
      expect(result.current.data).toBeUndefined();
    });
  });

  it("useSkillsLocalListQuery refetch returns null when workspaceId is null", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillsLocalListQuery(null), { wrapper });
    await act(async () => {
      const res = await result.current.refetch();
      expect(res.data).toBeNull();
    });

    expect(skillsLocalList).not.toHaveBeenCalled();
  });

  it("useSkillsLocalListQuery calls skillsLocalList when workspaceId is set", async () => {
    setTauriRuntime();
    vi.mocked(skillsLocalList).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useSkillsLocalListQuery(1), { wrapper });

    await waitFor(() => {
      expect(skillsLocalList).toHaveBeenCalledWith(1);
    });
  });

  it("useSkillsLocalListQuery drops stale data when workspaceId becomes null", async () => {
    setTauriRuntime();
    const rows: LocalSkillSummary[] = [
      {
        dir_name: "alpha",
        path: "/tmp/alpha",
        name: "Alpha",
        description: "",
        source_git_url: "https://example.com/acme/repo.git",
        source_branch: "main",
        source_subdir: "skills/alpha",
      },
    ];
    vi.mocked(skillsLocalList).mockResolvedValue(rows);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result, rerender } = renderHook(
      ({ workspaceId }: { workspaceId: number | null }) => useSkillsLocalListQuery(workspaceId),
      {
        wrapper,
        initialProps: { workspaceId: 1 as number | null },
      }
    );

    await waitFor(() => {
      expect(result.current.data).toEqual(rows);
    });

    rerender({ workspaceId: null as number | null });

    await waitFor(() => {
      expect(result.current.data).toBeUndefined();
    });
  });

  it("useSkillsDiscoverAvailableQuery respects options.enabled=false", async () => {
    setTauriRuntime();
    vi.mocked(skillsDiscoverAvailable).mockResolvedValue([]);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useSkillsDiscoverAvailableQuery(false, { enabled: false }), { wrapper });
    await Promise.resolve();

    expect(skillsDiscoverAvailable).not.toHaveBeenCalled();
  });

  it("useSkillsPathsQuery refetch returns null when cliKey is null", async () => {
    setTauriRuntime();

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillsPathsQuery(null), { wrapper });
    await act(async () => {
      const res = await result.current.refetch();
      expect(res.data).toBeNull();
    });

    expect(skillsPathsGet).not.toHaveBeenCalled();
  });

  it("useSkillsPathsQuery calls skillsPathsGet when cliKey is set", async () => {
    setTauriRuntime();

    const paths: SkillsPaths = {
      ssot_dir: "/tmp/ssot",
      repos_dir: "/tmp/repos",
      cli_dir: "/tmp/cli",
    };
    vi.mocked(skillsPathsGet).mockResolvedValue(paths);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);

    renderHook(() => useSkillsPathsQuery("claude"), { wrapper });

    await waitFor(() => {
      expect(skillsPathsGet).toHaveBeenCalledWith("claude");
    });
  });

  it("useSkillsDiscoverAvailableMutation handles null rows", async () => {
    setTauriRuntime();
    vi.mocked(skillsDiscoverAvailable).mockResolvedValue(null);

    const client = createTestQueryClient();
    const wrapper = createQueryWrapper(client);
    const setSpy = vi.spyOn(client, "setQueryData");
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");

    const { result } = renderHook(() => useSkillsDiscoverAvailableMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync(true);
    });

    expect(setSpy).not.toHaveBeenCalled();
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: skillsKeys.discoverAvailable(true) });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: skillsKeys.discoverAvailable(false) });
  });

  it("useSkillsDiscoverAvailableMutation updates both refresh and cached query keys", async () => {
    setTauriRuntime();

    const rows: AvailableSkillSummary[] = [
      {
        name: "S1",
        description: "d",
        source_git_url: "https://example.com/repo.git",
        source_branch: "main",
        source_subdir: "skills/s1",
        installed: false,
      },
    ];
    vi.mocked(skillsDiscoverAvailable).mockResolvedValue(rows);

    const client = createTestQueryClient();
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillsDiscoverAvailableMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync(true);
    });

    expect(client.getQueryData(skillsKeys.discoverAvailable(true))).toEqual(rows);
    expect(client.getQueryData(skillsKeys.discoverAvailable(false))).toEqual(rows);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: skillsKeys.discoverAvailable(true) });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: skillsKeys.discoverAvailable(false) });
  });

  it("useSkillRepoUpsertMutation no-ops on null response", async () => {
    setTauriRuntime();
    vi.mocked(skillRepoUpsert).mockResolvedValue(null);

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.reposList(), []);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillRepoUpsertMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        repoId: null,
        gitUrl: "https://x",
        branch: "main",
        enabled: true,
      });
    });

    expect(client.getQueryData(skillsKeys.reposList())).toEqual([]);
  });

  it("useSkillRepoUpsertMutation updates repos list and invalidates discoverAvailable(false)", async () => {
    setTauriRuntime();

    const repo: SkillRepoSummary = {
      id: 1,
      git_url: "https://example.com/repo.git",
      branch: "main",
      enabled: true,
      created_at: 0,
      updated_at: 0,
    };
    vi.mocked(skillRepoUpsert).mockResolvedValue(repo);

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.reposList(), []);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillRepoUpsertMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        repoId: null,
        gitUrl: repo.git_url,
        branch: repo.branch,
        enabled: repo.enabled,
      });
    });

    expect(client.getQueryData(skillsKeys.reposList())).toEqual([repo]);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: skillsKeys.discoverAvailable(false) });
  });

  it("useSkillRepoUpsertMutation updates an existing repo row", async () => {
    setTauriRuntime();

    const prev: SkillRepoSummary[] = [
      {
        id: 1,
        git_url: "https://example.com/repo.git",
        branch: "main",
        enabled: true,
        created_at: 0,
        updated_at: 0,
      },
      {
        id: 2,
        git_url: "https://example.com/repo2.git",
        branch: "main",
        enabled: false,
        created_at: 0,
        updated_at: 0,
      },
    ];
    const updated: SkillRepoSummary = { ...prev[1]!, enabled: true };
    vi.mocked(skillRepoUpsert).mockResolvedValue(updated);

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.reposList(), prev);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillRepoUpsertMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        repoId: updated.id,
        gitUrl: updated.git_url,
        branch: updated.branch,
        enabled: updated.enabled,
      });
    });

    expect(client.getQueryData(skillsKeys.reposList())).toEqual([prev[0], updated]);
  });

  it("useSkillRepoDeleteMutation no-ops on false result", async () => {
    setTauriRuntime();
    vi.mocked(skillRepoDelete).mockResolvedValue(false);

    const repos: SkillRepoSummary[] = [
      {
        id: 1,
        git_url: "https://example.com/repo.git",
        branch: "main",
        enabled: true,
        created_at: 0,
        updated_at: 0,
      },
    ];

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.reposList(), repos);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillRepoDeleteMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync(1);
    });

    expect(client.getQueryData(skillsKeys.reposList())).toEqual(repos);
    expect(invalidateSpy).not.toHaveBeenCalled();
  });

  it("useSkillRepoDeleteMutation removes repo and invalidates discoverAvailable(false)", async () => {
    setTauriRuntime();
    vi.mocked(skillRepoDelete).mockResolvedValue(true);

    const repos: SkillRepoSummary[] = [
      {
        id: 1,
        git_url: "https://example.com/repo.git",
        branch: "main",
        enabled: true,
        created_at: 0,
        updated_at: 0,
      },
      {
        id: 2,
        git_url: "https://example.com/repo2.git",
        branch: "main",
        enabled: false,
        created_at: 0,
        updated_at: 0,
      },
    ];

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.reposList(), repos);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillRepoDeleteMutation(), { wrapper });
    await act(async () => {
      await result.current.mutateAsync(1);
    });

    expect(client.getQueryData(skillsKeys.reposList())).toEqual([repos[1]]);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: skillsKeys.discoverAvailable(false) });
  });

  it("useSkillInstallMutation no-ops on null response", async () => {
    setTauriRuntime();
    vi.mocked(skillInstall).mockResolvedValue(null);

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.installedList(1), []);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillInstallMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        gitUrl: "https://x",
        branch: "main",
        sourceSubdir: "s",
        enabled: true,
      });
    });

    expect(client.getQueryData(skillsKeys.installedList(1))).toEqual([]);
  });

  it("useSkillInstallMutation inserts into installed list and invalidates discoverAvailable(false)", async () => {
    setTauriRuntime();

    const installed: InstalledSkillSummary = {
      id: 10,
      skill_key: "s1",
      name: "S1",
      description: "d",
      source_git_url: "https://example.com/repo.git",
      source_branch: "main",
      source_subdir: "skills/s1",
      enabled: true,
      created_at: 0,
      updated_at: 0,
    };

    vi.mocked(skillInstall).mockResolvedValue(installed);

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.installedList(1), []);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillInstallMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        gitUrl: installed.source_git_url,
        branch: installed.source_branch,
        sourceSubdir: installed.source_subdir,
        enabled: true,
      });
    });

    expect(client.getQueryData(skillsKeys.installedList(1))).toEqual([installed]);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: skillsKeys.discoverAvailable(false) });
  });

  it("useSkillInstallMutation updates an existing installed row", async () => {
    setTauriRuntime();

    const prev: InstalledSkillSummary = {
      id: 10,
      skill_key: "s1",
      name: "S1",
      description: "d",
      source_git_url: "https://example.com/repo.git",
      source_branch: "main",
      source_subdir: "skills/s1",
      enabled: true,
      created_at: 0,
      updated_at: 0,
    };
    const updated = { ...prev, enabled: false };
    vi.mocked(skillInstall).mockResolvedValue(updated);

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.installedList(1), [prev]);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillInstallMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        gitUrl: updated.source_git_url,
        branch: updated.source_branch,
        sourceSubdir: updated.source_subdir,
        enabled: updated.enabled,
      });
    });

    expect(client.getQueryData(skillsKeys.installedList(1))).toEqual([updated]);
  });

  it("useSkillInstallToLocalMutation no-ops on null response", async () => {
    setTauriRuntime();
    vi.mocked(skillInstallToLocal).mockResolvedValue(null);

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.localList(1), []);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillInstallToLocalMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        gitUrl: "https://x",
        branch: "main",
        sourceSubdir: "s",
      });
    });

    expect(client.getQueryData(skillsKeys.localList(1))).toEqual([]);
  });

  it("useSkillInstallToLocalMutation inserts or updates local list rows", async () => {
    setTauriRuntime();

    const next: LocalSkillSummary = {
      dir_name: "skill-a",
      path: "/tmp/skill-a",
      name: "Skill A",
      description: "desc",
      source_git_url: "https://example.com/repo.git",
      source_branch: "main",
      source_subdir: "skills/a",
    };
    vi.mocked(skillInstallToLocal).mockResolvedValue(next);

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.localList(1), []);
    const wrapper = createQueryWrapper(client);

    const { result, rerender } = renderHook(() => useSkillInstallToLocalMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({
        gitUrl: next.source_git_url ?? "",
        branch: next.source_branch ?? "",
        sourceSubdir: next.source_subdir ?? "",
      });
    });

    expect(client.getQueryData(skillsKeys.localList(1))).toEqual([next]);

    vi.mocked(skillInstallToLocal).mockResolvedValue({ ...next, description: "updated" });
    rerender();
    await act(async () => {
      await result.current.mutateAsync({
        gitUrl: next.source_git_url ?? "",
        branch: next.source_branch ?? "",
        sourceSubdir: next.source_subdir ?? "",
      });
    });

    expect(client.getQueryData(skillsKeys.localList(1))).toEqual([
      expect.objectContaining({ dir_name: "skill-a", description: "updated" }),
    ]);
  });

  it("useSkillSetEnabledMutation no-ops on null response", async () => {
    setTauriRuntime();
    vi.mocked(skillSetEnabled).mockResolvedValue(null);

    const prev: InstalledSkillSummary = {
      id: 10,
      skill_key: "s1",
      name: "S1",
      description: "d",
      source_git_url: "https://example.com/repo.git",
      source_branch: "main",
      source_subdir: "skills/s1",
      enabled: true,
      created_at: 0,
      updated_at: 0,
    };

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.installedList(1), [prev]);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillSetEnabledMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ skillId: 10, enabled: false });
    });

    expect(client.getQueryData(skillsKeys.installedList(1))).toEqual([prev]);
  });

  it("useSkillSetEnabledMutation updates installed list row", async () => {
    setTauriRuntime();

    const prev: InstalledSkillSummary = {
      id: 10,
      skill_key: "s1",
      name: "S1",
      description: "d",
      source_git_url: "https://example.com/repo.git",
      source_branch: "main",
      source_subdir: "skills/s1",
      enabled: true,
      created_at: 0,
      updated_at: 0,
    };
    const updated = { ...prev, enabled: false };
    vi.mocked(skillSetEnabled).mockResolvedValue(updated);

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.installedList(1), [prev]);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillSetEnabledMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync({ skillId: 10, enabled: false });
    });

    expect(client.getQueryData(skillsKeys.installedList(1))).toEqual([updated]);
  });

  it("useSkillUninstallMutation no-ops on false result", async () => {
    setTauriRuntime();
    vi.mocked(skillUninstall).mockResolvedValue(false);

    const prev: InstalledSkillSummary[] = [
      {
        id: 10,
        skill_key: "s1",
        name: "S1",
        description: "d",
        source_git_url: "https://example.com/repo.git",
        source_branch: "main",
        source_subdir: "skills/s1",
        enabled: true,
        created_at: 0,
        updated_at: 0,
      },
    ];

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.installedList(1), prev);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillUninstallMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync(10);
    });

    expect(client.getQueryData(skillsKeys.installedList(1))).toEqual(prev);
    expect(invalidateSpy).not.toHaveBeenCalled();
  });

  it("useSkillUninstallMutation removes installed row and invalidates discoverAvailable(false)", async () => {
    setTauriRuntime();
    vi.mocked(skillUninstall).mockResolvedValue(true);

    const prev: InstalledSkillSummary[] = [
      {
        id: 10,
        skill_key: "s1",
        name: "S1",
        description: "d",
        source_git_url: "https://example.com/repo.git",
        source_branch: "main",
        source_subdir: "skills/s1",
        enabled: true,
        created_at: 0,
        updated_at: 0,
      },
    ];

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.installedList(1), prev);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillUninstallMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync(10);
    });

    expect(client.getQueryData(skillsKeys.installedList(1))).toEqual([]);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: skillsKeys.discoverAvailable(false) });
  });

  it("useSkillReturnToLocalMutation no-ops on false result", async () => {
    setTauriRuntime();
    vi.mocked(skillReturnToLocal).mockResolvedValue(false);

    const prev: InstalledSkillSummary[] = [
      {
        id: 10,
        skill_key: "s1",
        name: "S1",
        description: "d",
        source_git_url: "https://example.com/repo.git",
        source_branch: "main",
        source_subdir: "skills/s1",
        enabled: true,
        created_at: 0,
        updated_at: 0,
      },
    ];

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.installedList(1), prev);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillReturnToLocalMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync(10);
    });

    expect(client.getQueryData(skillsKeys.installedList(1))).toEqual(prev);
    expect(invalidateSpy).not.toHaveBeenCalled();
  });

  it("useSkillReturnToLocalMutation removes installed row and invalidates local/discover", async () => {
    setTauriRuntime();
    vi.mocked(skillReturnToLocal).mockResolvedValue(true);

    const prev: InstalledSkillSummary[] = [
      {
        id: 10,
        skill_key: "s1",
        name: "S1",
        description: "d",
        source_git_url: "https://example.com/repo.git",
        source_branch: "main",
        source_subdir: "skills/s1",
        enabled: true,
        created_at: 0,
        updated_at: 0,
      },
    ];

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.installedList(1), prev);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillReturnToLocalMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync(10);
    });

    expect(client.getQueryData(skillsKeys.installedList(1))).toEqual([]);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: skillsKeys.localList(1) });
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: skillsKeys.discoverAvailable(false) });
  });

  it("useSkillLocalDeleteMutation no-ops on false result", async () => {
    setTauriRuntime();
    vi.mocked(skillLocalDelete).mockResolvedValue(false);

    const prev: LocalSkillSummary[] = [
      {
        dir_name: "local-skill",
        name: "Local Skill",
        description: "d",
        path: "/tmp/local-skill",
      },
    ];

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.localList(1), prev);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillLocalDeleteMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync("local-skill");
    });

    expect(client.getQueryData(skillsKeys.localList(1))).toEqual(prev);
  });

  it("useSkillLocalDeleteMutation removes local row on success", async () => {
    setTauriRuntime();
    vi.mocked(skillLocalDelete).mockResolvedValue(true);

    const prev: LocalSkillSummary[] = [
      {
        dir_name: "local-skill",
        name: "Local Skill",
        description: "d",
        path: "/tmp/local-skill",
      },
      {
        dir_name: "other-skill",
        name: "Other Skill",
        description: "d2",
        path: "/tmp/other-skill",
      },
    ];

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.localList(1), prev);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillLocalDeleteMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync("local-skill");
    });

    expect(client.getQueryData(skillsKeys.localList(1))).toEqual([prev[1]]);
  });

  it("useSkillImportLocalMutation no-ops on null response", async () => {
    setTauriRuntime();
    vi.mocked(skillImportLocal).mockResolvedValue(null);

    const locals: LocalSkillSummary[] = [
      { dir_name: "s2", path: "/tmp/s2", name: "S2", description: "d2" },
    ];
    vi.mocked(skillsLocalList).mockResolvedValue(locals);

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.installedList(1), []);
    client.setQueryData(skillsKeys.localList(1), locals);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillImportLocalMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync("s2");
    });

    expect(client.getQueryData(skillsKeys.installedList(1))).toEqual([]);
  });

  it("useSkillImportLocalMutation inserts into installed list and invalidates localList", async () => {
    setTauriRuntime();

    const next: InstalledSkillSummary = {
      id: 11,
      skill_key: "s2",
      name: "S2",
      description: "d2",
      source_git_url: "local",
      source_branch: "local",
      source_subdir: "skills/s2",
      enabled: true,
      created_at: 0,
      updated_at: 0,
    };
    vi.mocked(skillImportLocal).mockResolvedValue(next);

    const locals: LocalSkillSummary[] = [
      { dir_name: "s2", path: "/tmp/s2", name: "S2", description: "d2" },
    ];
    vi.mocked(skillsLocalList).mockResolvedValue(locals);

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.installedList(1), []);
    client.setQueryData(skillsKeys.localList(1), locals);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillImportLocalMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync("s2");
    });

    expect(client.getQueryData(skillsKeys.installedList(1))).toEqual([next]);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: skillsKeys.localList(1) });
  });

  it("useSkillImportLocalMutation updates an existing installed row", async () => {
    setTauriRuntime();

    const prev: InstalledSkillSummary = {
      id: 11,
      skill_key: "s2",
      name: "S2",
      description: "d2",
      source_git_url: "local",
      source_branch: "local",
      source_subdir: "skills/s2",
      enabled: true,
      created_at: 0,
      updated_at: 0,
    };
    const updated = { ...prev, enabled: false };
    vi.mocked(skillImportLocal).mockResolvedValue(updated);

    const locals: LocalSkillSummary[] = [
      { dir_name: "s2", path: "/tmp/s2", name: "S2", description: "d2" },
    ];
    vi.mocked(skillsLocalList).mockResolvedValue(locals);

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.installedList(1), [prev]);
    client.setQueryData(skillsKeys.localList(1), locals);
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillImportLocalMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync("s2");
    });

    expect(client.getQueryData(skillsKeys.installedList(1))).toEqual([updated]);
  });

  it("useSkillsImportLocalBatchMutation no-ops on null response", async () => {
    setTauriRuntime();
    vi.mocked(skillsImportLocalBatch).mockResolvedValue(null);

    const prev: InstalledSkillSummary[] = [
      {
        id: 11,
        skill_key: "s2",
        name: "S2",
        description: "d2",
        source_git_url: "local",
        source_branch: "local",
        source_subdir: "skills/s2",
        enabled: true,
        created_at: 0,
        updated_at: 0,
      },
    ];

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.installedList(1), prev);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillsImportLocalBatchMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync(["s2"]);
    });

    expect(client.getQueryData(skillsKeys.installedList(1))).toEqual(prev);
    expect(invalidateSpy).not.toHaveBeenCalled();
  });

  it("useSkillsImportLocalBatchMutation merges imported rows and invalidates localList", async () => {
    setTauriRuntime();

    vi.mocked(skillsImportLocalBatch).mockResolvedValue({
      imported: [
        {
          id: 12,
          skill_key: "s3",
          name: "S3",
          description: "d3",
          source_git_url: "local",
          source_branch: "local",
          source_subdir: "skills/s3",
          enabled: true,
          created_at: 0,
          updated_at: 0,
        },
      ],
      skipped: [],
      failed: [],
    });

    const prev: InstalledSkillSummary[] = [
      {
        id: 11,
        skill_key: "s2",
        name: "S2",
        description: "d2",
        source_git_url: "local",
        source_branch: "local",
        source_subdir: "skills/s2",
        enabled: true,
        created_at: 0,
        updated_at: 0,
      },
    ];

    const client = createTestQueryClient();
    client.setQueryData(skillsKeys.installedList(1), prev);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    const wrapper = createQueryWrapper(client);

    const { result } = renderHook(() => useSkillsImportLocalBatchMutation(1), { wrapper });
    await act(async () => {
      await result.current.mutateAsync(["s3"]);
    });

    expect(client.getQueryData(skillsKeys.installedList(1))).toEqual([
      prev[0],
      expect.objectContaining({ id: 12, skill_key: "s3" }),
    ]);
    expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: skillsKeys.localList(1) });
  });
});
