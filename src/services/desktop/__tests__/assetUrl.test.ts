import { describe, expect, it } from "vitest";
import tauriConfig from "../../../../src-tauri/tauri.conf.json";

function cspDirectiveSources(csp: string, name: string): string[] {
  const directive = csp
    .split(";")
    .map((entry) => entry.trim())
    .find((entry) => entry.startsWith(`${name} `));

  if (!directive) throw new Error(`CSP should define ${name}`);
  return directive.split(/\s+/).slice(1);
}

describe("Tauri asset URL CSP contract", () => {
  it("allows every asset origin emitted by convertFileSrc", () => {
    const sources = cspDirectiveSources(tauriConfig.app.security.csp, "img-src");
    const windowsAssetOrigins = tauriConfig.app.windows.map((window) => {
      const scheme =
        "useHttpsScheme" in window && window.useHttpsScheme === true ? "https" : "http";
      return `${scheme}://asset.localhost`;
    });

    expect(sources).toContain("asset:");
    expect(sources).toEqual(expect.arrayContaining(windowsAssetOrigins));
  });
});
