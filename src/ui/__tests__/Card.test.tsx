import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Card } from "../Card";

describe("ui/Card", () => {
  it("renders children", () => {
    render(<Card>Card content</Card>);
    expect(screen.getByText("Card content")).toBeInTheDocument();
  });

  it("applies md padding by default", () => {
    const { container } = render(<Card>Default</Card>);
    const card = container.firstElementChild;
    expect(card).toHaveClass("p-4");
  });

  it("applies sm padding", () => {
    const { container } = render(<Card padding="sm">Small</Card>);
    const card = container.firstElementChild;
    expect(card).toHaveClass("p-3");
  });

  it("applies no padding when padding=none", () => {
    const { container } = render(<Card padding="none">None</Card>);
    const card = container.firstElementChild;
    expect(card).not.toHaveClass("p-3");
    expect(card).not.toHaveClass("p-4");
  });

  it("merges custom className", () => {
    const { container } = render(<Card className="my-card">Styled</Card>);
    expect(container.firstElementChild).toHaveClass("my-card");
  });

  it("applies base styling classes", () => {
    const { container } = render(<Card>Base</Card>);
    const card = container.firstElementChild;
    expect(card).toHaveClass("overflow-hidden", "bg-surface-panel", "rounded-2xl");
  });

  it("applies visual variants", () => {
    const { container, rerender } = render(<Card variant="raised">Raised</Card>);
    expect(container.firstElementChild).toHaveClass("bg-surface-raised", "border-line");

    rerender(<Card variant="inset">Inset</Card>);
    expect(container.firstElementChild).toHaveClass("bg-surface-inset", "border-line-subtle");
  });

  it("passes through HTML div attributes", () => {
    render(
      <Card data-testid="test-card" id="card-1">
        Content
      </Card>
    );
    const card = screen.getByTestId("test-card");
    expect(card).toHaveAttribute("id", "card-1");
  });

  it("renders nested elements", () => {
    render(
      <Card>
        <h2>Title</h2>
        <p>Description</p>
      </Card>
    );
    expect(screen.getByText("Title")).toBeInTheDocument();
    expect(screen.getByText("Description")).toBeInTheDocument();
  });
});
