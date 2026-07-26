import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { ImageGenLightbox } from "../ImageGenLightbox";

describe("pages/image-gen/ImageGenLightbox", () => {
  it("renders nothing when preview is null", () => {
    const { container } = render(
      <ImageGenLightbox preview={null} onClose={vi.fn()} onStep={vi.fn()} />
    );
    expect(container).toBeEmptyDOMElement();
  });

  it("shows the current image with a counter and steps via buttons", () => {
    const onStep = vi.fn();
    render(
      <ImageGenLightbox
        preview={{ urls: ["blob:a", "blob:b", "blob:c"], index: 1 }}
        onClose={vi.fn()}
        onStep={onStep}
      />
    );

    expect(screen.getByAltText("预览图片 2")).toHaveAttribute("src", "blob:b");
    expect(screen.getByText("2 / 3")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "上一张" }));
    expect(onStep).toHaveBeenCalledWith(-1);
    fireEvent.click(screen.getByRole("button", { name: "下一张" }));
    expect(onStep).toHaveBeenCalledWith(1);
  });

  it("hides navigation for a single image", () => {
    render(
      <ImageGenLightbox
        preview={{ urls: ["blob:a"], index: 0 }}
        onClose={vi.fn()}
        onStep={vi.fn()}
      />
    );
    expect(screen.queryByRole("button", { name: "下一张" })).not.toBeInTheDocument();
  });

  it("steps with arrow keys and closes on Escape", () => {
    const onClose = vi.fn();
    const onStep = vi.fn();
    render(
      <ImageGenLightbox
        preview={{ urls: ["blob:a", "blob:b"], index: 0 }}
        onClose={onClose}
        onStep={onStep}
      />
    );

    const dialog = screen.getByRole("dialog");
    fireEvent.keyDown(dialog, { key: "ArrowRight" });
    expect(onStep).toHaveBeenCalledWith(1);
    fireEvent.keyDown(dialog, { key: "ArrowLeft" });
    expect(onStep).toHaveBeenCalledWith(-1);

    fireEvent.keyDown(dialog, { key: "Escape" });
    expect(onClose).toHaveBeenCalled();
  });

  it("closes when clicking the blank area but not the image", () => {
    const onClose = vi.fn();
    render(
      <ImageGenLightbox
        preview={{ urls: ["blob:a"], index: 0 }}
        onClose={onClose}
        onStep={vi.fn()}
      />
    );

    fireEvent.click(screen.getByAltText("预览图片 1"));
    expect(onClose).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("dialog"));
    expect(onClose).toHaveBeenCalled();
  });
});
