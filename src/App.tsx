import type { CSSProperties } from "react";
import { Toaster } from "sonner";
import { HashRouter } from "react-router-dom";
import { AppRoutes } from "./app/AppRoutes";
import { useAppBootstrap } from "./app/useAppBootstrap";
import { useGlobalFileDropGuard } from "./app/useGlobalFileDropGuard";

type CssVarsStyle = CSSProperties & Record<`--toast-${string}`, string | number>;

const TOASTER_STYLE: CssVarsStyle = {
  "--toast-close-button-start": "unset",
  "--toast-close-button-end": "0",
  "--toast-close-button-transform": "translate(35%, -35%)",
};

export default function App() {
  useAppBootstrap();
  useGlobalFileDropGuard();

  return (
    <>
      <Toaster richColors closeButton position="top-center" style={TOASTER_STYLE} />
      <HashRouter>
        <AppRoutes />
      </HashRouter>
    </>
  );
}
