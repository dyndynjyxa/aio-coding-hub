import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import { PinTierButton, SortModeSelect } from "../pinControls";
import { createQueryWrapper, createTestQueryClient } from "../../../test/utils/reactQuery";

// Per-test mutable spies so each case asserts exact mutation inputs.
// vi.hoisted keeps them stable across the vi.mock factory closure.
const spies = vi.hoisted(() => ({
  pin: vi.fn(),
  unpin: vi.fn(),
  persist: vi.fn(),
  unpersist: vi.fn(),
}));

vi.mock("../../../query/gateway", () => ({
  useGatewaySessionPinSortModeMutation: () => ({ mutate: spies.pin, isPending: false }),
  useGatewaySessionUnpinSortModeMutation: () => ({ mutate: spies.unpin, isPending: false }),
  useGatewaySessionPersistSortModeMutation: () => ({
    mutate: (input: unknown, opts?: { onSuccess?: () => void }) => {
      spies.persist(input);
      opts?.onSuccess?.();
    },
    isPending: false,
  }),
  useGatewaySessionUnpersistSortModeMutation: () => ({ mutate: spies.unpersist, isPending: false }),
}));

const SORT_MODES = [
  { id: 3, name: "Cheap-first", created_at: 1, updated_at: 1 },
  { id: 7, name: "Fast-first", created_at: 2, updated_at: 2 },
];

function renderWith(ui: React.ReactElement) {
  return render(ui, { wrapper: createQueryWrapper(createTestQueryClient()) });
}

function selectEl() {
  return screen.getByRole("combobox", { name: "路由策略" });
}

function pinButton() {
  // aria-label pattern: "pin 状态：<label>"
  return screen.getByRole("button", { name: /pin 状态/ });
}

function changeSelect(value: string) {
  fireEvent.change(selectEl(), { target: { value } });
}

describe("components/gateway/pinControls", () => {
  beforeEach(() => {
    spies.pin.mockClear();
    spies.unpin.mockClear();
    spies.persist.mockClear();
    spies.unpersist.mockClear();
  });

  describe("SortModeSelect — selected value (pinStateToValue)", () => {
    it("maps not-pinned (tier none) to 'auto'", () => {
      renderWith(
        <SortModeSelect
          cliKey="claude"
          sessionId="s1"
          tier="none"
          selectedModeId={undefined}
          modes={SORT_MODES}
          autoLabel="follow"
        />
      );
      expect(selectEl()).toHaveValue("auto");
    });

    it("maps pinned with no mode to 'default'", () => {
      renderWith(
        <SortModeSelect
          cliKey="claude"
          sessionId="s1"
          tier="ephemeral"
          selectedModeId={null}
          modes={SORT_MODES}
          autoLabel="clear"
        />
      );
      expect(selectEl()).toHaveValue("default");
    });

    it("maps pinned with a mode id to that id string", () => {
      renderWith(
        <SortModeSelect
          cliKey="claude"
          sessionId="s1"
          tier="persistent"
          selectedModeId={7}
          modes={SORT_MODES}
          autoLabel="clear"
        />
      );
      expect(selectEl()).toHaveValue("7");
    });
  });

  describe("SortModeSelect — onChange dispatch (valueToSortModeId + tier routing)", () => {
    it("selecting 'auto' on persistent tier calls unpersist", () => {
      renderWith(
        <SortModeSelect
          cliKey="claude"
          sessionId="s1"
          tier="persistent"
          selectedModeId={7}
          modes={SORT_MODES}
          autoLabel="clear"
        />
      );
      changeSelect("auto");
      expect(spies.unpersist).toHaveBeenCalledWith({ cliKey: "claude", sessionId: "s1" });
      expect(spies.pin).not.toHaveBeenCalled();
    });

    it("selecting 'auto' on ephemeral tier calls unpin", () => {
      renderWith(
        <SortModeSelect
          cliKey="codex"
          sessionId="s1"
          tier="ephemeral"
          selectedModeId={3}
          modes={SORT_MODES}
          autoLabel="clear"
        />
      );
      changeSelect("auto");
      expect(spies.unpin).toHaveBeenCalledWith({ cliKey: "codex", sessionId: "s1" });
    });

    it("selecting 'auto' on none tier does nothing (already clear)", () => {
      renderWith(
        <SortModeSelect
          cliKey="claude"
          sessionId="s1"
          tier="none"
          selectedModeId={undefined}
          modes={SORT_MODES}
          autoLabel="follow"
        />
      );
      changeSelect("auto");
      expect(spies.pin).not.toHaveBeenCalled();
      expect(spies.unpin).not.toHaveBeenCalled();
    });

    it("selecting 'default' on persistent tier re-pins persistent with null mode", () => {
      renderWith(
        <SortModeSelect
          cliKey="claude"
          sessionId="s1"
          tier="persistent"
          selectedModeId={7}
          modes={SORT_MODES}
          autoLabel="clear"
        />
      );
      changeSelect("default");
      expect(spies.persist).toHaveBeenCalledWith({
        cliKey: "claude",
        sessionId: "s1",
        sortModeId: null,
      });
    });

    it("selecting a numeric mode on none tier pins ephemeral", () => {
      renderWith(
        <SortModeSelect
          cliKey="claude"
          sessionId="s1"
          tier="none"
          selectedModeId={undefined}
          modes={SORT_MODES}
          autoLabel="follow"
        />
      );
      changeSelect("3");
      expect(spies.pin).toHaveBeenCalledWith({ cliKey: "claude", sessionId: "s1", sortModeId: 3 });
    });

    it("selecting a numeric mode on persistent tier re-pins persistent", () => {
      renderWith(
        <SortModeSelect
          cliKey="claude"
          sessionId="s1"
          tier="persistent"
          selectedModeId={7}
          modes={SORT_MODES}
          autoLabel="clear"
        />
      );
      changeSelect("3");
      expect(spies.persist).toHaveBeenCalledWith({
        cliKey: "claude",
        sessionId: "s1",
        sortModeId: 3,
      });
    });

    it("re-selecting the current value is a no-op", () => {
      renderWith(
        <SortModeSelect
          cliKey="claude"
          sessionId="s1"
          tier="ephemeral"
          selectedModeId={7}
          modes={SORT_MODES}
          autoLabel="clear"
        />
      );
      changeSelect("7"); // same as current
      expect(spies.pin).not.toHaveBeenCalled();
      expect(spies.unpin).not.toHaveBeenCalled();
    });
  });

  describe("SortModeSelect — disabled state", () => {
    it("disables when cliKey is unsupported", () => {
      renderWith(
        <SortModeSelect
          cliKey="not-a-real-cli"
          sessionId="s1"
          tier="none"
          selectedModeId={undefined}
          modes={SORT_MODES}
          autoLabel="follow"
        />
      );
      expect(selectEl()).toBeDisabled();
    });
  });

  describe("PinTierButton — click cycles", () => {
    it("none → ephemeral: pins with selected mode", () => {
      renderWith(<PinTierButton cliKey="claude" sessionId="s1" tier="none" selectedModeId={3} />);
      fireEvent.click(pinButton());
      expect(spies.pin).toHaveBeenCalledWith({ cliKey: "claude", sessionId: "s1", sortModeId: 3 });
    });

    it("none → ephemeral: pins with null when no mode selected (Default fallback)", () => {
      renderWith(
        <PinTierButton cliKey="claude" sessionId="s1" tier="none" selectedModeId={null} />
      );
      fireEvent.click(pinButton());
      expect(spies.pin).toHaveBeenCalledWith({
        cliKey: "claude",
        sessionId: "s1",
        sortModeId: null,
      });
    });

    it("ephemeral → persistent: persists then unpins the superseded ephemeral", () => {
      renderWith(
        <PinTierButton cliKey="claude" sessionId="s1" tier="ephemeral" selectedModeId={7} />
      );
      fireEvent.click(pinButton());
      expect(spies.persist).toHaveBeenCalledWith({
        cliKey: "claude",
        sessionId: "s1",
        sortModeId: 7,
      });
      expect(spies.unpin).toHaveBeenCalledWith({ cliKey: "claude", sessionId: "s1" });
    });

    it("persistent → none: unpersists", () => {
      renderWith(
        <PinTierButton cliKey="claude" sessionId="s1" tier="persistent" selectedModeId={7} />
      );
      fireEvent.click(pinButton());
      expect(spies.unpersist).toHaveBeenCalledWith({ cliKey: "claude", sessionId: "s1" });
    });

    it("disabled when cliKey is unsupported (no mutation fires)", () => {
      renderWith(<PinTierButton cliKey="bogus" sessionId="s1" tier="none" selectedModeId={3} />);
      expect(pinButton()).toBeDisabled();
    });
  });

  describe("PinTierButton — tier config drives label", () => {
    it("renders '未 pin' label when tier is none", () => {
      renderWith(
        <PinTierButton cliKey="claude" sessionId="s1" tier="none" selectedModeId={undefined} />
      );
      expect(pinButton()).toHaveAttribute("aria-label", "pin 状态：未 pin");
      expect(pinButton()).toHaveTextContent("未 pin");
    });

    it("renders '本次' label when tier is ephemeral", () => {
      renderWith(
        <PinTierButton cliKey="claude" sessionId="s1" tier="ephemeral" selectedModeId={3} />
      );
      expect(pinButton()).toHaveAttribute("aria-label", "pin 状态：本次");
    });

    it("renders '持久' label when tier is persistent", () => {
      renderWith(
        <PinTierButton cliKey="claude" sessionId="s1" tier="persistent" selectedModeId={7} />
      );
      expect(pinButton()).toHaveAttribute("aria-label", "pin 状态：持久");
    });
  });
});
