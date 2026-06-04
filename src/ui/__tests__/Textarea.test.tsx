import { fireEvent, render, screen } from "@testing-library/react";
import { createRef } from "react";
import { describe, expect, it, vi } from "vitest";
import { Textarea } from "../Textarea";

describe("ui/Textarea", () => {
  it("renders a textarea element", () => {
    render(<Textarea aria-label="notes" />);
    const ta = screen.getByLabelText("notes");
    expect(ta).toBeInTheDocument();
    expect(ta.tagName).toBe("TEXTAREA");
    expect(ta).toHaveClass("bg-surface-inset", "border-line", "rounded-lg");
  });

  it("forwards value and onChange", () => {
    const onChange = vi.fn();
    render(<Textarea aria-label="content" value="hello" onChange={onChange} />);
    const ta = screen.getByLabelText("content");
    expect(ta).toHaveValue("hello");
    fireEvent.change(ta, { target: { value: "world" } });
    expect(onChange).toHaveBeenCalled();
  });

  it("applies disabled state", () => {
    render(<Textarea aria-label="disabled-ta" disabled />);
    expect(screen.getByLabelText("disabled-ta")).toBeDisabled();
  });

  it("renders placeholder text", () => {
    render(<Textarea placeholder="Type here..." />);
    expect(screen.getByPlaceholderText("Type here...")).toBeInTheDocument();
  });

  it("applies mono class when mono prop is true", () => {
    const { rerender } = render(<Textarea aria-label="mono-ta" mono />);
    expect(screen.getByLabelText("mono-ta")).toHaveClass("font-mono");

    rerender(<Textarea aria-label="mono-ta" />);
    expect(screen.getByLabelText("mono-ta")).not.toHaveClass("font-mono");
  });

  it("merges custom className", () => {
    render(<Textarea aria-label="styled-ta" className="extra" />);
    expect(screen.getByLabelText("styled-ta")).toHaveClass("extra");
  });

  it("forwards ref", () => {
    const ref = createRef<HTMLTextAreaElement>();
    render(<Textarea ref={ref} aria-label="ref-ta" />);
    expect(ref.current).toBeInstanceOf(HTMLTextAreaElement);
  });

  it("supports defaultValue", () => {
    render(<Textarea aria-label="default-ta" defaultValue="initial" />);
    expect(screen.getByLabelText("default-ta")).toHaveValue("initial");
  });

  it("supports rows attribute", () => {
    render(<Textarea aria-label="rows-ta" rows={5} />);
    expect(screen.getByLabelText("rows-ta")).toHaveAttribute("rows", "5");
  });
});
