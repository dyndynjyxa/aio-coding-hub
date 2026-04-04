import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SettingsAboutCard } from "../SettingsAboutCard";

describe("pages/settings/SettingsAboutCard", () => {
  it("renders placeholder when about is null", () => {
    render(<SettingsAboutCard about={null} checkingUpdate={false} checkUpdate={vi.fn()} />);
    expect(screen.getByText("关于应用")).toBeInTheDocument();
    expect(screen.getByText("加载中…")).toBeInTheDocument();
  });

  it("renders about information when available", () => {
    const checkUpdate = vi.fn().mockResolvedValue(undefined);

    render(
      <SettingsAboutCard
        about={{
          os: "mac",
          arch: "arm64",
          profile: "dev",
          app_version: "0.0.0",
          bundle_type: null,
          run_mode: "desktop",
        }}
        checkingUpdate={false}
        checkUpdate={checkUpdate}
      />
    );

    expect(screen.getByText("版本")).toBeInTheDocument();
    expect(screen.getByText("0.0.0")).toBeInTheDocument();
    expect(screen.getByText("平台")).toBeInTheDocument();
    expect(screen.getByText("mac/arm64")).toBeInTheDocument();
    expect(screen.getByText("Bundle")).toBeInTheDocument();
    expect(screen.getByText("—")).toBeInTheDocument();
    expect(screen.getByText("运行模式")).toBeInTheDocument();
    expect(screen.getByText("desktop")).toBeInTheDocument();
    expect(screen.getByText("检查更新")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "检查" }));
    expect(checkUpdate).toHaveBeenCalledTimes(1);
  });

  it("renders portable action and checking state", () => {
    const checkUpdate = vi.fn().mockResolvedValue(undefined);
    const view = render(
      <SettingsAboutCard
        about={{
          os: "mac",
          arch: "arm64",
          profile: "dev",
          app_version: "0.0.0",
          bundle_type: null,
          run_mode: "portable",
        }}
        checkingUpdate={false}
        checkUpdate={checkUpdate}
      />
    );

    expect(screen.getByText("获取新版本")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "打开" }));
    expect(checkUpdate).toHaveBeenCalledTimes(1);

    view.rerender(
      <SettingsAboutCard
        about={{
          os: "mac",
          arch: "arm64",
          profile: "dev",
          app_version: "0.0.0",
          bundle_type: null,
          run_mode: "desktop",
        }}
        checkingUpdate
        checkUpdate={checkUpdate}
      />
    );

    expect(screen.getByRole("button", { name: "检查中…" })).toBeDisabled();
  });
});
