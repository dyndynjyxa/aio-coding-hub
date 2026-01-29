import { useEffect } from "react";
import type { CSSProperties } from "react";
import { Toaster } from "sonner";
import { HashRouter, Navigate, Route, Routes } from "react-router-dom";
import { AppLayout } from "./layout/AppLayout";
import { CliManagerPage } from "./pages/CliManagerPage";
import { ConsolePage } from "./pages/ConsolePage";
import { HomePage } from "./pages/HomePage";
import { LogsPage } from "./pages/LogsPage";
import { McpPage } from "./pages/McpPage";
import { PromptsPage } from "./pages/PromptsPage";
import { ProvidersPage } from "./pages/ProvidersPage";
import { SettingsPage } from "./pages/SettingsPage";
import { SkillsPage } from "./pages/SkillsPage";
import { SkillsMarketPage } from "./pages/SkillsMarketPage";
import { UsagePage } from "./pages/UsagePage";
import { listenGatewayEvents } from "./services/gatewayEvents";
import { listenNoticeEvents } from "./services/noticeEvents";
import {
  startupSyncDefaultPromptsFromFilesOncePerSession,
  startupSyncModelPricesOnce,
} from "./services/startup";

type CssVarsStyle = CSSProperties & Record<`--toast-${string}`, string | number>;

const TOASTER_STYLE: CssVarsStyle = {
  "--toast-close-button-start": "unset",
  "--toast-close-button-end": "0",
  "--toast-close-button-transform": "translate(35%, -35%)",
};

export default function App() {
  useEffect(() => {
    let cancelled = false;
    let cleanup: (() => void) | null = null;

    listenGatewayEvents()
      .then((unlisten) => {
        if (cancelled) {
          unlisten();
          return;
        }
        cleanup = unlisten;
      })
      .catch(() => {});

    return () => {
      cancelled = true;
      cleanup?.();
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    let cleanup: (() => void) | null = null;

    listenNoticeEvents()
      .then((unlisten) => {
        if (cancelled) {
          unlisten();
          return;
        }
        cleanup = unlisten;
      })
      .catch(() => {});

    return () => {
      cancelled = true;
      cleanup?.();
    };
  }, []);

  useEffect(() => {
    startupSyncModelPricesOnce().catch(() => {});
  }, []);

  useEffect(() => {
    startupSyncDefaultPromptsFromFilesOncePerSession().catch(() => {});
  }, []);

  return (
    <>
      <Toaster richColors closeButton position="top-center" style={TOASTER_STYLE} />
      <HashRouter>
        <Routes>
          <Route element={<AppLayout />}>
            <Route index element={<HomePage />} />
            <Route path="/providers" element={<ProvidersPage />} />
            <Route path="/prompts" element={<PromptsPage />} />
            <Route path="/mcp" element={<McpPage />} />
            <Route path="/skills" element={<SkillsPage />} />
            <Route path="/skills/market" element={<SkillsMarketPage />} />
            <Route path="/usage" element={<UsagePage />} />
            <Route path="/console" element={<ConsolePage />} />
            <Route path="/logs" element={<LogsPage />} />
            <Route path="/cli-manager" element={<CliManagerPage />} />
            <Route path="/settings" element={<SettingsPage />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Route>
        </Routes>
      </HashRouter>
    </>
  );
}
