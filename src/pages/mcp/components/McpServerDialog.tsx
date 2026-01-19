import { useEffect, useState } from "react";
import { toast } from "sonner";
import { logToConsole } from "../../../services/consoleLog";
import { mcpServerUpsert, type McpServerSummary, type McpTransport } from "../../../services/mcp";
import { Button } from "../../../ui/Button";
import { Dialog } from "../../../ui/Dialog";
import { Switch } from "../../../ui/Switch";
import { cn } from "../../../utils/cn";

export type McpServerDialogProps = {
  open: boolean;
  editTarget: McpServerSummary | null;
  onOpenChange: (open: boolean) => void;
  onSaved: () => void | Promise<void>;
};

function parseLines(text: string) {
  return text
    .split("\n")
    .map((l) => l.trim())
    .filter(Boolean);
}

function parseKeyValueLines(text: string, hint: string) {
  const out: Record<string, string> = {};
  const lines = parseLines(text);
  for (const line of lines) {
    const idx = line.indexOf("=");
    if (idx <= 0) {
      throw new Error(`${hint} 格式错误：请使用 KEY=VALUE（示例：FOO=bar）`);
    }
    const k = line.slice(0, idx).trim();
    const v = line.slice(idx + 1).trim();
    if (!k) throw new Error(`${hint} 格式错误：KEY 不能为空`);
    out[k] = v;
  }
  return out;
}

function enabledLabel(server: McpServerSummary) {
  const enabled: string[] = [];
  if (server.enabled_claude) enabled.push("Claude");
  if (server.enabled_codex) enabled.push("Codex");
  if (server.enabled_gemini) enabled.push("Gemini");
  return enabled.length ? enabled.join(" / ") : "未启用";
}

export function McpServerDialog({ open, editTarget, onOpenChange, onSaved }: McpServerDialogProps) {
  const [saving, setSaving] = useState(false);

  const [name, setName] = useState("");
  const [transport, setTransport] = useState<McpTransport>("stdio");
  const [command, setCommand] = useState("");
  const [argsText, setArgsText] = useState("");
  const [envText, setEnvText] = useState("");
  const [cwd, setCwd] = useState("");
  const [url, setUrl] = useState("");
  const [headersText, setHeadersText] = useState("");

  const [enabledClaude, setEnabledClaude] = useState(false);
  const [enabledCodex, setEnabledCodex] = useState(false);
  const [enabledGemini, setEnabledGemini] = useState(false);

  useEffect(() => {
    if (!open) return;
    if (editTarget) {
      setName(editTarget.name);
      setTransport(editTarget.transport);
      setCommand(editTarget.command ?? "");
      setArgsText((editTarget.args ?? []).join("\n"));
      setEnvText(
        Object.entries(editTarget.env ?? {})
          .map(([k, v]) => `${k}=${v}`)
          .join("\n")
      );
      setCwd(editTarget.cwd ?? "");
      setUrl(editTarget.url ?? "");
      setHeadersText(
        Object.entries(editTarget.headers ?? {})
          .map(([k, v]) => `${k}=${v}`)
          .join("\n")
      );
      setEnabledClaude(editTarget.enabled_claude);
      setEnabledCodex(editTarget.enabled_codex);
      setEnabledGemini(editTarget.enabled_gemini);
      return;
    }

    setName("");
    setTransport("stdio");
    setCommand("");
    setArgsText("");
    setEnvText("");
    setCwd("");
    setUrl("");
    setHeadersText("");
    setEnabledClaude(false);
    setEnabledCodex(false);
    setEnabledGemini(false);
  }, [open, editTarget]);

  const transportHint = transport === "http" ? "HTTP（远程服务）" : "STDIO（本地命令）";

  async function save() {
    if (saving) return;
    setSaving(true);
    try {
      const next = await mcpServerUpsert({
        server_id: editTarget?.id ?? null,
        // server_key 是内部标识，用于写入 CLI 配置文件：
        // - Claude/Gemini: JSON map key
        // - Codex: TOML table name
        // 为降低认知负担，创建时自动生成；编辑时保持不变。
        server_key: editTarget?.server_key ?? "",
        name,
        transport,
        command: transport === "stdio" ? command : null,
        args: transport === "stdio" ? parseLines(argsText) : [],
        env: transport === "stdio" ? parseKeyValueLines(envText, "Env") : {},
        cwd: transport === "stdio" ? (cwd.trim() ? cwd : null) : null,
        url: transport === "http" ? url : null,
        headers: transport === "http" ? parseKeyValueLines(headersText, "Headers") : {},
        enabled_claude: enabledClaude,
        enabled_codex: enabledCodex,
        enabled_gemini: enabledGemini,
      });

      if (!next) {
        toast("仅在 Tauri Desktop 环境可用");
        return;
      }

      logToConsole("info", editTarget ? "更新 MCP Server" : "新增 MCP Server", {
        id: next.id,
        server_key: next.server_key,
        transport: next.transport,
        enabled: enabledLabel(next),
      });

      toast(editTarget ? "已更新" : "已新增");
      onOpenChange(false);
      await onSaved();
    } catch (err) {
      logToConsole("error", "保存 MCP Server 失败", { error: String(err) });
      toast(`保存失败：${String(err)}`);
    } finally {
      setSaving(false);
    }
  }

  return (
    <Dialog
      open={open}
      title={editTarget ? "编辑 MCP 服务" : "添加 MCP 服务"}
      description={
        editTarget ? "修改后会自动同步到启用的 CLI 配置文件。" : `类型：${transportHint}`
      }
      onOpenChange={onOpenChange}
      className="max-w-3xl"
    >
      <div className="grid gap-4">
        <div className="rounded-2xl border border-slate-200 bg-gradient-to-b from-white to-slate-50/60 p-4 shadow-card">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="text-xs font-medium text-slate-500">基础信息</div>
          </div>

          <div className="mt-3">
            <div className="text-sm font-medium text-slate-700">名称</div>
            <input
              type="text"
              value={name}
              onChange={(e) => setName(e.currentTarget.value)}
              placeholder="例如：Fetch 工具"
              className="mt-2 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 shadow-sm outline-none focus:border-[#0052FF] focus:ring-2 focus:ring-[#0052FF]/20"
            />
          </div>

          <div className="mt-4">
            <div className="flex items-center justify-between gap-3">
              <div className="text-sm font-medium text-slate-700">类型</div>
              <div className="text-xs text-slate-500">二选一</div>
            </div>
            <div className="mt-2 grid gap-2 sm:grid-cols-2">
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
                      "bg-white",
                      "hover:border-slate-300 hover:bg-slate-50/60 hover:shadow",
                      "peer-focus-visible:ring-2 peer-focus-visible:ring-[#0052FF]/20 peer-focus-visible:ring-offset-2 peer-focus-visible:ring-offset-white",
                      "peer-checked:border-[#0052FF]/60 peer-checked:bg-[#0052FF]/5 peer-checked:shadow"
                    )}
                  >
                    <div
                      className={cn(
                        "mt-0.5 flex h-9 w-9 items-center justify-center rounded-lg border bg-white shadow-sm",
                        "border-slate-200 text-slate-700",
                        "peer-checked:border-[#0052FF]/40 peer-checked:bg-[#0052FF]/10 peer-checked:text-[#0052FF]"
                      )}
                    >
                      <span className="text-sm font-semibold">{item.icon}</span>
                    </div>

                    <div className="min-w-0 pr-7">
                      <div className="text-sm font-semibold text-slate-900">{item.title}</div>
                      <div className="mt-0.5 text-xs leading-relaxed text-slate-500">
                        {item.desc}
                      </div>
                    </div>

                    <div className="pointer-events-none absolute right-3 top-3 flex h-5 w-5 items-center justify-center rounded-full border border-slate-300 bg-white text-[11px] text-white shadow-sm transition peer-checked:border-[#0052FF] peer-checked:bg-[#0052FF]">
                      ✓
                    </div>
                  </div>
                </label>
              ))}
            </div>
          </div>
        </div>

        <div>
          <div className="text-sm font-medium text-slate-700">生效范围</div>
          <div className="mt-2 flex flex-wrap items-center gap-3">
            <div className="flex items-center gap-2">
              <Switch checked={enabledClaude} onCheckedChange={setEnabledClaude} />
              <span className="text-sm text-slate-700">Claude</span>
            </div>
            <div className="flex items-center gap-2">
              <Switch checked={enabledCodex} onCheckedChange={setEnabledCodex} />
              <span className="text-sm text-slate-700">Codex</span>
            </div>
            <div className="flex items-center gap-2">
              <Switch checked={enabledGemini} onCheckedChange={setEnabledGemini} />
              <span className="text-sm text-slate-700">Gemini</span>
            </div>
          </div>
        </div>

        {transport === "stdio" ? (
          <>
            <div>
              <div className="text-sm font-medium text-slate-700">Command</div>
              <input
                type="text"
                value={command}
                onChange={(e) => setCommand(e.currentTarget.value)}
                placeholder="例如：npx"
                className="mt-2 w-full rounded-lg border border-slate-200 bg-white px-3 py-2 font-mono text-sm text-slate-900 shadow-sm outline-none focus:border-[#0052FF] focus:ring-2 focus:ring-[#0052FF]/20"
              />
            </div>

            <div className="grid gap-3 sm:grid-cols-2">
              <div>
                <div className="text-sm font-medium text-slate-700">Args（每行一个）</div>
                <textarea
                  value={argsText}
                  onChange={(e) => setArgsText(e.currentTarget.value)}
                  placeholder={`例如：\n-y\n@modelcontextprotocol/server-fetch`}
                  rows={6}
                  className="mt-2 w-full resize-y rounded-lg border border-slate-200 bg-white px-3 py-2 font-mono text-xs text-slate-900 shadow-sm outline-none focus:border-[#0052FF] focus:ring-2 focus:ring-[#0052FF]/20"
                />
              </div>

              <div>
                <div className="text-sm font-medium text-slate-700">Env（每行 KEY=VALUE）</div>
                <textarea
                  value={envText}
                  onChange={(e) => setEnvText(e.currentTarget.value)}
                  placeholder={`例如：\nFOO=bar\nTOKEN=xxx`}
                  rows={6}
                  className="mt-2 w-full resize-y rounded-lg border border-slate-200 bg-white px-3 py-2 font-mono text-xs text-slate-900 shadow-sm outline-none focus:border-[#0052FF] focus:ring-2 focus:ring-[#0052FF]/20"
                />
              </div>
            </div>

            <div>
              <div className="text-sm font-medium text-slate-700">CWD（可选）</div>
              <input
                type="text"
                value={cwd}
                onChange={(e) => setCwd(e.currentTarget.value)}
                placeholder="例如：/Users/xxx/project"
                className="mt-2 w-full rounded-lg border border-slate-200 bg-white px-3 py-2 font-mono text-sm text-slate-900 shadow-sm outline-none focus:border-[#0052FF] focus:ring-2 focus:ring-[#0052FF]/20"
              />
            </div>
          </>
        ) : (
          <>
            <div>
              <div className="text-sm font-medium text-slate-700">URL</div>
              <input
                type="text"
                value={url}
                onChange={(e) => setUrl(e.currentTarget.value)}
                placeholder="例如：https://example.com/mcp"
                className="mt-2 w-full rounded-lg border border-slate-200 bg-white px-3 py-2 font-mono text-sm text-slate-900 shadow-sm outline-none focus:border-[#0052FF] focus:ring-2 focus:ring-[#0052FF]/20"
              />
            </div>

            <div>
              <div className="text-sm font-medium text-slate-700">Headers（每行 KEY=VALUE）</div>
              <textarea
                value={headersText}
                onChange={(e) => setHeadersText(e.currentTarget.value)}
                placeholder={`例如：\nAuthorization=Bearer xxx\nX-Env=dev`}
                rows={6}
                className="mt-2 w-full resize-y rounded-lg border border-slate-200 bg-white px-3 py-2 font-mono text-xs text-slate-900 shadow-sm outline-none focus:border-[#0052FF] focus:ring-2 focus:ring-[#0052FF]/20"
              />
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
