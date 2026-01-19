// Usage: Manage MCP servers per CLI (renders sub-view under `src/pages/mcp/*`). Backend commands: `mcp_servers_list`, `mcp_server_*`.

import { McpServersView } from "./mcp/McpServersView";

export function McpPage() {
  return (
    <div className="space-y-3">
      <h1 className="text-2xl font-semibold tracking-tight">MCP</h1>
      <McpServersView />
    </div>
  );
}
