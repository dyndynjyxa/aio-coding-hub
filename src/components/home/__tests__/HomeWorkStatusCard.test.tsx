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
        cliProxyEnabled={{ claude: true, codex: false, gemini: false } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={vi.fn()}
      />
    );
    expect(screen.getByText("数据不可用")).toBeInTheDocument();
  });

  it("drives proxy toggles and renders workspace as read-only pill info", () => {
    const onSetCliProxyEnabled = vi.fn();

    render(
      <HomeWorkStatusCard
        sortModes={[{ id: 1, name: "M1", created_at: 0, updated_at: 0 } as any]}
        sortModesLoading={false}
        sortModesAvailable={true}
        activeModeByCli={{ claude: null, codex: 1, gemini: null } as any}
        activeModeToggling={{ claude: false, codex: true, gemini: false } as any}
        cliProxyEnabled={{ claude: true, codex: false, gemini: false } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={onSetCliProxyEnabled}
      />
    );

    const switches = screen.getAllByRole("switch");
    fireEvent.click(switches[0]);
    expect(onSetCliProxyEnabled).toHaveBeenCalledWith("claude", false);

    expect(screen.getAllByText("Default").length).toBeGreaterThan(0);
    expect(screen.getByText("加载中…")).toBeInTheDocument();
    expect(screen.getAllByTitle("当前工作区：Default").length).toBeGreaterThan(0);
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
        cliProxyEnabled={{ claude: true, codex: false, gemini: false } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={vi.fn()}
      />
    );

    expect(screen.getByText("代理状态")).toBeInTheDocument();
    expect(screen.getByTitle("当前工作区：M1")).toBeInTheDocument();
  });
});
