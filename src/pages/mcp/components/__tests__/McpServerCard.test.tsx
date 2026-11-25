import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { McpServerSummary } from "../../../../services/workspace/mcp";
import { McpServerCard } from "../McpServerCard";

function createServer(overrides: Partial<McpServerSummary> = {}): McpServerSummary {
  return {
    id: 1,
    server_key: "fetch",
    name: "Fetch Tool",
    transport: "http",
    url: "https://example.com/mcp",
    enabled: false,
    command: null,
    args: [],
    env_keys: [],
    cwd: null,
    header_keys: [],
    created_at: 1,
    updated_at: 1,
    ...overrides,
  };
}

describe("pages/mcp/components/McpServerCard", () => {
  it("renders http server details and fires toggle/edit/delete callbacks", () => {
    const server = createServer();
    const onToggleEnabled = vi.fn();
    const onEdit = vi.fn();
    const onDelete = vi.fn();

    render(
      <McpServerCard
        server={server}
        toggling={false}
        onToggleEnabled={onToggleEnabled}
        onEdit={onEdit}
        onDelete={onDelete}
      />
    );

    expect(screen.getByText("Fetch Tool")).toBeInTheDocument();
    expect(screen.getByText("http")).toBeInTheDocument();
    expect(screen.getByText("https://example.com/mcp")).toBeInTheDocument();
    expect(screen.getByText("未启用")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("switch"));
    expect(onToggleEnabled).toHaveBeenCalledWith(server);

    fireEvent.click(screen.getByTitle("编辑"));
    expect(onEdit).toHaveBeenCalledWith(server);

    fireEvent.click(screen.getByTitle("删除"));
    expect(onDelete).toHaveBeenCalledWith(server);
  });

  it.each([true, false])("lets agy users disable but not enable SSE (enabled=%s)", (enabled) => {
    const server = createServer({ transport: "sse", enabled });
    const onToggleEnabled = vi.fn();
    render(
      <McpServerCard
        server={server}
        unsupportedTransport
        toggling={false}
        onToggleEnabled={onToggleEnabled}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
      />
    );
    expect(screen.getByText("Antigravity 仅支持 stdio 和 Streamable HTTP")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("switch"));
    expect(onToggleEnabled).toHaveBeenCalledTimes(enabled ? 1 : 0);
  });

  it("falls back to placeholder text when http url is missing", () => {
    render(
      <McpServerCard
        server={createServer({ url: "" })}
        toggling={false}
        onToggleEnabled={vi.fn()}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
      />
    );

    expect(screen.getByText("（未填写 url）")).toBeInTheDocument();
  });

  it("renders stdio placeholder text and blocks toggling while pending", () => {
    const server = createServer({
      id: 2,
      name: "Local Tool",
      transport: "stdio",
      command: null,
      enabled: true,
    });
    const onToggleEnabled = vi.fn();

    render(
      <McpServerCard
        server={server}
        toggling
        onToggleEnabled={onToggleEnabled}
        onEdit={vi.fn()}
        onDelete={vi.fn()}
      />
    );

    expect(screen.getByText("stdio")).toBeInTheDocument();
    expect(screen.getByText("（未填写 command）")).toBeInTheDocument();
    expect(screen.getByText("已启用")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("switch"));
    expect(onToggleEnabled).not.toHaveBeenCalled();
  });
});
