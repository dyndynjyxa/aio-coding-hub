import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { TabList } from "../TabList";

describe("ui/TabList", () => {
  const defaultItems = [
    { key: "tab1", label: "Tab One" },
    { key: "tab2", label: "Tab Two" },
    { key: "tab3", label: "Tab Three" },
  ] as const;

  it("renders a tablist with all tab items", () => {
    render(
      <TabList ariaLabel="test tabs" items={[...defaultItems]} value="tab1" onChange={() => {}} />
    );
    expect(screen.getByRole("tablist")).toHaveAttribute("aria-label", "test tabs");
    expect(screen.getAllByRole("tab")).toHaveLength(3);
    expect(screen.getByText("Tab One")).toBeInTheDocument();
    expect(screen.getByText("Tab Two")).toBeInTheDocument();
    expect(screen.getByText("Tab Three")).toBeInTheDocument();
  });

  it("marks the active tab with aria-selected=true", () => {
    render(<TabList ariaLabel="tabs" items={[...defaultItems]} value="tab2" onChange={() => {}} />);
    const tabs = screen.getAllByRole("tab");
    expect(tabs[0]).toHaveAttribute("aria-selected", "false");
    expect(tabs[1]).toHaveAttribute("aria-selected", "true");
    expect(tabs[2]).toHaveAttribute("aria-selected", "false");
  });

  it("calls onChange with the clicked tab key", () => {
    const onChange = vi.fn();
    render(<TabList ariaLabel="tabs" items={[...defaultItems]} value="tab1" onChange={onChange} />);
    fireEvent.click(screen.getByText("Tab Three"));
    expect(onChange).toHaveBeenCalledWith("tab3");
  });

  it("disables individual tabs", () => {
    const onChange = vi.fn();
    const items = [
      { key: "a", label: "A" },
      { key: "b", label: "B", disabled: true },
    ];
    render(<TabList ariaLabel="tabs" items={items} value="a" onChange={onChange} />);
    const tabs = screen.getAllByRole("tab");
    expect(tabs[1]).toBeDisabled();
    fireEvent.click(tabs[1]);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("applies active and inactive tab classes", () => {
    render(<TabList ariaLabel="tabs" items={[...defaultItems]} value="tab1" onChange={() => {}} />);
    const tabs = screen.getAllByRole("tab");
    expect(tabs[0]).toHaveClass("bg-primary");
    expect(tabs[1]).not.toHaveClass("bg-primary");
  });

  it("supports keyboard tab navigation", () => {
    const onChange = vi.fn();
    render(<TabList ariaLabel="tabs" items={[...defaultItems]} value="tab2" onChange={onChange} />);

    fireEvent.keyDown(screen.getByRole("tablist"), { key: "ArrowRight" });
    expect(onChange).toHaveBeenCalledWith("tab3");

    fireEvent.keyDown(screen.getByRole("tablist"), { key: "Home" });
    expect(onChange).toHaveBeenCalledWith("tab1");
  });

  it("supports reverse and end keyboard navigation", () => {
    const onChange = vi.fn();
    render(<TabList ariaLabel="tabs" items={[...defaultItems]} value="tab1" onChange={onChange} />);

    fireEvent.keyDown(screen.getByRole("tablist"), { key: "ArrowLeft" });
    expect(onChange).toHaveBeenCalledWith("tab3");

    fireEvent.keyDown(screen.getByRole("tablist"), { key: "End" });
    expect(onChange).toHaveBeenCalledWith("tab3");
  });

  it("ignores non-navigation keys and skips disabled tabs", () => {
    const onChange = vi.fn();
    const items = [
      { key: "a", label: "A" },
      { key: "b", label: "B", disabled: true },
      { key: "c", label: "C" },
    ] as const;
    render(<TabList ariaLabel="tabs" items={[...items]} value="a" onChange={onChange} />);

    fireEvent.keyDown(screen.getByRole("tablist"), { key: "Enter" });
    expect(onChange).not.toHaveBeenCalled();

    fireEvent.keyDown(screen.getByRole("tablist"), { key: "ArrowRight" });
    expect(onChange).toHaveBeenCalledWith("c");
  });

  it("does not change tabs when every item is disabled", () => {
    const onChange = vi.fn();
    const items = [{ key: "a", label: "A", disabled: true }] as const;
    render(<TabList ariaLabel="tabs" items={[...items]} value="a" onChange={onChange} />);

    fireEvent.keyDown(screen.getByRole("tablist"), { key: "ArrowRight" });

    expect(onChange).not.toHaveBeenCalled();
  });

  it("merges custom className on the container", () => {
    render(
      <TabList
        ariaLabel="tabs"
        items={[...defaultItems]}
        value="tab1"
        onChange={() => {}}
        className="my-tabs"
      />
    );
    expect(screen.getByRole("tablist")).toHaveClass("my-tabs");
  });

  it("merges buttonClassName on tab buttons", () => {
    render(
      <TabList
        ariaLabel="tabs"
        items={[...defaultItems]}
        value="tab1"
        onChange={() => {}}
        buttonClassName="btn-extra"
      />
    );
    const tabs = screen.getAllByRole("tab");
    tabs.forEach((tab) => {
      expect(tab).toHaveClass("btn-extra");
    });
  });
});
