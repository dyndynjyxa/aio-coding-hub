import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { HomeWorkStatusCard } from "../HomeWorkStatusCard";

describe("components/home/HomeWorkStatusCard", () => {
  it("renders loading and unavailable states", () => {
    render(
      <HomeWorkStatusCard
        cliProxyLoading={true}
        cliProxyAvailable={null}
        cliProxyEnabled={{ claude: true, codex: false, gemini: false } as any}
        cliProxyAppliedToCurrentGateway={{ claude: true, codex: null, gemini: null } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={vi.fn()}
      />
    );
    expect(screen.getByText("加载中…")).toBeInTheDocument();

    render(
      <HomeWorkStatusCard
        cliProxyLoading={false}
        cliProxyAvailable={false}
        cliProxyEnabled={{ claude: true, codex: false, gemini: false } as any}
        cliProxyAppliedToCurrentGateway={{ claude: true, codex: null, gemini: null } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={vi.fn()}
      />
    );
    expect(screen.getByText("数据不可用")).toBeInTheDocument();
  });

  it("drives proxy toggles", () => {
    const onSetCliProxyEnabled = vi.fn();

    render(
      <HomeWorkStatusCard
        cliProxyLoading={false}
        cliProxyAvailable={true}
        cliProxyEnabled={{ claude: true, codex: false, gemini: false } as any}
        cliProxyAppliedToCurrentGateway={{ claude: true, codex: null, gemini: null } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={onSetCliProxyEnabled}
      />
    );

    const switches = screen.getAllByRole("switch");
    fireEvent.click(switches[0]);
    expect(onSetCliProxyEnabled).toHaveBeenCalledWith("claude", false);
  });

  it("supports horizontal layout for the second overview row", () => {
    render(
      <HomeWorkStatusCard
        layout="horizontal"
        cliProxyLoading={false}
        cliProxyAvailable={true}
        cliProxyEnabled={{ claude: true, codex: false, gemini: false } as any}
        cliProxyAppliedToCurrentGateway={{ claude: true, codex: null, gemini: null } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={vi.fn()}
      />
    );

    expect(screen.getByText("代理状态")).toBeInTheDocument();
    expect(screen.getAllByRole("switch").length).toBe(3);
  });

  it("supports plain chrome when embedded into the info panel", () => {
    render(
      <HomeWorkStatusCard
        layout="vertical"
        chrome="plain"
        cliProxyLoading={false}
        cliProxyAvailable={true}
        cliProxyEnabled={{ claude: true, codex: false, gemini: false } as any}
        cliProxyAppliedToCurrentGateway={{ claude: true, codex: null, gemini: null } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={vi.fn()}
      />
    );

    expect(screen.getByText("代理状态")).toBeInTheDocument();
    expect(screen.getAllByRole("switch")).toHaveLength(3);
  });

  it("shows drift warning and repair button for enabled rows not pointing to current gateway", () => {
    const onSetCliProxyEnabled = vi.fn();

    render(
      <HomeWorkStatusCard
        cliProxyLoading={false}
        cliProxyAvailable={true}
        cliProxyEnabled={{ claude: false, codex: true, gemini: false } as any}
        cliProxyAppliedToCurrentGateway={{ claude: null, codex: false, gemini: null } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={onSetCliProxyEnabled}
      />
    );

    expect(screen.getByText("当前未指向本网关")).toBeInTheDocument();
    expect(screen.getAllByRole("switch")).toHaveLength(3);
    fireEvent.click(screen.getByRole("button", { name: "修复 Codex 代理" }));
    expect(onSetCliProxyEnabled).toHaveBeenCalledWith("codex", true);
  });

  it("does not show drift warning before the current gateway origin is known", () => {
    render(
      <HomeWorkStatusCard
        cliProxyLoading={false}
        cliProxyAvailable={true}
        cliProxyEnabled={{ claude: false, codex: true, gemini: false } as any}
        cliProxyAppliedToCurrentGateway={{ claude: null, codex: null, gemini: null } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={vi.fn()}
      />
    );

    expect(screen.queryByText("当前未指向本网关")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "修复 Codex 代理" })).not.toBeInTheDocument();
  });

  it("keeps the switch available for drifted rows so users can still disable proxy", () => {
    const onSetCliProxyEnabled = vi.fn();

    render(
      <HomeWorkStatusCard
        cliProxyLoading={false}
        cliProxyAvailable={true}
        cliProxyEnabled={{ claude: false, codex: true, gemini: false } as any}
        cliProxyAppliedToCurrentGateway={{ claude: null, codex: false, gemini: null } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={onSetCliProxyEnabled}
      />
    );

    const codexSwitch = screen.getByRole("switch", { name: "Codex 代理开关" });
    fireEvent.click(codexSwitch);
    expect(onSetCliProxyEnabled).toHaveBeenCalledWith("codex", false);
  });

  it("disables the repair button while proxy status is toggling", () => {
    render(
      <HomeWorkStatusCard
        cliProxyLoading={false}
        cliProxyAvailable={true}
        cliProxyEnabled={{ claude: false, codex: true, gemini: false } as any}
        cliProxyAppliedToCurrentGateway={{ claude: null, codex: false, gemini: null } as any}
        cliProxyToggling={{ claude: false, codex: true, gemini: false } as any}
        onSetCliProxyEnabled={vi.fn()}
      />
    );

    expect(screen.getByRole("button", { name: "修复 Codex 代理" })).toBeDisabled();
  });
});
