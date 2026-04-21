import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { useWslHostAddressQuery } from "../../../query/wsl";
import { NetworkSettingsCard } from "../NetworkSettingsCard";

let gatewayMetaMock: any = { gatewayAvailable: "available", gateway: null, preferredPort: 37123 };

vi.mock("sonner", () => ({ toast: vi.fn() }));

vi.mock("../../../hooks/useGatewayMeta", () => ({
  useGatewayMeta: () => gatewayMetaMock,
}));

vi.mock("../../../query/wsl", async () => {
  const actual = await vi.importActual<typeof import("../../../query/wsl")>("../../../query/wsl");
  return { ...actual, useWslHostAddressQuery: vi.fn() };
});

describe("components/cli-manager/NetworkSettingsCard", () => {
  it("switches listen mode and validates custom address", async () => {
    vi.mocked(useWslHostAddressQuery).mockReturnValue({ data: "172.20.0.1" } as any);

    gatewayMetaMock = { gatewayAvailable: "available", gateway: null, preferredPort: 37123 };

    const settings = {
      preferred_port: 37123,
      gateway_listen_mode: "custom",
      gateway_custom_listen_address: "0.0.0.0:37123",
    } as any;

    const onPersistSettings = vi.fn(async () => settings);

    render(
      <NetworkSettingsCard
        available={true}
        saving={false}
        settings={settings}
        onPersistSettings={onPersistSettings}
      />
    );

    expect(screen.getByText("网络设置")).toBeInTheDocument();

    // Switch to WSL auto mode -> should use host IP.
    const modeSelect = screen.getByRole("combobox");
    fireEvent.change(modeSelect, { target: { value: "wsl_auto" } });
    await waitFor(() => {
      expect(onPersistSettings).toHaveBeenCalledWith({ gateway_listen_mode: "wsl_auto" });
    });
    expect(screen.getByText("172.20.0.1:37123")).toBeInTheDocument();

    // Switch back to custom and enter an invalid address -> input resets on blur.
    fireEvent.change(modeSelect, { target: { value: "custom" } });
    const input = screen.getByPlaceholderText("0.0.0.0 或 0.0.0.0:37123");
    fireEvent.change(input, { target: { value: "http://bad" } });
    fireEvent.blur(input);

    await waitFor(() => {
      expect((input as HTMLInputElement).value).toBe("0.0.0.0:37123");
    });
  });

  it("prefers live gateway listen_addr when running", () => {
    vi.mocked(useWslHostAddressQuery).mockReturnValue({ data: null } as any);
    gatewayMetaMock = {
      gatewayAvailable: "available",
      gateway: { running: true, listen_addr: "1.2.3.4:9999" },
      preferredPort: 37123,
    };

    render(
      <NetworkSettingsCard
        available={true}
        saving={false}
        settings={
          {
            preferred_port: 37123,
            gateway_listen_mode: "localhost",
            gateway_custom_listen_address: "",
          } as any
        }
        onPersistSettings={vi.fn(async () => null)}
      />
    );

    expect(screen.getByText("1.2.3.4:9999")).toBeInTheDocument();
  });

  it("persists listen mode changes and reverts local state when save fails", async () => {
    vi.mocked(useWslHostAddressQuery).mockReturnValue({ data: "172.20.0.1" } as any);
    gatewayMetaMock = {
      gatewayAvailable: "available",
      gateway: { running: true, listen_addr: null },
      preferredPort: 37123,
    };

    const settings = {
      preferred_port: 40000,
      gateway_listen_mode: "localhost",
      gateway_custom_listen_address: "",
    } as any;
    const onPersistSettings = vi
      .fn()
      .mockResolvedValueOnce({ ...settings, gateway_listen_mode: "lan" })
      .mockRejectedValueOnce(new Error("save boom"));

    render(
      <NetworkSettingsCard
        available={true}
        saving={false}
        settings={settings}
        onPersistSettings={onPersistSettings}
      />
    );

    fireEvent.change(screen.getByRole("combobox"), { target: { value: "lan" } });
    await waitFor(() =>
      expect(onPersistSettings).toHaveBeenCalledWith({ gateway_listen_mode: "lan" })
    );
    expect(toast).toHaveBeenCalledWith("监听模式已保存");
    expect(screen.getByText("0.0.0.0:40000")).toBeInTheDocument();

    vi.mocked(toast).mockClear();
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "wsl_auto" } });
    await waitFor(() =>
      expect(onPersistSettings).toHaveBeenCalledWith({ gateway_listen_mode: "wsl_auto" })
    );
    await waitFor(() => expect(toast).toHaveBeenCalledWith("更新监听模式失败：请稍后重试"));
    await waitFor(() =>
      expect((screen.getByRole("combobox") as HTMLSelectElement).value).toBe("localhost")
    );
  });

  it("validates IPv6 custom address and handles non-tauri persist failure", async () => {
    vi.mocked(useWslHostAddressQuery).mockReturnValue({ data: null } as any);
    gatewayMetaMock = { gatewayAvailable: "available", gateway: null, preferredPort: 37123 };

    const settings = {
      preferred_port: 37123,
      gateway_listen_mode: "custom",
      gateway_custom_listen_address: "0.0.0.0:37123",
    } as any;

    const onPersistSettings = vi.fn(async () => null);

    render(
      <NetworkSettingsCard
        available={true}
        saving={false}
        settings={settings}
        onPersistSettings={onPersistSettings}
      />
    );

    const input = screen.getByPlaceholderText("0.0.0.0 或 0.0.0.0:37123");
    fireEvent.change(input, { target: { value: "[::1]" } });
    fireEvent.blur(input);

    await waitFor(() => expect(onPersistSettings).toHaveBeenCalled());
    await waitFor(() => expect((input as HTMLInputElement).value).toBe("0.0.0.0:37123"));

    vi.mocked(toast).mockClear();
    fireEvent.change(input, { target: { value: "0.0.0.0:80" } });
    fireEvent.blur(input);
    await waitFor(() => expect(toast).toHaveBeenCalledWith("端口必须 >= 1024"));
  });
});
