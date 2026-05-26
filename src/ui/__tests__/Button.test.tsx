import { fireEvent, render, screen } from "@testing-library/react";
import { createRef } from "react";
import { describe, expect, it, vi } from "vitest";
import { Button } from "../Button";

describe("ui/Button", () => {
  it("renders children and defaults to type=button", () => {
    render(<Button>Click me</Button>);
    const btn = screen.getByRole("button", { name: "Click me" });
    expect(btn).toBeInTheDocument();
    expect(btn).toHaveAttribute("type", "button");
  });

  it("fires onClick callback", () => {
    const onClick = vi.fn();
    render(<Button onClick={onClick}>Go</Button>);
    fireEvent.click(screen.getByRole("button", { name: "Go" }));
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("applies disabled state", () => {
    const onClick = vi.fn();
    render(
      <Button disabled onClick={onClick}>
        Disabled
      </Button>
    );
    const btn = screen.getByRole("button", { name: "Disabled" });
    expect(btn).toBeDisabled();
    fireEvent.click(btn);
    expect(onClick).not.toHaveBeenCalled();
  });

  it("applies variant classes", () => {
    const { rerender } = render(<Button variant="primary">Primary</Button>);
    expect(screen.getByRole("button")).toHaveClass("bg-state-selected");

    rerender(<Button variant="ghost">Ghost</Button>);
    expect(screen.getByRole("button")).toHaveClass("hover:bg-state-hover");

    rerender(<Button variant="danger">Danger</Button>);
    expect(screen.getByRole("button")).toHaveClass("text-rose-700");

    rerender(<Button variant="warning">Warning</Button>);
    expect(screen.getByRole("button")).toHaveClass("text-amber-800");
  });

  it("applies size classes", () => {
    const { rerender } = render(<Button size="sm">Small</Button>);
    expect(screen.getByRole("button")).toHaveClass("text-xs");

    rerender(<Button size="md">Medium</Button>);
    expect(screen.getByRole("button")).toHaveClass("text-sm");

    rerender(<Button size="icon">Icon</Button>);
    expect(screen.getByRole("button")).toHaveClass("h-8", "w-8");
  });

  it("merges custom className", () => {
    render(<Button className="my-custom">Styled</Button>);
    expect(screen.getByRole("button")).toHaveClass("my-custom");
  });

  it("forwards ref", () => {
    const ref = createRef<HTMLButtonElement>();
    render(<Button ref={ref}>Ref</Button>);
    expect(ref.current).toBeInstanceOf(HTMLButtonElement);
    expect(ref.current?.textContent).toBe("Ref");
  });

  it("allows overriding type attribute", () => {
    render(<Button type="submit">Submit</Button>);
    expect(screen.getByRole("button")).toHaveAttribute("type", "submit");
  });

  it("defaults to secondary variant and md size", () => {
    render(<Button>Default</Button>);
    const btn = screen.getByRole("button");
    // secondary variant class
    expect(btn).toHaveClass("bg-surface-panel");
    // md size class
    expect(btn).toHaveClass("text-sm");
  });

  it("renders as child when asChild is enabled", () => {
    render(
      <Button asChild className="child-button">
        <a href="/docs">Docs</a>
      </Button>
    );

    const link = screen.getByRole("link", { name: "Docs" });
    expect(link).toHaveAttribute("href", "/docs");
    expect(link).toHaveClass("child-button");
    expect(screen.queryByRole("button", { name: "Docs" })).not.toBeInTheDocument();
  });
});
