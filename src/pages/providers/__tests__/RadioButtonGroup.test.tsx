import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { RadioButtonGroup } from "../RadioButtonGroup";

describe("pages/providers/RadioButtonGroup", () => {
  it("uses a dark inactive background class so unselected items are not bright in dark mode", () => {
    render(
      <RadioButtonGroup
        ariaLabel="Base URL mode"
        value="order"
        onChange={vi.fn()}
        items={[
          { value: "order", label: "Order" },
          { value: "ping", label: "Ping" },
        ]}
        fullWidth={false}
      />
    );

    expect(screen.getByRole("radio", { name: "Ping" })).toHaveClass("dark:bg-secondary");
  });
});
