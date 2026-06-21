import { lazy, Suspense } from "react";
import type { ComponentType } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import { AppLayout } from "../layout/AppLayout";
import { HomePage } from "../pages/HomePage";
import { Spinner } from "../ui/Spinner";

const CliManagerPage = lazy(() =>
  import("../pages/CliManagerPage").then((m) => ({ default: m.CliManagerPage }))
);
const ConsolePage = lazy(() =>
  import("../pages/ConsolePage").then((m) => ({ default: m.ConsolePage }))
);
const LogsPage = lazy(() => import("../pages/LogsPage").then((m) => ({ default: m.LogsPage })));
const McpPage = lazy(() => import("../pages/McpPage").then((m) => ({ default: m.McpPage })));
const PluginsPage = lazy(() =>
  import("../pages/PluginsPage").then((m) => ({ default: m.PluginsPage }))
);
const PromptsPage = lazy(() =>
  import("../pages/PromptsPage").then((m) => ({ default: m.PromptsPage }))
);
const ProvidersPage = lazy(() =>
  import("../pages/ProvidersPage").then((m) => ({ default: m.ProvidersPage }))
);
const SessionsPage = lazy(() =>
  import("../pages/SessionsPage").then((m) => ({ default: m.SessionsPage }))
);
const SessionsProjectPage = lazy(() =>
  import("../pages/SessionsProjectPage").then((m) => ({ default: m.SessionsProjectPage }))
);
const SessionsMessagesPage = lazy(() =>
  import("../pages/SessionsMessagesPage").then((m) => ({ default: m.SessionsMessagesPage }))
);
const SettingsPage = lazy(() =>
  import("../pages/SettingsPage").then((m) => ({ default: m.SettingsPage }))
);
const SkillsPage = lazy(() =>
  import("../pages/SkillsPage").then((m) => ({ default: m.SkillsPage }))
);
const SkillsMarketPage = lazy(() =>
  import("../pages/SkillsMarketPage").then((m) => ({ default: m.SkillsMarketPage }))
);
const UsagePage = lazy(() => import("../pages/UsagePage").then((m) => ({ default: m.UsagePage })));
const WorkspacesPage = lazy(() =>
  import("../pages/WorkspacesPage").then((m) => ({ default: m.WorkspacesPage }))
);

function PageLoadingFallback() {
  return (
    <div className="flex h-full items-center justify-center">
      <Spinner />
    </div>
  );
}

function renderLazyPage(Page: ComponentType) {
  return (
    <Suspense fallback={<PageLoadingFallback />}>
      <Page />
    </Suspense>
  );
}

export function AppRoutes() {
  return (
    <Routes>
      <Route element={<AppLayout />}>
        <Route index element={<HomePage />} />
        <Route path="/providers" element={renderLazyPage(ProvidersPage)} />
        <Route path="/sessions" element={renderLazyPage(SessionsPage)} />
        <Route path="/sessions/:source/:projectId" element={renderLazyPage(SessionsProjectPage)} />
        <Route
          path="/sessions/:source/:projectId/session/*"
          element={renderLazyPage(SessionsMessagesPage)}
        />
        <Route path="/workspaces" element={renderLazyPage(WorkspacesPage)} />
        <Route path="/prompts" element={renderLazyPage(PromptsPage)} />
        <Route path="/mcp" element={renderLazyPage(McpPage)} />
        <Route path="/plugins" element={renderLazyPage(PluginsPage)} />
        <Route path="/logs" element={renderLazyPage(LogsPage)} />
        <Route path="/console" element={renderLazyPage(ConsolePage)} />
        <Route path="/usage" element={renderLazyPage(UsagePage)} />
        <Route path="/settings/*" element={renderLazyPage(SettingsPage)} />
        <Route path="/cli-manager" element={renderLazyPage(CliManagerPage)} />
        <Route path="/skills" element={renderLazyPage(SkillsPage)} />
        <Route path="/skills/market" element={renderLazyPage(SkillsMarketPage)} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}
