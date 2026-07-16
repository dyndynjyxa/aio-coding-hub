import { fireEvent, render, screen } from "@testing-library/react";
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

    fireEvent.click(screen.getByRole("button", { name: "保存配置" }));
    expect(controller.saveConfig).toHaveBeenCalled();
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

  it("disables the save button while saving", () => {
    render(<ImageGenParamsPanel controller={makeController({ savingConfig: true })} />);
    expect(screen.getByRole("button", { name: "保存中…" })).toBeDisabled();
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
});
