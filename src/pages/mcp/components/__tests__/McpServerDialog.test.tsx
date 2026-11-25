import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { useMcpServerUpsertMutation } from "../../../../query/mcp";
import { mcpParseJson } from "../../../../services/workspace/mcp";
import {
  type McpImportServer,
  type McpParseResult,
  type McpServerSummary,
} from "../../../../services/workspace/mcp";
import { McpServerDialog } from "../McpServerDialog";

vi.mock("sonner", () => ({ toast: vi.fn() }));
vi.mock("../../../../services/consoleLog", () => ({ logToConsole: vi.fn() }));

vi.mock("../../../../query/mcp", async () => {
  const actual =
    await vi.importActual<typeof import("../../../../query/mcp")>("../../../../query/mcp");
  return { ...actual, useMcpServerUpsertMutation: vi.fn() };
});

vi.mock("../../../../services/workspace/mcp", async () => {
  const actual = await vi.importActual<typeof import("../../../../services/workspace/mcp")>(
    "../../../../services/workspace/mcp"
  );
  return { ...actual, mcpParseJson: vi.fn() };
});

type UpsertMutationMock = Pick<
  ReturnType<typeof useMcpServerUpsertMutation>,
  "isPending" | "mutateAsync"
>;

function createUpsertMutation(overrides: Partial<UpsertMutationMock> = {}): UpsertMutationMock {
  return {
    isPending: false,
    mutateAsync: vi.fn(),
    ...overrides,
  };
}

function mockUpsertMutation(mutation: UpsertMutationMock) {
  vi.mocked(useMcpServerUpsertMutation).mockReturnValue(
    mutation as ReturnType<typeof useMcpServerUpsertMutation>
  );
}

function makeMcpServerSummary(overrides: Partial<McpServerSummary> = {}): McpServerSummary {
  return {
    id: 1,
    server_key: "fetch",
    name: "Fetch",
    transport: "stdio",
    command: "uvx",
    args: ["mcp-server-fetch"],
    env_keys: [],
    cwd: null,
    url: null,
    header_keys: [],
    enabled: true,
    created_at: 1,
    updated_at: 1,
    ...overrides,
  };
}

function makeMcpImportServer(overrides: Partial<McpImportServer> = {}): McpImportServer {
  return {
    server_key: "fetch",
    name: "Fetch",
    transport: "stdio",
    command: "uvx",
    args: ["mcp-server-fetch"],
    env: { FOO: "bar" },
    cwd: null,
    url: null,
    headers: {},
    enabled: true,
    ...overrides,
  };
}

function makeMcpParseResult(overrides: Partial<McpParseResult> = {}): McpParseResult {
  return {
    servers: [makeMcpImportServer()],
    ...overrides,
  };
}

describe("pages/mcp/components/McpServerDialog", () => {
  it("renders saving state and disables submit", () => {
    mockUpsertMutation(createUpsertMutation({ isPending: true }));

    render(
      <McpServerDialog workspaceId={1} open={true} editTarget={null} onOpenChange={vi.fn()} />
    );

    expect(screen.getByRole("button", { name: "保存中…" })).toBeDisabled();
  });

  it("preserves args while typing and saving, including whitespace and empty arguments", async () => {
    const user = userEvent.setup();
    const mutateAsync = vi.fn().mockResolvedValue(makeMcpServerSummary());
    mockUpsertMutation(createUpsertMutation({ mutateAsync }));

    render(
      <McpServerDialog
        workspaceId={1}
        open={true}
        editTarget={makeMcpServerSummary({ args: [] })}
        onOpenChange={vi.fn()}
      />
    );

    const args = screen.getByRole("textbox", { name: "Args（每行一个）" });
    await user.type(args, "--flag{Enter}{Enter}  lowercase \"quoted\" 'single' -- ...  {Enter}");
    expect(args).toHaveValue("--flag\n\n  lowercase \"quoted\" 'single' -- ...  \n");

    await user.click(screen.getByRole("button", { name: "保存并同步" }));
    expect(mutateAsync).toHaveBeenLastCalledWith(
      expect.objectContaining({
        args: ["--flag", "", "  lowercase \"quoted\" 'single' -- ...  ", ""],
      })
    );

    await user.clear(args);
    await user.click(screen.getByRole("button", { name: "保存并同步" }));
    expect(mutateAsync).toHaveBeenLastCalledWith(expect.objectContaining({ args: [] }));
  });

  it("validates env and can save stdio servers", async () => {
    const mutateAsync = vi.fn();
    mockUpsertMutation(createUpsertMutation({ mutateAsync }));

    const onOpenChange = vi.fn();

    render(
      <McpServerDialog workspaceId={1} open={true} editTarget={null} onOpenChange={onOpenChange} />
    );

    fireEvent.change(screen.getByPlaceholderText("例如：Fetch 工具"), {
      target: { value: "Fetch Tool" },
    });
    fireEvent.change(screen.getByPlaceholderText("例如：npx"), { target: { value: "node" } });

    // Invalid env: should fail before hitting mutation.
    fireEvent.change(screen.getByPlaceholderText("KEY（例如 TOKEN）"), {
      target: { value: "BADLINE" },
    });
    fireEvent.change(screen.getByPlaceholderText("VALUE（例如 sk-xxx）"), {
      target: { value: "" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存并同步" }));
    await waitFor(() => expect(mutateAsync).not.toHaveBeenCalled());

    // Valid env: mutation runs and returns the updated summary.
    fireEvent.change(screen.getByPlaceholderText("KEY（例如 TOKEN）"), {
      target: { value: "FOO" },
    });
    fireEvent.change(screen.getByPlaceholderText("VALUE（例如 sk-xxx）"), {
      target: { value: "bar" },
    });
    fireEvent.change(screen.getByPlaceholderText(/-y/), { target: { value: "-y\n@foo/bar" } });
    mutateAsync.mockResolvedValueOnce(
      makeMcpServerSummary({
        id: 1,
        server_key: "fetch",
        transport: "stdio",
        command: "node",
        args: ["-y", "@foo/bar"],
      })
    );
    fireEvent.click(screen.getByRole("button", { name: "保存并同步" }));
    await waitFor(() =>
      expect(mutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          serverId: null,
          serverKey: "",
          name: "Fetch Tool",
          transport: "stdio",
          command: "node",
          args: ["-y", "@foo/bar"],
          env: {
            preserveKeys: [],
            replace: { FOO: "bar" },
          },
          cwd: null,
          url: null,
          headers: {
            preserveKeys: [],
            replace: {},
          },
        })
      )
    );
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false));
  });

  it("blocks invalid env keys in pair editor", async () => {
    const mutateAsync = vi.fn();
    mockUpsertMutation(createUpsertMutation({ mutateAsync }));

    render(
      <McpServerDialog workspaceId={1} open={true} editTarget={null} onOpenChange={vi.fn()} />
    );

    fireEvent.change(screen.getByPlaceholderText("例如：Fetch 工具"), {
      target: { value: "Fetch Tool" },
    });
    fireEvent.change(screen.getByPlaceholderText("例如：npx"), { target: { value: "node" } });

    fireEvent.change(screen.getByPlaceholderText("KEY（例如 TOKEN）"), {
      target: { value: "FOO-BAR" },
    });
    fireEvent.change(screen.getByPlaceholderText("VALUE（例如 sk-xxx）"), {
      target: { value: "baz" },
    });

    fireEvent.click(screen.getByRole("button", { name: "保存并同步" }));
    await waitFor(() => expect(mutateAsync).not.toHaveBeenCalled());
  });

  it("saves stdio server with blank env row and cwd", async () => {
    const mutateAsync = vi.fn().mockResolvedValueOnce(
      makeMcpServerSummary({
        id: 1,
        server_key: "demo",
        name: "Demo",
        transport: "stdio",
        command: "node",
        cwd: "/tmp",
      })
    );
    mockUpsertMutation(createUpsertMutation({ mutateAsync }));

    const onOpenChange = vi.fn();
    render(
      <McpServerDialog workspaceId={1} open={true} editTarget={null} onOpenChange={onOpenChange} />
    );

    fireEvent.change(screen.getByPlaceholderText("例如：Fetch 工具"), {
      target: { value: "Demo" },
    });
    fireEvent.change(screen.getByPlaceholderText("例如：npx"), { target: { value: "node" } });
    fireEvent.change(screen.getByPlaceholderText("例如：/Users/xxx/project"), {
      target: { value: "/tmp" },
    });

    fireEvent.click(screen.getByRole("button", { name: "+ 添加一行" }));
    const envKeys = screen.getAllByPlaceholderText("KEY（例如 TOKEN）");
    const envValues = screen.getAllByPlaceholderText("VALUE（例如 sk-xxx）");
    fireEvent.change(envKeys[0], { target: { value: "FOO" } });
    fireEvent.change(envValues[0], { target: { value: "bar" } });

    fireEvent.click(screen.getByRole("button", { name: "保存并同步" }));
    await waitFor(() =>
      expect(mutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          serverId: null,
          serverKey: "",
          name: "Demo",
          transport: "stdio",
          command: "node",
          cwd: "/tmp",
          env: {
            preserveKeys: [],
            replace: { FOO: "bar" },
          },
          url: null,
          headers: {
            preserveKeys: [],
            replace: {},
          },
        })
      )
    );
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false));
  });

  it("prefills and saves http servers with headers parsing", async () => {
    const mutateAsync = vi.fn();
    mockUpsertMutation(createUpsertMutation({ mutateAsync }));

    const onOpenChange = vi.fn();

    render(
      <McpServerDialog
        workspaceId={1}
        open={true}
        editTarget={makeMcpServerSummary({
          id: 7,
          server_key: "remote",
          name: "Remote",
          transport: "http",
          command: null,
          args: [],
          url: "https://example.com/mcp",
          header_keys: ["Authorization"],
        })}
        onOpenChange={onOpenChange}
      />
    );

    expect(screen.getByDisplayValue("Remote")).toBeInTheDocument();
    expect(screen.getByDisplayValue("https://example.com/mcp")).toBeInTheDocument();

    // Invalid headers should block mutation.
    fireEvent.change(screen.getByPlaceholderText("Header（例如 Authorization）"), {
      target: { value: "" },
    });
    fireEvent.change(screen.getByPlaceholderText("Value（例如 Bearer xxx）"), {
      target: { value: "BAD" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存并同步" }));
    await waitFor(() => expect(mutateAsync).not.toHaveBeenCalled());

    fireEvent.change(screen.getByPlaceholderText("Header（例如 Authorization）"), {
      target: { value: "Authorization" },
    });
    fireEvent.change(screen.getByPlaceholderText("Value（例如 Bearer xxx）"), {
      target: { value: "Bearer y" },
    });
    mutateAsync.mockResolvedValueOnce(
      makeMcpServerSummary({
        id: 7,
        server_key: "remote",
        name: "Remote",
        transport: "http",
        command: null,
        args: [],
        url: "https://example.com/mcp",
        header_keys: ["Authorization"],
      })
    );
    fireEvent.click(screen.getByRole("button", { name: "保存并同步" }));
    await waitFor(() =>
      expect(mutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          serverId: 7,
          serverKey: "remote",
          name: "Remote",
          transport: "http",
          command: null,
          args: [],
          env: {
            preserveKeys: [],
            replace: {},
          },
          cwd: null,
          url: "https://example.com/mcp",
          headers: {
            preserveKeys: [],
            replace: { Authorization: "Bearer y" },
          },
        })
      )
    );
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false));
  });

  it("fills fields from JSON fallback parser when service returns empty", async () => {
    mockUpsertMutation(createUpsertMutation());
    vi.mocked(mcpParseJson).mockResolvedValueOnce(makeMcpParseResult({ servers: [] }));

    render(
      <McpServerDialog workspaceId={1} open={true} editTarget={null} onOpenChange={vi.fn()} />
    );

    fireEvent.change(screen.getByPlaceholderText(/示例：\{"type":"stdio"/), {
      target: {
        value:
          '{"type":"stdio","name":"Demo","command":"node","args":["-y","@foo/bar"],"env":{"FOO":"bar"}}',
      },
    });
    fireEvent.click(screen.getByRole("button", { name: "从 JSON 填充" }));

    await waitFor(() => expect(screen.getByDisplayValue("Demo")).toBeInTheDocument());
    expect(screen.getByDisplayValue("node")).toBeInTheDocument();
    expect(screen.getByDisplayValue("FOO")).toBeInTheDocument();
    expect(screen.getByDisplayValue("bar")).toBeInTheDocument();
  });

  it("fills fields from mcpServers JSON and preserves sse transport", async () => {
    const mutateAsync = vi.fn();
    mockUpsertMutation(createUpsertMutation({ mutateAsync }));
    vi.mocked(mcpParseJson).mockResolvedValueOnce(makeMcpParseResult({ servers: [] }));

    render(
      <McpServerDialog workspaceId={1} open={true} editTarget={null} onOpenChange={vi.fn()} />
    );

    fireEvent.change(screen.getByPlaceholderText(/示例：\{"type":"stdio"/), {
      target: {
        value:
          '{"mcpServers":{"remote":{"transport":"sse","url":"https://example.com/mcp","headers":{"Authorization":"Bearer x"}}}}',
      },
    });
    fireEvent.click(screen.getByRole("button", { name: "从 JSON 填充" }));

    await waitFor(() => expect(screen.getByDisplayValue("remote")).toBeInTheDocument());
    expect(screen.getByDisplayValue("https://example.com/mcp")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Authorization")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Bearer x")).toBeInTheDocument();

    mutateAsync.mockResolvedValueOnce(
      makeMcpServerSummary({
        id: 1,
        server_key: "remote",
        name: "remote",
        transport: "sse",
        command: null,
        args: [],
        url: "https://example.com/mcp",
        header_keys: ["Authorization"],
      })
    );
    fireEvent.click(screen.getByRole("button", { name: "保存并同步" }));
    await waitFor(() =>
      expect(mutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          serverId: null,
          serverKey: "",
          name: "remote",
          transport: "sse",
          command: null,
          args: [],
          env: {
            preserveKeys: [],
            replace: {},
          },
          cwd: null,
          url: "https://example.com/mcp",
          headers: {
            preserveKeys: [],
            replace: { Authorization: "Bearer x" },
          },
        })
      )
    );
  });

  it("fills fields from CLI-style JSON (codex.servers)", async () => {
    mockUpsertMutation(createUpsertMutation());
    vi.mocked(mcpParseJson).mockResolvedValueOnce(makeMcpParseResult({ servers: [] }));

    render(
      <McpServerDialog workspaceId={1} open={true} editTarget={null} onOpenChange={vi.fn()} />
    );

    fireEvent.change(screen.getByPlaceholderText(/示例：\{"type":"stdio"/), {
      target: {
        value:
          '{"codex":{"servers":{"fetch":{"command":"uvx","args":["mcp-server-fetch"],"env":{"FOO":"bar"}}}}}',
      },
    });
    fireEvent.click(screen.getByRole("button", { name: "从 JSON 填充" }));

    await waitFor(() => expect(screen.getByDisplayValue("fetch")).toBeInTheDocument());
    expect(screen.getByDisplayValue("uvx")).toBeInTheDocument();
    expect(screen.getByDisplayValue("mcp-server-fetch")).toBeInTheDocument();
    expect(screen.getByDisplayValue("FOO")).toBeInTheDocument();
    expect(screen.getByDisplayValue("bar")).toBeInTheDocument();
  });

  it.each(["url", "serverUrl"])("fills remote MCP fields from %s", async (urlField) => {
    mockUpsertMutation(createUpsertMutation());
    vi.mocked(mcpParseJson).mockResolvedValueOnce(makeMcpParseResult({ servers: [] }));

    render(
      <McpServerDialog workspaceId={1} open={true} editTarget={null} onOpenChange={vi.fn()} />
    );

    fireEvent.change(screen.getByPlaceholderText(/示例：\{"type":"stdio"/), {
      target: {
        value: JSON.stringify([
          {
            name: "ArrHttp",
            [urlField]: "https://example.com/mcp",
            headers: { Authorization: "Bearer x" },
          },
        ]),
      },
    });
    fireEvent.click(screen.getByRole("button", { name: "从 JSON 填充" }));

    await waitFor(() => expect(screen.getByDisplayValue("ArrHttp")).toBeInTheDocument());
    expect(screen.getByDisplayValue("https://example.com/mcp")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Authorization")).toBeInTheDocument();
    expect(screen.getByDisplayValue("Bearer x")).toBeInTheDocument();
  });

  it("fills fields from JSON in create mode", async () => {
    const mutateAsync = vi.fn();
    mockUpsertMutation(createUpsertMutation({ mutateAsync }));

    vi.mocked(mcpParseJson).mockResolvedValue(
      makeMcpParseResult({
        servers: [
          makeMcpImportServer({
            server_key: "fetch",
            name: "Fetch",
            transport: "stdio",
            command: "uvx",
            args: ["mcp-server-fetch"],
            env: { FOO: "bar" },
            cwd: null,
            url: null,
            headers: {},
            enabled: true,
          }),
        ],
      })
    );

    render(
      <McpServerDialog workspaceId={1} open={true} editTarget={null} onOpenChange={vi.fn()} />
    );

    fireEvent.change(screen.getByPlaceholderText(/示例：\{"type":"stdio"/), {
      target: { value: '{"type":"stdio","command":"uvx","args":["mcp-server-fetch"]}' },
    });
    fireEvent.click(screen.getByRole("button", { name: "从 JSON 填充" }));

    await waitFor(() => expect(screen.getByDisplayValue("uvx")).toBeInTheDocument());

    fireEvent.click(screen.getByRole("button", { name: "保存并同步" }));
    await waitFor(() =>
      expect(mutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          name: "Fetch",
          serverId: null,
          serverKey: "",
          transport: "stdio",
          command: "uvx",
          args: ["mcp-server-fetch"],
          env: {
            preserveKeys: [],
            replace: { FOO: "bar" },
          },
          cwd: null,
          url: null,
          headers: {
            preserveKeys: [],
            replace: {},
          },
        })
      )
    );
  });
});
