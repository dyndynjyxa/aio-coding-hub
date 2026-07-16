import { act, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ImageGenTaskPanel } from "../ImageGenTaskPanel";
import type { ImageGenReferenceImage } from "../useImageGenController";
import { makeController, makeDiskImage, makeMemoryImage, makeTask } from "./testUtils";

const makeImage = makeMemoryImage;

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

describe("pages/image-gen/ImageGenTaskPanel", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("renders the empty state without tasks and hides the toolbar", () => {
    render(<ImageGenTaskPanel controller={makeController()} />);
    expect(screen.getByText("还没有生成记录")).toBeInTheDocument();
    expect(screen.queryByLabelText("搜索提示词")).not.toBeInTheDocument();
  });

  it("renders a done task card with thumbnail, chips, elapsed and +N badge", () => {
    const controller = makeController({
      tasks: [
        makeTask({
          images: [makeImage("blob:a"), makeImage("blob:b"), makeImage("blob:c")],
          elapsedMs: 186_000,
        }),
      ],
    });
    render(<ImageGenTaskPanel controller={controller} />);

    expect(screen.getByAltText("任务缩略图：一只猫")).toHaveAttribute("src", "blob:a");
    expect(screen.getByText("+2")).toBeInTheDocument();
    expect(screen.getByText("一只猫")).toBeInTheDocument();
    // chips：尺寸 + 质量（默认参数均为 auto，两枚 chip）
    expect(screen.getAllByText("auto")).toHaveLength(2);
    expect(screen.getByText("耗时 3m6.0s")).toBeInTheDocument();
  });

  it("opens the detail dialog when a card is clicked", () => {
    const controller = makeController({ tasks: [makeTask({ id: "t9" })] });
    render(<ImageGenTaskPanel controller={controller} />);
    fireEvent.click(screen.getByRole("button", { name: "查看任务详情：一只猫" }));
    expect(controller.openDetail).toHaveBeenCalledWith("t9");
  });

  it("shows a ticking timer on a loading card", () => {
    vi.useFakeTimers();
    const controller = makeController({
      tasks: [makeTask({ status: "loading", startedAt: Date.now(), images: [] })],
    });
    render(<ImageGenTaskPanel controller={controller} />);
    expect(screen.getByText("0ms")).toBeInTheDocument();

    act(() => {
      vi.advanceTimersByTime(61_000);
    });
    expect(screen.getByText("1m1.0s")).toBeInTheDocument();
  });

  it("cancels a loading task directly without confirmation", () => {
    const controller = makeController({
      tasks: [makeTask({ id: "l1", status: "loading", images: [] })],
    });
    render(<ImageGenTaskPanel controller={controller} />);

    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    // 直删无确认弹窗。
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(controller.deleteTask).toHaveBeenCalledWith("l1");
  });

  it("renders an error card and deletes only after confirmation", () => {
    const controller = makeController({
      tasks: [makeTask({ id: "e1", status: "error", error: "HTTP 500: boom", images: [] })],
    });
    render(<ImageGenTaskPanel controller={controller} />);

    expect(screen.getByText("HTTP 500: boom")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "重试" }));
    expect(controller.retry).toHaveBeenCalledWith("e1");

    // 删除需二次确认：点卡片删除只开弹窗。
    fireEvent.click(screen.getByRole("button", { name: "删除" }));
    expect(controller.deleteTask).not.toHaveBeenCalled();
    expect(screen.getByText("删除任务")).toBeInTheDocument();
    fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "删除" }));
    expect(controller.deleteTask).toHaveBeenCalledWith("e1");

    // error 卡也可点开详情
    fireEvent.click(screen.getByRole("button", { name: "查看任务详情：一只猫" }));
    expect(controller.openDetail).toHaveBeenCalledWith("e1");
  });

  it("cancels the error-card deletion from the confirm dialog", () => {
    const controller = makeController({
      tasks: [makeTask({ id: "e1", status: "error", error: "boom", images: [] })],
    });
    render(<ImageGenTaskPanel controller={controller} />);

    fireEvent.click(screen.getByRole("button", { name: "删除" }));
    fireEvent.click(within(screen.getByRole("dialog")).getByRole("button", { name: "取消" }));
    expect(controller.deleteTask).not.toHaveBeenCalled();
  });

  it("filters rendered cards by search query (case-insensitive) and shows newest first", () => {
    const controller = makeController({
      tasks: [makeTask({ id: "t1", prompt: "A Cat" }), makeTask({ id: "t2", prompt: "一只狗" })],
      searchQuery: "cat",
    });
    render(<ImageGenTaskPanel controller={controller} />);
    expect(screen.getByText("A Cat")).toBeInTheDocument();
    expect(screen.queryByText("一只狗")).not.toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("搜索提示词"), { target: { value: "狗" } });
    expect(controller.setSearchQuery).toHaveBeenCalledWith("狗");
  });

  it("filters rendered cards by status and shows the no-match empty state", () => {
    const controller = makeController({
      tasks: [makeTask({ id: "t1", status: "done" })],
      statusFilter: "error",
    });
    render(<ImageGenTaskPanel controller={controller} />);
    expect(screen.getByText("没有匹配的任务")).toBeInTheDocument();

    fireEvent.change(screen.getByLabelText("状态筛选"), { target: { value: "done" } });
    expect(controller.setStatusFilter).toHaveBeenCalledWith("done");
  });

  it("renders newest tasks first in the grid", () => {
    const controller = makeController({
      tasks: [makeTask({ id: "old", prompt: "旧任务" }), makeTask({ id: "new", prompt: "新任务" })],
    });
    render(<ImageGenTaskPanel controller={controller} />);
    const cards = screen.getAllByRole("button", { name: /查看任务详情：/ });
    expect(cards[0]).toHaveAccessibleName("查看任务详情：新任务");
    expect(cards[1]).toHaveAccessibleName("查看任务详情：旧任务");
  });

  it("clears all tasks after confirmation", () => {
    const controller = makeController({ tasks: [makeTask()] });
    render(<ImageGenTaskPanel controller={controller} />);

    fireEvent.click(screen.getByRole("button", { name: "清空任务" }));
    expect(controller.clearTasks).not.toHaveBeenCalled();

    fireEvent.click(screen.getByRole("button", { name: "清空" }));
    expect(controller.clearTasks).toHaveBeenCalled();
  });

  it("cancels task clearing from the confirm dialog", () => {
    const controller = makeController({ tasks: [makeTask()] });
    render(<ImageGenTaskPanel controller={controller} />);

    fireEvent.click(screen.getByRole("button", { name: "清空任务" }));
    fireEvent.click(screen.getByRole("button", { name: "取消" }));
    expect(controller.clearTasks).not.toHaveBeenCalled();
  });

  it("keeps submit enabled while a task is generating", () => {
    const controller = makeController({
      tasks: [makeTask({ status: "loading", images: [] })],
      prompt: "下一张",
    });
    render(<ImageGenTaskPanel controller={controller} />);
    expect(screen.getByLabelText("Loading")).toBeInTheDocument();
    const submitButton = screen.getByRole("button", { name: "生成" });
    expect(submitButton).toBeEnabled();
    fireEvent.click(submitButton);
    expect(controller.submit).toHaveBeenCalled();
  });

  it("renders the lightbox when a preview is active", () => {
    const controller = makeController({
      preview: { urls: ["blob:generated-1"], index: 0 },
    });
    render(<ImageGenTaskPanel controller={controller} />);
    expect(screen.getByAltText("预览图片 1")).toHaveAttribute("src", "blob:generated-1");
  });

  it("lists pending reference images and removes one", () => {
    const controller = makeController({ referenceImages: [referenceImage()] });
    render(<ImageGenTaskPanel controller={controller} />);
    expect(screen.getByAltText("参考图 1")).toHaveAttribute("src", "blob:ref-1");
    fireEvent.click(screen.getByRole("button", { name: "移除参考图 1" }));
    expect(controller.removeReferenceImage).toHaveBeenCalledWith("r1");
  });

  it("edits the prompt and submits", () => {
    const controller = makeController({ prompt: "一只狗" });
    render(<ImageGenTaskPanel controller={controller} />);

    fireEvent.change(screen.getByLabelText("提示词"), { target: { value: "一只狗在跑" } });
    expect(controller.setPrompt).toHaveBeenCalledWith("一只狗在跑");

    fireEvent.click(screen.getByRole("button", { name: "生成" }));
    expect(controller.submit).toHaveBeenCalled();
  });

  it("disables submit when the prompt is empty", () => {
    render(<ImageGenTaskPanel controller={makeController({ prompt: "  " })} />);
    expect(screen.getByRole("button", { name: "生成" })).toBeDisabled();
  });

  it("renders a disk-task thumbnail and falls back to a placeholder when the file is missing", () => {
    const controller = makeController({
      tasks: [makeTask({ images: [makeDiskImage("/store/t1/image-1.png")] })],
    });
    render(<ImageGenTaskPanel controller={controller} />);

    const img = screen.getByAltText("任务缩略图：一只猫");
    expect(img).toHaveAttribute("src", "asset://localhost//store/t1/thumb-1.png");

    // 文件被外部删除：显示占位而非裂图。
    fireEvent.error(img);
    expect(screen.getByText("文件缺失")).toBeInTheDocument();
    expect(screen.queryByAltText("任务缩略图：一只猫")).not.toBeInTheDocument();
  });

  it("loads more history from the grid bottom when more is available", () => {
    const controller = makeController({ tasks: [makeTask()], hasMore: true });
    render(<ImageGenTaskPanel controller={controller} />);

    fireEvent.click(screen.getByRole("button", { name: "加载更多" }));
    expect(controller.loadMoreTasks).toHaveBeenCalled();
  });

  it("hides the load-more button when all history is loaded", () => {
    render(<ImageGenTaskPanel controller={makeController({ tasks: [makeTask()] })} />);
    expect(screen.queryByRole("button", { name: "加载更多" })).not.toBeInTheDocument();
  });

  it("advertises paste/drag and the Ctrl+Enter shortcut in the prompt placeholder", () => {
    render(<ImageGenTaskPanel controller={makeController()} />);
    expect(screen.getByLabelText("提示词")).toHaveAttribute(
      "placeholder",
      "描述你想生成的图片…（支持粘贴 / 拖拽参考图，Ctrl+Enter 生成）"
    );
  });

  it("submits on Ctrl+Enter and Cmd+Enter when the prompt is non-empty", () => {
    const controller = makeController({ prompt: "一只猫" });
    render(<ImageGenTaskPanel controller={controller} />);
    const textarea = screen.getByLabelText("提示词");

    fireEvent.keyDown(textarea, { key: "Enter", ctrlKey: true });
    expect(controller.submit).toHaveBeenCalledTimes(1);

    fireEvent.keyDown(textarea, { key: "Enter", metaKey: true });
    expect(controller.submit).toHaveBeenCalledTimes(2);

    // 普通 Enter 不提交（保留换行）。
    fireEvent.keyDown(textarea, { key: "Enter" });
    expect(controller.submit).toHaveBeenCalledTimes(2);
  });

  it("ignores Ctrl+Enter when the prompt is blank", () => {
    const controller = makeController({ prompt: "  " });
    render(<ImageGenTaskPanel controller={controller} />);
    fireEvent.keyDown(screen.getByLabelText("提示词"), { key: "Enter", ctrlKey: true });
    expect(controller.submit).not.toHaveBeenCalled();
  });

  it("highlights the drop zone on file drag over and clears on leave", () => {
    render(<ImageGenTaskPanel controller={makeController()} />);
    const zone = screen.getByTestId("image-gen-drop-zone");

    fireEvent.dragOver(zone, { dataTransfer: { types: ["Files"] } });
    expect(zone.className).toContain("ring-1");

    fireEvent.dragLeave(zone);
    expect(zone.className).not.toContain("ring-1");
  });

  it("does not highlight when the drag carries no files", () => {
    render(<ImageGenTaskPanel controller={makeController()} />);
    const zone = screen.getByTestId("image-gen-drop-zone");
    fireEvent.dragOver(zone, { dataTransfer: { types: ["text/plain"] } });
    expect(zone.className).not.toContain("ring-1");
  });

  it("forwards dropped image files to addReferenceFiles and clears the highlight", () => {
    const controller = makeController();
    render(<ImageGenTaskPanel controller={controller} />);
    const zone = screen.getByTestId("image-gen-drop-zone");
    const image = new File(["x"], "drop.png", { type: "image/png" });
    const text = new File(["x"], "note.txt", { type: "text/plain" });

    fireEvent.dragOver(zone, { dataTransfer: { types: ["Files"] } });
    fireEvent.drop(zone, { dataTransfer: { types: ["Files"], files: [image, text] } });

    expect(controller.addReferenceFiles).toHaveBeenCalledWith([image]);
    expect(zone.className).not.toContain("ring-1");
  });

  it("ignores drops without image files", () => {
    const controller = makeController();
    render(<ImageGenTaskPanel controller={controller} />);
    const zone = screen.getByTestId("image-gen-drop-zone");
    const text = new File(["x"], "note.txt", { type: "text/plain" });

    fireEvent.drop(zone, { dataTransfer: { types: ["Files"], files: [text] } });
    expect(controller.addReferenceFiles).not.toHaveBeenCalled();
  });

  it("forwards selected files to addReferenceFiles", () => {
    const controller = makeController();
    render(<ImageGenTaskPanel controller={controller} />);

    // 触发文件选择按钮（对隐藏 input 的 click 代理）。
    fireEvent.click(screen.getByRole("button", { name: "参考图" }));

    const file = new File(["x"], "ref.png", { type: "image/png" });
    fireEvent.change(screen.getByLabelText("上传参考图"), { target: { files: [file] } });
    expect(controller.addReferenceFiles).toHaveBeenCalled();
  });
});
