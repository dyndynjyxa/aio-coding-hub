import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ConfirmDialog } from "../ConfirmDialog";

describe("ui/ConfirmDialog", () => {
  it("renders content and forwards confirm and close actions", () => {
    const onClose = vi.fn();
    const onConfirm = vi.fn();

    render(
      <ConfirmDialog
        open={true}
        title="Delete provider"
        description="This cannot be undone."
        confirmLabel="Delete"
        confirmingLabel="Deleting"
        confirming={false}
        onClose={onClose}
        onConfirm={onConfirm}
      >
        <p>Provider name</p>
      </ConfirmDialog>
    );

    const dialog = screen.getByRole("dialog");
    expect(within(dialog).getByText("Provider name")).toBeInTheDocument();

    fireEvent.click(within(dialog).getByRole("button", { name: "Delete" }));
    expect(onConfirm).toHaveBeenCalledTimes(1);

    const buttons = within(dialog).getAllByRole("button");
    fireEvent.click(buttons[1]);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it("shows confirming state and disables the confirm action", () => {
    const onConfirm = vi.fn();

    render(
      <ConfirmDialog
        open={true}
        title="Delete provider"
        confirmLabel="Delete"
        confirmingLabel="Deleting"
        confirming={true}
        onClose={() => {}}
        onConfirm={onConfirm}
      />
    );

    const confirmButton = screen.getByRole("button", { name: "Deleting" });
    expect(confirmButton).toBeDisabled();

    fireEvent.click(confirmButton);
    expect(onConfirm).not.toHaveBeenCalled();
  });

  it("applies explicit disabled and confirm variant options", () => {
    render(
      <ConfirmDialog
        open={true}
        title="Reset settings"
        confirmLabel="Reset"
        confirmingLabel="Resetting"
        confirming={false}
        disabled={true}
        confirmVariant="danger"
        onClose={() => {}}
        onConfirm={() => {}}
      />
    );

    const confirmButton = screen.getByRole("button", { name: "Reset" });
    expect(confirmButton).toBeDisabled();
    expect(confirmButton).toHaveClass("text-destructive");
  });
});
