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

describe("services/desktop/assetUrl", () => {
  it("allows Tauri asset protocol origins in the image CSP", () => {
    const sources = cspDirectiveSources(tauriConfig.app.security.csp, "img-src");

    expect(sources).toContain("asset:");
    // Tauri maps custom protocols to http://<scheme>.localhost on Windows by default.
    expect(sources).toContain("http://asset.localhost");
  });
});
