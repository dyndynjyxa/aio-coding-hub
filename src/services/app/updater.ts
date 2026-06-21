import {
  desktopUpdaterCheck,
  desktopUpdaterDownloadAndInstall,
  parseDesktopUpdaterCheck,
  type DesktopUpdaterDownloadEvent,
} from "../desktop/updater";
import { AIO_REPO_URL } from "../../constants/urls";

export type UpdaterCheckUpdate = {
  rid: number;
  version?: string;
  currentVersion?: string;
  date?: string;
  body?: string;
};

export type UpdaterCheckResult = UpdaterCheckUpdate | null;

const GITHUB_RELEASE_FALLBACK_RE = /^See release:\s+(\S+)$/;
const GITHUB_RELEASE_BODY_TIMEOUT_MS = 5_000;
const AIO_REPO_PATH = new URL(AIO_REPO_URL).pathname.split("/").filter(Boolean);

export function parseUpdaterCheckResult(value: unknown): UpdaterCheckResult {
  return parseDesktopUpdaterCheck(value);
}

function parseGitHubReleaseFallbackBody(body?: string) {
  const match = typeof body === "string" ? GITHUB_RELEASE_FALLBACK_RE.exec(body.trim()) : null;
  if (!match) return null;

  let url: URL;
  try {
    url = new URL(match[1]);
  } catch {
    return null;
  }

  const [owner, repo, releases, tagMarker, encodedTag, ...rest] = url.pathname
    .split("/")
    .filter(Boolean);
  if (
    url.protocol !== "https:" ||
    url.hostname !== "github.com" ||
    owner !== AIO_REPO_PATH[0] ||
    repo !== AIO_REPO_PATH[1] ||
    releases !== "releases" ||
    tagMarker !== "tag" ||
    !encodedTag ||
    rest.length > 0
  ) {
    return null;
  }

  return {
    owner,
    repo,
    tag: decodeURIComponent(encodedTag),
  };
}

function createReleaseBodyAbortSignal(): AbortSignal | undefined {
  if (typeof AbortSignal === "undefined" || typeof AbortSignal.timeout !== "function") {
    return undefined;
  }
  return AbortSignal.timeout(GITHUB_RELEASE_BODY_TIMEOUT_MS);
}

async function fetchGitHubReleaseBody(release: {
  owner: string;
  repo: string;
  tag: string;
}): Promise<string | null> {
  const response = await fetch(
    `https://api.github.com/repos/${release.owner}/${release.repo}/releases/tags/${encodeURIComponent(release.tag)}`,
    {
      headers: { accept: "application/vnd.github+json" },
      signal: createReleaseBodyAbortSignal(),
    }
  );
  if (!response.ok) return null;

  const value = (await response.json()) as { body?: unknown };
  if (typeof value.body !== "string" || value.body.trim().length === 0) {
    return null;
  }

  return value.body;
}

async function resolveGitHubReleaseFallbackBody(
  update: UpdaterCheckResult
): Promise<UpdaterCheckResult> {
  const release = parseGitHubReleaseFallbackBody(update?.body);
  if (!update || !release) return update;

  try {
    const body = await fetchGitHubReleaseBody(release);
    return body ? { ...update, body } : update;
  } catch {
    return update;
  }
}

export async function updaterCheck(): Promise<UpdaterCheckResult> {
  return resolveGitHubReleaseFallbackBody(await desktopUpdaterCheck());
}

export async function updaterDownloadAndInstall(options: {
  rid: number;
  onEvent?: (event: DesktopUpdaterDownloadEvent) => void;
  timeoutMs?: number;
}): Promise<boolean | null> {
  return desktopUpdaterDownloadAndInstall(options);
}

export type UpdaterDownloadEvent = DesktopUpdaterDownloadEvent;
