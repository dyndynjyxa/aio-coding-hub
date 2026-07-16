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
const ImageGenPage = lazy(() =>
  import("../pages/ImageGenPage").then((m) => ({ default: m.ImageGenPage }))
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

function LazyPage({ Page }: { Page: ComponentType }) {
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
        <Route path="/image-gen" element={<LazyPage Page={ImageGenPage} />} />
        <Route path="/providers" element={<LazyPage Page={ProvidersPage} />} />
        <Route path="/sessions" element={<LazyPage Page={SessionsPage} />} />
        <Route
          path="/sessions/:source/:projectId"
          element={<LazyPage Page={SessionsProjectPage} />}
        />
        <Route
          path="/sessions/:source/:projectId/session/*"
          element={<LazyPage Page={SessionsMessagesPage} />}
        />
        <Route path="/workspaces" element={<LazyPage Page={WorkspacesPage} />} />
        <Route path="/prompts" element={<LazyPage Page={PromptsPage} />} />
        <Route path="/mcp" element={<LazyPage Page={McpPage} />} />
        <Route path="/plugins" element={<LazyPage Page={PluginsPage} />} />
        <Route path="/logs" element={<LazyPage Page={LogsPage} />} />
        <Route path="/console" element={<LazyPage Page={ConsolePage} />} />
        <Route path="/usage" element={<LazyPage Page={UsagePage} />} />
        <Route path="/settings/*" element={<LazyPage Page={SettingsPage} />} />
        <Route path="/cli-manager" element={<LazyPage Page={CliManagerPage} />} />
        <Route path="/skills" element={<LazyPage Page={SkillsPage} />} />
        <Route path="/skills/market" element={<LazyPage Page={SkillsMarketPage} />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}
