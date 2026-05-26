import { fireEvent, render, screen } from "@testing-library/react";
import { createRef } from "react";
import { describe, expect, it, vi } from "vitest";
import { Input } from "../Input";

describe("ui/Input", () => {
  it("renders an input element", () => {
    render(<Input aria-label="test-input" />);
    const input = screen.getByLabelText("test-input");
    expect(input).toBeInTheDocument();
    expect(input).toHaveClass("bg-surface-inset", "border-line", "rounded-lg");
  });

  it("forwards value and onChange", () => {
    const onChange = vi.fn();
    render(<Input aria-label="name" value="hello" onChange={onChange} />);
    const input = screen.getByLabelText("name");
    expect(input).toHaveValue("hello");
    fireEvent.change(input, { target: { value: "world" } });
    expect(onChange).toHaveBeenCalled();
  });

  it("applies disabled state", () => {
    render(<Input aria-label="disabled-input" disabled />);
    expect(screen.getByLabelText("disabled-input")).toBeDisabled();
  });

  it("renders placeholder text", () => {
    render(<Input placeholder="Enter text..." />);
    expect(screen.getByPlaceholderText("Enter text...")).toBeInTheDocument();
  });

  it("applies mono class when mono prop is true", () => {
    const { rerender } = render(<Input aria-label="mono-input" mono />);
    expect(screen.getByLabelText("mono-input")).toHaveClass("font-mono");

    rerender(<Input aria-label="mono-input" />);
    expect(screen.getByLabelText("mono-input")).not.toHaveClass("font-mono");
  });

  it("merges custom className", () => {
    render(<Input aria-label="styled" className="my-class" />);
    expect(screen.getByLabelText("styled")).toHaveClass("my-class");
  });

  it("forwards ref", () => {
    const ref = createRef<HTMLInputElement>();
    render(<Input ref={ref} aria-label="ref-input" />);
    expect(ref.current).toBeInstanceOf(HTMLInputElement);
  });

  it("supports different input types", () => {
    render(<Input aria-label="password" type="password" />);
    expect(screen.getByLabelText("password")).toHaveAttribute("type", "password");
  });
});
