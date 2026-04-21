import { useEffect, useState } from "react";
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

type KVPair = { key: string; value: string };

const ENV_KEY_RE = /^[A-Za-z_][A-Za-z0-9_]*$/;
const HEADER_KEY_RE = /^[!#$%&'*+.^_`|~0-9A-Za-z-]+$/;

function recordToPairs(record: Record<string, string>): KVPair[] {
  const pairs = Object.entries(record).map(([key, value]) => ({ key, value }));
  return pairs.length > 0 ? pairs : [{ key: "", value: "" }];
}

function keysToPreservedPairs(keys: string[]): KVPair[] {
  const pairs = keys.map((key) => ({ key, value: "" }));
  return pairs.length > 0 ? pairs : [{ key: "", value: "" }];
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
        patch: { preserve_keys: [], replace: {} },
      };
    }
    if (!spec.keyPattern.test(key)) {
      return {
        error: `${spec.label} 第 ${i + 1} 行：${spec.keyLabel} 格式不正确`,
        patch: { preserve_keys: [], replace: {} },
      };
    }
    if (seenKeys.has(key)) {
      return {
        error: `${spec.label} 第 ${i + 1} 行：${spec.keyLabel} ${key} 重复`,
        patch: { preserve_keys: [], replace: {} },
      };
    }
    seenKeys.add(key);

    if (!value) {
      if (!spec.existingKeys.has(key)) {
        return {
          error: `${spec.label} 第 ${i + 1} 行：新增 ${spec.keyLabel} 必须填写 ${spec.valueLabel}`,
          patch: { preserve_keys: [], replace: {} },
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
      preserve_keys: preserveKeys,
      replace,
    },
  };
}

function parseLines(text: string) {
  return text
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean);
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

    for (const cliKey of ["claude", "codex", "gemini"] as const) {
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
    headerPairs: recordToPairs(readStringMap(spec.headers ?? spec.http_headers ?? spec.httpHeaders)),
  };
}

function fromServerSummary(
  server: Pick<
    McpServerSummary,
    | "name"
    | "transport"
    | "command"
    | "args"
    | "env_keys"
    | "cwd"
    | "url"
    | "header_keys"
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
  const inputCls =
    "rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-800 px-3 py-1.5 font-mono text-xs text-slate-900 dark:text-slate-100 shadow-sm outline-none focus:border-accent focus:ring-2 focus:ring-accent/20";

  const updatePair = (index: number, field: "key" | "value", val: string) => {
    const next = pairs.map((p, i) => (i === index ? { ...p, [field]: val } : p));
    onChange(next);
  };

  const removePair = (index: number) => {
    const next = pairs.filter((_, i) => i !== index);
    onChange(next.length > 0 ? next : [{ key: "", value: "" }]);
  };

  const addPair = () => {
    onChange([...pairs, { key: "", value: "" }]);
  };

  return (
    <div className="space-y-2">
      {pairs.map((pair, index) => (
        <div key={index} className="flex items-center gap-2">
          <input
            type="text"
            value={pair.key}
            onChange={(e) => updatePair(index, "key", e.currentTarget.value)}
            placeholder={keyPlaceholder}
            className={cn("w-[40%] shrink-0", inputCls)}
          />
          <span className="text-xs text-slate-400 select-none">=</span>
          <input
            type="text"
            value={pair.value}
            onChange={(e) => updatePair(index, "value", e.currentTarget.value)}
            placeholder={valuePlaceholder}
            className={cn("min-w-0 flex-1", inputCls)}
          />
          <button
            type="button"
            onClick={() => removePair(index)}
            className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg text-slate-400 hover:bg-red-50 hover:text-red-500 dark:hover:bg-red-900/20 dark:hover:text-red-400 transition-colors"
            title="删除"
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

export function McpServerDialog({
  workspaceId,
  open,
  editTarget,
  onOpenChange,
}: McpServerDialogProps) {
  const upsertMutation = useMcpServerUpsertMutation(workspaceId);
  const saving = upsertMutation.isPending;

  const [name, setName] = useState("");
  const [transport, setTransport] = useState<McpTransport>("stdio");
  const [command, setCommand] = useState("");
  const [argsText, setArgsText] = useState("");
  const [envPairs, setEnvPairs] = useState<KVPair[]>([{ key: "", value: "" }]);
  const [cwd, setCwd] = useState("");
  const [url, setUrl] = useState("");
  const [headerPairs, setHeaderPairs] = useState<KVPair[]>([{ key: "", value: "" }]);
  const [jsonText, setJsonText] = useState("");

  useEffect(() => {
    if (!open) return;
    if (editTarget) {
      const draft = fromServerSummary(editTarget);
      setName(draft.name);
      setTransport(draft.transport);
      setCommand(draft.command);
      setArgsText(draft.args.join("\n"));
      setEnvPairs(draft.envPairs);
      setCwd(draft.cwd);
      setUrl(draft.url);
      setHeaderPairs(draft.headerPairs);
      setJsonText("");
      return;
    }

    setName("");
    setTransport("stdio");
    setCommand("");
    setArgsText("");
    setEnvPairs([{ key: "", value: "" }]);
    setCwd("");
    setUrl("");
    setHeaderPairs([{ key: "", value: "" }]);
    setJsonText("");
  }, [open, editTarget]);

  const transportHint =
    transport === "sse"
      ? "SSE（Server-Sent Events）"
      : transport === "http"
        ? "HTTP（远程服务）"
        : "STDIO（本地命令）";

  function applyDraft(draft: McpDialogDraft) {
    setName((prev) => (draft.name.trim() ? draft.name.trim() : prev.trim() ? prev : "MCP Server"));
    setTransport(draft.transport);
    setCommand(draft.command);
    setArgsText(draft.args.join("\n"));
    setEnvPairs(draft.envPairs);
    setCwd(draft.cwd);
    setUrl(draft.url);
    setHeaderPairs(draft.headerPairs);
  }

  async function fillFromJson() {
    const trimmed = jsonText.trim();
    if (!trimmed) {
      toast("请先粘贴 JSON");
      return;
    }

    try {
      const parsed = await mcpParseJson(trimmed);
      if (parsed?.servers?.length) {
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
        args: isStdio ? parseLines(argsText) : [],
        env: isStdio ? patch : { preserve_keys: [], replace: {} },
        cwd: isStdio ? (cwd.trim() ? cwd : null) : null,
        url: !isStdio ? url : null,
        headers: !isStdio ? patch : { preserve_keys: [], replace: {} },
      });

      if (!next) {
        return;
      }

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
          <div className="rounded-2xl border border-slate-200 dark:border-slate-700 bg-slate-50 dark:bg-slate-800 p-4 shadow-sm">
            <div className="text-xs font-medium text-slate-500 dark:text-slate-400">
              快速导入 JSON（可选）
            </div>
            <textarea
              value={jsonText}
              onChange={(e) => setJsonText(e.currentTarget.value)}
              placeholder='示例：{"type":"stdio","command":"uvx","args":["mcp-server-fetch"]}'
              rows={4}
              className="mt-2 w-full resize-y rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-800 px-3 py-2 font-mono text-xs text-slate-900 dark:text-slate-100 shadow-sm outline-none focus:border-accent focus:ring-2 focus:ring-accent/20"
            />
            <div className="mt-2 flex justify-end">
              <Button variant="secondary" onClick={() => void fillFromJson()} disabled={saving}>
                从 JSON 填充
              </Button>
            </div>
          </div>
        ) : null}

        <div className="rounded-2xl border border-slate-200 dark:border-slate-700 bg-gradient-to-b from-white to-slate-50/60 dark:from-slate-800 dark:to-slate-800/60 p-4 shadow-card">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="text-xs font-medium text-slate-500 dark:text-slate-400">基础信息</div>
          </div>

          <div className="mt-3">
            <div className="text-sm font-medium text-slate-700 dark:text-slate-300">名称</div>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.currentTarget.value)}
              placeholder="例如：Fetch 工具"
              className="mt-2 w-full rounded-xl border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-800 px-3 py-2 text-sm text-slate-900 dark:text-slate-100 shadow-sm outline-none focus:border-accent focus:ring-2 focus:ring-accent/20"
            />
          </div>

          <div className="mt-4">
            <div className="flex items-center justify-between gap-3">
              <div className="text-sm font-medium text-slate-700 dark:text-slate-300">类型</div>
              <div className="text-xs text-slate-500 dark:text-slate-400">二选一</div>
            </div>
            <div className="mt-2 grid gap-2 sm:grid-cols-3">
              {(
                [
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
                ] as const
              ).map((item) => (
                <label key={item.value} className="relative block">
                  <input
                    type="radio"
                    name="mcp-transport"
                    value={item.value}
                    checked={transport === item.value}
                    onChange={() => setTransport(item.value)}
                    className="peer sr-only"
                  />
                  <div
                    className={cn(
                      "flex h-full cursor-pointer items-start gap-3 rounded-xl border px-3 py-3 shadow-sm transition-all",
                      "bg-white dark:bg-slate-800",
                      "hover:border-slate-300 hover:bg-slate-50/60 dark:hover:border-slate-600 dark:hover:bg-slate-700",
                      "peer-focus-visible:ring-2 peer-focus-visible:ring-accent/20 peer-focus-visible:ring-offset-2 peer-focus-visible:ring-offset-white dark:peer-focus-visible:ring-offset-slate-900",
                      "peer-checked:border-accent/60 peer-checked:bg-accent/5 peer-checked:shadow dark:peer-checked:bg-accent/10"
                    )}
                  >
                    <div
                      className={cn(
                        "mt-0.5 flex h-9 w-9 items-center justify-center rounded-lg border bg-white dark:bg-slate-800 shadow-sm",
                        "border-slate-200 text-slate-700 dark:border-slate-600 dark:text-slate-300",
                        "peer-checked:border-accent/40 peer-checked:bg-accent/10 peer-checked:text-accent"
                      )}
                    >
                      <span className="text-sm font-semibold">{item.icon}</span>
                    </div>

                    <div className="min-w-0 pr-7">
                      <div className="text-sm font-semibold text-slate-900 dark:text-slate-100">
                        {item.title}
                      </div>
                      <div className="mt-0.5 text-xs leading-relaxed text-slate-500 dark:text-slate-400">
                        {item.desc}
                      </div>
                    </div>

                    <div className="pointer-events-none absolute right-3 top-3 flex h-5 w-5 items-center justify-center rounded-full border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-800 text-[11px] text-white shadow-sm transition peer-checked:border-accent peer-checked:bg-accent">
                      ✓
                    </div>
                  </div>
                </label>
              ))}
            </div>
          </div>
        </div>

        {transport === "stdio" ? (
          <>
            <div>
              <div className="text-sm font-medium text-slate-700 dark:text-slate-300">Command</div>
              <input
                type="text"
                value={command}
                onChange={(e) => setCommand(e.currentTarget.value)}
                placeholder="例如：npx"
                className="mt-2 w-full rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-800 px-3 py-2 font-mono text-sm text-slate-900 dark:text-slate-100 shadow-sm outline-none focus:border-accent focus:ring-2 focus:ring-accent/20"
              />
            </div>

            <div>
              <div className="text-sm font-medium text-slate-700 dark:text-slate-300">
                Args（每行一个）
              </div>
              <textarea
                value={argsText}
                onChange={(e) => setArgsText(e.currentTarget.value)}
                placeholder={`例如：\n-y\n@modelcontextprotocol/server-fetch`}
                rows={4}
                className="mt-2 w-full resize-y rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-800 px-3 py-2 font-mono text-xs text-slate-900 dark:text-slate-100 shadow-sm outline-none focus:border-accent focus:ring-2 focus:ring-accent/20"
              />
            </div>

            <div>
              <div className="text-sm font-medium text-slate-700 dark:text-slate-300">
                Env（环境变量）
              </div>
              {editTarget ? (
                <div className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                  旧值默认不显示。留空保留，删行删除，填新值替换。
                </div>
              ) : null}
              <div className="mt-2">
                <KeyValuePairEditor
                  pairs={envPairs}
                  onChange={setEnvPairs}
                  keyPlaceholder="KEY（例如 TOKEN）"
                  valuePlaceholder="VALUE（例如 sk-xxx）"
                />
              </div>
            </div>

            <div>
              <div className="text-sm font-medium text-slate-700 dark:text-slate-300">
                CWD（可选）
              </div>
              <input
                type="text"
                value={cwd}
                onChange={(e) => setCwd(e.currentTarget.value)}
                placeholder="例如：/Users/xxx/project"
                className="mt-2 w-full rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-800 px-3 py-2 font-mono text-sm text-slate-900 dark:text-slate-100 shadow-sm outline-none focus:border-accent focus:ring-2 focus:ring-accent/20"
              />
            </div>
          </>
        ) : (
          <>
            <div>
              <div className="text-sm font-medium text-slate-700 dark:text-slate-300">URL</div>
              <input
                type="text"
                value={url}
                onChange={(e) => setUrl(e.currentTarget.value)}
                placeholder="例如：https://example.com/mcp"
                className="mt-2 w-full rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-800 px-3 py-2 font-mono text-sm text-slate-900 dark:text-slate-100 shadow-sm outline-none focus:border-accent focus:ring-2 focus:ring-accent/20"
              />
            </div>

            <div>
              <div className="text-sm font-medium text-slate-700 dark:text-slate-300">Headers</div>
              {editTarget ? (
                <div className="mt-1 text-xs text-slate-500 dark:text-slate-400">
                  旧值默认不显示。留空保留，删行删除，填新值替换。
                </div>
              ) : null}
              <div className="mt-2">
                <KeyValuePairEditor
                  pairs={headerPairs}
                  onChange={setHeaderPairs}
                  keyPlaceholder="Header（例如 Authorization）"
                  valuePlaceholder="Value（例如 Bearer xxx）"
                />
              </div>
            </div>
          </>
        )}

        <div className="flex flex-wrap items-center gap-2">
          <Button
            onClick={save}
            variant="primary"
            disabled={saving || (transport === "stdio" ? !command.trim() : !url.trim())}
          >
            {saving ? "保存中…" : "保存并同步"}
          </Button>
          <Button onClick={() => onOpenChange(false)} variant="secondary" disabled={saving}>
            取消
          </Button>
        </div>
      </div>
    </Dialog>
  );
}
