// Usage: Manage MCP servers per CLI (renders sub-view under `src/pages/mcp/*`). Backend commands: `mcp_servers_list`, `mcp_server_*`.

import { PageHeader } from "../ui/PageHeader";
import { McpServersView } from "./mcp/McpServersView";

export function McpPage() {
  return (
    <div className="space-y-6">
      <PageHeader title="MCP" />
      <McpServersView />
    </div>
  );
}
