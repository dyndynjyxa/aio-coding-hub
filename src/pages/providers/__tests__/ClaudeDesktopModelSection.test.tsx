import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { ProviderModelMapping } from "../../../services/providers/providers";
import { ClaudeDesktopModelSection } from "../ClaudeDesktopModelSection";

describe("ClaudeDesktopModelSection", () => {
  it("maps the installed Desktop roles while preserving unrelated policy mappings", () => {
    const onChange = vi.fn();
    const mappings: ProviderModelMapping[] = [
      { source: "claude-sonnet-5", target: "upstream-sonnet" },
      { source: "custom-*", target: "legacy-*" },
    ];

    render(
      <ClaudeDesktopModelSection
        mappings={mappings}
        onChange={onChange}
        supports1m={false}
        onSupports1mChange={vi.fn()}
        disabled={false}
      />
    );
    expect(screen.getByRole("textbox", { name: "Sonnet 上游模型" })).toHaveValue("upstream-sonnet");
    fireEvent.change(screen.getByRole("textbox", { name: "Fable 上游模型" }), {
      target: { value: "upstream-fable" },
    });

    expect(onChange).toHaveBeenCalledWith([
      ...mappings,
      { source: "claude-fable-5", target: "upstream-fable" },
    ]);
  });

  it("removes a route mapping when its upstream model is cleared", () => {
    const onChange = vi.fn();
    render(
      <ClaudeDesktopModelSection
        mappings={[{ source: "claude-opus-5", target: "old-opus" }]}
        onChange={onChange}
        supports1m={false}
        onSupports1mChange={vi.fn()}
        disabled={false}
      />
    );

    fireEvent.change(screen.getByRole("textbox", { name: "Opus 上游模型" }), {
      target: { value: "" },
    });
    expect(onChange).toHaveBeenCalledWith([]);
  });

  it("toggles the provider's single 1M checkbox", () => {
    const onSupports1mChange = vi.fn();
    render(
      <ClaudeDesktopModelSection
        mappings={[]}
        onChange={vi.fn()}
        supports1m={false}
        onSupports1mChange={onSupports1mChange}
        disabled={false}
      />
    );

    const checkbox = screen.getByRole("checkbox", { name: "支持 1M 上下文" });
    expect(checkbox).not.toBeChecked();
    fireEvent.click(checkbox);
    expect(onSupports1mChange).toHaveBeenCalledWith(true);
  });
});
