import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { HomeWorkStatusCard } from "../HomeWorkStatusCard";

const baseProxyProps = {
  cliProxyLoading: false,
  cliProxyAvailable: true,
  cliProxyEnabled: { claude: true, codex: false, gemini: false } as any,
  cliProxyAppliedToCurrentGateway: { claude: true, codex: null, gemini: null } as any,
  cliProxyToggling: { claude: false, codex: false, gemini: false } as any,
  onSetCliProxyEnabled: vi.fn(),
};

const baseRouteStrategyProps = {
  sortModes: [{ id: 1, name: "工作策略", created_at: 1, updated_at: 1 }],
  sortModesLoading: false,
  sortModesAvailable: true,
  activeModeByCli: { claude: 1, codex: null, gemini: null } as any,
  activeModeToggling: { claude: false, codex: false, gemini: false } as any,
  onSetCliActiveMode: vi.fn(),
};

describe("components/home/HomeWorkStatusCard", () => {
  it("renders loading and unavailable states", () => {
    render(
      <HomeWorkStatusCard {...baseProxyProps} cliProxyLoading={true} cliProxyAvailable={null} />
    );
    expect(screen.getByText("加载中…")).toBeInTheDocument();

    render(
      <HomeWorkStatusCard {...baseProxyProps} cliProxyLoading={false} cliProxyAvailable={false} />
    );
    expect(screen.getByText("数据不可用")).toBeInTheDocument();
  });

  it("drives proxy toggles", () => {
    const onSetCliProxyEnabled = vi.fn();

    render(<HomeWorkStatusCard {...baseProxyProps} onSetCliProxyEnabled={onSetCliProxyEnabled} />);

    const switches = screen.getAllByRole("switch");
    fireEvent.click(switches[0]);
    expect(onSetCliProxyEnabled).toHaveBeenCalledWith("claude", false);
  });

  it("supports horizontal layout for the second overview row", () => {
    render(<HomeWorkStatusCard {...baseProxyProps} layout="horizontal" />);

    expect(screen.getByText("代理状态")).toBeInTheDocument();
    expect(screen.getAllByRole("switch")).toHaveLength(3);
    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
  });

  it("supports plain chrome when embedded into the info panel", () => {
    render(<HomeWorkStatusCard {...baseProxyProps} layout="vertical" chrome="plain" />);

    expect(screen.getByText("代理状态")).toBeInTheDocument();
    expect(screen.getAllByRole("switch")).toHaveLength(3);
    expect(screen.queryByRole("combobox")).not.toBeInTheDocument();
  });

  it("renders route strategy selectors in the vertical plain card and forwards changes", () => {
    const onSetCliActiveMode = vi.fn();

    render(
      <HomeWorkStatusCard
        {...baseProxyProps}
        layout="vertical"
        chrome="plain"
        {...baseRouteStrategyProps}
        onSetCliActiveMode={onSetCliActiveMode}
      />
    );

    expect(screen.getByRole("combobox", { name: "Claude Code 路由策略" })).toHaveValue("1");
    expect(screen.getByRole("combobox", { name: "Codex 路由策略" })).toHaveValue("");
    expect(screen.getByRole("combobox", { name: "Gemini 路由策略" })).toHaveValue("");

    fireEvent.change(screen.getByRole("combobox", { name: "Codex 路由策略" }), {
      target: { value: "1" },
    });
    expect(onSetCliActiveMode).toHaveBeenCalledWith("codex", 1);
  });

  it("disables route strategy selectors while loading or unavailable", () => {
    const { rerender } = render(
      <HomeWorkStatusCard
        {...baseProxyProps}
        layout="vertical"
        chrome="plain"
        {...baseRouteStrategyProps}
        sortModesLoading={true}
      />
    );

    expect(screen.getByRole("combobox", { name: "Claude Code 路由策略" })).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "Codex 路由策略" })).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "Gemini 路由策略" })).toBeDisabled();

    rerender(
      <HomeWorkStatusCard
        {...baseProxyProps}
        layout="vertical"
        chrome="plain"
        {...baseRouteStrategyProps}
        sortModesAvailable={false}
      />
    );

    expect(screen.getByRole("combobox", { name: "Claude Code 路由策略" })).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "Codex 路由策略" })).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "Gemini 路由策略" })).toBeDisabled();
  });

  it("shows drift warning and repair button for enabled rows not pointing to current gateway", () => {
    const onSetCliProxyEnabled = vi.fn();

    render(
      <HomeWorkStatusCard
        {...baseProxyProps}
        cliProxyEnabled={{ claude: false, codex: true, gemini: false } as any}
        cliProxyAppliedToCurrentGateway={{ claude: null, codex: false, gemini: null } as any}
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
        {...baseProxyProps}
        cliProxyEnabled={{ claude: false, codex: true, gemini: false } as any}
        cliProxyAppliedToCurrentGateway={{ claude: null, codex: null, gemini: null } as any}
      />
    );

    expect(screen.queryByText("当前未指向本网关")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "修复 Codex 代理" })).not.toBeInTheDocument();
  });

  it("keeps the switch available for drifted rows so users can still disable proxy", () => {
    const onSetCliProxyEnabled = vi.fn();

    render(
      <HomeWorkStatusCard
        {...baseProxyProps}
        cliProxyEnabled={{ claude: false, codex: true, gemini: false } as any}
        cliProxyAppliedToCurrentGateway={{ claude: null, codex: false, gemini: null } as any}
        onSetCliProxyEnabled={onSetCliProxyEnabled}
      />
    );

    const codexSwitch = screen.getByRole("switch", { name: "Codex 代理开关" });
    fireEvent.click(codexSwitch);
    expect(onSetCliProxyEnabled).toHaveBeenCalledWith("codex", false);
  });

  it("keeps route strategy visible for drifted rows and disables only the toggling cli", () => {
    render(
      <HomeWorkStatusCard
        {...baseProxyProps}
        layout="vertical"
        chrome="plain"
        {...baseRouteStrategyProps}
        cliProxyEnabled={{ claude: false, codex: true, gemini: false } as any}
        cliProxyAppliedToCurrentGateway={{ claude: null, codex: false, gemini: null } as any}
        cliProxyToggling={{ claude: false, codex: true, gemini: false } as any}
        activeModeToggling={{ claude: false, codex: true, gemini: false } as any}
      />
    );

    expect(screen.getByText("当前未指向本网关")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "修复 Codex 代理" })).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "Codex 路由策略" })).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "Claude Code 路由策略" })).not.toBeDisabled();
  });
});
