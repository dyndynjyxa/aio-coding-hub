import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { RadioGroup } from "../RadioGroup";

describe("ui/RadioGroup", () => {
  const ariaLabel = "Test options";
  const defaultOptions = [
    { value: "a", label: "Alpha" },
    { value: "b", label: "Beta" },
    { value: "c", label: "Gamma" },
  ];

  it("renders all options with labels", () => {
    render(
      <RadioGroup
        name="test"
        ariaLabel={ariaLabel}
        value="a"
        onChange={() => {}}
        options={defaultOptions}
      />
    );
    expect(screen.getByRole("radiogroup", { name: ariaLabel })).toBeInTheDocument();
    expect(screen.getByText("Alpha")).toBeInTheDocument();
    expect(screen.getByText("Beta")).toBeInTheDocument();
    expect(screen.getByText("Gamma")).toBeInTheDocument();
  });

  it("checks the radio matching the current value", () => {
    render(
      <RadioGroup
        name="test"
        ariaLabel={ariaLabel}
        value="b"
        onChange={() => {}}
        options={defaultOptions}
      />
    );
    expect(screen.getByLabelText("Beta")).toBeChecked();
    expect(screen.getByLabelText("Alpha")).not.toBeChecked();
    expect(screen.getByLabelText("Gamma")).not.toBeChecked();
  });

  it("calls onChange with the selected option value", () => {
    const onChange = vi.fn();
    render(
      <RadioGroup
        name="test"
        ariaLabel={ariaLabel}
        value="a"
        onChange={onChange}
        options={defaultOptions}
      />
    );
    fireEvent.click(screen.getByLabelText("Gamma"));
    expect(onChange).toHaveBeenCalledWith("c");
  });

  it("disables all radios when disabled is true", () => {
    render(
      <RadioGroup
        name="test"
        ariaLabel={ariaLabel}
        value="a"
        onChange={() => {}}
        options={defaultOptions}
        disabled
      />
    );
    expect(screen.getByLabelText("Alpha")).toBeDisabled();
    expect(screen.getByLabelText("Beta")).toBeDisabled();
    expect(screen.getByLabelText("Gamma")).toBeDisabled();
  });

  it("applies disabled styling with opacity and cursor classes", () => {
    const { container } = render(
      <RadioGroup
        name="test"
        ariaLabel={ariaLabel}
        value="a"
        onChange={() => {}}
        options={defaultOptions}
        disabled
      />
    );
    // All label wrappers should have the disabled styling
    const labels = container.querySelectorAll("label");
    labels.forEach((label) => {
      expect(label).toHaveClass("opacity-50", "cursor-not-allowed");
    });
  });

  it("uses the name attribute on all radio inputs", () => {
    render(
      <RadioGroup
        name="color"
        ariaLabel="Color options"
        value="a"
        onChange={() => {}}
        options={defaultOptions}
      />
    );
    const radios = screen.getAllByRole("radio");
    radios.forEach((radio) => {
      expect(radio).toHaveAttribute("name", "color");
    });
  });

  it("renders with a single option", () => {
    render(
      <RadioGroup
        name="single"
        ariaLabel="Single option"
        value="only"
        onChange={() => {}}
        options={[{ value: "only", label: "Only Option" }]}
      />
    );
    expect(screen.getByLabelText("Only Option")).toBeChecked();
  });
});
