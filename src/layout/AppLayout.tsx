import { Outlet } from "react-router-dom";
import { AppStartupStatusBanner } from "../components/app/AppStartupStatusBanner";
import { UpdateDialog } from "../components/UpdateDialog";
import { Sidebar } from "../ui/Sidebar";

export function AppLayout() {
  return (
    <div className="h-screen overflow-hidden bg-background text-foreground">
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-card focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:text-foreground focus:shadow-lg focus:ring-2 focus:ring-ring"
      >
        Skip to content
      </a>

      <div className="flex h-full">
        <Sidebar />

        <div className="relative min-w-0 flex-1 flex flex-col overflow-hidden bg-grid-pattern">
          {/* Window drag region for titleBarStyle: overlay */}
          <div data-tauri-drag-region className="absolute inset-x-0 top-0 z-10 h-8" />
          <main id="main-content" className="flex-1 min-h-0 px-8 pb-5 pt-11">
            <AppStartupStatusBanner />
            <Outlet />
          </main>
        </div>
      </div>

      <UpdateDialog />
    </div>
  );
}
