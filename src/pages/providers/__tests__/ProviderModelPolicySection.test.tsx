import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type {
  CliKey,
  ProviderModelPolicyV1,
  ProviderModelPolicyStatus,
} from "../../../services/providers/providers";
import { ProviderModelPolicySection } from "../ProviderModelPolicySection";
import type { ProviderModelDiscoveryUiState } from "../providerModelPolicy";

const allPolicy: ProviderModelPolicyV1 = {
  version: 1,
  mode: "all",
  modelPatterns: [],
  mappings: [],
};

function renderSection(
  cliKey: CliKey,
  status: ProviderModelPolicyStatus = "ready",
  policy: ProviderModelPolicyV1 | null = allPolicy,
  modelDiscoveryState: ProviderModelDiscoveryUiState = { status: "idle" },
  hasMultipleBaseUrls = false
) {
  const onChange = vi.fn();
  render(
    <ProviderModelPolicySection
      cliKey={cliKey}
      status={status}
      policy={policy}
      saving={false}
      onChange={onChange}
      modelDiscoveryState={modelDiscoveryState}
      onDiscoverModels={vi.fn()}
      hasMultipleBaseUrls={hasMultipleBaseUrls}
    />
  );
  return onChange;
}

describe("pages/providers/ProviderModelPolicySection", () => {
  it.each<CliKey>(["claude", "codex", "gemini", "grok"])(
    "renders the shared model section for %s",
    (cliKey) => {
      renderSection(cliKey);
      expect(screen.getByText("模型路由")).toBeInTheDocument();
      expect(screen.getByText("模型范围")).toBeInTheDocument();
      expect(screen.getByText("模型映射（可选）")).toBeInTheDocument();
      expect(
        screen.queryByText("规则只决定模型资格和重定向，不改变供应商排序")
      ).not.toBeInTheDocument();
      expect(screen.getByRole("radio", { name: "全部可用" })).toBeInTheDocument();
      expect(screen.getByRole("radio", { name: "仅这些可用" })).toBeInTheDocument();
      expect(screen.getByRole("radio", { name: "排除这些" })).toBeInTheDocument();
    }
  );

  it("commits a complete range model and restores focus after deletion", () => {
    const onChange = renderSection("codex", "ready", {
      version: 1,
      mode: "selected",
      modelPatterns: ["gpt-5.4"],
      mappings: [],
    });

    const composer = screen.getByLabelText("新增可用模型");
    fireEvent.change(composer, { target: { value: "new-model" } });
    fireEvent.keyDown(composer, { key: "Enter" });
    expect(onChange).toHaveBeenCalledWith({
      version: 1,
      mode: "selected",
      modelPatterns: ["gpt-5.4", "new-model"],
      mappings: [],
    });

    fireEvent.click(screen.getByRole("button", { name: "删除可用模型 1" }));
    expect(composer).toHaveFocus();
  });

  it.each(["all", "selected", "excluded"] as const)(
    "typing a discovery candidate in %s mode never auto-commits; Enter commits",
    (mode) => {
      const onChange = renderSection(
        "codex",
        "ready",
        { version: 1, mode, modelPatterns: [], mappings: [] },
        {
          status: "ready",
          models: ["candidate-model"],
          origin: "https://example.com",
          baseUrlIndex: 1,
        }
      );

      const composer = screen.getByLabelText(
        `新增${mode === "all" ? "显式" : mode === "selected" ? "可用" : "排除"}模型`
      );
      // Typing text that equals a candidate (e.g. a prefix of a longer model
      // name) must not hijack the input by auto-adding the pattern.
      fireEvent.change(composer, { target: { value: "candidate-model" } });
      expect(onChange).not.toHaveBeenCalled();
      expect(composer).toHaveValue("candidate-model");

      fireEvent.keyDown(composer, { key: "Enter" });
      expect(onChange).toHaveBeenCalledWith({
        version: 1,
        mode,
        modelPatterns: ["candidate-model"],
        mappings: [],
      });
      expect(onChange).not.toHaveBeenCalledWith(
        expect.objectContaining({ modelPatterns: expect.arrayContaining([""]) })
      );
    }
  );

  it("adds a mapping only after source and target are complete", () => {
    const onChange = renderSection("codex");
    fireEvent.change(screen.getByLabelText("映射请求模型"), {
      target: { value: "gpt-5.6-luna" },
    });
    expect(screen.getByRole("button", { name: "添加映射" })).toBeDisabled();
    expect(onChange).not.toHaveBeenCalled();

    fireEvent.change(screen.getByLabelText("映射上游模型"), {
      target: { value: "deepseek-v4-flash" },
    });
    fireEvent.click(screen.getByRole("button", { name: "添加映射" }));
    expect(onChange).toHaveBeenCalledWith({
      version: 1,
      mode: "all",
      modelPatterns: [],
      mappings: [{ source: "gpt-5.6-luna", target: "deepseek-v4-flash" }],
    });
  });

  it("accepts model names longer than 200 Unicode characters", () => {
    renderSection("codex", "ready", {
      version: 1,
      mode: "selected",
      modelPatterns: ["😀".repeat(201)],
      mappings: [],
    });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("shows legacy opt-in and invalid reset consequences", () => {
    const legacyChange = renderSection("claude", "legacy", null);
    expect(screen.getByText("当前 Claude 使用旧版模型映射")).toBeInTheDocument();
    expect(screen.getByText("未配置，沿用请求模型。")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "改用通用模型策略" }));
    expect(legacyChange).toHaveBeenCalledWith(allPolicy);
    expect(screen.getByText("保存后旧版映射不再生效，且无法在界面切回旧策略")).toBeInTheDocument();

    cleanup();
    renderSection("codex", "invalid", null);
    expect(screen.getByRole("alert")).toHaveTextContent("模型策略无效");
    fireEvent.click(screen.getByRole("button", { name: "重置为全部可用" }));
  });

  it("keeps legacy mappings visible for reference after cutover", () => {
    render(
      <ProviderModelPolicySection
        cliKey="claude"
        status="legacy"
        policy={null}
        legacyClaudeModels={{ sonnet_model: "legacy-sonnet" }}
        saving={false}
        onChange={vi.fn()}
        modelDiscoveryState={{ status: "idle" }}
        onDiscoverModels={vi.fn()}
        hasMultipleBaseUrls={false}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: "改用通用模型策略" }));
    expect(screen.getByLabelText("旧版模型映射参考")).toHaveTextContent("legacy-sonnet");
    expect(screen.getByText("保存后旧版映射不再生效，且无法在界面切回旧策略")).toBeInTheDocument();
  });

  it("hides the generic mapping editor when showMappings is false", () => {
    render(
      <ProviderModelPolicySection
        cliKey="claude"
        status="ready"
        policy={{
          version: 1,
          mode: "all",
          modelPatterns: [],
          mappings: [{ source: "gpt-5.6-luna", target: "deepseek-v4-flash" }],
        }}
        saving={false}
        onChange={vi.fn()}
        modelDiscoveryState={{ status: "idle" }}
        onDiscoverModels={vi.fn()}
        hasMultipleBaseUrls={false}
        showMappings={false}
      />
    );

    expect(screen.getByText("模型范围")).toBeInTheDocument();
    expect(screen.queryByText("模型映射（可选）")).not.toBeInTheDocument();
    // The header summary must not count mappings the user can neither see nor use.
    expect(screen.queryByText(/映射 1/)).not.toBeInTheDocument();
  });

  it("hides the legacy copy reference when the mapping editor is hidden", () => {
    render(
      <ProviderModelPolicySection
        cliKey="claude"
        status="legacy"
        policy={null}
        legacyClaudeModels={{ sonnet_model: "legacy-sonnet" }}
        saving={false}
        onChange={vi.fn()}
        modelDiscoveryState={{ status: "idle" }}
        onDiscoverModels={vi.fn()}
        hasMultipleBaseUrls={false}
        showMappings={false}
      />
    );

    fireEvent.click(screen.getByRole("button", { name: "改用通用模型策略" }));
    // The reference block tells the user to copy values into the mapping editor
    // below — pointless when that editor is not rendered.
    expect(screen.queryByLabelText("旧版模型映射参考")).not.toBeInTheDocument();
  });

  it("tells unsaved OAuth providers to save before discovering models", () => {
    renderSection("codex", "ready", allPolicy, { status: "oauth_unsaved" });
    expect(screen.getByText("请先保存并完成 OAuth 登录后再获取")).toBeInTheDocument();
  });

  it("shows configured legacy Claude mappings without editing controls", () => {
    render(
      <ProviderModelPolicySection
        cliKey="claude"
        status="legacy"
        policy={null}
        legacyClaudeModels={{ main_model: "legacy-main", reasoning_model: "legacy-thinking" }}
        saving={false}
        onChange={vi.fn()}
        modelDiscoveryState={{ status: "idle" }}
        onDiscoverModels={vi.fn()}
        hasMultipleBaseUrls={false}
      />
    );

    expect(screen.getByLabelText("旧版模型映射摘要")).toHaveTextContent("legacy-main");
    expect(screen.getByLabelText("旧版模型映射摘要")).toHaveTextContent("legacy-thinking");
    expect(screen.queryByLabelText("请求模型 1")).not.toBeInTheDocument();
  });

  it("keeps range editing enabled while discovery is loading", () => {
    render(
      <ProviderModelPolicySection
        cliKey="codex"
        status="ready"
        policy={{
          version: 1,
          mode: "all",
          modelPatterns: ["gpt-5.4"],
          mappings: [],
        }}
        saving={false}
        onChange={vi.fn()}
        modelDiscoveryState={{ status: "loading" }}
        onDiscoverModels={vi.fn()}
        hasMultipleBaseUrls={false}
      />
    );

    expect(screen.getByRole("button", { name: "获取上游模型" })).toBeDisabled();
    expect(screen.getByLabelText("新增显式模型")).toBeEnabled();
    expect(screen.getByLabelText("显式模型 1")).toBeEnabled();
    expect(screen.getByText("正在获取上游模型…")).toBeInTheDocument();
  });

  it("keeps unsupported and error states actionable without endpoint claims", () => {
    renderSection("claude", "ready", allPolicy, {
      status: "unsupported",
      reason: "cx_2cc",
    });
    expect(screen.getByText("CX2CC 请在对应 Codex Provider 获取")).toBeInTheDocument();
    expect(screen.queryByText("仅代表当前端点")).not.toBeInTheDocument();

    cleanup();
    renderSection("grok", "ready", allPolicy, {
      status: "error",
      code: "redirect",
      httpStatus: null,
    });
    expect(screen.getByText("端点发生重定向，请配置最终 endpoint")).toBeInTheDocument();
    expect(screen.queryByText("仅代表当前端点")).not.toBeInTheDocument();
  });

  it("describes all, selected, and excluded modes plainly", () => {
    renderSection("codex");
    expect(
      screen.getByText(
        "未列出的模型也可用。一个模型只要被任何供应商显式列出，请求它时就只走列出它的供应商。"
      )
    ).toBeInTheDocument();

    cleanup();
    renderSection("codex", "ready", {
      version: 1,
      mode: "selected",
      modelPatterns: [],
      mappings: [{ source: "gpt-5.6-luna", target: "deepseek-v4-flash" }],
    });
    expect(screen.getByText("只接收下列模型和映射中的请求模型。")).toBeInTheDocument();

    cleanup();
    renderSection("codex", "ready", {
      version: 1,
      mode: "excluded",
      modelPatterns: ["legacy-model"],
      mappings: [],
    });
    expect(screen.getByText("下列模型不可用；其余模型保持可用。")).toBeInTheDocument();
  });

  it("does not claim a route boundary twice in discovery status", () => {
    renderSection("codex", "ready", allPolicy, {
      status: "ready",
      models: ["a", "b", "c"],
      origin: "https://example.com:8443",
      baseUrlIndex: 2,
    });

    expect(screen.getByText(/已获取 3 个候选/)).toBeInTheDocument();
    expect(screen.getByText(/https:\/\/example\.com:8443 · 地址 2/)).toBeInTheDocument();
    expect(screen.queryByText("仅代表当前端点")).not.toBeInTheDocument();
  });

  it.each([
    [429, "上游限流（HTTP 429），请稍后重试"],
    [503, "上游服务异常（HTTP 503），请稍后重试"],
    [418, "上游请求失败（HTTP 418）"],
  ] as const)("shows HTTP %s discovery failures accurately", (httpStatus, message) => {
    renderSection("codex", "ready", allPolicy, {
      status: "error",
      code: "invalid_response",
      httpStatus,
    });

    expect(screen.getByText(message)).toBeInTheDocument();
    expect(screen.queryByText("上游模型目录格式无法使用")).not.toBeInTheDocument();
  });
});
