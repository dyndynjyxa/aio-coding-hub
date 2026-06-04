import { Outlet, useLocation } from "react-router-dom";
import { AppStartupStatusBanner } from "../components/app/AppStartupStatusBanner";
import { UpdateDialog } from "../components/UpdateDialog";
import { Sidebar } from "../ui/Sidebar";
import { cn } from "../utils/cn";

export function AppLayout() {
  const location = useLocation();

  // Map route pathnames to corresponding page grid themes
  const getRouteTheme = (pathname: string): string => {
    if (pathname === "/") return "theme-blue";
    if (pathname.startsWith("/providers")) return "theme-cyan";
    if (pathname.startsWith("/sessions")) return "theme-violet";
    if (pathname.startsWith("/workspaces")) return "theme-emerald";
    if (pathname.startsWith("/prompts")) return "theme-amber";
    if (pathname.startsWith("/mcp")) return "theme-indigo";
    if (pathname.startsWith("/skills")) return "theme-pink";
    if (pathname.startsWith("/usage")) return "theme-orange";
    if (pathname.startsWith("/logs")) return "theme-slate";
    if (pathname.startsWith("/cli-manager")) return "theme-sky";
    if (pathname.startsWith("/console")) return "theme-rose";
    if (pathname.startsWith("/settings")) return "theme-slate";
    return "theme-blue";
  };

  const themeClass = getRouteTheme(location.pathname);

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

        <div
          className={cn(
            "relative min-w-0 flex-1 flex flex-col overflow-hidden bg-grid-pattern",
            themeClass
          )}
        >
          {/* Window drag region for titleBarStyle: overlay (aligned with Sidebar top safe area) */}
          <div data-tauri-drag-region className="absolute inset-x-0 top-0 z-10 h-8" />
          <main id="main-content" className="flex-1 min-h-0 px-8 py-5">
            <AppStartupStatusBanner />
            <Outlet />
          </main>
        </div>
      </div>

      <UpdateDialog />
    </div>
  );
}
