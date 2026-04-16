import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CliManagerCx2ccTab } from "../Cx2ccTab";
import type { AppSettings } from "../../../../services/settings/settings";

function createAppSettings(overrides: Partial<AppSettings> = {}): AppSettings {
  return {
    ui_locale: "zh-CN",
    ui_theme: "system",
    cx2cc_fallback_model_opus: "gpt-5.4-opus",
    cx2cc_fallback_model_sonnet: "gpt-5.4-sonnet",
    cx2cc_fallback_model_haiku: "gpt-5.4-haiku",
    cx2cc_fallback_model_main: "gpt-5.4-main",
    cx2cc_service_tier: "fast",
    cx2cc_model_reasoning_effort: "medium",
    cx2cc_disable_response_storage: true,
    cx2cc_enable_reasoning_to_thinking: true,
    cx2cc_drop_stop_sequences: true,
    cx2cc_clean_schema: true,
    cx2cc_filter_batch_tool: true,
    // Other required fields with defaults
    gateway_provider_cooldown_secs: 30,
    gateway_upstream_first_byte_timeout_secs: 30,
    gateway_upstream_stream_idle_timeout_secs: 60,
    gateway_upstream_request_timeout_non_streaming_secs: 120,
    gateway_max_attempts_per_provider: 3,
    gateway_abort_stuck_stream_threshold_secs: 300,
    gateway_verbose_provider_error: false,
    gateway_enable_response_fixer: true,
    gateway_enable_thinking_signature_rectifier: true,
    gateway_enable_thinking_budget_rectifier: true,
    gateway_circuit_breaker_failure_threshold: 5,
    gateway_circuit_breaker_open_duration_secs: 30,
    ...overrides,
  } as AppSettings;
}

describe("components/cli-manager/tabs/Cx2ccTab", () => {
  it("renders all setting sections", () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings()}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    expect(screen.getByText("模型 Fallback 映射")).toBeInTheDocument();
    expect(screen.getByText("上游请求注入")).toBeInTheDocument();
    expect(screen.getByText("转换行为开关")).toBeInTheDocument();
  });

  it("renders all fallback model inputs with initial values", () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings()}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    expect(screen.getByDisplayValue("gpt-5.4-opus")).toBeInTheDocument();
    expect(screen.getByDisplayValue("gpt-5.4-sonnet")).toBeInTheDocument();
    expect(screen.getByDisplayValue("gpt-5.4-haiku")).toBeInTheDocument();
    expect(screen.getByDisplayValue("gpt-5.4-main")).toBeInTheDocument();
  });

  it("persists opus fallback model on blur", async () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings()}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    const opusInput = screen.getByDisplayValue("gpt-5.4-opus");
    fireEvent.change(opusInput, { target: { value: "new-opus-model" } });
    fireEvent.blur(opusInput);

    expect(persistSettings).toHaveBeenCalledWith({
      cx2cc_fallback_model_opus: "new-opus-model",
    });
  });

  it("persists sonnet fallback model on blur", async () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings()}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    const sonnetInput = screen.getByDisplayValue("gpt-5.4-sonnet");
    fireEvent.change(sonnetInput, { target: { value: "new-sonnet-model" } });
    fireEvent.blur(sonnetInput);

    expect(persistSettings).toHaveBeenCalledWith({
      cx2cc_fallback_model_sonnet: "new-sonnet-model",
    });
  });

  it("persists haiku fallback model on blur", async () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings()}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    const haikuInput = screen.getByDisplayValue("gpt-5.4-haiku");
    fireEvent.change(haikuInput, { target: { value: "new-haiku-model" } });
    fireEvent.blur(haikuInput);

    expect(persistSettings).toHaveBeenCalledWith({
      cx2cc_fallback_model_haiku: "new-haiku-model",
    });
  });

  it("persists main fallback model on blur", async () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings()}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    const mainInput = screen.getByDisplayValue("gpt-5.4-main");
    fireEvent.change(mainInput, { target: { value: "new-main-model" } });
    fireEvent.blur(mainInput);

    expect(persistSettings).toHaveBeenCalledWith({
      cx2cc_fallback_model_main: "new-main-model",
    });
  });

  it("does not persist empty fallback model on blur", async () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings()}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    const opusInput = screen.getByDisplayValue("gpt-5.4-opus");
    fireEvent.change(opusInput, { target: { value: "" } });
    fireEvent.blur(opusInput);

    expect(persistSettings).not.toHaveBeenCalled();
  });

  it("persists service tier on blur", async () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings()}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    const serviceTierInput = screen.getByDisplayValue("fast");
    fireEvent.change(serviceTierInput, { target: { value: "standard" } });
    fireEvent.blur(serviceTierInput);

    expect(persistSettings).toHaveBeenCalledWith({
      cx2cc_service_tier: "standard",
    });
  });

  it("persists empty service tier on blur", async () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings()}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    const serviceTierInput = screen.getByDisplayValue("fast");
    fireEvent.change(serviceTierInput, { target: { value: "" } });
    fireEvent.blur(serviceTierInput);

    expect(persistSettings).toHaveBeenCalledWith({
      cx2cc_service_tier: "",
    });
  });

  it("renders reasoning effort radio group with correct value", () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings({ cx2cc_model_reasoning_effort: "high" })}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    const highRadio = screen.getByRole("radio", { name: "high" });
    expect(highRadio).toBeChecked();
  });

  it("persists reasoning effort change", () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings({ cx2cc_model_reasoning_effort: "medium" })}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    const lowRadio = screen.getByRole("radio", { name: "low" });
    fireEvent.click(lowRadio);

    expect(persistSettings).toHaveBeenCalledWith({
      cx2cc_model_reasoning_effort: "low",
    });
  });

  it("persists disable response storage switch change", () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings({ cx2cc_disable_response_storage: true })}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    const switches = screen.getAllByRole("switch");
    // First switch is "禁用响应存储"
    fireEvent.click(switches[0]);

    expect(persistSettings).toHaveBeenCalledWith({
      cx2cc_disable_response_storage: false,
    });
  });

  it("persists enable reasoning to thinking switch change", () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings({ cx2cc_enable_reasoning_to_thinking: true })}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    const switches = screen.getAllByRole("switch");
    // Second switch is "启用推理转思考"
    fireEvent.click(switches[1]);

    expect(persistSettings).toHaveBeenCalledWith({
      cx2cc_enable_reasoning_to_thinking: false,
    });
  });

  it("persists drop stop sequences switch change", () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings({ cx2cc_drop_stop_sequences: true })}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    const switches = screen.getAllByRole("switch");
    // Third switch is "丢弃停止序列"
    fireEvent.click(switches[2]);

    expect(persistSettings).toHaveBeenCalledWith({
      cx2cc_drop_stop_sequences: false,
    });
  });

  it("persists clean schema switch change", () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings({ cx2cc_clean_schema: true })}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    const switches = screen.getAllByRole("switch");
    // Fourth switch is "清理 Schema"
    fireEvent.click(switches[3]);

    expect(persistSettings).toHaveBeenCalledWith({
      cx2cc_clean_schema: false,
    });
  });

  it("persists filter batch tool switch change", () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings({ cx2cc_filter_batch_tool: true })}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    const switches = screen.getAllByRole("switch");
    // Fifth switch is "过滤 BatchTool"
    fireEvent.click(switches[4]);

    expect(persistSettings).toHaveBeenCalledWith({
      cx2cc_filter_batch_tool: false,
    });
  });

  it("disables controls when saving", () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings()}
        commonSettingsSaving={true}
        onPersistCommonSettings={persistSettings}
      />
    );

    const opusInput = screen.getByDisplayValue("gpt-5.4-opus");
    expect(opusInput).toBeDisabled();

    const switches = screen.getAllByRole("switch");
    switches.forEach((switchEl) => {
      expect(switchEl).toBeDisabled();
    });

    const radios = screen.getAllByRole("radio");
    radios.forEach((radio) => {
      expect(radio).toBeDisabled();
    });
  });

  it("disables controls when appSettings is null", () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    render(
      <CliManagerCx2ccTab
        appSettings={null}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    const switches = screen.getAllByRole("switch");
    switches.forEach((switchEl) => {
      expect(switchEl).toBeDisabled();
    });
  });

  it("updates local state when appSettings changes", () => {
    const persistSettings = vi.fn().mockResolvedValue(null);
    const { rerender } = render(
      <CliManagerCx2ccTab
        appSettings={createAppSettings({ cx2cc_fallback_model_opus: "old-model" })}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    expect(screen.getByDisplayValue("old-model")).toBeInTheDocument();

    rerender(
      <CliManagerCx2ccTab
        appSettings={createAppSettings({ cx2cc_fallback_model_opus: "new-model" })}
        commonSettingsSaving={false}
        onPersistCommonSettings={persistSettings}
      />
    );

    expect(screen.getByDisplayValue("new-model")).toBeInTheDocument();
  });
});
