import { useId, useState } from "react";
import { toast } from "sonner";
import { useMcpServerUpsertMutation } from "../../../query/mcp";
import { logToConsole } from "../../../services/consoleLog";
import {
  mcpParseJson,
  type McpImportServer,
  type McpSecretPatchInput,
  type McpServerSummary,
  type McpTransport,
} from "../../../services/workspace/mcp";
import { Button } from "../../../ui/Button";
import { Dialog } from "../../../ui/Dialog";
import { cn } from "../../../utils/cn";
import { cliKeysWith } from "../../../constants/clis";

export type McpServerDialogProps = {
  workspaceId: number;
  open: boolean;
  editTarget: McpServerSummary | null;
  onOpenChange: (open: boolean) => void;
};

type McpDialogDraft = {
  name: string;
  transport: McpTransport;
  command: string;
  args: string[];
  envPairs: KVPair[];
  cwd: string;
  url: string;
  headerPairs: KVPair[];
};

type KVPair = { id: string; key: string; value: string };

type McpDialogFormDraft = McpDialogDraft & {
  resetKey: string;
  jsonText: string;
};

const ENV_KEY_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;
const HEADER_KEY_RE = /^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/;
const FORM_CONTROL_CLASS =
  "rounded-lg border border-line bg-surface-inset px-3 text-foreground outline-none transition-colors focus:border-ring focus:bg-surface-panel focus:ring-2 focus:ring-ring/20 disabled:cursor-not-allowed disabled:bg-surface-muted disabled:opacity-60";
const TEXT_INPUT_CLASS = cn("h-10 w-full text-sm", FORM_CONTROL_CLASS);
const MONO_INPUT_CLASS = cn("h-10 w-full font-mono text-sm", FORM_CONTROL_CLASS);
const MONO_TEXTAREA_CLASS = cn("w-full resize-y py-2 font-mono text-xs", FORM_CONTROL_CLASS);
const SECTION_PANEL_CLASS = "rounded-2xl border border-line-subtle bg-surface-inset p-4";
const PRIMARY_PANEL_CLASS =
  "rounded-2xl border border-line bg-surface-panel p-4 shadow-[0_1px_2px_rgba(15,23,42,0.04)]";
const MCP_TRANSPORT_OPTIONS = [
  {
    value: "stdio",
    title: "STDIO",
    desc: "本地命令（通过 command/args 启动）",
    icon: "⌘",
  },
  {
    value: "http",
    title: "HTTP",
    desc: "远程服务（通过 URL 调用）",
    icon: "⇄",
  },
  {
    value: "sse",
    title: "SSE",
    desc: "Server-Sent Events（流式远程服务）",
    icon: "↯",
  },
] as const;

let kvPairIdSeq = 0;

function kvPair(key = "", value = ""): KVPair {
  kvPairIdSeq += 1;
  return { id: `kv-${kvPairIdSeq}`, key, value };
}

function recordToPairs(record: Record<string, string>): KVPair[] {
  const pairs = Object.entries(record).map(([key, value]) => kvPair(key, value));
  return pairs.length > 0 ? pairs : [kvPair()];
}

function keysToPreservedPairs(keys: string[]): KVPair[] {
  const pairs = keys.map((key) => kvPair(key));
  return pairs.length > 0 ? pairs : [kvPair()];
}

function emptyDialogDraft(resetKey: string): McpDialogFormDraft {
  return {
    resetKey,
    name: "",
    transport: "stdio",
    command: "",
    args: [],
    envPairs: [kvPair()],
    cwd: "",
    url: "",
    headerPairs: [kvPair()],
    jsonText: "",
  };
}

function buildDialogResetKey(open: boolean, editTarget: McpServerSummary | null) {
  if (!open) return "closed";
  return editTarget ? `edit:${editTarget.id}` : "create";
}

function buildDialogFormDraft(
  resetKey: string,
  editTarget: McpServerSummary | null
): McpDialogFormDraft {
  if (!editTarget) return emptyDialogDraft(resetKey);
  const draft = fromServerSummary(editTarget);
  return {
    resetKey,
    ...draft,
    jsonText: "",
  };
}

function buildSecretPatch(
  pairs: KVPair[],
  spec: {
    label: string;
    keyLabel: string;
    valueLabel: string;
    keyPattern: RegExp;
    existingKeys: ReadonlySet<string>;
  }
): { error: string | null; patch: McpSecretPatchInput } {
  const preserveKeys: string[] = [];
  const replace: Record<string, string> = {};
  const seenKeys = new Set<string>();

  for (const [i, pair] of pairs.entries()) {
    const key = pair.key.trim();
    const value = pair.value.trim();

    if (!key && !value) {
      continue;
    }
    if (!key) {
      return {
        error: `${spec.label} 第 ${i + 1} 行：请填写 ${spec.keyLabel}`,
        patch: { preserveKeys: [], replace: {} },
      };
    }
    if (!spec.keyPattern.test(key)) {
      return {
        error: `${spec.label} 第 ${i + 1} 行：${spec.keyLabel} 格式不正确`,
        patch: { preserveKeys: [], replace: {} },
      };
    }
    if (seenKeys.has(key)) {
      return {
        error: `${spec.label} 第 ${i + 1} 行：${spec.keyLabel} ${key} 重复`,
        patch: { preserveKeys: [], replace: {} },
      };
    }
    seenKeys.add(key);

    if (!value) {
      if (!spec.existingKeys.has(key)) {
        return {
          error: `${spec.label} 第 ${i + 1} 行：新增 ${spec.keyLabel} 必须填写 ${spec.valueLabel}`,
          patch: { preserveKeys: [], replace: {} },
        };
      }
      preserveKeys.push(key);
      continue;
    }

    replace[key] = pair.value;
  }

  return {
    error: null,
    patch: {
      preserveKeys,
      replace,
    },
  };
}

function parseLines(text: string) {
  return text.split("\n").flatMap((l) => {
    const line = l.trim();
    return line ? [line] : [];
  });
}

function asObject(value: unknown): Record<string, unknown> | null {
  if (!value || typeof value !== "object" || Array.isArray(value)) return null;
  return value as Record<string, unknown>;
}

function readString(value: unknown) {
  return typeof value === "string" ? value : "";
}

function readStringArray(value: unknown): string[] {
  if (!Array.isArray(value)) return [];
  return value.filter((item): item is string => typeof item === "string");
}

function readStringMap(value: unknown): Record<string, string> {
  const object = asObject(value);
  if (!object) return {};

  const out: Record<string, string> = {};
  for (const [key, val] of Object.entries(object)) {
    if (typeof val === "string") {
      out[key] = val;
    }
  }
  return out;
}

function inferTransport(spec: Record<string, unknown>): McpTransport {
  const transportValue =
    readString(spec.type) || readString(spec.transport) || readString(spec.transport_type);
  const transport = transportValue.trim().toLowerCase();
  if (transport === "sse") return "sse";
  if (transport === "http") return "http";
  if (transport === "stdio") return "stdio";

  if (
    readString(spec.url).trim() ||
    readString(spec.httpUrl).trim() ||
    asObject(spec.headers) ||
    asObject(spec.http_headers) ||
    asObject(spec.httpHeaders)
  ) {
    return "http";
  }

  return "stdio";
}

function selectCandidate(
  root: unknown
): { nameHint: string; entry: Record<string, unknown> } | null {
  const rootObj = asObject(root);
  if (rootObj) {
    const mcpServers = asObject(rootObj.mcpServers);
    if (mcpServers) {
      const first = Object.entries(mcpServers)[0];
      if (first) {
        const [nameHint, entry] = first;
        const entryObj = asObject(entry);
        if (entryObj) return { nameHint, entry: entryObj };
      }
    }

    for (const cliKey of cliKeysWith("mcp")) {
      const cliSection = asObject(rootObj[cliKey]);
      const cliServers = asObject(cliSection?.servers);
      if (!cliServers) continue;

      const first = Object.entries(cliServers)[0];
      if (!first) continue;

      const [nameHint, entry] = first;
      const entryObj = asObject(entry);
      if (entryObj) return { nameHint, entry: entryObj };
    }
  }

  if (Array.isArray(root)) {
    const first = root.map((item) => asObject(item)).find(Boolean);
    if (first) {
      return { nameHint: readString(first.name), entry: first };
    }
  }

  if (rootObj) {
    return { nameHint: readString(rootObj.name), entry: rootObj };
  }

  return null;
}

function parseJsonDraftFallback(jsonText: string): McpDialogDraft {
  const root = JSON.parse(jsonText) as unknown;
  const candidate = selectCandidate(root);
  if (!candidate) {
    throw new Error("JSON 结构不支持：请提供 mcpServers、code-switch 格式或单条 server 配置");
  }

  const spec =
    asObject(candidate.entry.server) ?? asObject(candidate.entry.spec) ?? candidate.entry;
  const transport = inferTransport(spec);
  const command = readString(spec.command).trim();
  const url = (readString(spec.url) || readString(spec.httpUrl)).trim();

  if (transport === "stdio" && !command) {
    throw new Error("JSON 缺少 stdio command 字段");
  }

  if (transport === "http" && !url) {
    throw new Error("JSON 缺少 http url 字段");
  }

  const name =
    candidate.nameHint.trim() ||
    readString(candidate.entry.name).trim() ||
    readString(spec.name).trim();

  return {
    name,
    transport,
    command,
    args: readStringArray(spec.args),
    envPairs: recordToPairs(readStringMap(spec.env)),
    cwd: readString(spec.cwd).trim(),
    url,
    headerPairs: recordToPairs(
      readStringMap(spec.headers ?? spec.http_headers ?? spec.httpHeaders)
    ),
  };
}

function fromServerSummary(
  server: Pick<
    McpServerSummary,
    "name" | "transport" | "command" | "args" | "env_keys" | "cwd" | "url" | "header_keys"
  >
): McpDialogDraft {
  return {
    name: server.name,
    transport: server.transport,
    command: server.command ?? "",
    args: server.args ?? [],
    envPairs: keysToPreservedPairs(server.env_keys ?? []),
    cwd: server.cwd ?? "",
    url: server.url ?? "",
    headerPairs: keysToPreservedPairs(server.header_keys ?? []),
  };
}

function fromImportServer(
  server: Pick<
    McpImportServer,
    "name" | "transport" | "command" | "args" | "env" | "cwd" | "url" | "headers"
  >
): McpDialogDraft {
  return {
    name: server.name,
    transport: server.transport,
    command: server.command ?? "",
    args: server.args ?? [],
    envPairs: recordToPairs((server.env ?? {}) as Record<string, string>),
    cwd: server.cwd ?? "",
    url: server.url ?? "",
    headerPairs: recordToPairs((server.headers ?? {}) as Record<string, string>),
  };
}

function KeyValuePairEditor({
  pairs,
  onChange,
  keyPlaceholder = "KEY",
  valuePlaceholder = "VALUE",
}: {
  pairs: KVPair[];
  onChange: (pairs: KVPair[]) => void;
  keyPlaceholder?: string;
  valuePlaceholder?: string;
}) {
  const updatePair = (index: number, field: "key" | "value", val: string) => {
    const next = pairs.map((p, i) => (i === index ? { ...p, [field]: val } : p));
    onChange(next);
  };

  const removePair = (index: number) => {
    const next = pairs.filter((_, i) => i !== index);
    onChange(next.length > 0 ? next : [kvPair()]);
  };

  const addPair = () => {
    onChange([...pairs, kvPair()]);
  };

  return (
    <div className="space-y-2">
      {pairs.map((pair, index) => (
        <div key={pair.id} className="flex items-center gap-2">
          <input
            type="text"
            value={pair.key}
            onChange={(e) => updatePair(index, "key", e.currentTarget.value)}
            placeholder={keyPlaceholder}
            aria-label={`${keyPlaceholder} ${index + 1}`}
            className={cn("w-[40%] shrink-0 py-1.5 font-mono text-xs", FORM_CONTROL_CLASS)}
          />
          <span className="text-xs text-muted-foreground select-none">=</span>
          <input
            type="text"
            value={pair.value}
            onChange={(e) => updatePair(index, "value", e.currentTarget.value)}
            placeholder={valuePlaceholder}
            aria-label={`${valuePlaceholder} ${index + 1}`}
            className={cn("min-w-0 flex-1 py-1.5 font-mono text-xs", FORM_CONTROL_CLASS)}
          />
          <button
            type="button"
            onClick={() => removePair(index)}
            className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg text-muted-foreground hover:bg-red-50 hover:text-red-500 dark:hover:bg-red-900/20 dark:hover:text-red-400 transition-colors"
            title="删除"
            aria-label={`删除键值行 ${index + 1}`}
          >
            ×
          </button>
        </div>
      ))}
      <button
        type="button"
        onClick={addPair}
        className="text-xs text-accent hover:text-accent/80 font-medium transition-colors"
      >
        + 添加一行
      </button>
    </div>
  );
}

type UpdateMcpDialogDraft = (patch: Partial<Omit<McpDialogFormDraft, "resetKey">>) => void;

function McpJsonImportPanel({
  jsonImportId,
  jsonText,
  saving,
  updateFormDraft,
  fillFromJson,
}: {
  jsonImportId: string;
  jsonText: string;
  saving: boolean;
  updateFormDraft: UpdateMcpDialogDraft;
  fillFromJson: () => Promise<void>;
}) {
  return (
    <div className={SECTION_PANEL_CLASS}>
      <label htmlFor={jsonImportId} className="text-xs font-medium text-muted-foreground">
        快速导入 JSON（可选）
      </label>
      <textarea
        id={jsonImportId}
        value={jsonText}
        onChange={(e) => updateFormDraft({ jsonText: e.currentTarget.value })}
        placeholder='示例：{"type":"stdio","command":"uvx","args":["mcp-server-fetch"]}'
        rows={4}
        className={cn("mt-2", MONO_TEXTAREA_CLASS)}
      />
      <div className="mt-2 flex justify-end">
        <Button variant="secondary" onClick={() => void fillFromJson()} disabled={saving}>
          从 JSON 填充
        </Button>
      </div>
    </div>
  );
}

function McpTransportOption({
  value,
  title,
  desc,
  icon,
  transport,
  updateFormDraft,
}: {
  value: McpTransport;
  title: string;
  desc: string;
  icon: string;
  transport: McpTransport;
  updateFormDraft: UpdateMcpDialogDraft;
}) {
  return (
    <label className="relative block">
      <input
        type="radio"
        name="mcp-transport"
        value={value}
        checked={transport === value}
        onChange={() => updateFormDraft({ transport: value })}
        className="peer sr-only"
      />
      <div
        className={cn(
          "flex h-full cursor-pointer items-start gap-3 rounded-xl border px-3 py-3 transition-colors",
          "border-line-subtle bg-surface-panel hover:border-line hover:bg-state-hover",
          "peer-focus-visible:ring-2 peer-focus-visible:ring-ring/35 peer-focus-visible:ring-offset-2 peer-focus-visible:ring-offset-background",
          "peer-checked:border-state-selected-border peer-checked:bg-state-selected"
        )}
      >
        <div
          className={cn(
            "mt-0.5 flex h-9 w-9 items-center justify-center rounded-lg border",
            "border-line-subtle bg-surface-inset text-secondary-foreground",
            "peer-checked:border-state-selected-border peer-checked:bg-surface-panel peer-checked:text-state-selected-foreground"
          )}
        >
          <span className="text-sm font-semibold">{icon}</span>
        </div>

        <div className="min-w-0 pr-7">
          <div className="text-sm font-semibold text-foreground">{title}</div>
          <div className="mt-0.5 text-xs leading-relaxed text-muted-foreground">{desc}</div>
        </div>

        <div className="pointer-events-none absolute right-3 top-3 flex h-5 w-5 items-center justify-center rounded-full border border-line bg-surface-inset text-[11px] text-transparent transition peer-checked:border-state-selected-border peer-checked:bg-state-selected-foreground peer-checked:text-white">
          ✓
        </div>
      </div>
    </label>
  );
}

function McpBasicInfoPanel({
  nameInputId,
  name,
  transport,
  updateFormDraft,
}: {
  nameInputId: string;
  name: string;
  transport: McpTransport;
  updateFormDraft: UpdateMcpDialogDraft;
}) {
  return (
    <div className={PRIMARY_PANEL_CLASS}>
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="text-xs font-medium text-muted-foreground">基础信息</div>
      </div>

      <div className="mt-3">
        <label htmlFor={nameInputId} className="text-sm font-medium text-secondary-foreground">
          名称
        </label>
        <input
          id={nameInputId}
          type="text"
          value={name}
          onChange={(e) => updateFormDraft({ name: e.currentTarget.value })}
          placeholder="例如：Fetch 工具"
          className={cn("mt-2", TEXT_INPUT_CLASS)}
        />
      </div>

      <div className="mt-4">
        <div className="flex items-center justify-between gap-3">
          <div className="text-sm font-medium text-secondary-foreground">类型</div>
          <div className="text-xs text-muted-foreground">二选一</div>
        </div>
        <div className="mt-2 grid gap-2 sm:grid-cols-3">
          {MCP_TRANSPORT_OPTIONS.map((item) => (
            <McpTransportOption
              key={item.value}
              value={item.value}
              title={item.title}
              desc={item.desc}
              icon={item.icon}
              transport={transport}
              updateFormDraft={updateFormDraft}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function McpStdioFields({
  commandInputId,
  argsTextareaId,
  cwdInputId,
  command,
  argsText,
  envPairs,
  cwd,
  editTarget,
  updateFormDraft,
}: {
  commandInputId: string;
  argsTextareaId: string;
  cwdInputId: string;
  command: string;
  argsText: string;
  envPairs: KVPair[];
  cwd: string;
  editTarget: McpServerSummary | null;
  updateFormDraft: UpdateMcpDialogDraft;
}) {
  return (
    <>
      <div>
        <label htmlFor={commandInputId} className="text-sm font-medium text-secondary-foreground">
          Command
        </label>
        <input
          id={commandInputId}
          type="text"
          value={command}
          onChange={(e) => updateFormDraft({ command: e.currentTarget.value })}
          placeholder="例如：npx"
          className={cn("mt-2", MONO_INPUT_CLASS)}
        />
      </div>

      <div>
        <label htmlFor={argsTextareaId} className="text-sm font-medium text-secondary-foreground">
          Args（每行一个）
        </label>
        <textarea
          id={argsTextareaId}
          value={argsText}
          onChange={(e) => updateFormDraft({ args: parseLines(e.currentTarget.value) })}
          placeholder={`例如：\n-y\n@modelcontextprotocol/server-fetch`}
          rows={4}
          className={cn("mt-2", MONO_TEXTAREA_CLASS)}
        />
      </div>

      <div>
        <div className="text-sm font-medium text-secondary-foreground">Env（环境变量）</div>
        {editTarget ? (
          <div className="mt-1 text-xs text-muted-foreground">
            旧值默认不显示。留空保留，删行删除，填新值替换。
          </div>
        ) : null}
        <div className="mt-2">
          <KeyValuePairEditor
            pairs={envPairs}
            onChange={(next) => updateFormDraft({ envPairs: next })}
            keyPlaceholder="KEY（例如 TOKEN）"
            valuePlaceholder="VALUE（例如 sk-xxx）"
          />
        </div>
      </div>

      <div>
        <label htmlFor={cwdInputId} className="text-sm font-medium text-secondary-foreground">
          CWD（可选）
        </label>
        <input
          id={cwdInputId}
          type="text"
          value={cwd}
          onChange={(e) => updateFormDraft({ cwd: e.currentTarget.value })}
          placeholder="例如：/Users/xxx/project"
          className={cn("mt-2", MONO_INPUT_CLASS)}
        />
      </div>
    </>
  );
}

function McpRemoteFields({
  urlInputId,
  url,
  headerPairs,
  editTarget,
  updateFormDraft,
}: {
  urlInputId: string;
  url: string;
  headerPairs: KVPair[];
  editTarget: McpServerSummary | null;
  updateFormDraft: UpdateMcpDialogDraft;
}) {
  return (
    <>
      <div>
        <label htmlFor={urlInputId} className="text-sm font-medium text-secondary-foreground">
          URL
        </label>
        <input
          id={urlInputId}
          type="text"
          value={url}
          onChange={(e) => updateFormDraft({ url: e.currentTarget.value })}
          placeholder="例如：https://example.com/mcp"
          className={cn("mt-2", MONO_INPUT_CLASS)}
        />
      </div>

      <div>
        <div className="text-sm font-medium text-secondary-foreground">Headers</div>
        {editTarget ? (
          <div className="mt-1 text-xs text-muted-foreground">
            旧值默认不显示。留空保留，删行删除，填新值替换。
          </div>
        ) : null}
        <div className="mt-2">
          <KeyValuePairEditor
            pairs={headerPairs}
            onChange={(next) => updateFormDraft({ headerPairs: next })}
            keyPlaceholder="Header（例如 Authorization）"
            valuePlaceholder="Value（例如 Bearer xxx）"
          />
        </div>
      </div>
    </>
  );
}

function McpDialogActions({
  saving,
  transport,
  command,
  url,
  save,
  onCancel,
}: {
  saving: boolean;
  transport: McpTransport;
  command: string;
  url: string;
  save: () => Promise<void>;
  onCancel: () => void;
}) {
  return (
    <div className="flex flex-wrap items-center gap-2">
      <Button
        onClick={() => void save()}
        variant="primary"
        disabled={saving || (transport === "stdio" ? !command.trim() : !url.trim())}
      >
        {saving ? "保存中…" : "保存并同步"}
      </Button>
      <Button onClick={onCancel} variant="secondary" disabled={saving}>
        取消
      </Button>
    </div>
  );
}

export function McpServerDialog({
  workspaceId,
  open,
  editTarget,
  onOpenChange,
}: McpServerDialogProps) {
  const upsertMutation = useMcpServerUpsertMutation(workspaceId);
  const saving = upsertMutation.isPending;
  const fieldIdPrefix = useId();
  const jsonImportId = `${fieldIdPrefix}-json-import`;
  const nameInputId = `${fieldIdPrefix}-name`;
  const commandInputId = `${fieldIdPrefix}-command`;
  const argsTextareaId = `${fieldIdPrefix}-args`;
  const cwdInputId = `${fieldIdPrefix}-cwd`;
  const urlInputId = `${fieldIdPrefix}-url`;

  const resetKey = buildDialogResetKey(open, editTarget);
  const [formDraft, setFormDraft] = useState<McpDialogFormDraft>(() =>
    buildDialogFormDraft(resetKey, editTarget)
  );
  let effectiveFormDraft = formDraft;

  if (formDraft.resetKey !== resetKey) {
    effectiveFormDraft = buildDialogFormDraft(resetKey, editTarget);
    setFormDraft(effectiveFormDraft);
  }

  const {
    name,
    transport,
    command,
    args: draftArgs,
    envPairs,
    cwd,
    url,
    headerPairs,
    jsonText,
  } = effectiveFormDraft;
  const argsText = draftArgs.join("\n");

  function updateFormDraft(patch: Partial<Omit<McpDialogFormDraft, "resetKey">>) {
    setFormDraft((current) => ({ ...current, ...patch }));
  }

  const transportHint =
    transport === "sse"
      ? "SSE（Server-Sent Events）"
      : transport === "http"
        ? "HTTP（远程服务）"
        : "STDIO（本地命令）";

  function applyDraft(draft: McpDialogDraft) {
    setFormDraft((current) => ({
      ...current,
      name: draft.name.trim()
        ? draft.name.trim()
        : current.name.trim()
          ? current.name
          : "MCP Server",
      transport: draft.transport,
      command: draft.command,
      args: draft.args,
      envPairs: draft.envPairs,
      cwd: draft.cwd,
      url: draft.url,
      headerPairs: draft.headerPairs,
    }));
  }

  async function fillFromJson() {
    const trimmed = jsonText.trim();
    if (!trimmed) {
      toast("请先粘贴 JSON");
      return;
    }

    try {
      const parsed = await mcpParseJson(trimmed);
      if (parsed.servers.length) {
        const server = parsed.servers[0];
        applyDraft(
          fromImportServer({
            name: server.name,
            transport: server.transport,
            command: server.command,
            args: server.args,
            env: server.env,
            cwd: server.cwd,
            url: server.url,
            headers: server.headers,
          })
        );
        toast("已从 JSON 填充字段");
        return;
      }

      const fallback = parseJsonDraftFallback(trimmed);
      applyDraft(fallback);
      toast("已从 JSON 填充字段");
    } catch (primaryErr) {
      try {
        const fallback = parseJsonDraftFallback(trimmed);
        applyDraft(fallback);
        toast("已从 JSON 填充字段");
      } catch (fallbackErr) {
        const message = String(fallbackErr || primaryErr);
        logToConsole("error", "从 JSON 填充 MCP 字段失败", { error: message });
        toast(`JSON 解析失败：${message}`);
      }
    }
  }

  async function save() {
    if (saving) return;

    const existingEnvKeys = new Set(editTarget?.env_keys ?? []);
    const existingHeaderKeys = new Set(editTarget?.header_keys ?? []);
    const { error: pairError, patch } =
      transport === "stdio"
        ? buildSecretPatch(envPairs, {
            label: "Env",
            keyLabel: "KEY",
            valueLabel: "VALUE",
            keyPattern: ENV_KEY_RE,
            existingKeys: existingEnvKeys,
          })
        : buildSecretPatch(headerPairs, {
            label: "Headers",
            keyLabel: "Header",
            valueLabel: "Value",
            keyPattern: HEADER_KEY_RE,
            existingKeys: existingHeaderKeys,
          });
    if (pairError) {
      toast(pairError);
      return;
    }

    try {
      const isStdio = transport === "stdio";
      const next = await upsertMutation.mutateAsync({
        serverId: editTarget?.id ?? null,
        serverKey: editTarget?.server_key ?? "",
        name,
        transport,
        command: isStdio ? command : null,
        args: isStdio ? draftArgs : [],
        env: isStdio ? patch : { preserveKeys: [], replace: {} },
        cwd: isStdio ? (cwd.trim() ? cwd : null) : null,
        url: !isStdio ? url : null,
        headers: !isStdio ? patch : { preserveKeys: [], replace: {} },
      });

      logToConsole("info", editTarget ? "更新 MCP Server" : "新增 MCP Server", {
        id: next.id,
        server_key: next.server_key,
        transport: next.transport,
      });

      toast(editTarget ? "已更新" : "已新增");
      onOpenChange(false);
    } catch (err) {
      logToConsole("error", "保存 MCP Server 失败", { error: String(err) });
      toast(`保存失败：${String(err)}`);
    }
  }

  return (
    <Dialog
      open={open}
      title={editTarget ? "编辑 MCP 服务" : "添加 MCP 服务"}
      description={
        editTarget
          ? "敏感值不会回显。留空保留旧值，删行删除，填新值替换。"
          : `类型：${transportHint}`
      }
      onOpenChange={onOpenChange}
      className="max-w-5xl"
    >
      <div className="grid gap-4">
        {!editTarget ? (
          <McpJsonImportPanel
            jsonImportId={jsonImportId}
            jsonText={jsonText}
            saving={saving}
            updateFormDraft={updateFormDraft}
            fillFromJson={fillFromJson}
          />
        ) : null}

        <McpBasicInfoPanel
          nameInputId={nameInputId}
          name={name}
          transport={transport}
          updateFormDraft={updateFormDraft}
        />

        {transport === "stdio" ? (
          <McpStdioFields
            commandInputId={commandInputId}
            argsTextareaId={argsTextareaId}
            cwdInputId={cwdInputId}
            command={command}
            argsText={argsText}
            envPairs={envPairs}
            cwd={cwd}
            editTarget={editTarget}
            updateFormDraft={updateFormDraft}
          />
        ) : (
          <McpRemoteFields
            urlInputId={urlInputId}
            url={url}
            headerPairs={headerPairs}
            editTarget={editTarget}
            updateFormDraft={updateFormDraft}
          />
        )}

        <McpDialogActions
          saving={saving}
          transport={transport}
          command={command}
          url={url}
          save={save}
          onCancel={() => onOpenChange(false)}
        />
      </div>
    </Dialog>
  );
}
