import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { McpServerDialog } from "../McpServerDialog";
import { useMcpServerUpsertMutation } from "../../../../query/mcp";
import { mcpParseJson } from "../../../../services/workspace/mcp";

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

describe("pages/mcp/components/McpServerDialog", () => {
  it("renders saving state and disables submit", () => {
    vi.mocked(useMcpServerUpsertMutation).mockReturnValue({
      isPending: true,
      mutateAsync: vi.fn(),
    } as any);

    render(
      <McpServerDialog workspaceId={1} open={true} editTarget={null} onOpenChange={vi.fn()} />
    );

    expect(screen.getByRole("button", { name: "保存中…" })).toBeDisabled();
  });

  it("validates env and can save stdio servers", async () => {
    const mutateAsync = vi.fn();
    vi.mocked(useMcpServerUpsertMutation).mockReturnValue({ isPending: false, mutateAsync } as any);

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

    // Valid env: mutation runs but returns null => "Tauri only" path.
    mutateAsync.mockResolvedValueOnce(null);
    fireEvent.change(screen.getByPlaceholderText("KEY（例如 TOKEN）"), {
      target: { value: "FOO" },
    });
    fireEvent.change(screen.getByPlaceholderText("VALUE（例如 sk-xxx）"), {
      target: { value: "bar" },
    });
    fireEvent.change(screen.getByPlaceholderText(/-y/), { target: { value: "-y\n@foo/bar" } });
    fireEvent.click(screen.getByRole("button", { name: "保存并同步" }));
    await waitFor(() =>
      expect(mutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          serverId: null,
          name: "Fetch Tool",
          transport: "stdio",
          command: "node",
          args: ["-y", "@foo/bar"],
          env: { FOO: "bar" },
        })
      )
    );

    mutateAsync.mockResolvedValueOnce({ id: 1, server_key: "fetch", transport: "stdio" });
    fireEvent.click(screen.getByRole("button", { name: "保存并同步" }));
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false));
  });

  it("blocks invalid env keys in pair editor", async () => {
    const mutateAsync = vi.fn();
    vi.mocked(useMcpServerUpsertMutation).mockReturnValue({ isPending: false, mutateAsync } as any);

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
    const mutateAsync = vi
      .fn()
      .mockResolvedValueOnce({ id: 1, server_key: "demo", transport: "stdio" });
    vi.mocked(useMcpServerUpsertMutation).mockReturnValue({ isPending: false, mutateAsync } as any);

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
          transport: "stdio",
          command: "node",
          cwd: "/tmp",
          env: { FOO: "bar" },
        })
      )
    );
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false));
  });

  it("prefills and saves http servers with headers parsing", async () => {
    const mutateAsync = vi.fn();
    vi.mocked(useMcpServerUpsertMutation).mockReturnValue({ isPending: false, mutateAsync } as any);

    const onOpenChange = vi.fn();

    render(
      <McpServerDialog
        workspaceId={1}
        open={true}
        editTarget={
          {
            id: 7,
            server_key: "remote",
            name: "Remote",
            transport: "http",
            url: "https://example.com/mcp",
            headers: { Authorization: "Bearer x" },
            enabled: true,
          } as any
        }
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

    mutateAsync.mockResolvedValueOnce({ id: 7, server_key: "remote", transport: "http" });
    fireEvent.change(screen.getByPlaceholderText("Header（例如 Authorization）"), {
      target: { value: "Authorization" },
    });
    fireEvent.change(screen.getByPlaceholderText("Value（例如 Bearer xxx）"), {
      target: { value: "Bearer y" },
    });
    fireEvent.click(screen.getByRole("button", { name: "保存并同步" }));
    await waitFor(() =>
      expect(mutateAsync).toHaveBeenCalledWith(
        expect.objectContaining({
          serverId: 7,
          transport: "http",
          url: "https://example.com/mcp",
          headers: { Authorization: "Bearer y" },
        })
      )
    );
    await waitFor(() => expect(onOpenChange).toHaveBeenCalledWith(false));
  });

  it("fills fields from JSON fallback parser when service returns empty", async () => {
    vi.mocked(useMcpServerUpsertMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(mcpParseJson).mockResolvedValueOnce({ servers: [] } as any);

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

  it("fills fields from mcpServers JSON and normalizes sse transport to http", async () => {
    vi.mocked(useMcpServerUpsertMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(mcpParseJson).mockResolvedValueOnce({ servers: [] } as any);

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
  });

  it("fills fields from CLI-style JSON (codex.servers)", async () => {
    vi.mocked(useMcpServerUpsertMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(mcpParseJson).mockResolvedValueOnce({ servers: [] } as any);

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

  it("fills fields from array JSON and infers http from url", async () => {
    vi.mocked(useMcpServerUpsertMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn(),
    } as any);
    vi.mocked(mcpParseJson).mockResolvedValueOnce({ servers: [] } as any);

    render(
      <McpServerDialog workspaceId={1} open={true} editTarget={null} onOpenChange={vi.fn()} />
    );

    fireEvent.change(screen.getByPlaceholderText(/示例：\{"type":"stdio"/), {
      target: {
        value:
          '[{"name":"ArrHttp","url":"https://example.com/mcp","headers":{"Authorization":"Bearer x"}}]',
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
    vi.mocked(useMcpServerUpsertMutation).mockReturnValue({ isPending: false, mutateAsync } as any);

    vi.mocked(mcpParseJson).mockResolvedValue({
      servers: [
        {
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
        },
      ],
    } as any);

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
          transport: "stdio",
          command: "uvx",
          args: ["mcp-server-fetch"],
          env: { FOO: "bar" },
        })
      )
    );
  });
});
