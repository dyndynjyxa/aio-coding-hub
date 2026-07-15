import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ProviderCircuitBadge } from "../ProviderCircuitBadge";

describe("components/ProviderCircuitBadge", () => {
  it("returns null when rows is empty", () => {
    const { container } = render(
      <ProviderCircuitBadge rows={[]} onResetProvider={() => {}} resettingProviderIds={new Set()} />
    );
    expect(container).toBeEmptyDOMElement();
  });

  it("renders rows, opens popover, and calls onResetProvider", async () => {
    const onResetProvider = vi.fn();
    const nowUnix = Math.floor(Date.now() / 1000);
    render(
      <ProviderCircuitBadge
        rows={[
          {
            cli_key: "claude",
            provider_id: 1,
            provider_name: "P1",
            displayState: "open",
            open_until: nowUnix + 10,
          },
          {
            cli_key: "claude",
            provider_id: 2,
            provider_name: "P2",
            displayState: "open",
            open_until: null,
          },
          {
            cli_key: "codex",
            provider_id: 3,
            provider_name: "P3",
            displayState: "open",
            open_until: nowUnix + 5,
          },
        ]}
        onResetProvider={onResetProvider}
        resettingProviderIds={new Set([2])}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: "当前熔断 3" }));
    await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());

    expect(screen.getByText("熔断列表 (3)")).toBeInTheDocument();
    expect(screen.getByText("P1")).toBeInTheDocument();
    expect(screen.getByText("P2")).toBeInTheDocument();
    expect(screen.getByText("P3")).toBeInTheDocument();

    // disabled state
    expect(screen.getAllByRole("button", { name: "解除中..." })[0]).toBeDisabled();

    // click reset (first one is for P1 in this fixture)
    fireEvent.click(screen.getAllByRole("button", { name: "解除熔断" })[0]);
    expect(onResetProvider).toHaveBeenCalledWith(1);
  });

  it("renders half-open rows as amber probe state without countdown and keeps reset action", async () => {
    const onResetProvider = vi.fn();
    render(
      <ProviderCircuitBadge
        rows={[
          {
            cli_key: "claude",
            provider_id: 1,
            provider_name: "P1",
            displayState: "half_open",
            open_until: null,
          },
        ]}
        onResetProvider={onResetProvider}
        resettingProviderIds={new Set()}
      />
    );

    // 仅半开行：触发器整体转琥珀“试探恢复 M”，非红色。
    const trigger = screen.getByText("试探恢复 1");
    expect(trigger.className).toContain("amber");
    expect(trigger.className).not.toContain("rose");
    expect(screen.queryByText(/当前熔断/)).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "试探恢复 1" }));
    await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());

    // 仅半开行时 popover 头部与分组不再称"熔断"，与触发器状态词一致。
    expect(screen.getByText("试探恢复列表 (1)")).toBeInTheDocument();
    expect(screen.queryByText(/个熔断/)).not.toBeInTheDocument();
    expect(screen.getByText("1 个供应商")).toBeInTheDocument();

    const status = screen.getByText("试探恢复中");
    expect(status.className).toContain("amber");
    // 半开行无倒计时。
    expect(screen.queryByText(/\d{2}:\d{2}/)).not.toBeInTheDocument();

    // 半开行保留“解除”按钮（跳过试探直接恢复）。
    fireEvent.click(screen.getByRole("button", { name: "解除熔断" }));
    expect(onResetProvider).toHaveBeenCalledWith(1);
  });

  it("keeps red trigger with recovering suffix and per-state row labels for mixed rows", async () => {
    const nowUnix = Math.floor(Date.now() / 1000);
    render(
      <ProviderCircuitBadge
        rows={[
          {
            cli_key: "claude",
            provider_id: 1,
            provider_name: "P1",
            displayState: "open",
            open_until: nowUnix + 60,
          },
          {
            cli_key: "claude",
            provider_id: 2,
            provider_name: "P2",
            displayState: "cooldown",
            open_until: nowUnix + 30,
          },
          {
            cli_key: "codex",
            provider_id: 3,
            provider_name: "P3",
            displayState: "half_open",
            open_until: null,
          },
        ]}
        onResetProvider={() => {}}
        resettingProviderIds={new Set()}
      />
    );

    // 半开行不计入“当前熔断 N”的 N。
    const trigger = screen.getByText("当前熔断 2 · 恢复中 1");
    expect(trigger.className).toContain("rose");

    fireEvent.click(screen.getByRole("button", { name: "当前熔断 2 · 恢复中 1" }));
    await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());

    expect(screen.getByText("熔断")).toBeInTheDocument();
    expect(screen.getByText("冷却中")).toBeInTheDocument();
    expect(screen.getByText("试探恢复中")).toBeInTheDocument();
    // open/cooldown 行保留倒计时。
    expect(screen.getAllByText(/^\d{2}:\d{2}$/)).toHaveLength(2);
  });

  it("auto closes popover when rows become empty", async () => {
    const nowUnix = Math.floor(Date.now() / 1000);
    const { rerender } = render(
      <ProviderCircuitBadge
        rows={[
          {
            cli_key: "claude",
            provider_id: 1,
            provider_name: "P1",
            displayState: "open",
            open_until: nowUnix + 10,
          },
        ]}
        onResetProvider={() => {}}
        resettingProviderIds={new Set()}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: "当前熔断 1" }));
    await waitFor(() => expect(screen.getByRole("dialog")).toBeInTheDocument());

    rerender(
      <ProviderCircuitBadge rows={[]} onResetProvider={() => {}} resettingProviderIds={new Set()} />
    );

    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });
});
