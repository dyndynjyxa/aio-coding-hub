import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { useWslHostAddressQuery } from "../../../query/wsl";
import { gatewayStart, gatewayStop } from "../../../services/gateway/gateway";
import { NetworkSettingsCard } from "../NetworkSettingsCard";

let gatewayMetaMock: any = { gatewayAvailable: "available", gateway: null, preferredPort: 37123 };

vi.mock("sonner", () => ({ toast: vi.fn() }));

vi.mock("../../../hooks/useGatewayMeta", () => ({
  useGatewayMeta: () => gatewayMetaMock,
}));

vi.mock("../../../services/gateway/gateway", async () => {
  const actual = await vi.importActual<typeof import("../../../services/gateway/gateway")>(
    "../../../services/gateway/gateway"
  );
  return { ...actual, gatewayStart: vi.fn(), gatewayStop: vi.fn() };
});

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

  it("restarts gateway when changing listen mode and handles restart results", async () => {
    vi.mocked(useWslHostAddressQuery).mockReturnValue({ data: "172.20.0.1" } as any);
    gatewayMetaMock = {
      gatewayAvailable: "available",
      gateway: { running: true, listen_addr: null },
      preferredPort: 37123,
    };

    vi.mocked(gatewayStop).mockResolvedValue({ running: false } as any);
    vi.mocked(gatewayStart).mockResolvedValue({ running: true, port: 40001 } as any);

    const settings = {
      preferred_port: 40000,
      gateway_listen_mode: "localhost",
      gateway_custom_listen_address: "",
    } as any;
    const onPersistSettings = vi.fn(async (patch: any) => ({ ...settings, ...patch }));

    render(
      <NetworkSettingsCard
        available={true}
        saving={false}
        settings={settings}
        onPersistSettings={onPersistSettings}
      />
    );

    // Switch to LAN triggers restart and port-in-use toast branch.
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "lan" } });
    await waitFor(() => expect(gatewayStop).toHaveBeenCalled());
    await waitFor(() => expect(gatewayStart).toHaveBeenCalledWith(40000));
    expect(toast).toHaveBeenCalledWith("端口被占用，已切换到 40001");

    vi.mocked(gatewayStop).mockClear();
    vi.mocked(gatewayStart).mockClear();
    vi.mocked(toast).mockClear();

    vi.mocked(gatewayStop).mockResolvedValue({ running: false } as any);
    vi.mocked(gatewayStart).mockResolvedValue({ running: true, port: 40000 } as any);

    // Switch to WSL auto triggers restart and "网关已重启" branch.
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "wsl_auto" } });
    await waitFor(() => expect(gatewayStart).toHaveBeenCalledWith(40000));
    expect(toast).toHaveBeenCalledWith("网关已重启");
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
