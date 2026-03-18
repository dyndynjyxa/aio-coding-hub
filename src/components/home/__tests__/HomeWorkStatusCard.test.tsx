import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { HomeWorkStatusCard } from "../HomeWorkStatusCard";

describe("components/home/HomeWorkStatusCard", () => {
  it("renders loading and unavailable states", () => {
    render(
      <HomeWorkStatusCard
        sortModesLoading={true}
        sortModesAvailable={null}
        cliProxyEnabled={{ claude: true, codex: false, gemini: false } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={vi.fn()}
      />
    );
    expect(screen.getByText("加载中…")).toBeInTheDocument();

    render(
      <HomeWorkStatusCard
        sortModesLoading={false}
        sortModesAvailable={false}
        cliProxyEnabled={{ claude: true, codex: false, gemini: false } as any}
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
        sortModesLoading={false}
        sortModesAvailable={true}
        cliProxyEnabled={{ claude: true, codex: false, gemini: false } as any}
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
        sortModesLoading={false}
        sortModesAvailable={true}
        cliProxyEnabled={{ claude: true, codex: false, gemini: false } as any}
        cliProxyToggling={{ claude: false, codex: false, gemini: false } as any}
        onSetCliProxyEnabled={vi.fn()}
      />
    );

    expect(screen.getByText("代理状态")).toBeInTheDocument();
    expect(screen.getAllByRole("switch").length).toBe(3);
  });
});
