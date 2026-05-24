import { describe, expect, it, vi } from "vitest";
import { commands } from "../../../generated/bindings";
import { logToConsole } from "../../consoleLog";
import {
  SKILLS_IMPORT_LOCAL_MAX_DIR_NAMES,
  type InstalledSkillSummary,
  type LocalSkillSummary,
  type SkillImportLocalBatchReport,
  type SkillRepoSummary,
  skillImportLocal,
  skillLocalDelete,
  skillReturnToLocal,
  skillInstall,
  skillInstallToLocal,
  skillRepoDelete,
  skillRepoUpsert,
  skillReposList,
  skillSetEnabled,
  skillUninstall,
  skillsDiscoverAvailable,
  skillsImportLocalBatch,
  skillsLocalList,
  skillsPathsGet,
  normalizeSkillLocalDirName,
  normalizeSkillsLocalDirNames,
  validateSkillId,
  validateSkillRepoId,
  validateSkillsCliKey,
  validateSkillsWorkspaceId,
} from "../skills";

vi.mock("../../../generated/bindings", async () => {
  const actual = await vi.importActual<typeof import("../../../generated/bindings")>(
    "../../../generated/bindings"
  );
  return {
    ...actual,
    commands: {
      ...actual.commands,
      skillReposList: vi.fn(),
      skillRepoUpsert: vi.fn(),
      skillRepoDelete: vi.fn(),
      skillsDiscoverAvailable: vi.fn(),
      skillInstall: vi.fn(),
      skillSetEnabled: vi.fn(),
      skillInstallToLocal: vi.fn(),
      skillUninstall: vi.fn(),
      skillReturnToLocal: vi.fn(),
      skillsLocalList: vi.fn(),
      skillLocalDelete: vi.fn(),
      skillImportLocal: vi.fn(),
      skillsImportLocalBatch: vi.fn(),
      skillsPathsGet: vi.fn(),
    },
  };
});

vi.mock("../../consoleLog", async () => {
  const actual = await vi.importActual<typeof import("../../consoleLog")>("../../consoleLog");
  return {
    ...actual,
    logToConsole: vi.fn(),
  };
});

describe("services/workspace/skills", () => {
  function createSkillRepoSummary(overrides: Partial<SkillRepoSummary> = {}): SkillRepoSummary {
    return {
      id: 1,
      git_url: "https://example.com/repo.git",
      branch: "main",
      enabled: true,
      created_at: 0,
      updated_at: 0,
      ...overrides,
    };
  }

  function createInstalledSkillSummary(
    overrides: Partial<InstalledSkillSummary> = {}
  ): InstalledSkillSummary {
    return {
      id: 1,
      skill_key: "skill-a",
      name: "Skill A",
      description: "desc",
      source_git_url: "https://example.com/repo.git",
      source_branch: "main",
      source_subdir: "skills/a",
      installed_commit: null,
      enabled: true,
      created_at: 0,
      updated_at: 0,
      ...overrides,
    };
  }

  function createLocalSkillSummary(overrides: Partial<LocalSkillSummary> = {}): LocalSkillSummary {
    return {
      dir_name: "skill-a",
      path: "/tmp/skill-a",
      name: "Skill A",
      description: "desc",
      source_git_url: "https://example.com/repo.git",
      source_branch: "main",
      source_subdir: "skills/a",
      ...overrides,
    };
  }

  function createSkillImportLocalBatchReport(
    overrides: Partial<SkillImportLocalBatchReport> = {}
  ): SkillImportLocalBatchReport {
    return {
      imported: [],
      skipped: [],
      failed: [],
      ...overrides,
    };
  }

  it("rethrows invoke errors and logs", async () => {
    vi.mocked(commands.skillReposList).mockRejectedValueOnce(new Error("skills boom"));

    await expect(skillReposList()).rejects.toThrow("skills boom");
    expect(logToConsole).toHaveBeenCalledWith(
      "error",
      "读取技能仓库列表失败",
      expect.objectContaining({
        cmd: "skill_repos_list",
        error: expect.stringContaining("skills boom"),
      })
    );
  });

  it("treats null invoke result as error with runtime", async () => {
    vi.mocked(commands.skillReposList).mockResolvedValueOnce(null as never);

    await expect(skillReposList()).rejects.toThrow("IPC_NULL_RESULT: skill_repos_list");
  });

  it("keeps argument mapping unchanged", async () => {
    vi.mocked(commands.skillRepoUpsert).mockResolvedValue({
      status: "ok",
      data: createSkillRepoSummary(),
    });
    vi.mocked(commands.skillRepoDelete).mockResolvedValue({ status: "ok", data: true });
    vi.mocked(commands.skillsDiscoverAvailable).mockResolvedValue({
      status: "ok",
      data: [],
    });
    vi.mocked(commands.skillInstall).mockResolvedValue({
      status: "ok",
      data: createInstalledSkillSummary(),
    });
    vi.mocked(commands.skillSetEnabled).mockResolvedValue({
      status: "ok",
      data: createInstalledSkillSummary({ enabled: false }),
    });
    vi.mocked(commands.skillInstallToLocal).mockResolvedValue({
      status: "ok",
      data: createLocalSkillSummary(),
    });
    vi.mocked(commands.skillUninstall).mockResolvedValue({ status: "ok", data: true });
    vi.mocked(commands.skillReturnToLocal).mockResolvedValue({ status: "ok", data: true });
    vi.mocked(commands.skillsLocalList).mockResolvedValue({ status: "ok", data: [] });
    vi.mocked(commands.skillLocalDelete).mockResolvedValue({ status: "ok", data: true });
    vi.mocked(commands.skillImportLocal).mockResolvedValue({
      status: "ok",
      data: createInstalledSkillSummary(),
    });
    vi.mocked(commands.skillsImportLocalBatch).mockResolvedValue({
      status: "ok",
      data: createSkillImportLocalBatchReport(),
    });
    vi.mocked(commands.skillsPathsGet).mockResolvedValue({
      status: "ok",
      data: { ssot_dir: "", repos_dir: "", cli_dir: "" },
    });

    await skillRepoUpsert({
      repoId: null,
      gitUrl: " https://example.com/repo.git ",
      branch: " main ",
      enabled: true,
    });
    expect(commands.skillRepoUpsert).toHaveBeenCalledWith(
      null,
      "https://example.com/repo.git",
      "main",
      true
    );

    await skillRepoDelete(1);
    expect(commands.skillRepoDelete).toHaveBeenCalledWith(1);

    await skillsDiscoverAvailable(true);
    expect(commands.skillsDiscoverAvailable).toHaveBeenCalledWith(true);

    await skillInstall({
      workspaceId: 1,
      gitUrl: " https://example.com/repo.git ",
      branch: " main ",
      sourceSubdir: " skills/a ",
      enabled: true,
    });
    expect(commands.skillInstall).toHaveBeenCalledWith(
      1,
      "https://example.com/repo.git",
      "main",
      "skills/a",
      true
    );

    await skillSetEnabled({ workspaceId: 1, skillId: 2, enabled: false });
    expect(commands.skillSetEnabled).toHaveBeenCalledWith(1, 2, false);

    await skillInstallToLocal({
      workspaceId: 1,
      gitUrl: " https://example.com/repo.git ",
      branch: " main ",
      sourceSubdir: " skills/a ",
    });
    expect(commands.skillInstallToLocal).toHaveBeenCalledWith(
      1,
      "https://example.com/repo.git",
      "main",
      "skills/a"
    );

    await skillUninstall(2);
    expect(commands.skillUninstall).toHaveBeenCalledWith(2);

    await skillReturnToLocal({ workspaceId: 1, skillId: 2 });
    expect(commands.skillReturnToLocal).toHaveBeenCalledWith(1, 2);

    await skillsLocalList(1);
    expect(commands.skillsLocalList).toHaveBeenCalledWith(1);

    await skillLocalDelete({ workspaceId: 1, dirName: " my-skill " });
    expect(commands.skillLocalDelete).toHaveBeenCalledWith(
      1,
      "my-skill",
      expect.objectContaining({
        confirm: expect.objectContaining({
          action: "skill_local_delete",
          resource: "workspace:1:skill-local:my-skill",
          nonce: expect.any(String),
        }),
      })
    );

    await skillImportLocal({ workspaceId: 1, dirName: " my-skill " });
    expect(commands.skillImportLocal).toHaveBeenCalledWith(1, "my-skill");

    await skillsImportLocalBatch({ workspaceId: 1, dirNames: [" a ", "", "b", "a"] });
    expect(commands.skillsImportLocalBatch).toHaveBeenCalledWith(1, ["a", "b"]);

    await skillsPathsGet(" claude " as Parameters<typeof skillsPathsGet>[0]);
    expect(commands.skillsPathsGet).toHaveBeenCalledWith("claude");
  });

  it("normalizes skills path CLI keys before generated commands", async () => {
    vi.mocked(commands.skillsPathsGet).mockResolvedValue({
      status: "ok",
      data: { ssot_dir: "", repos_dir: "", cli_dir: "" },
    });

    expect(validateSkillsCliKey(" claude ")).toBe("claude");
    await skillsPathsGet(" codex " as Parameters<typeof skillsPathsGet>[0]);
    expect(commands.skillsPathsGet).toHaveBeenCalledWith("codex");
  });

  it("normalizes skills local directory batches", () => {
    expect(normalizeSkillLocalDirName(" my-skill ")).toBe("my-skill");
    expect(normalizeSkillsLocalDirNames([" a ", "", "b", "a"])).toEqual(["a", "b"]);
    expect(() => normalizeSkillLocalDirName("../x")).toThrow("SEC_INVALID_INPUT");
    expect(() => normalizeSkillLocalDirName("a/b")).toThrow("SEC_INVALID_INPUT");
    expect(() => normalizeSkillsLocalDirNames(["", "   "])).toThrow("SEC_INVALID_INPUT");
  });

  it("rejects invalid ids, source paths, and oversized batches before generated commands", async () => {
    expect(validateSkillsWorkspaceId(1)).toBe(1);
    expect(validateSkillId(2)).toBe(2);
    expect(validateSkillRepoId(3)).toBe(3);
    expect(() => validateSkillsWorkspaceId(0)).toThrow("SEC_INVALID_INPUT");
    expect(() => validateSkillId(Number.NaN)).toThrow("SEC_INVALID_INPUT");
    expect(() => validateSkillRepoId(-1)).toThrow("SEC_INVALID_INPUT");

    await expect(skillsLocalList(0)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(skillRepoDelete(0)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(skillUninstall(Number.NaN)).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(skillSetEnabled({ workspaceId: 1, skillId: 0, enabled: true })).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );
    await expect(skillReturnToLocal({ workspaceId: 0, skillId: 1 })).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );
    await expect(skillImportLocal({ workspaceId: 1, dirName: "../bad" })).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );
    await expect(skillLocalDelete({ workspaceId: 1, dirName: "bad/path" })).rejects.toThrow(
      "SEC_INVALID_INPUT"
    );
    await expect(
      skillInstall({
        workspaceId: 1,
        gitUrl: "https://example.com/repo.git",
        branch: "main",
        sourceSubdir: "../skills/a",
        enabled: true,
      })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      skillInstallToLocal({
        workspaceId: 1,
        gitUrl: "https://example.com/repo.git",
        branch: "main",
        sourceSubdir: "/skills/a",
      })
    ).rejects.toThrow("SEC_INVALID_INPUT");

    const tooManyDirNames = Array.from(
      { length: SKILLS_IMPORT_LOCAL_MAX_DIR_NAMES + 1 },
      (_, index) => `skill-${index}`
    );
    await expect(
      skillsImportLocalBatch({ workspaceId: 1, dirNames: tooManyDirNames })
    ).rejects.toThrow("SEC_INVALID_INPUT");
    await expect(
      skillsPathsGet("opencode" as Parameters<typeof skillsPathsGet>[0])
    ).rejects.toThrow("SEC_INVALID_INPUT");

    expect(commands.skillsLocalList).not.toHaveBeenCalledWith(0);
    expect(commands.skillRepoDelete).not.toHaveBeenCalledWith(0);
    expect(commands.skillUninstall).not.toHaveBeenCalledWith(Number.NaN);
    expect(commands.skillImportLocal).not.toHaveBeenCalledWith(1, "../bad");
    expect(commands.skillLocalDelete).not.toHaveBeenCalledWith(1, "bad/path", expect.anything());
    expect(commands.skillsImportLocalBatch).not.toHaveBeenCalledWith(1, tooManyDirNames);
    expect(commands.skillsPathsGet).not.toHaveBeenCalledWith("opencode");
  });
});
