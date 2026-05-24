import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Spinner } from "../Spinner";

describe("ui/Spinner", () => {
  it("renders with default md size", () => {
    const { container } = render(<Spinner />);
    const spinner = container.firstElementChild;
    expect(spinner).toBeInTheDocument();
    expect(spinner).toHaveClass("h-6", "w-6", "border-2");
  });

  it("renders with sm size", () => {
    const { container } = render(<Spinner size="sm" />);
    const spinner = container.firstElementChild;
    expect(spinner).toHaveClass("h-4", "w-4", "border-2");
  });

  it("renders with lg size", () => {
    const { container } = render(<Spinner size="lg" />);
    const spinner = container.firstElementChild;
    expect(spinner).toHaveClass("h-8", "w-8");
  });

  it("applies animate-spin class", () => {
    const { container } = render(<Spinner />);
    const spinner = container.firstElementChild;
    expect(spinner).toHaveClass("animate-spin");
  });

  it("applies rounded-full for circular shape", () => {
    const { container } = render(<Spinner />);
    const spinner = container.firstElementChild;
    expect(spinner).toHaveClass("rounded-full");
  });

  it("has role=status and aria-label for accessibility", () => {
    render(<Spinner />);
    const spinner = screen.getByRole("status");
    expect(spinner).toHaveAttribute("aria-label", "Loading");
  });

  it("merges custom className", () => {
    const { container } = render(<Spinner className="my-spinner" />);
    const spinner = container.firstElementChild;
    expect(spinner).toHaveClass("my-spinner");
  });

  it("supports semantic border classes", () => {
    const { container } = render(<Spinner />);
    const spinner = container.firstElementChild;
    expect(spinner).toHaveClass("border-muted-foreground/30");
  });
});
