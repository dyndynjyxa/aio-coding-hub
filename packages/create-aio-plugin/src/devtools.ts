import {
  createHash,
  createPrivateKey,
  createPublicKey,
  generateKeyPairSync,
  sign,
  verify,
} from "node:crypto";
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import type { GatewayHookName, PluginManifest, ValidationResult } from "@aio-coding-hub/plugin-sdk";
import { validateManifest } from "@aio-coding-hub/plugin-sdk";
import { createPluginScaffold, type ScaffoldFiles, type ScaffoldTemplate } from "./scaffold";

export type PackedPlugin = {
  bytes: Uint8Array;
  checksum: string;
};

export type PluginFileBytes = Record<string, Uint8Array>;

export type SigningKeyPair = {
  privateKey: string;
  publicKey: string;
};

export type CliIo = {
  log: (line: string) => void;
  error: (line: string) => void;
};

const USAGE = [
  "Usage:",
  "  create-aio-plugin <publisher.plugin-name> [rule|wasm]",
  "  create-aio-plugin validate <plugin-dir>",
  "  create-aio-plugin replay <plugin-dir> <fixture.json> <hook>",
  "  create-aio-plugin pack <plugin-dir>",
].join("\n");

export function runCreateAioPluginCli(args: string[], cwd: string, io: CliIo = console): number {
  const [commandOrId, firstArg, secondArg, thirdArg] = args;

  if (!commandOrId) {
    io.error(USAGE);
    return 1;
  }

  if (commandOrId === "validate") {
    try {
      io.log(JSON.stringify(validatePluginDirectory(resolve(cwd, firstArg ?? "."))));
      return 0;
    } catch (error) {
      io.error(`failed to validate plugin directory: ${errorMessage(error)}`);
      return 1;
    }
  }

  if (commandOrId === "replay") {
    if (!firstArg || !secondArg || !thirdArg) {
      io.error("Usage: create-aio-plugin replay <plugin-dir> <fixture.json> <hook>");
      return 1;
    }
    try {
      const files = readPluginDirectory(resolve(cwd, firstArg));
      const fixture = JSON.parse(readFileSync(resolve(cwd, secondArg), "utf8")) as unknown;
      io.log(JSON.stringify(replayHook(files, thirdArg as GatewayHookName, fixture)));
      return 0;
    } catch (error) {
      io.error(`failed to replay plugin hook: ${errorMessage(error)}`);
      return 1;
    }
  }

  if (commandOrId === "pack") {
    try {
      const root = resolve(cwd, firstArg ?? ".");
      const files = readPluginDirectoryBytes(root);
      const manifest = JSON.parse(
        textFromBytes(files["plugin.json"]) ?? "{}"
      ) as Partial<PluginManifest>;
      const packed = packPluginBytes(files);
      const outputPath = join(cwd, `${manifest.id ?? firstArg ?? "plugin"}.aio-plugin`);
      writeFileSync(outputPath, packed.bytes);
      io.log(
        JSON.stringify({
          path: outputPath,
          checksum: packed.checksum,
          sizeBytes: packed.bytes.length,
        })
      );
      return 0;
    } catch (error) {
      io.error(`failed to pack plugin directory: ${errorMessage(error)}`);
      return 1;
    }
  }

  if (commandOrId === "sign") {
    const bytes = new TextEncoder().encode(firstArg ?? "");
    const keyPair = secondArg
      ? { privateKey: secondArg, publicKey: createPublicKeyFromPrivateKey(secondArg) }
      : generateSigningKeyPair();
    io.log(JSON.stringify(signPackage(bytes, keyPair.privateKey, keyPair.publicKey)));
    return 0;
  }

  if (commandOrId === "verify") {
    if (!secondArg || !thirdArg) {
      io.error("Usage: create-aio-plugin verify <bytes> <signature> <publicKey>");
      return 1;
    }
    const bytes = new TextEncoder().encode(firstArg ?? "");
    io.log(JSON.stringify(verifyPackage(bytes, secondArg, thirdArg)));
    return 0;
  }

  const idArg = commandOrId;
  const template = (firstArg ?? "rule") as ScaffoldTemplate;
  const files = createPluginScaffold({
    id: idArg,
    name: titleFromId(idArg),
    template,
  });

  for (const [path, content] of Object.entries(files)) {
    const fullPath = join(cwd, idArg, path);
    mkdirSync(dirname(fullPath), { recursive: true });
    writeFileSync(fullPath, content);
  }
  return 0;
}

export function validatePluginFiles(files: ScaffoldFiles): ValidationResult {
  const manifestText = files["plugin.json"];
  if (!manifestText) {
    return {
      ok: false,
      error: { code: "PLUGIN_MISSING_MANIFEST", message: "missing plugin.json" },
    };
  }
  try {
    return validateManifest(JSON.parse(manifestText) as PluginManifest);
  } catch (error) {
    return {
      ok: false,
      error: {
        code: "PLUGIN_INVALID_MANIFEST",
        message: error instanceof Error ? error.message : "invalid manifest",
      },
    };
  }
}

export function readPluginDirectory(root: string): ScaffoldFiles {
  const files: ScaffoldFiles = {};
  walkPluginDirectory(root, root, files);
  return files;
}

export function readPluginDirectoryBytes(root: string): PluginFileBytes {
  const files: PluginFileBytes = {};
  walkPluginDirectoryBytes(root, root, files);
  return files;
}

export function validatePluginDirectory(root: string): ValidationResult {
  return validatePluginFiles(readPluginDirectory(root));
}

export function packPluginDirectory(root: string): PackedPlugin {
  return packPluginBytes(readPluginDirectoryBytes(root));
}

function walkPluginDirectory(root: string, dir: string, files: ScaffoldFiles): void {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === "node_modules" || entry.name === ".git") continue;
    const fullPath = join(dir, entry.name);
    const relativePath = relative(root, fullPath).replace(/\\/g, "/");
    if (entry.isDirectory()) {
      walkPluginDirectory(root, fullPath, files);
    } else if (entry.isFile()) {
      files[relativePath] = readFileSync(fullPath, "utf8");
    }
  }
}

function walkPluginDirectoryBytes(root: string, dir: string, files: PluginFileBytes): void {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name === "node_modules" || entry.name === ".git") continue;
    const fullPath = join(dir, entry.name);
    const relativePath = relative(root, fullPath).replace(/\\/g, "/");
    if (entry.isDirectory()) {
      walkPluginDirectoryBytes(root, fullPath, files);
    } else if (entry.isFile()) {
      files[relativePath] = new Uint8Array(readFileSync(fullPath));
    }
  }
}

export function replayHook(files: ScaffoldFiles, hook: GatewayHookName, context: unknown): unknown {
  const validation = validatePluginFiles(files);
  if (!validation.ok) {
    throw new Error(`${validation.error.code}: ${validation.error.message}`);
  }
  const manifest = JSON.parse(files["plugin.json"] ?? "{}") as PluginManifest;
  if (manifest.runtime.kind !== "declarativeRules") {
    return { action: "pass" };
  }
  for (const rulePath of manifest.runtime.rules) {
    const document = JSON.parse(files[rulePath] ?? '{"rules":[]}') as { rules?: unknown[] };
    for (const rule of document.rules ?? []) {
      const result = replayDeclarativeRule(rule, hook, context);
      if (result.action !== "pass") return result;
    }
  }
  return { action: "pass" };
}

export function packPlugin(files: ScaffoldFiles): PackedPlugin {
  const bytes = createStoredZipBytes(textFilesToBytes(files));
  return {
    bytes,
    checksum: sha256(bytes),
  };
}

export function packPluginBytes(files: PluginFileBytes): PackedPlugin {
  const bytes = createStoredZipBytes(
    Object.entries(files)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([path, content]) => [path, content] as const)
  );
  return {
    bytes,
    checksum: sha256(bytes),
  };
}

function textFilesToBytes(files: ScaffoldFiles): readonly (readonly [string, Uint8Array])[] {
  return Object.entries(files)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([path, content]) => [path, new TextEncoder().encode(content)] as const);
}

function textFromBytes(bytes: Uint8Array | undefined): string | undefined {
  return bytes == null ? undefined : new TextDecoder().decode(bytes);
}

export function generateSigningKeyPair(): SigningKeyPair {
  const { privateKey, publicKey } = generateKeyPairSync("ed25519");
  return {
    privateKey: privateKey.export({ format: "der", type: "pkcs8" }).toString("base64"),
    publicKey: rawPublicKeyFromSpki(
      publicKey.export({ format: "der", type: "spki" }) as Buffer
    ).toString("base64"),
  };
}

export function signPackage(
  bytes: Uint8Array,
  privateKey: string,
  publicKey?: string
): { checksum: string; signature: string; publicKey: string } {
  const checksum = sha256(bytes);
  const key = createPrivateKey({
    key: Buffer.from(privateKey, "base64"),
    format: "der",
    type: "pkcs8",
  });
  const signature = sign(null, Buffer.from(bytes), key).toString("base64");
  return {
    checksum,
    signature,
    publicKey:
      publicKey ??
      rawPublicKeyFromSpki(
        createPublicKey(key).export({ format: "der", type: "spki" }) as Buffer
      ).toString("base64"),
  };
}

export function verifyPackage(
  bytes: Uint8Array,
  signature: string,
  publicKey: string
): { ok: boolean; checksum: string } {
  const key = createPublicKey({
    key: spkiFromRawPublicKey(Buffer.from(publicKey, "base64")),
    format: "der",
    type: "spki",
  });
  return {
    ok: verify(null, Buffer.from(bytes), key, Buffer.from(signature, "base64")),
    checksum: sha256(bytes),
  };
}

function createPublicKeyFromPrivateKey(privateKey: string): string {
  return signPackage(new Uint8Array(), privateKey).publicKey;
}

function titleFromId(id: string): string {
  const segments = id.split(".");
  const slug = segments[segments.length - 1] ?? id;
  return slug
    .split("-")
    .map((part: string) => `${part.slice(0, 1).toUpperCase()}${part.slice(1)}`)
    .join(" ");
}

function replayDeclarativeRule(
  rawRule: unknown,
  hook: GatewayHookName,
  context: unknown
): ReplayRuleResult {
  const rule = asRecord(rawRule);
  if (rule?.hook !== hook) return { action: "pass" };
  const target = asRecord(rule.target);
  const matcher = asRecord(rule.matcher) ?? asRecord(rule.match);
  const action = asRecord(rule.action);
  if (!target || !matcher || !action) return { action: "pass" };
  const regex = compileReplayRegex(matcher);
  if (!regex) return { action: "pass" };

  const targetField = typeof target.field === "string" ? target.field : "request.body";
  const text = textFromFixture(context, targetField);
  if (!text) return { action: "pass" };

  const path =
    typeof target.jsonPath === "string"
      ? target.jsonPath
      : target.kind === "jsonPath" && typeof target.path === "string"
        ? target.path
        : undefined;
  if (
    !path ||
    (target.field && target.field !== "request.body" && target.field !== "response.body")
  ) {
    return replayTextAction(text, regex, action, targetField);
  }
  return replayJsonPathAction(text, path, regex, action, targetField);
}

type ReplayRuleResult =
  | { action: "pass" }
  | { action: "replace"; requestBody: string }
  | { action: "replace"; responseBody: string }
  | { action: "replace"; streamChunk: string }
  | { action: "replace"; logMessage: string }
  | { action: "block"; reason: string }
  | { action: "warn"; message: string };

type JsonPathSegment = { kind: "key"; key: string } | { kind: "wildcardArray" };

function compileReplayRegex(matcher: Record<string, unknown>): RegExp | null {
  if (typeof matcher.regex !== "string") return null;
  try {
    return new RegExp(matcher.regex, matcher.caseSensitive === false ? "gi" : "g");
  } catch {
    return null;
  }
}

function textFromFixture(context: unknown, field: string): string | undefined {
  if (typeof context === "string") return context;
  const contextRecord = asRecord(context);
  if (field === "request.body") {
    if (typeof contextRecord?.body === "string") return contextRecord.body;
    const request = asRecord(contextRecord?.request);
    return typeof request?.body === "string" ? request.body : undefined;
  }
  if (field === "response.body") {
    const response = asRecord(contextRecord?.response);
    return typeof response?.body === "string" ? response.body : undefined;
  }
  if (field === "stream.chunk") {
    const stream = asRecord(contextRecord?.stream);
    return typeof stream?.chunk === "string" ? stream.chunk : undefined;
  }
  if (field === "log.message") {
    const log = asRecord(contextRecord?.log);
    return typeof log?.message === "string" ? log.message : undefined;
  }
  return undefined;
}

function replayTextAction(
  body: string,
  regex: RegExp,
  action: Record<string, unknown>,
  field: string
): ReplayRuleResult {
  if (!regex.test(body)) return { action: "pass" };
  regex.lastIndex = 0;
  if (action.kind === "replace" && typeof action.replacement === "string") {
    return replaceResult(field, body.replace(regex, action.replacement));
  }
  if (action.kind === "block" && typeof action.reason === "string") {
    return { action: "block", reason: action.reason };
  }
  if (action.kind === "warn" && typeof action.message === "string") {
    return { action: "warn", message: action.message };
  }
  if (
    action.kind === "appendMessage" &&
    typeof action.role === "string" &&
    typeof action.content === "string"
  ) {
    return appendMessageToBody(body, action.role, action.content);
  }
  return { action: "pass" };
}

function replayJsonPathAction(
  body: string,
  path: string,
  regex: RegExp,
  action: Record<string, unknown>,
  field: string
): ReplayRuleResult {
  const segments = parseReplayJsonPath(path);
  if (!segments) return { action: "pass" };
  let root: unknown;
  try {
    root = JSON.parse(body) as unknown;
  } catch {
    return { action: "pass" };
  }

  let matched = false;
  let changed = false;
  applyToJsonStrings(root, segments, (candidate) => {
    if (!regex.test(candidate.value)) return;
    regex.lastIndex = 0;
    matched = true;
    if (action.kind === "replace" && typeof action.replacement === "string") {
      const next = candidate.value.replace(regex, action.replacement);
      if (next !== candidate.value) {
        candidate.set(next);
        changed = true;
      }
    }
  });

  if (!matched) return { action: "pass" };
  if (action.kind === "replace") {
    return changed ? replaceResult(field, JSON.stringify(root)) : { action: "pass" };
  }
  if (action.kind === "block" && typeof action.reason === "string") {
    return { action: "block", reason: action.reason };
  }
  if (action.kind === "warn" && typeof action.message === "string") {
    return { action: "warn", message: action.message };
  }
  if (
    action.kind === "appendMessage" &&
    typeof action.role === "string" &&
    typeof action.content === "string"
  ) {
    return appendMessageToBody(body, action.role, action.content);
  }
  return { action: "pass" };
}

function replaceResult(field: string, value: string): ReplayRuleResult {
  switch (field) {
    case "response.body":
      return { action: "replace", responseBody: value };
    case "stream.chunk":
      return { action: "replace", streamChunk: value };
    case "log.message":
      return { action: "replace", logMessage: value };
    case "request.body":
    default:
      return { action: "replace", requestBody: value };
  }
}

function appendMessageToBody(body: string, role: string, content: string): ReplayRuleResult {
  if (role !== "system" && role !== "developer") return { action: "pass" };
  if (!content.trim()) return { action: "pass" };
  let root: unknown;
  try {
    root = JSON.parse(body) as unknown;
  } catch {
    return { action: "pass" };
  }
  const record = asRecord(root);
  const messages = Array.isArray(record?.messages) ? record.messages : null;
  if (!messages) return { action: "pass" };
  messages.push({ role, content });
  return { action: "replace", requestBody: JSON.stringify(root) };
}

function parseReplayJsonPath(path: string): JsonPathSegment[] | null {
  if (!path.startsWith("$")) return null;
  const segments: JsonPathSegment[] = [];
  let index = 1;
  while (index < path.length) {
    if (path[index] === ".") {
      index += 1;
      const start = index;
      while (index < path.length && path[index] !== "." && path[index] !== "[") {
        index += 1;
      }
      if (start === index) return null;
      const key = path.slice(start, index);
      if (key.includes('"') || key.includes("'")) return null;
      segments.push({ kind: "key", key });
      continue;
    }
    if (path.slice(index, index + 3) === "[*]") {
      segments.push({ kind: "wildcardArray" });
      index += 3;
      continue;
    }
    return null;
  }
  return segments;
}

function applyToJsonStrings(
  value: unknown,
  path: JsonPathSegment[],
  visit: (candidate: { value: string; set: (next: string) => void }) => void
): void {
  if (path.length === 0) return;
  const [segment, ...rest] = path;
  if (!segment) return;

  if (segment.kind === "key") {
    const record = asRecord(value);
    if (!record || !(segment.key in record)) return;
    if (rest.length === 0) {
      const current = record[segment.key];
      if (typeof current === "string") {
        visit({
          value: current,
          set: (next) => {
            record[segment.key] = next;
          },
        });
      }
      return;
    }
    applyToJsonStrings(record[segment.key], rest, visit);
    return;
  }

  if (!Array.isArray(value)) return;
  for (const item of value) {
    applyToJsonStrings(item, rest, visit);
  }
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return value && typeof value === "object" && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : null;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function sha256(bytes: Uint8Array): string {
  return `sha256:${createHash("sha256").update(bytes).digest("hex")}`;
}

function createStoredZipBytes(entries: readonly (readonly [string, Uint8Array])[]): Uint8Array {
  const chunks: Uint8Array[] = [];
  const centralDirectory: Uint8Array[] = [];
  let offset = 0;

  for (const [path, data] of entries) {
    const name = new TextEncoder().encode(path.replace(/\\/g, "/"));
    const crc = crc32(data);
    const localHeader = concatBytes([
      u32(0x04034b50),
      u16(20),
      u16(0x0800),
      u16(0),
      u16(0),
      u16(0),
      u32(crc),
      u32(data.length),
      u32(data.length),
      u16(name.length),
      u16(0),
      name,
    ]);
    chunks.push(localHeader, data);

    centralDirectory.push(
      concatBytes([
        u32(0x02014b50),
        u16(20),
        u16(20),
        u16(0x0800),
        u16(0),
        u16(0),
        u16(0),
        u32(crc),
        u32(data.length),
        u32(data.length),
        u16(name.length),
        u16(0),
        u16(0),
        u16(0),
        u16(0),
        u32(0),
        u32(offset),
        name,
      ])
    );
    offset += localHeader.length + data.length;
  }

  const centralDirectoryOffset = offset;
  const centralDirectoryBytes = concatBytes(centralDirectory);
  const endOfCentralDirectory = concatBytes([
    u32(0x06054b50),
    u16(0),
    u16(0),
    u16(entries.length),
    u16(entries.length),
    u32(centralDirectoryBytes.length),
    u32(centralDirectoryOffset),
    u16(0),
  ]);

  return concatBytes([...chunks, centralDirectoryBytes, endOfCentralDirectory]);
}

function crc32(bytes: Uint8Array): number {
  let crc = 0xffffffff;
  for (const byte of bytes) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function concatBytes(chunks: readonly Uint8Array[]): Uint8Array {
  const total = chunks.reduce((sum, chunk) => sum + chunk.length, 0);
  const output = new Uint8Array(total);
  let offset = 0;
  for (const chunk of chunks) {
    output.set(chunk, offset);
    offset += chunk.length;
  }
  return output;
}

function u16(value: number): Uint8Array {
  return new Uint8Array([value & 0xff, (value >>> 8) & 0xff]);
}

function u32(value: number): Uint8Array {
  return new Uint8Array([
    value & 0xff,
    (value >>> 8) & 0xff,
    (value >>> 16) & 0xff,
    (value >>> 24) & 0xff,
  ]);
}

function rawPublicKeyFromSpki(spki: Buffer): Buffer {
  const prefix = Buffer.from("302a300506032b6570032100", "hex");
  if (spki.length !== prefix.length + 32 || !spki.subarray(0, prefix.length).equals(prefix)) {
    throw new Error("Unsupported Ed25519 SPKI public key format");
  }
  return spki.subarray(prefix.length);
}

function spkiFromRawPublicKey(raw: Buffer): Buffer {
  if (raw.length !== 32) {
    throw new Error("Ed25519 public key must be 32 bytes");
  }
  return Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), raw]);
}
