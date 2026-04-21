import React from "react";
import ReactDOM from "react-dom/client";
import type { Root } from "react-dom/client";
import { QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import { AppErrorBoundary } from "./components/AppErrorBoundary";
import "@mdxeditor/editor/style.css";
import "./styles/globals.css";
import { queryClient } from "./query/queryClient";
import { installGlobalErrorReporting } from "./services/frontendErrorReporter";

export function renderApp(rootElement: HTMLElement): Root {
  const root = ReactDOM.createRoot(rootElement);
  root.render(
    <React.StrictMode>
      <QueryClientProvider client={queryClient}>
        <AppErrorBoundary>
          <App />
        </AppErrorBoundary>
      </QueryClientProvider>
    </React.StrictMode>
  );
  return root;
}

installGlobalErrorReporting();

export const appRoot = renderApp(document.getElementById("root") as HTMLElement);
