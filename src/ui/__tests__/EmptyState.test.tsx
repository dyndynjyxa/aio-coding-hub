import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { EmptyState } from "../EmptyState";

describe("ui/EmptyState", () => {
  it("renders title text", () => {
    render(<EmptyState title="No data" />);
    expect(screen.getByText("No data")).toBeInTheDocument();
  });

  it("renders description when provided", () => {
    render(<EmptyState title="No data" description="Try adding some items." />);
    expect(screen.getByText("Try adding some items.")).toBeInTheDocument();
  });

  it("does not render description when not provided", () => {
    const { container } = render(<EmptyState title="No data" />);
    // Only one text element (the title)
    const textElements = container.querySelectorAll(".text-sm");
    expect(textElements.length).toBe(1);
  });

  it("renders icon when provided", () => {
    render(<EmptyState title="No data" icon={<span data-testid="icon">icon</span>} />);
    expect(screen.getByTestId("icon")).toBeInTheDocument();
  });

  it("renders action when provided", () => {
    render(<EmptyState title="No data" action={<button>Add item</button>} />);
    expect(screen.getByRole("button", { name: "Add item" })).toBeInTheDocument();
  });

  it("applies default variant (no border)", () => {
    const { container } = render(<EmptyState title="No data" />);
    const el = container.firstElementChild;
    expect(el).not.toHaveClass("border-dashed");
  });

  it("applies dashed variant with border classes", () => {
    const { container } = render(<EmptyState title="No data" variant="dashed" />);
    const el = container.firstElementChild;
    expect(el).toHaveClass("border-dashed", "rounded-xl", "p-6");
  });

  it("centers content", () => {
    const { container } = render(<EmptyState title="No data" />);
    const el = container.firstElementChild;
    expect(el).toHaveClass("flex", "flex-col", "items-center", "justify-center", "text-center");
  });

  it("merges custom className", () => {
    const { container } = render(<EmptyState title="No data" className="my-empty" />);
    const el = container.firstElementChild;
    expect(el).toHaveClass("my-empty");
  });

  it("supports dark mode text classes", () => {
    render(<EmptyState title="No data" />);
    const titleEl = screen.getByText("No data");
    expect(titleEl).toHaveClass("text-muted-foreground");
  });
});
