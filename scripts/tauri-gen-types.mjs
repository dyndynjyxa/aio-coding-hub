import { spawn } from "node:child_process";
import path, { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const projectRoot = resolve(__dirname, "..");
const tauriRoot = resolve(projectRoot, "src-tauri");
const defaultTargetDir = resolve(tauriRoot, "target-bindings");

function sanitizeWindowsPath(rawPath) {
  if (process.platform !== "win32" || typeof rawPath !== "string") {
    return rawPath;
  }

  return rawPath
    .split(path.delimiter)
    .filter((entry) => !/Windows Performance Toolkit/i.test(entry))
    .join(path.delimiter);
}

function run() {
  const userArgs = process.argv.slice(2);
  if (userArgs[0] === "--") {
    userArgs.shift();
  }

  const child = spawn(
    "cargo",
    ["run", "--locked", "--example", "export-bindings", ...userArgs],
    {
      cwd: tauriRoot,
      stdio: "inherit",
      shell: false,
      env: {
        ...process.env,
        PATH: sanitizeWindowsPath(process.env.PATH),
        CARGO_TARGET_DIR: process.env.CARGO_TARGET_DIR || defaultTargetDir,
        ...(process.platform === "win32" && !process.env.CARGO_BUILD_JOBS
          ? { CARGO_BUILD_JOBS: "1" }
          : {}),
        ...(process.platform === "win32" && !process.env.CARGO_INCREMENTAL
          ? { CARGO_INCREMENTAL: "0" }
          : {}),
      },
    }
  );

  child.on("exit", (code, signal) => {
    if (signal) {
      console.error(`[tauri:gen-types] exited with signal: ${signal}`);
      process.exit(1);
    }
    process.exit(code ?? 1);
  });

  child.on("error", (err) => {
    console.error(`[tauri:gen-types] failed to spawn cargo: ${err?.message ?? err}`);
    process.exit(1);
  });
}

run();
