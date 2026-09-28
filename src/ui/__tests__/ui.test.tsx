import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Dialog } from "../Dialog";
import { FormField } from "../FormField";
import { Popover } from "../Popover";
import { RadioGroup } from "../RadioGroup";
import { Select } from "../Select";
import { Textarea } from "../Textarea";
import { Tooltip } from "../Tooltip";

describe("ui components", () => {
  it("Popover opens and closes (click outside + toggle)", async () => {
    const user = userEvent.setup();
    render(
      <Popover trigger={<span>trigger</span>} placement="bottom" align="center">
        <div>content</div>
      </Popover>
    );

    await user.click(screen.getByRole("button"));
    expect(await screen.findByText("content")).toBeInTheDocument();

    fireEvent.pointerDown(document.body);
    await waitFor(() => expect(screen.queryByText("content")).not.toBeInTheDocument());

    // toggle close
    await user.click(screen.getByRole("button"));
    expect(await screen.findByText("content")).toBeInTheDocument();
    await user.click(screen.getByRole("button"));
    await waitFor(() => expect(screen.queryByText("content")).not.toBeInTheDocument());
  });

  it("Tooltip shows and hides (top/bottom placement)", async () => {
    const user = userEvent.setup();
    const { rerender } = render(
      <Tooltip content="hello" placement="top">
        <span>anchor</span>
      </Tooltip>
    );

    await user.hover(screen.getByText("anchor"));
    await waitFor(() => expect(document.querySelector(".bg-foreground")).not.toBeNull());
    await user.unhover(screen.getByText("anchor"));
    await waitFor(() => expect(document.querySelector(".bg-foreground")).toBeNull());

    rerender(
      <Tooltip content="world" placement="bottom">
        <span>anchor</span>
      </Tooltip>
    );
    await user.hover(screen.getByText("anchor"));
    await waitFor(() => expect(document.querySelector(".bg-foreground")).not.toBeNull());
  });

  it("RadioGroup calls onChange and respects disabled", () => {
    const onChange = vi.fn();
    const { rerender } = render(
      <RadioGroup
        name="t"
        ariaLabel="测试选项"
        value="a"
        onChange={onChange}
        options={[
          { value: "a", label: "A" },
          { value: "b", label: "B" },
        ]}
      />
    );

    fireEvent.click(screen.getByLabelText("B"));
    expect(onChange).toHaveBeenCalledWith("b");

    onChange.mockClear();
    rerender(
      <RadioGroup
        name="t"
        ariaLabel="测试选项"
        value="a"
        onChange={onChange}
        disabled
        options={[
          { value: "a", label: "A" },
          { value: "b", label: "B" },
        ]}
      />
    );
    expect(screen.getByLabelText("B")).toBeDisabled();
  });

  it("Select renders mono style and accepts change", () => {
    const onChange = vi.fn();
    render(
      <Select aria-label="sel" mono onChange={onChange}>
        <option value="a">A</option>
        <option value="b">B</option>
      </Select>
    );
    fireEvent.change(screen.getByLabelText("sel"), { target: { value: "b" } });
    expect(onChange).toHaveBeenCalled();
  });

  it("Textarea forwards props", () => {
    render(<Textarea aria-label="ta" defaultValue="hi" />);
    expect(screen.getByLabelText("ta")).toHaveValue("hi");
  });

  it("FormField renders label + hint", () => {
    render(
      <FormField label="L" hint="H">
        <div>child</div>
      </FormField>
    );
    expect(screen.getByText("L")).toBeInTheDocument();
    expect(screen.getByText("H")).toBeInTheDocument();
    expect(screen.getByText("child")).toBeInTheDocument();
  });

  it("Dialog calls onOpenChange from overlay and Escape", async () => {
    const onOpenChange = vi.fn();
    render(
      <Dialog open title="T" description="D" onOpenChange={onOpenChange}>
        <div>content</div>
      </Dialog>
    );

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    fireEvent.click(document.querySelector(".bg-black\\/30") as HTMLElement);
    expect(onOpenChange).toHaveBeenCalledWith(false);

    onOpenChange.mockClear();
    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });
});
