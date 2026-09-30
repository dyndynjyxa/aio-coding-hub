import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ProviderModelMapping } from "../../../services/providers/providers";
import { ClaudeDesktopModelSection } from "../ClaudeDesktopModelSection";

describe("ClaudeDesktopModelSection", () => {
  it("maps whole Desktop model families while preserving other policy mappings", () => {
    const onChange = vi.fn();
    const mappings: ProviderModelMapping[] = [
      { source: "claude-sonnet-*", target: "upstream-sonnet" },
      { source: "claude-opus-5", target: "exact-opus" },
    ];

    render(<ClaudeDesktopModelSection mappings={mappings} onChange={onChange} disabled={false} />);
    expect(screen.getByRole("textbox", { name: "Sonnet 上游模型" })).toHaveValue("upstream-sonnet");
    expect(screen.getByRole("textbox", { name: "Opus 上游模型" })).toHaveValue("");
    fireEvent.change(screen.getByRole("textbox", { name: "Fable 上游模型" }), {
      target: { value: "upstream-fable" },
    });

    expect(onChange).toHaveBeenCalledWith([
      ...mappings,
      { source: "claude-fable-*", target: "upstream-fable" },
    ]);
  });

  it("removes a family mapping when its upstream model is cleared", () => {
    const onChange = vi.fn();
    render(
      <ClaudeDesktopModelSection
        mappings={[{ source: "claude-opus-*", target: "old-opus" }]}
        onChange={onChange}
        disabled={false}
      />
    );

    fireEvent.change(screen.getByRole("textbox", { name: "Opus 上游模型" }), {
      target: { value: "" },
    });
    expect(onChange).toHaveBeenCalledWith([]);
  });
});
