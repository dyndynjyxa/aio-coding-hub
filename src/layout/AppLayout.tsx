import { Outlet } from "react-router-dom";
import { AppStartupStatusBanner } from "../components/app/AppStartupStatusBanner";
import { UpdateDialog } from "../components/UpdateDialog";
import { useResponsive } from "../hooks/useMediaQuery";
import { useSidebarState } from "../hooks/useSidebarState";
import { MobileHeader, MobileNav } from "../ui/MobileNav";
import { Sidebar } from "../ui/Sidebar";

export function AppLayout() {
  const { isDesktop } = useResponsive();
  const sidebar = useSidebarState();

  return (
    <div className="h-screen overflow-hidden bg-slate-50 text-slate-900 dark:bg-slate-950 dark:text-slate-100">
      <a
        href="#main-content"
        className="sr-only focus:not-sr-only focus:fixed focus:left-4 focus:top-4 focus:z-50 focus:rounded-md focus:bg-white focus:px-4 focus:py-2 focus:text-sm focus:font-medium focus:text-slate-900 focus:shadow-lg focus:ring-2 focus:ring-slate-400 dark:focus:bg-slate-800 dark:focus:text-slate-100"
      >
        Skip to content
      </a>
      {/* Mobile header - only shown on small screens */}
      {!isDesktop && <MobileHeader onMenuClick={sidebar.toggleMobileDrawer} />}

      <div className="flex h-full">
        {/* Desktop sidebar - hidden on mobile via CSS */}
        <div className="hidden lg:block">
          <Sidebar isOpen={sidebar.isOpen} />
        </div>

        {/* Main content area */}
        <div className="relative min-w-0 flex-1 flex flex-col overflow-hidden bg-grid-pattern">
          {/* Window drag region for titleBarStyle: overlay */}
          <div data-tauri-drag-region className="absolute inset-x-0 top-0 z-10 h-8" />
          <main
            id="main-content"
            className="flex-1 min-h-0 px-4 pb-4 pt-10 sm:px-6 sm:pb-5 sm:pt-11"
          >
            <AppStartupStatusBanner />
            <Outlet />
          </main>
        </div>
      </div>

      {/* Mobile navigation drawer */}
      <MobileNav isOpen={sidebar.isMobileDrawerOpen} onClose={sidebar.closeMobileDrawer} />

      <UpdateDialog />
    </div>
  );
}
