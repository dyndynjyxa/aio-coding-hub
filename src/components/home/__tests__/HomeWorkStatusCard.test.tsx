import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { HomeWorkStatusCard } from "../HomeWorkStatusCard";

describe("components/home/HomeWorkStatusCard", () => {
  it("renders loading and unavailable states", () => {
    render(
      <HomeWorkStatusCard
        sortModes={[]}
        sortModesLoading={true}
        sortModesAvailable={null}
        activeModeByCli={{ claude: null, codex: null, gemini: null } as any}
        activeModeToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliActiveMode={vi.fn()}
        cliProxyEnabled={{ claude: true, codex: false, gemini: false } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={vi.fn()}
      />
    );
    expect(screen.getByText("加载中…")).toBeInTheDocument();

    render(
      <HomeWorkStatusCard
        sortModes={[]}
        sortModesLoading={false}
        sortModesAvailable={false}
        activeModeByCli={{ claude: null, codex: null, gemini: null } as any}
        activeModeToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliActiveMode={vi.fn()}
        cliProxyEnabled={{ claude: true, codex: false, gemini: false } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={vi.fn()}
      />
    );
    expect(screen.getByText("数据不可用")).toBeInTheDocument();
  });

  it("drives proxy toggles and active mode selection", () => {
    const onSetCliProxyEnabled = vi.fn();
    const onSetCliActiveMode = vi.fn();

    render(
      <HomeWorkStatusCard
        sortModes={[{ id: 1, name: "M1", created_at: 0, updated_at: 0 } as any]}
        sortModesLoading={false}
        sortModesAvailable={true}
        activeModeByCli={{ claude: null, codex: 1, gemini: null } as any}
        activeModeToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliActiveMode={onSetCliActiveMode}
        cliProxyEnabled={{ claude: true, codex: false, gemini: false } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={onSetCliProxyEnabled}
      />
    );

    const switches = screen.getAllByRole("switch");
    fireEvent.click(switches[0]);
    expect(onSetCliProxyEnabled).toHaveBeenCalledWith("claude", false);

    fireEvent.click(screen.getAllByRole("button", { name: "Default" })[0]);
    expect(onSetCliActiveMode).toHaveBeenCalledWith("claude", null);

    fireEvent.click(screen.getAllByRole("button", { name: "M1" })[0]);
    expect(onSetCliActiveMode).toHaveBeenCalledWith("claude", 1);
  });

  it("supports horizontal layout for the second overview row", () => {
    render(
      <HomeWorkStatusCard
        layout="horizontal"
        sortModes={[{ id: 1, name: "M1", created_at: 0, updated_at: 0 } as any]}
        sortModesLoading={false}
        sortModesAvailable={true}
        activeModeByCli={{ claude: 1, codex: null, gemini: null } as any}
        activeModeToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliActiveMode={vi.fn()}
        cliProxyEnabled={{ claude: true, codex: false, gemini: false } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={vi.fn()}
      />
    );

    expect(screen.getByText("代理状态")).toBeInTheDocument();
    expect(screen.getAllByRole("button", { name: "M1" }).length).toBeGreaterThan(0);
  });
});
