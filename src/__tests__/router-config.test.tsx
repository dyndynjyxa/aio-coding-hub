import { render, screen } from "@testing-library/react";
import { QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import { createTestQueryClient } from "../test/utils/reactQuery";

const hashRouterPropsRef = vi.hoisted(() => ({
  current: null as any,
}));

vi.mock("react-router-dom", async () => {
  const actual = await vi.importActual<typeof import("react-router-dom")>("react-router-dom");
  return {
    ...actual,
    HashRouter: (props: any) => {
      hashRouterPropsRef.current = props;
      return <actual.HashRouter {...props} />;
    },
  };
});

vi.mock("../app/useAppBootstrap", () => ({
  useAppBootstrap: vi.fn(),
}));

vi.mock("../app/AppRoutes", () => ({
  AppRoutes: () => <div data-testid="app-routes">mock routes</div>,
}));

describe("router config", () => {
  it("renders app routes inside the hash router", async () => {
    const { default: App } = await import("../App");

    const client = createTestQueryClient();
    render(
      <QueryClientProvider client={client}>
        <App />
      </QueryClientProvider>
    );

    expect(hashRouterPropsRef.current).toBeTruthy();
    expect(screen.getByTestId("app-routes")).toBeInTheDocument();
  }, 30000);
});
