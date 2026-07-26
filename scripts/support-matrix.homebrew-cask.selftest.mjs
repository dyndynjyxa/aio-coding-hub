import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const command = ["scripts/support-matrix.mjs", "homebrew-cask"];

function runSupportMatrix(args) {
  return spawnSync("node", [...command, ...args], {
    cwd: process.cwd(),
    encoding: "utf8",
  });
}

function assertIncludes(value, expected) {
  if (!value.includes(expected)) {
    throw new Error(`Expected output to include:\n${expected}\n\nActual output:\n${value}`);
  }
}

function assertEqual(actual, expected, label) {
  if (actual !== expected) {
    throw new Error(`${label}\nExpected: ${expected}\nActual: ${actual}`);
  }
}

function testPrintsCaskForCurrentRelease() {
  const result = runSupportMatrix([
    "--tag",
    "aio-coding-hub-v0.60.4",
    "--repo",
    "dyndynjyxa/aio-coding-hub",
    "--macos-arm-sha256",
    "6b126f39ec625e97d182301fafcbfff81ce6f332e297880aef2b0eab0a3c0c4a",
    "--macos-intel-sha256",
    "18f376bc6266e8cef4fb3978240ba0247c56b703370f6a95269443c2adbbbcc6",
  ]);

  assertEqual(result.status, 0, "homebrew-cask command should succeed");
  assertIncludes(result.stdout, 'cask "aio-coding-hub" do');
  assertIncludes(result.stdout, 'version "0.60.4"');
  assertIncludes(result.stdout, 'arch arm: "arm", intel: "intel"');
  assertIncludes(
    result.stdout,
    'sha256 arm:   "6b126f39ec625e97d182301fafcbfff81ce6f332e297880aef2b0eab0a3c0c4a",'
  );
  assertIncludes(
    result.stdout,
    '       intel: "18f376bc6266e8cef4fb3978240ba0247c56b703370f6a95269443c2adbbbcc6"'
  );
  assertIncludes(
    result.stdout,
    'url "https://github.com/dyndynjyxa/aio-coding-hub/releases/download/aio-coding-hub-v#{version}/aio-coding-hub-macos-#{arch}.zip"'
  );
  assertIncludes(result.stdout, 'app "AIO Coding Hub.app"');
  assertIncludes(result.stdout, "auto_updates true");
  assertIncludes(result.stdout, "depends_on :macos");
}

function testWritesCaskToOutputPath() {
  const root = mkdtempSync(join(tmpdir(), "aio-homebrew-cask-"));
  const outputPath = join(root, "Casks/aio-coding-hub.rb");

  try {
    const result = runSupportMatrix([
      "--tag",
      "aio-coding-hub-v0.60.4",
      "--repo",
      "dyndynjyxa/aio-coding-hub",
      "--macos-arm-sha256",
      "6b126f39ec625e97d182301fafcbfff81ce6f332e297880aef2b0eab0a3c0c4a",
      "--macos-intel-sha256",
      "18f376bc6266e8cef4fb3978240ba0247c56b703370f6a95269443c2adbbbcc6",
      "--output",
      outputPath,
    ]);

    assertEqual(result.status, 0, "homebrew-cask command should write an output file");
    assertIncludes(readFileSync(outputPath, "utf8"), 'cask "aio-coding-hub" do');
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

function testRequiresMacosHashes() {
  const result = runSupportMatrix([
    "--tag",
    "aio-coding-hub-v0.60.4",
    "--repo",
    "dyndynjyxa/aio-coding-hub",
    "--macos-arm-sha256",
    "6b126f39ec625e97d182301fafcbfff81ce6f332e297880aef2b0eab0a3c0c4a",
  ]);

  assertEqual(result.status, 1, "homebrew-cask command should fail without both hashes");
  assertIncludes(result.stderr, "Missing required argument: --macos-intel-sha256");
}

for (const testCase of [
  testPrintsCaskForCurrentRelease,
  testWritesCaskToOutputPath,
  testRequiresMacosHashes,
]) {
  testCase();
}

console.log("[support-matrix] Homebrew Cask self-test passed.");
