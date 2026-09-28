import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { access, mkdtemp, mkdir, readFile, realpath, rm, writeFile } from "node:fs/promises";
import { arch, homedir, platform, tmpdir } from "node:os";
import { delimiter, join, resolve } from "node:path";
import { once } from "node:events";

// Real CLI protocol check using an isolated home and a loopback model provider.
// Usage: node scripts/codex-responses-ws-probe.mjs [--case NAME] [--report PATH] [--codex PATH]
const require = createRequire(import.meta.url);
let WebSocketServer;
try {
  ({ WebSocketServer } = require(require.resolve("ws", { paths: [require.resolve("jsdom")] })));
} catch {
  console.error(
    "Missing installed jsdom/ws dependency; run this probe in the prepared project workspace."
  );
  process.exit(1);
}
const options = {};
for (let i = 2; i < process.argv.length; i += 2) {
  const key = process.argv[i];
  assert(
    ["--case", "--report", "--codex"].includes(key) && process.argv[i + 1],
    "Invalid argument"
  );
  options[key.slice(2)] = process.argv[i + 1];
}
const cases = [
  "upgrade-http",
  "ws-tool",
  "context-retry",
  "context-http",
  "context-http-no-retries",
  "context-http-tools",
  "context-status-400",
  "prewarm",
  "prewarm-http",
  "context-two-tools",
  "business-400",
];
const nonceCases = new Set(["context-retry", "context-http"]);
const initialTurnState = "probe-initial-turn-state";
const replacementTurnState = "probe-replacement-turn-state";
const toolCases = new Set(
  cases.filter((name) => name.startsWith("context-")).concat("ws-tool", "business-400")
);
assert(!options.case || cases.includes(options.case), "Unknown case");
const safeHeaders = [
  "x-client-request-id",
  "x-codex-turn-metadata",
  "traceparent",
  "tracestate",
  "session_id",
  "session-id",
  "thread-id",
  "x-codex-window-id",
  "conversation_id",
  "openai-beta",
];
const report = {
  schema: 1,
  timestamp: new Date().toISOString(),
  platform: platform(),
  arch: arch(),
  node: process.version,
  cases: [],
  note: "Synthetic loopback model provider; isolated home and allowlisted environment. No user config or credentials copied. This does not prove OS-wide network isolation.",
};

function canonicalItem(value) {
  if (Array.isArray(value)) return value.map(canonicalItem);
  if (value !== null && typeof value === "object")
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, canonicalItem(value[key])])
    );
  return value;
}
function appendHistory(history, items) {
  let { digest, count } = history;
  for (const item of items) {
    digest = createHash("sha256")
      .update(digest)
      .update(JSON.stringify(canonicalItem(item)))
      .digest();
    count += 1;
  }
  return { digest, count };
}
function emptyHistory() {
  return { digest: Buffer.alloc(32), count: 0 };
}

function requestOwner(body) {
  const value = body.client_metadata?.["x-codex-turn-metadata"];
  if (typeof value !== "string") return null;
  try {
    const metadata = JSON.parse(value);
    const keys = ["session_id", "thread_id", "window_id", "context_window_id", "turn_id"];
    if (!keys.every((key) => typeof metadata[key] === "string" && metadata[key].length > 0))
      return null;
    return Object.fromEntries(keys.map((key) => [key, metadata[key]]));
  } catch {
    return null;
  }
}
function summarize(body) {
  const input = Array.isArray(body.input) ? body.input : [];
  const types = {};
  for (const item of input)
    types[item.type ?? item.role ?? "unknown"] =
      (types[item.type ?? item.role ?? "unknown"] ?? 0) + 1;
  return {
    type: body.type ?? null,
    request_owner: requestOwner(body),
    generate: body.generate ?? null,
    previous_response_id_present: Boolean(body.previous_response_id),
    input_count: input.length,
    input_types: types,
    client_metadata: Object.fromEntries(
      Object.entries(body.client_metadata ?? {}).filter(([key]) => safeHeaders.includes(key))
    ),
    tool_history_paired: input
      .filter((item) => item.type === "function_call_output")
      .every((output) =>
        input.some((item) => item.type === "function_call" && item.call_id === output.call_id)
      ),
    tool_names: flattenTools(body.tools).map(({ name, namespace, type }) => ({
      name,
      namespace,
      type,
    })),
  };
}
function flattenTools(tools = [], namespace) {
  return tools.flatMap((tool) =>
    tool.type === "namespace" ? flattenTools(tool.tools, tool.name) : [{ ...tool, namespace }]
  );
}
let interrupted = false;
let activeCleanup;
for (const signal of ["SIGINT", "SIGTERM"])
  process.once(signal, () => {
    interrupted = true;
    void activeCleanup?.();
  });

function completed(id, output = []) {
  return {
    type: "response.completed",
    response: {
      id,
      status: "completed",
      output,
      usage: { input_tokens: 1, output_tokens: 1, total_tokens: 2 },
    },
  };
}
function finalEvents(id) {
  return [
    { type: "response.created", response: { id } },
    {
      type: "response.output_item.done",
      item: {
        type: "message",
        id: `msg_${id}`,
        role: "assistant",
        content: [{ type: "output_text", text: "Local protocol probe completed." }],
      },
    },
    completed(id),
  ];
}
function toolEvents(body, sequence) {
  const tool = flattenTools(body.tools).find(
    (item) =>
      item.type === "function" && ["exec_command", "shell_command", "shell"].includes(item.name)
  );
  assert(tool, "CLI did not advertise a supported local shell tool");
  assert(
    platform() !== "win32" || tool.name !== "shell",
    "Legacy shell tool unsupported on Windows by this probe"
  );
  // Fixed benign command, with no body-derived arguments or external executable.
  const command =
    platform() === "win32"
      ? `Add-Content -LiteralPath probe-count.txt -Value executed_${sequence}`
      : `printf 'executed_${sequence}\\n' >> probe-count.txt`;
  const args =
    tool.name === "exec_command"
      ? { cmd: command, max_output_tokens: 50 }
      : tool.name === "shell_command"
        ? { command, timeout_ms: 1000 }
        : { command: ["sh", "-c", command], timeout_ms: 1000 };
  return [
    { type: "response.created", response: { id: `resp_tool_${sequence}` } },
    {
      type: "response.output_item.done",
      item: {
        type: "function_call",
        id: `fc_probe_call_${sequence}`,
        call_id: `probe_call_${sequence}`,
        name: tool.name,
        ...(tool.namespace ? { namespace: tool.namespace } : {}),
        arguments: JSON.stringify(args),
      },
    },
    completed(`resp_tool_${sequence}`),
  ];
}
function isolatedEnv(root) {
  const env = {};
  for (const key of ["PATH", "SystemRoot", "WINDIR", "COMSPEC", "PATHEXT"])
    if (process.env[key]) env[key] = process.env[key];
  return {
    ...env,
    HOME: root,
    USERPROFILE: root,
    CODEX_HOME: join(root, "codex"),
    TMPDIR: root,
    TMP: root,
    TEMP: root,
    VOLTA_HOME: process.env.VOLTA_HOME ?? join(homedir(), ".volta"),
    AIO_PROBE_TOKEN: "local-probe-token",
    NO_PROXY: "127.0.0.1,localhost",
    RUST_LOG: "off",
  };
}
async function runCase(name) {
  const root = await mkdtemp(join(tmpdir(), "aio-ws-probe-"));
  const work = join(root, "work");
  await Promise.all([mkdir(work), mkdir(join(root, "codex"))]);
  const result = {
    name,
    requests: [],
    cli_events: [],
    checks: {},
    failure: null,
    max_create_bytes: 0,
    max_http_body_bytes: 0,
    max_output_event_bytes: 0,
  };
  report.cases.push(result);
  const server = createServer();
  const wss = new WebSocketServer({ noServer: true, maxPayload: 4 * 1024 * 1024 });
  const sockets = new Set();
  let cli;
  let versionProcess;
  let cliGroupStopped = false;
  let connection = 0;
  const originalTools = [];
  let initialInput;
  const inputOutputs = new Map();
  const failedInputs = [];
  const failedHistories = [];
  let rollingHistory = emptyHistory();
  let needsRecovery = false;
  let injectedError = false;
  let errorCount = 0;
  let upgradeRejections = 0;
  let completedGenerations = 0;
  let nonceEventsSent = 0;
  let timer;
  const start = Date.now();
  const record = (request, transport, body, number) => {
    assert(result.requests.length < 64, "Request count exceeded probe limit");
    result.requests.push({
      elapsed_ms: Date.now() - start,
      transport,
      connection: number,
      turn_state_header_present: typeof request.headers["x-codex-turn-state"] === "string",
      turn_state_header_matches_initial: request.headers["x-codex-turn-state"] === initialTurnState,
      turn_state_metadata_present:
        typeof body?.client_metadata?.["x-codex-turn-state"] === "string",
      turn_state_metadata_matches_initial:
        body?.client_metadata?.["x-codex-turn-state"] === initialTurnState,
      method: request.method,
      path: request.url?.split("?")[0],
      client_cli_version:
        String(request.headers["user-agent"] ?? "").match(
          /\bcodex_(?:cli_rs|exec)\/(\d+\.\d+\.\d+)\b/
        )?.[1] ?? null,
      header_keys: Object.keys(request.headers).sort(),
      headers: Object.fromEntries(
        safeHeaders.filter((key) => request.headers[key]).map((key) => [key, request.headers[key]])
      ),
      ...(body ? summarize(body) : {}),
    });
  };
  const safeError = (error) =>
    error instanceof SyntaxError ? "Malformed JSON in local probe" : error.message;
  const failure = (error) => {
    result.failure ??= safeError(error);
    killCli();
  };
  const killCli = () => {
    if (!cli?.pid || cliGroupStopped) return;
    cliGroupStopped = true;
    if (platform() !== "win32") {
      try {
        process.kill(-cli.pid, "SIGKILL");
        return;
      } catch {}
    }
    if (platform() === "win32")
      spawnSync("taskkill", ["/PID", String(cli.pid), "/T", "/F"], {
        stdio: "ignore",
        timeout: 3000,
      });
    cli.kill("SIGKILL");
  };
  let cleanupPromise;
  const cleanup = () =>
    (cleanupPromise ??= (async () => {
      clearTimeout(timer);
      if (interrupted) result.failure ??= "Probe interrupted";
      killCli();
      if (versionProcess && versionProcess.exitCode === null && versionProcess.signalCode === null)
        versionProcess.kill("SIGKILL");
      for (const ws of wss.clients) ws.terminate();
      for (const socket of sockets) socket.destroy();
      await Promise.all([
        new Promise((done) => wss.close(done)),
        new Promise((done) => server.close(done)),
      ]);
    })());
  activeCleanup = cleanup;
  const contextError = (status) => [
    {
      type: "error",
      ...(status ? { status } : {}),
      error: {
        type: "invalid_request_error",
        code: name === "business-400" ? "invalid_prompt" : "previous_response_not_found",
        message: "Synthetic local context loss",
      },
    },
  ];
  const eventsFor = (body, transport) => {
    rollingHistory = appendHistory(
      body.previous_response_id ? rollingHistory : emptyHistory(),
      body.input
    );
    if (body.generate === false) {
      if (name === "prewarm-http") {
        injectedError = true;
        return contextError();
      }
      return [{ type: "response.created", response: { id: "resp_warm" } }, completed("resp_warm")];
    }
    initialInput ??= structuredClone(body.input);
    if (needsRecovery) {
      const originalInputPreserved = initialInput.every(
        (item, index) => JSON.stringify(body.input[index]) === JSON.stringify(item)
      );
      const toolsPreserved = originalTools.every((tool) =>
        body.input.some(
          (item) =>
            item.type === "function_call" &&
            item.call_id === tool.call_id &&
            item.name === tool.name &&
            item.arguments === tool.arguments
        )
      );
      const outputsPreserved = [...inputOutputs].every(([id, output]) =>
        body.input.some(
          (item) =>
            item.type === "function_call_output" &&
            item.call_id === id &&
            JSON.stringify(item) === JSON.stringify(output)
        )
      );
      const expectedHistory = failedHistories.at(-1);
      result.requests.at(-1).expected_full_input_count = expectedHistory?.count ?? null;
      result.requests.at(-1).recovery = {
        expected_full_input_matches: Boolean(
          expectedHistory &&
          expectedHistory.count === rollingHistory.count &&
          expectedHistory.digest.equals(rollingHistory.digest)
        ),
        original_input_preserved: originalInputPreserved,
        tool_arguments_preserved: toolsPreserved,
        tool_outputs_preserved: outputsPreserved,
        expected_tools: originalTools.length,
        owner_preserved:
          failedInputs.at(-1)?.request_owner !== null &&
          requestOwner(body) !== null &&
          JSON.stringify(failedInputs.at(-1)?.request_owner) === JSON.stringify(requestOwner(body)),
        failed_input_was_incremental: failedInputs.at(-1)?.previous_response_id_present ?? false,
      };
      needsRecovery = false;
    } else if (
      toolCases.has(name) &&
      originalTools.length > errorCount &&
      (name.startsWith("context-") || name === "business-400") &&
      transport === "ws"
    ) {
      for (const item of body.input)
        if (item.type === "function_call_output")
          inputOutputs.set(item.call_id, structuredClone(item));
      failedInputs.push(summarize(body));
      failedHistories.push(rollingHistory);
      injectedError = true;
      errorCount += 1;
      needsRecovery = true;
      return contextError(["context-status-400", "business-400"].includes(name) ? 400 : undefined);
    }
    const requiredTools = ["context-two-tools", "context-http-tools"].includes(name) ? 2 : 1;
    if (toolCases.has(name) && originalTools.length < requiredTools) {
      const events = toolEvents(body, originalTools.length + 1);
      originalTools.push(events[1].item);
      events.at(-1).response.output = [events[1].item];
      rollingHistory = appendHistory(rollingHistory, events.at(-1).response.output);
      completedGenerations += 1;
      return events;
    }
    completedGenerations += 1;
    const events = finalEvents("resp_final");
    events.at(-1).response.output = [events[1].item];
    rollingHistory = appendHistory(rollingHistory, events.at(-1).response.output);
    return events;
  };
  server.on("connection", (socket) => {
    sockets.add(socket);
    socket.on("close", () => sockets.delete(socket));
  });
  server.on("request", async (request, response) => {
    try {
      if (request.method !== "POST" || request.url !== "/v1/responses") {
        response.writeHead(404).end();
        return;
      }
      const chunks = [];
      let size = 0;
      for await (const chunk of request) {
        size += chunk.length;
        assert(size <= 4 * 1024 * 1024, "HTTP body exceeded probe limit");
        chunks.push(chunk);
      }
      result.max_http_body_bytes = Math.max(result.max_http_body_bytes, size);
      const body = JSON.parse(Buffer.concat(chunks));
      record(request, "http", body, null);
      response.writeHead(200, { "content-type": "text/event-stream" });
      for (const event of eventsFor(body, "http")) {
        const payload = JSON.stringify(event);
        result.max_output_event_bytes = Math.max(
          result.max_output_event_bytes,
          Buffer.byteLength(payload)
        );
        response.write(`data: ${payload}\n\n`);
      }
      response.end();
    } catch (error) {
      response.destroy();
      failure(error);
    }
  });
  server.on("upgrade", (request, socket, head) => {
    try {
      record(request, "handshake", null, ++connection);
      if (
        name === "upgrade-http" ||
        (["context-http", "context-http-tools", "prewarm-http"].includes(name) && injectedError)
      ) {
        upgradeRejections += 1;
        socket.end(
          "HTTP/1.1 426 Upgrade Required\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        return;
      }
      wss.handleUpgrade(request, socket, head, (ws) => {
        const number = connection;
        ws.on("error", failure);
        ws.on("message", (message) => {
          try {
            result.max_create_bytes = Math.max(result.max_create_bytes, message.length);
            const body = JSON.parse(message.toString());
            record(request, "ws", body, number);
            const events = eventsFor(body, "ws");
            if (
              nonceCases.has(name) &&
              body.generate !== false &&
              (nonceEventsSent === 0 || events.some((event) => event.type === "error"))
            ) {
              events.unshift({
                type: "response.metadata",
                headers: {
                  "x-codex-turn-state":
                    nonceEventsSent === 0 ? initialTurnState : replacementTurnState,
                },
              });
              nonceEventsSent += 1;
            }
            for (const event of events) {
              const payload = JSON.stringify(event);
              result.max_output_event_bytes = Math.max(
                result.max_output_event_bytes,
                Buffer.byteLength(payload)
              );
              ws.send(payload);
            }
            if (events.some((event) => event.type === "error")) ws.close(1000, "context reset");
          } catch (error) {
            failure(error);
          }
        });
      });
    } catch (error) {
      socket.destroy();
      failure(error);
    }
  });
  try {
    server.listen(0, "127.0.0.1");
    await once(server, "listening");
    const config = [
      'model = "gpt-5.4"',
      'model_provider = "probe"',
      'approval_policy = "never"',
      'sandbox_mode = "workspace-write"',
      'web_search = "disabled"',
      "[features]",
      "shell_snapshot = false",
      "background_shell = false",
      "multi_agent = false",
      "plugins = false",
      "apps = false",
      "[model_providers.probe]",
      `name = "${name === "prewarm" ? "OpenAI" : "Local probe"}"`,
      `base_url = "http://127.0.0.1:${server.address().port}/v1"`,
      'env_key = "AIO_PROBE_TOKEN"',
      'wire_api = "responses"',
      "supports_websockets = true",
      "requires_openai_auth = false",
      "request_max_retries = 0",
      `stream_max_retries = ${name === "context-http-no-retries" ? 0 : 1}`,
      "stream_idle_timeout_ms = 3000",
    ].join("\n");
    await writeFile(join(root, "codex", "config.toml"), config);
    if (!report.cli) {
      const command = options.codex ?? "codex";
      let resolved = command.includes("/") || command.includes("\\") ? resolve(command) : null;
      if (!resolved)
        for (const dir of (process.env.PATH ?? "").split(delimiter)) {
          const path = join(dir, platform() === "win32" ? `${command}.exe` : command);
          if (
            await access(path).then(
              () => true,
              () => false
            )
          ) {
            resolved = resolve(path);
            break;
          }
        }
      assert(resolved, "Cannot resolve Codex executable; supply --codex with its path");
      versionProcess = spawn(resolved, ["--version"], {
        cwd: work,
        env: isolatedEnv(root),
        stdio: ["ignore", "pipe", "ignore"],
      });
      let version = "";
      versionProcess.stdout.on("data", (chunk) => {
        if (version.length < 256) version += chunk.toString();
      });
      const versionTimer = setTimeout(() => versionProcess.kill("SIGKILL"), 5000);
      try {
        await once(versionProcess, "close");
      } finally {
        clearTimeout(versionTimer);
      }
      assert.equal(versionProcess.exitCode, 0, "Selected Codex executable must report its version");
      report.cli = {
        command,
        resolved,
        launcher: resolved ? await realpath(resolved) : null,
        version: version.trim().match(/^codex-cli [0-9.]+[^\n]*$/)?.[0] ?? "unknown",
      };
    }
    if (interrupted) throw new Error("Probe interrupted before CLI start");
    cli = spawn(
      report.cli.resolved,
      [
        "exec",
        "--skip-git-repo-check",
        "--ephemeral",
        "--ignore-rules",
        "--json",
        "--color",
        "never",
        "--cd",
        work,
        "Run the local protocol probe. If instructed by the test server, append one line to probe-count.txt exactly once, then finish.",
      ],
      {
        cwd: work,
        env: isolatedEnv(root),
        stdio: ["ignore", "pipe", "pipe"],
        detached: platform() !== "win32",
      }
    );
    let pending = "";
    let stdoutBytes = 0;
    let stderrBytes = 0;
    const stderrFlags = new Set();
    cli.stdout.on("data", (chunk) => {
      stdoutBytes += chunk.length;
      if (stdoutBytes > 1024 * 1024) {
        failure(new Error("CLI output exceeded probe limit"));
        return;
      }
      pending += chunk.toString();
      let end;
      while ((end = pending.indexOf("\n")) !== -1) {
        const line = pending.slice(0, end);
        pending = pending.slice(end + 1);
        try {
          const event = JSON.parse(line);
          result.cli_events.push({
            type: event.type,
            item_type: event.item?.type,
            status: event.item?.status,
            ...(event.type === "error" || event.type === "turn.failed"
              ? {
                  context_error: String(event.message ?? event.error?.message ?? "").includes(
                    "previous_response_not_found"
                  ),
                }
              : {}),
          });
        } catch {
          /* Non-JSON CLI output is deliberately discarded. */
        }
      }
    });
    cli.stderr.on("data", (chunk) => {
      stderrBytes += chunk.length;
      const text = chunk.toString();
      for (const flag of ["config", "sandbox", "Authentication", "unknown", "error", "not found"])
        if (text.includes(flag)) stderrFlags.add(flag);
    });
    timer = setTimeout(() => {
      result.failure ??= "CLI deadline exceeded (25 seconds)";
      killCli();
    }, 25_000);
    const [code, signal] = await once(cli, "close");
    result.cli_exit = { code, signal, stderr_bytes: stderrBytes, stderr_flags: [...stderrFlags] };
    result.elapsed_ms = Date.now() - start;
    const countText = await readFile(join(work, "probe-count.txt"), "utf8").catch(() => "");
    const executions = countText.split(/\r?\n/).filter(Boolean);
    result.tool_execution_count = executions.length;
    result.tool_execution_counts = Object.fromEntries(
      originalTools.map((_, index) => [
        index + 1,
        executions.filter((line) => line === `executed_${index + 1}`).length,
      ])
    );
    result.upgrade_rejections = upgradeRejections;
    result.completed_generations = completedGenerations;
    const ws = result.requests.filter((item) => item.transport === "ws" && item.generate !== false);
    const http = result.requests.filter((item) => item.transport === "http");
    result.checks.cli_completed =
      code === 0 && result.cli_events.some((item) => item.type === "turn.completed");
    if (name === "upgrade-http")
      result.checks.fallback_http =
        upgradeRejections === 1 && connection === 1 && http.length === 1 && ws.length === 0;
    if (toolCases.has(name)) {
      const expectedTools = ["context-two-tools", "context-http-tools"].includes(name) ? 2 : 1;
      result.checks.tool_once_each =
        result.tool_execution_count === expectedTools &&
        Object.values(result.tool_execution_counts).every((count) => count === 1);
      result.checks.incremental_tool_round = ws.some(
        (item) =>
          item.previous_response_id_present &&
          item.input_types.function_call_output === 1 &&
          !item.input_types.function_call
      );
    }
    if (name === "ws-tool")
      result.checks.same_connection = ws.length === 2 && ws[0].connection === ws[1].connection;
    if (name.startsWith("context-")) {
      const recovered = result.requests.filter((item) => item.recovery);
      const expectedRecoveries = name === "context-two-tools" ? 2 : 1;
      result.checks.error_injected = errorCount === expectedRecoveries;
      result.checks.automatic_full_retry =
        recovered.length === expectedRecoveries &&
        recovered.every(
          (item) =>
            !item.previous_response_id_present &&
            item.tool_history_paired &&
            Object.entries(item.recovery)
              .filter(([key]) => key !== "expected_tools")
              .every(([, value]) => value)
        );
      result.checks.retry_transport = recovered.every(
        (item) => item.transport === (name.startsWith("context-http") ? "http" : "ws")
      );
    }
    if (nonceCases.has(name)) {
      result.checks.first_generation_has_no_turn_state =
        !ws[0]?.turn_state_header_present && !ws[0]?.turn_state_metadata_present;
      result.checks.turn_state_metadata_event_sent = nonceEventsSent === 2;
      result.checks.turn_state_returned_in_incremental_metadata = ws.some(
        (item) =>
          item.previous_response_id_present &&
          item.input_types.function_call_output === 1 &&
          item.turn_state_metadata_matches_initial
      );
      const recovered = result.requests.find((item) => item.recovery);
      result.checks.turn_state_survives_retry =
        name === "context-http"
          ? recovered?.turn_state_header_matches_initial === true
          : recovered?.turn_state_metadata_matches_initial === true;
      result.checks.turn_state_once_lock_preserved =
        nonceEventsSent === 2 &&
        result.checks.turn_state_returned_in_incremental_metadata &&
        result.checks.turn_state_survives_retry;
      result.checks.turn_state_absent_from_ws_handshake = result.requests
        .filter((item) => item.transport === "handshake")
        .every((item) => !item.turn_state_header_present);
    }
    if (name === "business-400") {
      delete result.checks.cli_completed;
      result.checks.business_400_terminates =
        code !== 0 && result.cli_events.some((item) => item.type === "turn.failed");
      result.checks.no_automatic_replay = ws.length === 2 && http.length === 0;
    }
    if (name === "context-http-no-retries")
      result.checks.http_without_reconnect = http.length === 1 && connection === 1;
    if (name === "context-http-tools") {
      const firstHttp = result.requests.findIndex((item) => item.transport === "http");
      result.checks.http_sticky_after_recovery =
        firstHttp !== -1 &&
        http.length === 2 &&
        result.requests.slice(firstHttp).every((item) => item.transport === "http");
    }
    if (name === "prewarm" || name === "prewarm-http")
      result.checks.prewarm_observed = result.requests.some((item) => item.generate === false);
    if (name === "prewarm-http") {
      result.checks.http_full_without_warm_reference =
        http.length === 1 && !http[0].previous_response_id_present && http[0].input_count >= 1;
      result.checks.no_prewarm_generation = completedGenerations === 1 && ws.length === 0;
      result.checks.reconnect_rejected = upgradeRejections === 1;
    }
    if (Object.values(result.checks).some((ok) => !ok))
      result.failure ??= "One or more protocol assertions failed";
  } catch (error) {
    result.failure ??= safeError(error);
  } finally {
    await cleanup();
    if (cli && cli.exitCode === null && cli.signalCode === null)
      await once(cli, "close").catch(() => {});
    activeCleanup = undefined;
    await rm(root, { recursive: true, force: true, maxRetries: 3, retryDelay: 100 }).catch(
      (error) => {
        result.failure ??= `Temporary directory cleanup failed (${error.code ?? "unknown"})`;
      }
    );
  }
}
for (const name of options.case ? [options.case] : cases) {
  if (interrupted) break;
  await runCase(name);
}
report.passed = !interrupted && report.cases.every((item) => item.failure === null);
const output = JSON.stringify(report, null, 2) + "\n";
if (options.report) await writeFile(resolve(options.report), output, { mode: 0o600 });
else process.stdout.write(output);
process.exitCode = report.passed ? 0 : 1;
