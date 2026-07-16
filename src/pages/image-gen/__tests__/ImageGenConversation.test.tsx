import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ImageGenConversation } from "../ImageGenConversation";
import type {
  ImageGenAssistantMessage,
  ImageGenGeneratedImage,
  ImageGenReferenceImage,
  ImageGenUserMessage,
} from "../useImageGenController";
import type { GptImageRequest } from "../../../services/image-gen/gptImageAdapter";
import { makeController } from "./testUtils";

const REQUEST: GptImageRequest = {
  prompt: "一只猫",
  referenceImages: [],
  n: 1,
  size: "auto",
  options: {
    model: "gpt-image-2",
    quality: "auto",
    outputFormat: "png",
    outputCompression: null,
    moderation: "auto",
  },
};

function makeImage(): ImageGenGeneratedImage {
  return {
    objectUrl: "blob:generated-1",
    mime: "image/png",
    blob: new Blob(["x"], { type: "image/png" }),
  };
}

function userMessage(overrides: Partial<ImageGenUserMessage> = {}): ImageGenUserMessage {
  return { id: "u1", role: "user", prompt: "一只猫", refThumbs: [], ...overrides };
}

function assistantMessage(
  overrides: Partial<ImageGenAssistantMessage> = {}
): ImageGenAssistantMessage {
  return {
    id: "a1",
    role: "assistant",
    status: "done",
    images: [],
    request: REQUEST,
    ...overrides,
  };
}

function referenceImage(overrides: Partial<ImageGenReferenceImage> = {}): ImageGenReferenceImage {
  return {
    id: "r1",
    mime: "image/png",
    b64: "AAA",
    sizeBytes: 3,
    objectUrl: "blob:ref-1",
    ...overrides,
  };
}

describe("pages/image-gen/ImageGenConversation", () => {
  it("renders the empty state without messages", () => {
    render(<ImageGenConversation controller={makeController()} />);
    expect(screen.getByText("还没有生成记录")).toBeInTheDocument();
  });

  it("renders user messages with prompt and reference thumbnails", () => {
    const controller = makeController({
      messages: [userMessage({ refThumbs: ["blob:thumb-1"] })],
    });
    render(<ImageGenConversation controller={controller} />);
    expect(screen.getByText("一只猫")).toBeInTheDocument();
    expect(screen.getByAltText("参考图 1")).toHaveAttribute("src", "blob:thumb-1");
  });

  it("keeps submit enabled while a message is generating", () => {
    const controller = makeController({
      messages: [assistantMessage({ status: "loading" })],
      prompt: "下一张",
    });
    render(<ImageGenConversation controller={controller} />);
    expect(screen.getByLabelText("Loading")).toBeInTheDocument();
    const submitButton = screen.getByRole("button", { name: "生成" });
    expect(submitButton).toBeEnabled();
    fireEvent.click(submitButton);
    expect(controller.submit).toHaveBeenCalled();
  });

  it("shows the error message and retries the failed message", () => {
    const controller = makeController({
      messages: [assistantMessage({ status: "error", error: "HTTP 500: boom" })],
    });
    render(<ImageGenConversation controller={controller} />);
    expect(screen.getByText("HTTP 500: boom")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "重试" }));
    expect(controller.retry).toHaveBeenCalledWith("a1");
  });

  it("renders generated images with usage and image actions", () => {
    const image = makeImage();
    const controller = makeController({
      messages: [
        assistantMessage({
          images: [image],
          usage: { inputTokens: 10, outputTokens: 20, totalTokens: 30 },
        }),
      ],
    });
    render(<ImageGenConversation controller={controller} />);

    expect(screen.getByAltText("生成图片 1")).toHaveAttribute("src", "blob:generated-1");
    expect(screen.getByText("tokens：输入 10 · 输出 20 · 合计 30")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "下载" }));
    expect(controller.downloadImage).toHaveBeenCalledWith(image);

    fireEvent.click(screen.getByRole("button", { name: "设为参考图" }));
    expect(controller.setAsReference).toHaveBeenCalledWith(image);
  });

  it("omits the usage line when usage is missing", () => {
    const controller = makeController({
      messages: [assistantMessage({ images: [makeImage()] })],
    });
    render(<ImageGenConversation controller={controller} />);
    expect(screen.queryByText(/tokens：/)).not.toBeInTheDocument();
  });

  it("opens the preview when a generated image is clicked", () => {
    const controller = makeController({
      messages: [assistantMessage({ images: [makeImage()] })],
    });
    render(<ImageGenConversation controller={controller} />);

    fireEvent.click(screen.getByRole("button", { name: "预览生成图片 1" }));
    expect(controller.openPreview).toHaveBeenCalledWith(["blob:generated-1"], 0);
  });

  it("renders the lightbox when a preview is active", () => {
    const controller = makeController({
      preview: { urls: ["blob:generated-1"], index: 0 },
    });
    render(<ImageGenConversation controller={controller} />);
    expect(screen.getByAltText("预览图片 1")).toHaveAttribute("src", "blob:generated-1");
  });

  it("lists pending reference images and removes one", () => {
    const controller = makeController({ referenceImages: [referenceImage()] });
    render(<ImageGenConversation controller={controller} />);
    expect(screen.getByAltText("参考图 1")).toHaveAttribute("src", "blob:ref-1");
    fireEvent.click(screen.getByRole("button", { name: "移除参考图 1" }));
    expect(controller.removeReferenceImage).toHaveBeenCalledWith("r1");
  });

  it("edits the prompt and submits", () => {
    const controller = makeController({ prompt: "一只狗" });
    render(<ImageGenConversation controller={controller} />);

    fireEvent.change(screen.getByLabelText("提示词"), { target: { value: "一只狗在跑" } });
    expect(controller.setPrompt).toHaveBeenCalledWith("一只狗在跑");

    fireEvent.click(screen.getByRole("button", { name: "生成" }));
    expect(controller.submit).toHaveBeenCalled();
  });

  it("disables submit when the prompt is empty", () => {
    render(<ImageGenConversation controller={makeController({ prompt: "  " })} />);
    expect(screen.getByRole("button", { name: "生成" })).toBeDisabled();
  });

  it("forwards selected files to addReferenceFiles", () => {
    const controller = makeController();
    render(<ImageGenConversation controller={controller} />);

    // 触发文件选择按钮（对隐藏 input 的 click 代理）。
    fireEvent.click(screen.getByRole("button", { name: "参考图" }));

    const file = new File(["x"], "ref.png", { type: "image/png" });
    fireEvent.change(screen.getByLabelText("上传参考图"), { target: { files: [file] } });
    expect(controller.addReferenceFiles).toHaveBeenCalled();
  });
});
