import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Skeleton } from "../Skeleton";

describe("ui/Skeleton", () => {
  it("renders with default text variant", () => {
    const { container } = render(<Skeleton />);
    const el = container.firstElementChild;
    expect(el).toBeInTheDocument();
    expect(el).toHaveClass("h-4", "w-full", "rounded-md");
  });

  it("renders circular variant", () => {
    const { container } = render(<Skeleton variant="circular" />);
    const el = container.firstElementChild;
    expect(el).toHaveClass("rounded-full");
  });

  it("renders rectangular variant", () => {
    const { container } = render(<Skeleton variant="rectangular" />);
    const el = container.firstElementChild;
    expect(el).toHaveClass("rounded-lg");
  });

  it("applies animate-pulse class", () => {
    const { container } = render(<Skeleton />);
    const el = container.firstElementChild;
    expect(el).toHaveClass("animate-pulse");
  });

  it("applies background color classes", () => {
    const { container } = render(<Skeleton />);
    const el = container.firstElementChild;
    expect(el).toHaveClass("bg-muted");
  });

  it("supports semantic token classes", () => {
    const { container } = render(<Skeleton />);
    const el = container.firstElementChild;
    expect(el).toHaveClass("bg-muted");
  });

  it("merges custom className", () => {
    const { container } = render(<Skeleton className="h-8 w-32" />);
    const el = container.firstElementChild;
    expect(el).toHaveClass("h-8", "w-32");
  });

  it("is hidden from accessibility tree", () => {
    const { container } = render(<Skeleton />);
    const el = container.firstElementChild;
    expect(el).toHaveAttribute("aria-hidden", "true");
  });
});
