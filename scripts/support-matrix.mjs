import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const logger = {
  info(message, ...args) {
    console.error(message, ...args);
  },
  error(message, ...args) {
    console.error(message, ...args);
  },
};

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = dirname(scriptDir);

/*
 * ============================================================================
 * 步骤1：集中定义支持矩阵
 * ============================================================================
 * 目标：
 *   1) 用一份定义覆盖 release workflow、latest.json 和 README 文案
 *   2) 明确区分官方支持目标与仅本地构建目标
 * 数据源：
 *   1) 当前 release workflow 实际产物
 *   2) 当前 package scripts 中存在的跨平台构建命令
 * 操作要点：
 *   1) 进入官方矩阵的目标必须同时具备 release 产物与 updater 合约
 *   2) 仅本地构建目标保留脚本，但不进入 release / latest.json
 */
const OFFICIAL_RELEASE_TARGETS = Object.freeze([
  {
    id: "windows-x64",
    osFamily: "windows",
    runner: "windows-latest",
    target: "x86_64-pc-windows-msvc",
    bundles: "msi",
    updaterPlatform: "windows-x86_64",
    stableLabel: "win64",
    stableAssetKind: "msi",
    packageScript: "tauri:build:win:x64",
    packageCommand: "node scripts/tauri-build.mjs --target x86_64-pc-windows-msvc",
    releaseDownloadLabel: {
      zh: "Windows x64",
      en: "Windows x64",
    },
    releaseDownloadPackages: {
      zh: "`.msi` / `-portable.zip`",
      en: "`.msi` / `-portable.zip`",
    },
    sourceBuildNote: {
      zh: "官方支持；进入 Release / updater 矩阵",
      en: "Official; included in Release / updater matrix",
    },
    latestAssetName: "aio-coding-hub-win64.msi",
    latestSignatureName: "aio-coding-hub-win64.msi.sig",
  },
  {
    id: "macos-x64",
    osFamily: "macos",
    runner: "macos-latest",
    target: "x86_64-apple-darwin",
    bundles: "app",
    updaterPlatform: "darwin-x86_64",
    stableLabel: "macos-intel",
    stableAssetKind: "tarball",
    packageScript: "tauri:build:mac:x64",
    packageCommand: "node scripts/tauri-build.mjs --target x86_64-apple-darwin",
    releaseDownloadLabel: {
      zh: "macOS Intel",
      en: "macOS Intel",
    },
    releaseDownloadPackages: {
      zh: "`.zip`",
      en: "`.zip`",
    },
    sourceBuildNote: {
      zh: "官方支持；进入 Release / updater 矩阵",
      en: "Official; included in Release / updater matrix",
    },
    latestAssetName: "aio-coding-hub-macos-intel.tar.gz",
    latestSignatureName: "aio-coding-hub-macos-intel.tar.gz.sig",
  },
  {
    id: "macos-arm64",
    osFamily: "macos",
    runner: "macos-latest",
    target: "aarch64-apple-darwin",
    bundles: "app",
    updaterPlatform: "darwin-aarch64",
    stableLabel: "macos-arm",
    stableAssetKind: "tarball",
    packageScript: "tauri:build:mac:arm64",
    packageCommand: "node scripts/tauri-build.mjs --target aarch64-apple-darwin",
    releaseDownloadLabel: {
      zh: "macOS Apple Silicon",
      en: "macOS Apple Silicon",
    },
    releaseDownloadPackages: {
      zh: "`.zip`",
      en: "`.zip`",
    },
    sourceBuildNote: {
      zh: "官方支持；进入 Release / updater 矩阵",
      en: "Official; included in Release / updater matrix",
    },
    latestAssetName: "aio-coding-hub-macos-arm.tar.gz",
    latestSignatureName: "aio-coding-hub-macos-arm.tar.gz.sig",
  },
  {
    id: "linux-x64",
    osFamily: "linux",
    runner: "ubuntu-22.04",
    target: "x86_64-unknown-linux-gnu",
    bundles: "deb,appimage",
    updaterPlatform: "linux-x86_64",
    stableLabel: "linux-amd64",
    stableAssetKind: "appimage",
    packageScript: "tauri:build:linux:x64",
    packageCommand: "node scripts/tauri-build.mjs --target x86_64-unknown-linux-gnu",
    releaseDownloadLabel: {
      zh: "Linux x64",
      en: "Linux x64",
    },
    releaseDownloadPackages: {
      zh: "`.deb` / `.AppImage` / `-wayland.AppImage`",
      en: "`.deb` / `.AppImage` / `-wayland.AppImage`",
    },
    sourceBuildNote: {
      zh: "官方支持；进入 Release / updater 矩阵",
      en: "Official; included in Release / updater matrix",
    },
    latestAssetName: "aio-coding-hub-linux-amd64.AppImage",
    latestSignatureName: "aio-coding-hub-linux-amd64.AppImage.sig",
  },
]);

const LOCAL_BUILD_ONLY_TARGETS = Object.freeze([
  {
    id: "macos-universal",
    packageScript: "tauri:build:mac:universal",
    packageCommand: "node scripts/tauri-build.mjs --target universal-apple-darwin",
    buildLabel: {
      zh: "macOS Universal",
      en: "macOS Universal",
    },
    sourceBuildNote: {
      zh: "仅本地构建；不进入官方发布 / updater 矩阵",
      en: "Local build only; excluded from the official release / updater matrix",
    },
  },
  {
    id: "windows-arm64",
    packageScript: "tauri:build:win:arm64",
    packageCommand: "node scripts/tauri-build.mjs --target aarch64-pc-windows-msvc",
    buildLabel: {
      zh: "Windows ARM64",
      en: "Windows ARM64",
    },
    sourceBuildNote: {
      zh: "仅本地构建；不进入官方发布 / updater 矩阵",
      en: "Local build only; excluded from the official release / updater matrix",
    },
  },
]);

const README_MARKERS = Object.freeze({
  releaseDownload: {
    start: "<!-- SUPPORT_MATRIX_RELEASE_DOWNLOAD:START -->",
    end: "<!-- SUPPORT_MATRIX_RELEASE_DOWNLOAD:END -->",
  },
  sourceBuild: {
    start: "<!-- SUPPORT_MATRIX_SOURCE_BUILD:START -->",
    end: "<!-- SUPPORT_MATRIX_SOURCE_BUILD:END -->",
  },
});

const README_LOCALES = Object.freeze([
  {
    fileName: "README.md",
    locale: "zh",
  },
  {
    fileName: "README_EN.md",
    locale: "en",
  },
]);

const EXPECTED_DESKTOP_OS_FAMILIES = Object.freeze(["windows", "macos", "linux"]);

const WORKFLOW_PATHS = Object.freeze({
  ci: join(repoRoot, ".github/workflows/ci.yml"),
  release: join(repoRoot, ".github/workflows/release.yml"),
  releasePrSyncCargoLock: join(repoRoot, ".github/workflows/release-pr-sync-cargo-lock.yml"),
});

function getAllBuildTargets() {
  return [
    ...OFFICIAL_RELEASE_TARGETS.map((item) => ({
      packageScript: item.packageScript,
      packageCommand: item.packageCommand,
      buildLabel: item.releaseDownloadLabel,
      sourceBuildNote: item.sourceBuildNote,
      official: true,
    })),
    ...LOCAL_BUILD_ONLY_TARGETS.map((item) => ({
      packageScript: item.packageScript,
      packageCommand: item.packageCommand,
      buildLabel: item.buildLabel,
      sourceBuildNote: item.sourceBuildNote,
      official: false,
    })),
  ];
}

function renderMarkdownTable(headers, rows) {
  const headerLine = `| ${headers.join(" | ")} |`;
  const separatorLine = `| ${headers.map(() => "---").join(" | ")} |`;
  const bodyLines = rows.map((row) => `| ${row.join(" | ")} |`);
  return [headerLine, separatorLine, ...bodyLines].join("\n");
}

function renderReadmeReleaseDownloadTable(locale) {
  const headers =
    locale === "zh" ? ["平台", "官方发布安装包"] : ["Platform", "Official release packages"];
  const rows = OFFICIAL_RELEASE_TARGETS.map((item) => [
    item.releaseDownloadLabel[locale],
    item.releaseDownloadPackages[locale],
  ]);
  return renderMarkdownTable(headers, rows);
}

function renderReadmeSourceBuildTable(locale) {
  const headers = locale === "zh" ? ["分类", "命令", "说明"] : ["Scope", "Command", "Notes"];
  const separator = locale === "zh" ? "；" : "; ";
  const rows = getAllBuildTargets().map((item) => [
    item.official
      ? locale === "zh"
        ? "官方支持"
        : "Official"
      : locale === "zh"
        ? "本地构建"
        : "Local only",
    `\`pnpm ${item.packageScript}\``,
    `${item.buildLabel[locale]}${separator}${item.sourceBuildNote[locale]}`,
  ]);
  return renderMarkdownTable(headers, rows);
}

function renderReadmeBlock(section, locale) {
  const markers = README_MARKERS[section];
  const table =
    section === "releaseDownload"
      ? renderReadmeReleaseDownloadTable(locale)
      : renderReadmeSourceBuildTable(locale);
  return `${markers.start}\n${table}\n${markers.end}`;
}

function buildWorkflowMatrix() {
  return OFFICIAL_RELEASE_TARGETS.map((item) => ({
    platform: item.runner,
    target: item.target,
    bundles: item.bundles,
    updater_platform: item.updaterPlatform,
    stable_label: item.stableLabel,
  }));
}

function buildDesktopCiMatrix() {
  const seenFamilies = new Set();

  return OFFICIAL_RELEASE_TARGETS.filter((item) => {
    if (seenFamilies.has(item.osFamily)) {
      return false;
    }
    seenFamilies.add(item.osFamily);
    return true;
  }).map((item) => ({
    os_family: item.osFamily,
    runner: item.runner,
  }));
}

function parseArgs(rawArgs) {
  const args = new Map();

  for (let index = 0; index < rawArgs.length; index += 1) {
    const token = rawArgs[index];
    if (!token.startsWith("--")) {
      throw new Error(`Unexpected argument: ${token}`);
    }

    const key = token.slice(2);
    const value = rawArgs[index + 1];
    if (value == null || value.startsWith("--")) {
      throw new Error(`Missing value for argument: ${token}`);
    }

    args.set(key, value);
    index += 1;
  }

  return args;
}

function requireArg(args, key) {
  const value = args.get(key);
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`Missing required argument: --${key}`);
  }
  return value;
}

function normalizeReleaseVersion(tag, repo) {
  const repoName = repo.split("/").at(-1) ?? "";
  if (repoName.length > 0 && tag.startsWith(`${repoName}-v`)) {
    return tag.slice(repoName.length + 2);
  }
  if (tag.startsWith("v")) {
    return tag.slice(1);
  }
  return tag;
}

function loadSignature(stableAssetsDir, signatureName) {
  const signaturePath = join(stableAssetsDir, signatureName);
  if (!existsSync(signaturePath)) {
    throw new Error(`Missing signature file: ${signaturePath}`);
  }
  return readFileSync(signaturePath, "utf8").replace(/[\r\n]+/g, "");
}

function findOfficialTargetByUpdaterPlatform(updaterPlatform) {
  return OFFICIAL_RELEASE_TARGETS.find((item) => item.updaterPlatform === updaterPlatform) ?? null;
}

function assertExpectedOsFamilies() {
  const actualFamilies = [...new Set(OFFICIAL_RELEASE_TARGETS.map((item) => item.osFamily))].sort();
  const expectedFamilies = [...EXPECTED_DESKTOP_OS_FAMILIES].sort();

  if (actualFamilies.length !== expectedFamilies.length) {
    throw new Error(
      `Desktop OS family drifted. Expected: ${expectedFamilies.join(", ")}. Actual: ${actualFamilies.join(", ")}.`
    );
  }

  for (let index = 0; index < expectedFamilies.length; index += 1) {
    if (actualFamilies[index] !== expectedFamilies[index]) {
      throw new Error(
        `Desktop OS family drifted. Expected: ${expectedFamilies.join(", ")}. Actual: ${actualFamilies.join(", ")}.`
      );
    }
  }
}

function pickArtifact(artifactPaths, predicate, label) {
  const picked = artifactPaths.find((item) => typeof item === "string" && predicate(item));
  if (!picked) {
    throw new Error(
      `Missing required artifact: ${label}\nAvailable artifacts:\n${artifactPaths.map((item) => `- ${item}`).join("\n")}`
    );
  }
  return picked;
}

function copyArtifact(sourcePath, outputDir, outputName) {
  const destinationPath = join(outputDir, outputName);
  copyFileSync(sourcePath, destinationPath);
  logger.info("[support-matrix] 复制产物：%s -> %s", sourcePath, destinationPath);
}

function buildLatestJson({ tag, repo, pubDate, stableAssetsDir, releaseBody, fallbackNotes }) {
  const platforms = {};

  for (const target of OFFICIAL_RELEASE_TARGETS) {
    platforms[target.updaterPlatform] = {
      signature: loadSignature(stableAssetsDir, target.latestSignatureName),
      url: `https://github.com/${repo}/releases/download/${tag}/${target.latestAssetName}`,
    };
  }

  const notes =
    typeof releaseBody === "string" && releaseBody.trim().length > 0 ? releaseBody : fallbackNotes;

  return {
    version: normalizeReleaseVersion(tag, repo),
    notes,
    pub_date: pubDate,
    platforms,
  };
}

function extractMarkedBlock(content, markerName) {
  const { start, end } = README_MARKERS[markerName];
  const startIndex = content.indexOf(start);
  const endIndex = content.indexOf(end);

  if (startIndex === -1 || endIndex === -1 || endIndex < startIndex) {
    throw new Error(`Missing README markers: ${start} ... ${end}`);
  }

  const blockEnd = endIndex + end.length;
  return content.slice(startIndex, blockEnd);
}

function assertUniqueTargets(items, getValue, label) {
  const seen = new Set();

  for (const item of items) {
    const value = getValue(item);
    if (seen.has(value)) {
      throw new Error(`Duplicate ${label}: ${value}`);
    }
    seen.add(value);
  }
}

function checkPackageScripts() {
  const packageJsonPath = join(repoRoot, "package.json");
  const packageJson = JSON.parse(readFileSync(packageJsonPath, "utf8"));
  const scripts = packageJson.scripts ?? {};
  const expectedBuildTargets = getAllBuildTargets();
  const allowedBuildScripts = new Set(expectedBuildTargets.map((item) => item.packageScript));

  for (const item of expectedBuildTargets) {
    if (scripts[item.packageScript] !== item.packageCommand) {
      throw new Error(
        `package.json script drifted: ${item.packageScript}\nExpected: ${item.packageCommand}\nActual: ${scripts[item.packageScript] ?? "<missing>"}`
      );
    }
  }

  const unexpectedBuildScripts = Object.keys(scripts)
    .filter((name) => name.startsWith("tauri:build:"))
    .filter((name) => !allowedBuildScripts.has(name));

  if (unexpectedBuildScripts.length > 0) {
    throw new Error(
      `Unexpected tauri build scripts outside support matrix: ${unexpectedBuildScripts.join(", ")}`
    );
  }

  if (scripts["check:support-matrix"] !== "node scripts/support-matrix.mjs check") {
    throw new Error("package.json must expose check:support-matrix.");
  }

  if (scripts["audit:deps"] !== "node scripts/check-pnpm-audit.mjs") {
    throw new Error("package.json must expose a fail-close audit:deps script.");
  }
}

function checkReadmes() {
  for (const item of README_LOCALES) {
    const readmePath = join(repoRoot, item.fileName);
    const content = readFileSync(readmePath, "utf8");

    for (const markerName of Object.keys(README_MARKERS)) {
      const actualBlock = extractMarkedBlock(content, markerName).trim();
      const expectedBlock = renderReadmeBlock(markerName, item.locale).trim();
      if (actualBlock !== expectedBlock) {
        throw new Error(
          `${item.fileName} drifted in ${markerName}. Update the support matrix block.`
        );
      }
    }
  }
}

function assertWorkflowContains(content, snippet, label) {
  if (!content.includes(snippet)) {
    throw new Error(`Workflow contract drifted: missing ${label}`);
  }
}

function checkPinnedGithubActions(workflowPath) {
  const content = readFileSync(workflowPath, "utf8");
  const usesMatches = content.matchAll(/^\s*(?:-\s+)?uses:\s+([^@\s]+)@([^\s#]+)/gm);

  for (const match of usesMatches) {
    const actionRef = match[1];
    const versionRef = match[2];

    if (actionRef.startsWith("./") || actionRef.startsWith("docker://")) {
      continue;
    }

    if (!/^[0-9a-f]{40}$/.test(versionRef)) {
      throw new Error(
        `Workflow action must pin to a full commit SHA: ${workflowPath} -> ${actionRef}@${versionRef}`
      );
    }
  }
}

function checkWorkflowContracts() {
  const ciWorkflow = readFileSync(WORKFLOW_PATHS.ci, "utf8");
  const releaseWorkflow = readFileSync(WORKFLOW_PATHS.release, "utf8");

  assertWorkflowContains(
    ciWorkflow,
    "desktop_matrix=$(node scripts/support-matrix.mjs ci-matrix)",
    "ci desktop matrix loader"
  );
  assertWorkflowContains(
    ciWorkflow,
    "include: ${{ fromJson(needs.support-contract.outputs.desktop_matrix) }}",
    "ci desktop matrix usage"
  );
  assertWorkflowContains(ciWorkflow, "run: pnpm check:support-matrix", "ci support matrix check");
  assertWorkflowContains(ciWorkflow, "run: pnpm audit:deps", "ci fail-close dependency audit");

  assertWorkflowContains(
    releaseWorkflow,
    "run: node scripts/support-matrix.mjs check",
    "release support matrix validation"
  );
  assertWorkflowContains(
    releaseWorkflow,
    'echo "build_matrix=$(node scripts/support-matrix.mjs build-matrix)" >> "$GITHUB_OUTPUT"',
    "release matrix output"
  );
  assertWorkflowContains(
    releaseWorkflow,
    "include: ${{ fromJson(needs.release-please.outputs.build_matrix) }}",
    "release matrix usage"
  );
  assertWorkflowContains(
    releaseWorkflow,
    "node scripts/support-matrix.mjs prepare-stable-assets \\",
    "stable asset preparation delegation"
  );
  assertWorkflowContains(
    releaseWorkflow,
    "node scripts/support-matrix.mjs generate-latest-json \\",
    "latest.json generation delegation"
  );
}

function runSupportMatrixCheck() {
  /*
   * ============================================================================
   * 步骤2：校验单一矩阵与外部引用是否一致
   * ============================================================================
   * 目标：
   *   1) 防止 package scripts、workflow 与 README 再次各写一份
   *   2) 在 CI / release 中提前拦截支持矩阵和 action pin 漂移
   * 数据源：
   *   1) package.json
   *   2) README.md / README_EN.md
   *   3) .github/workflows/*.yml
   * 操作要点：
   *   1) 只允许矩阵中登记过的 tauri:build:* 脚本
   *   2) README 标记块必须与矩阵渲染结果完全一致
   *   3) release 关键 workflow 只能消费 support-matrix 导出的契约
   */
  logger.info("[support-matrix] 开始校验支持矩阵...");

  // 2.1 先校验内部定义没有重复键
  assertUniqueTargets(OFFICIAL_RELEASE_TARGETS, (item) => item.id, "official target id");
  assertUniqueTargets(OFFICIAL_RELEASE_TARGETS, (item) => item.target, "rust target");
  assertUniqueTargets(OFFICIAL_RELEASE_TARGETS, (item) => item.updaterPlatform, "updater platform");
  assertUniqueTargets(OFFICIAL_RELEASE_TARGETS, (item) => item.stableLabel, "stable label");
  assertUniqueTargets(getAllBuildTargets(), (item) => item.packageScript, "package script");
  assertUniqueTargets(buildDesktopCiMatrix(), (item) => item.os_family, "desktop os family");
  assertExpectedOsFamilies();

  // 2.2 再校验 package.json 的构建脚本
  checkPackageScripts();

  // 2.3 校验 workflow 契约和 action pin
  checkWorkflowContracts();
  checkPinnedGithubActions(WORKFLOW_PATHS.ci);
  checkPinnedGithubActions(WORKFLOW_PATHS.release);
  checkPinnedGithubActions(WORKFLOW_PATHS.releasePrSyncCargoLock);

  // 2.4 最后校验 README 中的支持矩阵文案
  checkReadmes();

  logger.info("[support-matrix] 支持矩阵校验通过。");
}

function prepareStableAssets(args) {
  /*
   * ============================================================================
   * 步骤3：按矩阵整理稳定发布产物
   * ============================================================================
   * 目标：
   *   1) 让 release workflow 不再内联维护一份资产挑选规则
   *   2) 让 stable asset 命名与 latest.json 平台条目共用同一矩阵
   * 数据源：
   *   1) tauri-action 返回的 artifactPaths
   *   2) OFFICIAL_RELEASE_TARGETS 中的 stable asset 定义
   * 操作要点：
   *   1) updater platform 必须能反查到唯一官方目标
   *   2) 资产复制失败直接中断 release，禁止静默降级
   */
  logger.info("[support-matrix] 开始整理稳定发布产物...");

  // 3.1 读取 CLI 参数并反查目标定义
  const rawArtifactPaths = requireArg(args, "artifact-paths");
  const updaterPlatform = requireArg(args, "updater-platform");
  const stableLabel = requireArg(args, "stable-label");
  const outputDir = requireArg(args, "output-dir");
  const target = findOfficialTargetByUpdaterPlatform(updaterPlatform);

  if (!target) {
    throw new Error(`Unsupported updater platform: ${updaterPlatform}`);
  }
  if (target.stableLabel !== stableLabel) {
    throw new Error(
      `Stable label drifted for ${updaterPlatform}. Expected: ${target.stableLabel}. Actual: ${stableLabel}.`
    );
  }

  let artifactPaths;
  try {
    artifactPaths = JSON.parse(rawArtifactPaths);
  } catch (error) {
    throw new Error(
      `Failed to parse --artifact-paths as JSON: ${error instanceof Error ? error.message : error}`
    );
  }

  if (!Array.isArray(artifactPaths) || artifactPaths.length === 0) {
    throw new Error("No artifacts found (artifactPaths is empty).");
  }

  // 3.2 创建输出目录，并按目标类型挑选主产物与签名
  mkdirSync(outputDir, { recursive: true });

  if (target.stableAssetKind === "msi") {
    const msi = pickArtifact(
      artifactPaths,
      (item) => item.toLowerCase().endsWith(".msi") && !item.toLowerCase().endsWith(".msi.sig"),
      "*.msi"
    );
    const msiSig = pickArtifact(
      artifactPaths,
      (item) => item.toLowerCase().endsWith(".msi.sig"),
      "*.msi.sig"
    );
    copyArtifact(msi, outputDir, target.latestAssetName);
    copyArtifact(msiSig, outputDir, target.latestSignatureName);
    logger.info("[support-matrix] 稳定发布产物整理完成：%s", updaterPlatform);
    return;
  }

  if (target.stableAssetKind === "appimage") {
    const appImage = pickArtifact(
      artifactPaths,
      (item) =>
        item.toLowerCase().endsWith(".appimage") && !item.toLowerCase().endsWith(".appimage.sig"),
      "*.AppImage"
    );
    const appImageSig = pickArtifact(
      artifactPaths,
      (item) => item.toLowerCase().endsWith(".appimage.sig"),
      "*.AppImage.sig"
    );
    copyArtifact(appImage, outputDir, target.latestAssetName);
    copyArtifact(appImageSig, outputDir, target.latestSignatureName);

    const deb = artifactPaths.find(
      (item) => typeof item === "string" && item.toLowerCase().endsWith(".deb")
    );
    if (deb) {
      copyArtifact(deb, outputDir, `aio-coding-hub-${target.stableLabel}.deb`);
    }

    logger.info("[support-matrix] 稳定发布产物整理完成：%s", updaterPlatform);
    return;
  }

  const tarball = pickArtifact(
    artifactPaths,
    (item) =>
      item.toLowerCase().endsWith(".app.tar.gz") ||
      (item.toLowerCase().endsWith(".tar.gz") && !item.toLowerCase().endsWith(".tar.gz.sig")),
    "*.app.tar.gz / *.tar.gz"
  );
  const tarballSignature = pickArtifact(
    artifactPaths,
    (item) =>
      item.toLowerCase().endsWith(".app.tar.gz.sig") || item.toLowerCase().endsWith(".tar.gz.sig"),
    "*.app.tar.gz.sig / *.tar.gz.sig"
  );
  copyArtifact(tarball, outputDir, target.latestAssetName);
  copyArtifact(tarballSignature, outputDir, target.latestSignatureName);

  logger.info("[support-matrix] 稳定发布产物整理完成：%s", updaterPlatform);
}

function writeLatestJsonFile(args) {
  /*
   * ============================================================================
   * 步骤4：按矩阵生成 latest.json
   * ============================================================================
   * 目标：
   *   1) 只为官方支持目标生成 updater 平台条目
   *   2) 复用同一份 stable asset 命名规则
   * 数据源：
   *   1) release tag / repo
   *   2) stable-assets 目录中的签名文件
   * 操作要点：
   *   1) 任一官方目标缺签名文件时直接失败
   *   2) latest.json 结构写入后再次解析，避免生成坏 JSON
   */
  logger.info("[support-matrix] 开始生成 latest.json...");

  // 4.1 读取 CLI 参数与 release 文案环境变量
  const tag = requireArg(args, "tag");
  const repo = requireArg(args, "repo");
  const pubDate = requireArg(args, "pub-date");
  const stableAssetsDir = requireArg(args, "stable-assets-dir");
  const outputPath = requireArg(args, "output");
  const releaseBody = process.env.RELEASE_BODY ?? "";
  const fallbackNotes = process.env.FALLBACK_NOTES ?? "";

  // 4.2 按支持矩阵组装 latest.json 内容
  const latestJson = buildLatestJson({
    tag,
    repo,
    pubDate,
    stableAssetsDir,
    releaseBody,
    fallbackNotes,
  });

  // 4.3 写盘并回读校验 JSON 结构
  writeFileSync(outputPath, `${JSON.stringify(latestJson, null, 2)}\n`, "utf8");
  JSON.parse(readFileSync(outputPath, "utf8"));

  logger.info("[support-matrix] latest.json 生成完成：%s", outputPath);
}

function printBuildMatrix() {
  process.stdout.write(JSON.stringify(buildWorkflowMatrix()));
}

function printDesktopCiMatrix() {
  process.stdout.write(JSON.stringify(buildDesktopCiMatrix()));
}

function printReadmeBlock(args) {
  const locale = requireArg(args, "locale");
  const section = requireArg(args, "section");
  if (!README_MARKERS[section]) {
    throw new Error(`Unsupported README section: ${section}`);
  }
  process.stdout.write(`${renderReadmeBlock(section, locale)}\n`);
}

function printUsageAndExit() {
  logger.error(
    "Usage: node scripts/support-matrix.mjs <build-matrix|ci-matrix|check|prepare-stable-assets|generate-latest-json|readme-block> [--key value]"
  );
  process.exit(1);
}

function main() {
  const [command, ...restArgs] = process.argv.slice(2);
  if (!command) {
    printUsageAndExit();
  }

  const args = parseArgs(restArgs);

  switch (command) {
    case "build-matrix":
      printBuildMatrix();
      return;
    case "ci-matrix":
      printDesktopCiMatrix();
      return;
    case "check":
      runSupportMatrixCheck();
      return;
    case "prepare-stable-assets":
      prepareStableAssets(args);
      return;
    case "generate-latest-json":
      writeLatestJsonFile(args);
      return;
    case "readme-block":
      printReadmeBlock(args);
      return;
    default:
      throw new Error(`Unsupported command: ${command}`);
  }
}

main();
