import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ImageGenParamsPanel } from "../ImageGenParamsPanel";
import { DEFAULT_IMAGE_GEN_PARAMS } from "../useImageGenController";
import { makeController } from "./testUtils";

describe("pages/image-gen/ImageGenParamsPanel", () => {
  it("renders connection and params cards with editable fields", () => {
    const controller = makeController();
    render(<ImageGenParamsPanel controller={controller} />);

    expect(screen.getByRole("heading", { name: "连接配置" })).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "生成参数" })).toBeInTheDocument();
    expect(screen.getByText("未配置")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("Base URL"), {
      target: { value: "api.example.com" },
    });
    expect(controller.setBaseUrl).toHaveBeenCalledWith("api.example.com");

    fireEvent.change(screen.getByLabelText("API Key"), { target: { value: "sk-1" } });
    expect(controller.setApiKeyDraft).toHaveBeenCalledWith("sk-1");

    fireEvent.change(screen.getByLabelText("模型"), {
      target: { value: "gpt-image-2-2026-04-21" },
    });
    expect(controller.setModel).toHaveBeenCalledWith("gpt-image-2-2026-04-21");
  });

  it("auto-saves the config when any connection input loses focus", () => {
    const controller = makeController();
    render(<ImageGenParamsPanel controller={controller} />);

    fireEvent.blur(screen.getByLabelText("Base URL"));
    expect(controller.autoSaveConfig).toHaveBeenCalledTimes(1);

    fireEvent.blur(screen.getByLabelText("API Key"));
    expect(controller.autoSaveConfig).toHaveBeenCalledTimes(2);

    fireEvent.blur(screen.getByLabelText("模型"));
    expect(controller.autoSaveConfig).toHaveBeenCalledTimes(3);
  });

  it("shows the configured state and the request url preview", () => {
    const controller = makeController({
      apiKeyConfigured: true,
      requestUrlPreview: "https://api.example.com/v1/images/generations",
    });
    render(<ImageGenParamsPanel controller={controller} />);

    expect(screen.getByText("已配置")).toBeInTheDocument();
    expect(screen.getByLabelText("API Key")).toHaveAttribute(
      "placeholder",
      "已配置（输入新值可替换）"
    );
    expect(
      screen.getByText("请求 URL：https://api.example.com/v1/images/generations")
    ).toBeInTheDocument();
  });

  it("disables compression for png and updates it for jpeg", () => {
    const pngController = makeController();
    const { unmount } = render(<ImageGenParamsPanel controller={pngController} />);
    expect(screen.getByLabelText("压缩率")).toBeDisabled();
    unmount();

    const jpegController = makeController({
      params: { ...DEFAULT_IMAGE_GEN_PARAMS, outputFormat: "jpeg" },
    });
    const jpegRender = render(<ImageGenParamsPanel controller={jpegController} />);
    const compression = screen.getByLabelText("压缩率");
    expect(compression).toBeEnabled();

    fireEvent.change(compression, { target: { value: "80" } });
    expect(jpegController.updateParams).toHaveBeenCalledWith({ outputCompression: 80 });

    fireEvent.change(compression, { target: { value: "150" } });
    expect(jpegController.updateParams).toHaveBeenCalledWith({ outputCompression: 100 });
    jpegRender.unmount();

    // 受控值需非空才能触发清空事件：以 80 起始再清空。
    const presetController = makeController({
      params: { ...DEFAULT_IMAGE_GEN_PARAMS, outputFormat: "jpeg", outputCompression: 80 },
    });
    render(<ImageGenParamsPanel controller={presetController} />);
    fireEvent.change(screen.getByLabelText("压缩率"), { target: { value: "" } });
    expect(presetController.updateParams).toHaveBeenCalledWith({ outputCompression: null });
  });

  it("updates size, quality, format, moderation and clamps n", () => {
    const controller = makeController();
    render(<ImageGenParamsPanel controller={controller} />);

    fireEvent.change(screen.getByLabelText("尺寸"), { target: { value: "1024x1024" } });
    expect(controller.updateParams).toHaveBeenCalledWith({ size: "1024x1024" });

    fireEvent.change(screen.getByLabelText("质量"), { target: { value: "high" } });
    expect(controller.updateParams).toHaveBeenCalledWith({ quality: "high" });

    fireEvent.change(screen.getByLabelText("格式"), { target: { value: "webp" } });
    expect(controller.updateParams).toHaveBeenCalledWith({ outputFormat: "webp" });

    fireEvent.change(screen.getByLabelText("审核"), { target: { value: "low" } });
    expect(controller.updateParams).toHaveBeenCalledWith({ moderation: "low" });

    fireEvent.change(screen.getByLabelText("数量"), { target: { value: "5" } });
    expect(controller.updateParams).toHaveBeenCalledWith({ n: 5 });

    fireEvent.change(screen.getByLabelText("数量"), { target: { value: "99" } });
    expect(controller.updateParams).toHaveBeenCalledWith({ n: 10 });
  });

  it("renders the storage card with directory, usage and task count", () => {
    const controller = makeController({
      storage: { dir: "/Users/tester/.aio-coding-hub/image-gen", totalBytes: 1536, taskCount: 3 },
    });
    render(<ImageGenParamsPanel controller={controller} />);

    expect(screen.getByRole("heading", { name: "存储" })).toBeInTheDocument();
    const dir = screen.getByText("/Users/tester/.aio-coding-hub/image-gen");
    expect(dir).toHaveAttribute("title", "/Users/tester/.aio-coding-hub/image-gen");
    expect(screen.getByText("占用 1.5 KB · 3 条任务")).toBeInTheDocument();
  });

  it("renders storage placeholders before stats load", () => {
    render(<ImageGenParamsPanel controller={makeController({ storage: null })} />);
    expect(screen.getByText("占用 — · 0 条任务")).toBeInTheDocument();
    expect(screen.getByText("—")).toBeInTheDocument();
  });

  it("invokes the change-directory storage action", () => {
    const controller = makeController({
      storage: { dir: "/store", totalBytes: 0, taskCount: 0 },
    });
    render(<ImageGenParamsPanel controller={controller} />);

    fireEvent.click(screen.getByRole("button", { name: "更改目录" }));
    expect(controller.changeStorageDir).toHaveBeenCalled();
  });

  it("cleans up history only after confirmation", () => {
    const controller = makeController({
      storage: { dir: "/store", totalBytes: 0, taskCount: 60 },
    });
    render(<ImageGenParamsPanel controller={controller} />);

    // 点卡片清理只开确认弹窗。
    fireEvent.click(screen.getByRole("button", { name: "清理" }));
    expect(controller.cleanupStorage).not.toHaveBeenCalled();
    expect(screen.getByText("清理历史任务")).toBeInTheDocument();

    fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "清理" }));
    expect(controller.cleanupStorage).toHaveBeenCalledTimes(1);
  });

  it("cancels the cleanup from the confirm dialog", () => {
    const controller = makeController({
      storage: { dir: "/store", totalBytes: 0, taskCount: 60 },
    });
    render(<ImageGenParamsPanel controller={controller} />);

    fireEvent.click(screen.getByRole("button", { name: "清理" }));
    fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "取消" }));
    expect(controller.cleanupStorage).not.toHaveBeenCalled();
  });
});
