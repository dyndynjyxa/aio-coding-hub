import { fireEvent, render, screen } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import { SettingsDialogs } from "../SettingsDialogs";

vi.mock("../../../components/settings/ModelPriceAliasesDialog", () => ({
  ModelPriceAliasesDialog: () => <div>aliases-dialog</div>,
}));

function wrapper({ children }: { children: React.ReactNode }) {
  return <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>;
}

function createDialogsProps(overrides: Partial<React.ComponentProps<typeof SettingsDialogs>> = {}) {
  return {
    modelPriceAliases: {
      open: false,
      setOpen: vi.fn(),
    },
    clearRequestLogs: {
      open: false,
      setOpen: vi.fn(),
      pending: false,
      confirm: vi.fn().mockResolvedValue(undefined),
    },
    resetAll: {
      open: false,
      setOpen: vi.fn(),
      pending: false,
      confirm: vi.fn().mockResolvedValue(undefined),
    },
    configImport: {
      open: false,
      setOpen: vi.fn(),
      pending: false,
      confirm: vi.fn().mockResolvedValue(undefined),
      pendingFilePath: null,
    },
    ...overrides,
  };
}

describe("pages/settings/SettingsDialogs", () => {
  it("prevents closing clear request logs dialog while in progress", () => {
    const setClearOpen = vi.fn();

    render(
      <SettingsDialogs
        {...createDialogsProps({
          clearRequestLogs: {
            open: true,
            setOpen: setClearOpen,
            pending: true,
            confirm: vi.fn().mockResolvedValue(undefined),
          },
        })}
      />
    );

    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(setClearOpen).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "取消" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "清理中…" })).toBeDisabled();
  });

  it("closes clear request logs dialog and resets pending flag when dismissed", () => {
    const setClearOpen = vi.fn();

    render(
      <SettingsDialogs
        {...createDialogsProps({
          clearRequestLogs: {
            open: true,
            setOpen: setClearOpen,
            pending: false,
            confirm: vi.fn().mockResolvedValue(undefined),
          },
        })}
      />
    );

    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });

    expect(setClearOpen).toHaveBeenCalledWith(false);
  });

  it("prevents closing reset all dialog while in progress", () => {
    const setResetOpen = vi.fn();

    render(
      <SettingsDialogs
        {...createDialogsProps({
          resetAll: {
            open: true,
            setOpen: setResetOpen,
            pending: true,
            confirm: vi.fn().mockResolvedValue(undefined),
          },
        })}
      />
    );

    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(setResetOpen).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "取消" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "清理中…" })).toBeDisabled();
  });

  it("closes reset all dialog and resets pending flag when dismissed", () => {
    const setResetOpen = vi.fn();

    render(
      <SettingsDialogs
        {...createDialogsProps({
          resetAll: {
            open: true,
            setOpen: setResetOpen,
            pending: false,
            confirm: vi.fn().mockResolvedValue(undefined),
          },
        })}
      />
    );

    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });

    expect(setResetOpen).toHaveBeenCalledWith(false);
  });

  it("renders config import confirmation path and warnings", () => {
    render(
      <SettingsDialogs
        {...createDialogsProps({
          configImport: {
            open: true,
            setOpen: vi.fn(),
            pending: false,
            confirm: vi.fn().mockResolvedValue(undefined),
            pendingFilePath: "/tmp/aio-config.json",
          },
        })}
      />,
      { wrapper }
    );

    expect(screen.getByText("确认导入配置")).toBeInTheDocument();
    expect(screen.getByText(/API Key 等敏感信息/)).toBeInTheDocument();
    expect(screen.getByText(/导入将覆盖当前所有配置/)).toBeInTheDocument();
    expect(screen.getByText("/tmp/aio-config.json")).toBeInTheDocument();
  });
});
