import { createPublicKey, verify } from "node:crypto";
import { dirname } from "node:path";
import { existsSync, mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { createPluginScaffold } from "./scaffold";
import {
  generateSigningKeyPair,
  packPlugin,
  packPluginBytes,
  packPluginDirectory,
  replayHook,
  runCreateAioPluginCli,
  signPackage,
  validatePluginDirectory,
  validatePluginFiles,
  verifyPackage,
} from "./devtools";

describe("create-aio-plugin scaffold", () => {
  it("creates a declarative rule plugin template", () => {
    const files = createPluginScaffold({
      id: "acme.redactor",
      name: "Redactor",
      template: "rule",
    });

    expect(files["plugin.json"]).toContain('"id": "acme.redactor"');
    expect(files["plugin.json"]).toContain('"kind": "declarativeRules"');
    expect(files["rules/main.json"]).toContain('"kind": "replace"');
    expect(files["README.md"]).toContain("acme.redactor");
  });

  it("creates a declarative rule template with the host rule ABI", () => {
    const files = createPluginScaffold({
      id: "acme.redactor",
      name: "Redactor",
      template: "rule",
    });

    const document = JSON.parse(files["rules/main.json"] ?? "{}") as {
      rules?: Array<Record<string, unknown>>;
    };
    const rule = document.rules?.[0] ?? {};

    expect(rule.target).toEqual({
      field: "request.body",
      jsonPath: "$.messages[*].content",
    });
    expect(rule.match).toMatchObject({
      regex: "SECRET_[A-Za-z0-9_]+",
      caseSensitive: true,
    });
    expect(rule).not.toHaveProperty("matcher");
  });

  it("creates a WASM plugin template without enabling marketplace execution", () => {
    const files = createPluginScaffold({
      id: "acme.policy",
      name: "Policy",
      template: "wasm",
    });

    expect(files["plugin.json"]).toContain('"kind": "wasm"');
    expect(files["src/lib.rs"]).toContain("aio_plugin_handle");
    expect(files["README.md"]).toContain("gateway execution remains policy-gated");
    expect(files["README.md"]).toContain("PLUGIN_RUNTIME_DISABLED");
  });

  it("packs binary wasm artifacts without utf8 rewriting", () => {
    const wasmBytes = new Uint8Array([0x00, 0x61, 0x73, 0x6d, 0xff, 0x00, 0x80]);
    const packed = packPluginBytes({
      "plugin.json": new TextEncoder().encode(JSON.stringify(validWasmManifest())),
      "plugin.wasm": wasmBytes,
    });

    expect(packed.checksum).toMatch(/^sha256:/);
    expect(readStoredZipEntry(packed.bytes, "plugin.wasm")).toEqual(wasmBytes);
  });

  it("validates manifests, replays hook fixtures, and verifies package signatures", () => {
    const files = createPluginScaffold({
      id: "acme.redactor",
      name: "Redactor",
      template: "rule",
    });

    expect(validatePluginFiles(files).ok).toBe(true);
    expect(replayHook(files, "gateway.request.afterBodyRead", { body: "SECRET_TOKEN" })).toEqual({
      action: "pass",
    });

    const packed = packPlugin(files);
    const entries = unpackStoredZipEntries(packed.bytes);

    expect(entries.get("plugin.json")).toContain('"id": "acme.redactor"');
    expect(entries.get("rules/main.json")).toContain('"kind": "replace"');

    const keyPair = generateSigningKeyPair();
    const signed = signPackage(packed.bytes, keyPair.privateKey);

    expect(signed.publicKey).toBe(keyPair.publicKey);
    expect(Buffer.from(signed.publicKey, "base64")).toHaveLength(32);
    expect(
      verify(
        null,
        Buffer.from(packed.bytes),
        createPublicKey({
          key: Buffer.concat([
            Buffer.from("302a300506032b6570032100", "hex"),
            Buffer.from(keyPair.publicKey, "base64"),
          ]),
          format: "der",
          type: "spki",
        }),
        Buffer.from(signed.signature, "base64")
      )
    ).toBe(true);
    expect(verifyPackage(packed.bytes, signed.signature, keyPair.publicKey)).toMatchObject({
      ok: true,
      checksum: packed.checksum,
    });
    expect(
      verifyPackage(new TextEncoder().encode("tampered"), signed.signature, keyPair.publicKey)
    ).toMatchObject({
      ok: false,
    });
  });

  it("signs and verifies package bytes through the CLI helper", () => {
    const keyPair = generateSigningKeyPair();
    const signedOutput: string[] = [];
    const verifyOutput: string[] = [];

    expect(
      runCreateAioPluginCli(["sign", "package-bytes", keyPair.privateKey], process.cwd(), {
        log: (line) => signedOutput.push(line),
        error: () => undefined,
      })
    ).toBe(0);
    const signed = JSON.parse(signedOutput[0] ?? "{}") as {
      checksum: string;
      signature: string;
      publicKey: string;
    };

    expect(
      runCreateAioPluginCli(
        ["verify", "package-bytes", signed.signature, signed.publicKey],
        process.cwd(),
        {
          log: (line) => verifyOutput.push(line),
          error: () => undefined,
        }
      )
    ).toBe(0);

    expect(JSON.parse(verifyOutput[0] ?? "{}")).toMatchObject({
      ok: true,
      checksum: signed.checksum,
    });
  });

  it("packs a scaffold into an .aio-plugin file through the CLI helper", () => {
    const cwd = mkdtempSync(join(tmpdir(), "aio-plugin-pack-"));
    writeScaffold(
      join(cwd, "acme.redactor"),
      createPluginScaffold({
        id: "acme.redactor",
        name: "Redactor",
        template: "rule",
      })
    );
    const output: string[] = [];

    expect(
      runCreateAioPluginCli(["pack", "./acme.redactor"], cwd, {
        log: (line) => output.push(line),
        error: () => undefined,
      })
    ).toBe(0);

    const result = JSON.parse(output[0] ?? "{}") as {
      path: string;
      checksum: string;
      sizeBytes: number;
    };

    expect(result.path).toBe(join(cwd, "acme.redactor.aio-plugin"));
    expect(result.checksum).toMatch(/^sha256:[a-f0-9]{64}$/);
    expect(result.sizeBytes).toBeGreaterThan(0);
    expect(existsSync(result.path)).toBe(true);
  });

  it("validate command reads plugin.json from a real plugin directory", () => {
    const root = mkdtempSync(join(tmpdir(), "aio-plugin-"));
    writeScaffold(root, createPluginScaffold({ id: "acme.real", name: "Real", template: "rule" }));

    const result = validatePluginDirectory(root);

    expect(result).toEqual({ ok: true });
  });

  it("pack command writes package bytes from a real plugin directory", () => {
    const root = mkdtempSync(join(tmpdir(), "aio-plugin-"));
    writeScaffold(root, createPluginScaffold({ id: "acme.real", name: "Real", template: "rule" }));

    const packed = packPluginDirectory(root);

    expect(packed.checksum).toMatch(/^sha256:/);
    expect(packed.bytes.length).toBeGreaterThan(64);
  });

  it("replay command applies scaffold rule to fixture context", () => {
    const files = createPluginScaffold({ id: "acme.real", name: "Real", template: "rule" });

    const result = replayHook(files, "gateway.request.afterBodyRead", {
      request: {
        body: JSON.stringify({
          messages: [{ role: "user", content: "SECRET_TOKEN" }],
        }),
      },
    });

    expect(result).toMatchObject({ action: "replace" });
    expect(JSON.stringify(result)).toContain("[REDACTED]");
  });

  it("replay command emits the active vNext mutation envelope", () => {
    const files = createPluginScaffold({ id: "acme.real", name: "Real", template: "rule" });

    const result = replayHook(files, "gateway.request.afterBodyRead", {
      request: {
        body: JSON.stringify({
          messages: [{ role: "user", content: "SECRET_TOKEN" }],
        }),
      },
    });

    expect(result).toMatchObject({
      action: "replace",
      requestBody: expect.stringContaining("[REDACTED]"),
    });
    expect(result).not.toHaveProperty("contextPatch");
  });

  it("replay applies same-target JSONPath replacement like the host rule runtime", () => {
    const files = createPluginScaffold({ id: "acme.redactor", name: "Redactor", template: "rule" });

    const result = replayHook(files, "gateway.request.afterBodyRead", {
      request: {
        body: JSON.stringify({
          messages: [{ role: "user", content: "SECRET_TOKEN" }],
        }),
      },
    });

    expect(result).toEqual({
      action: "replace",
      requestBody: JSON.stringify({
        messages: [{ role: "user", content: "[REDACTED]" }],
      }),
    });
  });

  it("replay supports block, warn, and appendMessage actions", () => {
    const blockFiles = rulePluginFilesWithAction({ kind: "block", reason: "blocked" });

    expect(
      replayHook(blockFiles, "gateway.request.afterBodyRead", { request: { body: "danger" } })
    ).toEqual({ action: "block", reason: "blocked" });

    const warnFiles = rulePluginFilesWithAction({ kind: "warn", message: "careful" });

    expect(
      replayHook(warnFiles, "gateway.request.afterBodyRead", { request: { body: "danger" } })
    ).toEqual({ action: "warn", message: "careful" });

    const appendFiles = rulePluginFilesWithAction({
      kind: "appendMessage",
      role: "developer",
      content: "Use safe mode",
    });

    const result = replayHook(appendFiles, "gateway.request.afterBodyRead", {
      request: { body: JSON.stringify({ messages: [{ role: "user", content: "hello" }] }) },
    });

    expect(JSON.stringify(result)).toContain("Use safe mode");
    expect(result).not.toHaveProperty("contextPatch");
  });

  it("replay replaces Codex/OpenAI Responses input text like the host rule runtime", () => {
    const files = rulePluginFilesWithTarget("$.input[*].content[*].text");

    const result = replayHook(files, "gateway.request.afterBodyRead", {
      request: {
        body: JSON.stringify({
          input: [
            {
              type: "message",
              role: "user",
              content: [{ type: "input_text", text: "SECRET_TOKEN" }],
            },
          ],
        }),
      },
    });

    expect(result).toEqual({
      action: "replace",
      requestBody: JSON.stringify({
        input: [
          {
            type: "message",
            role: "user",
            content: [{ type: "input_text", text: "[REDACTED]" }],
          },
        ],
      }),
    });
  });

  it("replay maps response, stream, and log targets to their host mutation fields", () => {
    expect(
      replayHook(
        rulePluginFilesWithRule({
          hook: "gateway.response.after",
          target: { field: "response.body" },
        }),
        "gateway.response.after",
        { response: { body: "SECRET_TOKEN" } }
      )
    ).toEqual({ action: "replace", responseBody: "[REDACTED]" });

    expect(
      replayHook(
        rulePluginFilesWithRule({
          hook: "gateway.response.chunk",
          target: { field: "stream.chunk" },
        }),
        "gateway.response.chunk",
        { stream: { chunk: "data: SECRET_TOKEN\n\n" } }
      )
    ).toEqual({ action: "replace", streamChunk: "data: [REDACTED]\n\n" });

    expect(
      replayHook(
        rulePluginFilesWithRule({
          hook: "log.beforePersist",
          target: { field: "log.message" },
        }),
        "log.beforePersist",
        { log: { message: "apiKey=SECRET_TOKEN" } }
      )
    ).toEqual({ action: "replace", logMessage: "apiKey=[REDACTED]" });
  });
});

function writeScaffold(root: string, files: Record<string, string>): void {
  for (const [path, content] of Object.entries(files)) {
    const fullPath = join(root, path);
    mkdirSync(dirname(fullPath), { recursive: true });
    writeFileSync(fullPath, content);
  }
}

function validWasmManifest() {
  return {
    id: "acme.policy",
    name: "Policy",
    version: "0.1.0",
    apiVersion: "1.0.0",
    runtime: { kind: "wasm", abiVersion: "1.0.0", memoryLimitBytes: 16777216 },
    hooks: [{ name: "gateway.request.afterBodyRead", priority: 100 }],
    permissions: ["request.meta.read"],
    hostCompatibility: { app: ">=0.56.0 <1.0.0", pluginApi: "^1.0.0" },
    entry: "plugin.wasm",
  };
}

function rulePluginFilesWithAction(action: Record<string, unknown>): Record<string, string> {
  const files = rulePluginFilesWithTarget(undefined);
  const document = JSON.parse(files["rules/main.json"] ?? "{}") as {
    rules?: Array<Record<string, unknown>>;
  };
  const rule = document.rules?.[0];
  if (rule) {
    rule.target = { field: "request.body" };
    rule.match = { regex: "danger|hello", caseSensitive: true };
    rule.action = action;
  }
  return {
    ...files,
    "rules/main.json": `${JSON.stringify(document, null, 2)}\n`,
  };
}

function rulePluginFilesWithTarget(jsonPath: string | undefined): Record<string, string> {
  return rulePluginFilesWithRule({
    hook: "gateway.request.afterBodyRead",
    target: jsonPath ? { field: "request.body", jsonPath } : { field: "request.body" },
  });
}

function rulePluginFilesWithRule(options: {
  hook: string;
  target: Record<string, unknown>;
}): Record<string, string> {
  const files = createPluginScaffold({ id: "acme.redactor", name: "Redactor", template: "rule" });
  const manifest = JSON.parse(files["plugin.json"] ?? "{}") as {
    hooks?: Array<Record<string, unknown>>;
    permissions?: string[];
  };
  if (manifest.hooks?.[0]) {
    manifest.hooks[0].name = options.hook;
  }
  manifest.permissions = permissionsForReplayTarget(options.target.field);
  const document = JSON.parse(files["rules/main.json"] ?? "{}") as {
    rules?: Array<Record<string, unknown>>;
  };
  const rule = document.rules?.[0];
  if (rule) {
    rule.hook = options.hook;
    rule.target = options.target;
  }
  return {
    ...files,
    "plugin.json": `${JSON.stringify(manifest, null, 2)}\n`,
    "rules/main.json": `${JSON.stringify(document, null, 2)}\n`,
  };
}

function permissionsForReplayTarget(field: unknown): string[] {
  switch (field) {
    case "response.body":
      return ["response.body.read", "response.body.write"];
    case "stream.chunk":
      return ["stream.inspect", "stream.modify"];
    case "log.message":
      return ["log.redact"];
    case "request.body":
    default:
      return ["request.body.read", "request.body.write"];
  }
}

function unpackStoredZipEntries(bytes: Uint8Array): Map<string, string> {
  const entries = new Map<string, string>();
  let offset = 0;
  while (offset + 30 <= bytes.length) {
    const signature = readU32(bytes, offset);
    if (signature !== 0x04034b50) break;
    const compression = readU16(bytes, offset + 8);
    const compressedSize = readU32(bytes, offset + 18);
    const nameLength = readU16(bytes, offset + 26);
    const extraLength = readU16(bytes, offset + 28);
    expect(compression).toBe(0);
    const nameStart = offset + 30;
    const dataStart = nameStart + nameLength + extraLength;
    const name = new TextDecoder().decode(bytes.subarray(nameStart, nameStart + nameLength));
    const data = bytes.subarray(dataStart, dataStart + compressedSize);
    entries.set(name, new TextDecoder().decode(data));
    offset = dataStart + compressedSize;
  }
  return entries;
}

function readStoredZipEntry(bytes: Uint8Array, expectedName: string): Uint8Array | null {
  let offset = 0;
  while (offset + 30 <= bytes.length) {
    const signature = readU32(bytes, offset);
    if (signature !== 0x04034b50) break;
    const compression = readU16(bytes, offset + 8);
    const compressedSize = readU32(bytes, offset + 18);
    const nameLength = readU16(bytes, offset + 26);
    const extraLength = readU16(bytes, offset + 28);
    expect(compression).toBe(0);
    const nameStart = offset + 30;
    const dataStart = nameStart + nameLength + extraLength;
    const name = new TextDecoder().decode(bytes.subarray(nameStart, nameStart + nameLength));
    const data = bytes.subarray(dataStart, dataStart + compressedSize);
    if (name === expectedName) {
      return new Uint8Array(data);
    }
    offset = dataStart + compressedSize;
  }
  return null;
}

function readU16(bytes: Uint8Array, offset: number): number {
  return bytes[offset] | ((bytes[offset + 1] ?? 0) << 8);
}

function readU32(bytes: Uint8Array, offset: number): number {
  return (
    ((bytes[offset] ?? 0) |
      ((bytes[offset + 1] ?? 0) << 8) |
      ((bytes[offset + 2] ?? 0) << 16) |
      ((bytes[offset + 3] ?? 0) << 24)) >>>
    0
  );
}
