import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { copyText } from "../../../services/clipboard";
import { ImageGenTaskDetail } from "../ImageGenTaskDetail";
import type { ImageGenGeneratedImage } from "../useImageGenController";
import { makeController, makeTask, TEST_REQUEST } from "./testUtils";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

vi.mock("../../../services/clipboard", () => ({
  copyText: vi.fn(async () => {}),
}));

function makeImage(objectUrl: string): ImageGenGeneratedImage {
  return {
    objectUrl,
    mime: "image/png",
    blob: new Blob(["x"], { type: "image/png" }),
  };
}

describe("pages/image-gen/ImageGenTaskDetail", () => {
  it("renders nothing when no detail task is open", () => {
    render(<ImageGenTaskDetail controller={makeController()} />);
    expect(screen.queryByText("任务详情")).not.toBeInTheDocument();
  });

  it("renders the parameter snapshot, timestamps, tokens and estimated cost", () => {
    const task = makeTask({
      images: [makeImage("blob:a")],
      request: {
        ...TEST_REQUEST,
        n: 3,
        size: "1024x1536",
        options: {
          ...TEST_REQUEST.options,
          quality: "high",
          outputFormat: "jpeg",
          outputCompression: 80,
        },
      },
      usage: {
        inputTokens: 100,
        outputTokens: 1000,
        totalTokens: 1100,
        inputTokensDetails: { textTokens: 60, imageTokens: 40 },
      },
      elapsedMs: 186_000,
    });
    render(<ImageGenTaskDetail controller={makeController({ detailTask: task })} />);

    expect(screen.getByText("任务详情")).toBeInTheDocument();
    expect(screen.getByText("gpt-image-2")).toBeInTheDocument();
    expect(screen.getByText("1024x1536")).toBeInTheDocument();
    expect(screen.getByText("high")).toBeInTheDocument();
    expect(screen.getByText("jpeg")).toBeInTheDocument();
    expect(screen.getByText("80")).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
    expect(screen.getByText(/创建于 .+ · 耗时 03:06/)).toBeInTheDocument();
    expect(
      screen.getByText("tokens：输入 100（文本 60 / 图像 40） · 输出 1000 · 合计 1100")
    ).toBeInTheDocument();
    // (60*5 + 40*8 + 1000*30) / 1e6 = 0.0306
    expect(screen.getByText("预估 $0.0306")).toBeInTheDocument();
  });

  it("hides the compression row for png and omits usage when missing", () => {
    const task = makeTask({ images: [makeImage("blob:a")] });
    render(<ImageGenTaskDetail controller={makeController({ detailTask: task })} />);
    expect(screen.queryByText("压缩率")).not.toBeInTheDocument();
    expect(screen.queryByText(/tokens：/)).not.toBeInTheDocument();
    expect(screen.queryByText(/预估 \$/)).not.toBeInTheDocument();
  });

  it("switches the current image via thumbnails and opens the lightbox from the main image", () => {
    const task = makeTask({ images: [makeImage("blob:a"), makeImage("blob:b")] });
    const controller = makeController({ detailTask: task });
    render(<ImageGenTaskDetail controller={controller} />);

    expect(screen.getByAltText("生成图片 1")).toHaveAttribute("src", "blob:a");
    fireEvent.click(screen.getByRole("button", { name: "切换到第 2 张" }));
    expect(screen.getByAltText("生成图片 2")).toHaveAttribute("src", "blob:b");

    fireEvent.click(screen.getByRole("button", { name: "预览大图" }));
    expect(controller.openPreview).toHaveBeenCalledWith(["blob:a", "blob:b"], 1);
  });

  it("reuses the task config and closes the detail", () => {
    const task = makeTask({ id: "t7", images: [makeImage("blob:a")] });
    const controller = makeController({ detailTask: task });
    render(<ImageGenTaskDetail controller={controller} />);

    fireEvent.click(screen.getByRole("button", { name: "复用配置" }));
    expect(controller.reuseTask).toHaveBeenCalledWith("t7");
    expect(controller.closeDetail).toHaveBeenCalled();
  });

  it("downloads and sets the current image as reference", () => {
    const task = makeTask({ images: [makeImage("blob:a")] });
    const controller = makeController({ detailTask: task });
    render(<ImageGenTaskDetail controller={controller} />);

    fireEvent.click(screen.getByRole("button", { name: "下载" }));
    expect(controller.downloadImage).toHaveBeenCalledWith(task.images[0]);

    fireEvent.click(screen.getByRole("button", { name: "设为参考图" }));
    expect(controller.setAsReference).toHaveBeenCalledWith(task.images[0]);
  });

  it("disables download and reference actions without images", () => {
    const task = makeTask({ status: "error", error: "HTTP 500: boom", images: [] });
    render(<ImageGenTaskDetail controller={makeController({ detailTask: task })} />);
    expect(screen.getByText("HTTP 500: boom")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "下载" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "设为参考图" })).toBeDisabled();
  });

  it("shows a spinner with timer for a loading task", () => {
    const task = makeTask({ status: "loading", startedAt: Date.now(), images: [] });
    render(<ImageGenTaskDetail controller={makeController({ detailTask: task })} />);
    expect(screen.getByLabelText("Loading")).toBeInTheDocument();
    expect(screen.getByText("00:00")).toBeInTheDocument();
  });

  it("deletes the task after confirmation and closes the detail", () => {
    const task = makeTask({ id: "t8", images: [makeImage("blob:a")] });
    const controller = makeController({ detailTask: task });
    render(<ImageGenTaskDetail controller={controller} />);

    fireEvent.click(screen.getByRole("button", { name: "删除任务" }));
    expect(controller.deleteTask).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "删除" }));
    expect(controller.deleteTask).toHaveBeenCalledWith("t8");
    expect(controller.closeDetail).toHaveBeenCalled();
  });

  it("cancels deletion from the confirm dialog", () => {
    const task = makeTask({ images: [makeImage("blob:a")] });
    const controller = makeController({ detailTask: task });
    render(<ImageGenTaskDetail controller={controller} />);

    fireEvent.click(screen.getByRole("button", { name: "删除任务" }));
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(controller.deleteTask).not.toHaveBeenCalled();
    expect(controller.closeDetail).not.toHaveBeenCalled();
  });

  it("copies the prompt and renders reference thumbnails", async () => {
    const task = makeTask({ images: [makeImage("blob:a")], refThumbs: ["blob:ref-1"] });
    render(<ImageGenTaskDetail controller={makeController({ detailTask: task })} />);

    expect(screen.getByAltText("参考图 1")).toHaveAttribute("src", "blob:ref-1");

    fireEvent.click(screen.getByRole("button", { name: "复制" }));
    expect(copyText).toHaveBeenCalledWith("一只猫");
    await waitFor(() => {
      expect(toast.success).toHaveBeenCalledWith("已复制提示词");
    });
  });
});
