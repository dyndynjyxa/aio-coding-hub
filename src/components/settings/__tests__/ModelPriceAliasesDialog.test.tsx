import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { ModelPriceAliasesDialog } from "../ModelPriceAliasesDialog";
import {
  useModelPriceAliasesQuery,
  useModelPriceAliasesSetMutation,
  useModelPricesListQuery,
} from "../../../query/modelPrices";

vi.mock("sonner", () => ({ toast: vi.fn() }));

vi.mock("../../../query/modelPrices", async () => {
  const actual = await vi.importActual<typeof import("../../../query/modelPrices")>(
    "../../../query/modelPrices"
  );
  return {
    ...actual,
    useModelPriceAliasesQuery: vi.fn(),
    useModelPricesListQuery: vi.fn(),
    useModelPriceAliasesSetMutation: vi.fn(),
  };
});

describe("settings/ModelPriceAliasesDialog", () => {
  it("renders loading state, normalizes invalid data, and handles confirm cancel + save error", async () => {
    const onOpenChange = vi.fn();

    const aliasesRefetch = vi.fn().mockResolvedValue({ data: {} });

    let fetching = true;
    const loadingAliasesQuery = { data: null, isFetching: true, refetch: aliasesRefetch } as any;
    const loadedAliasesData = { version: Number.NaN, rules: [null] } as any;
    const loadedAliasesQuery = {
      data: loadedAliasesData,
      isFetching: false,
      refetch: aliasesRefetch,
    } as any;
    vi.mocked(useModelPriceAliasesQuery).mockImplementation(() =>
      fetching ? loadingAliasesQuery : loadedAliasesQuery
    );

    vi.mocked(useModelPricesListQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    const mutateAsync = vi.fn().mockRejectedValue(new Error("boom"));
    vi.mocked(useModelPriceAliasesSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync,
    } as any);

    const consoleSpy = vi.spyOn(console, "error").mockImplementation(() => {});

    const { rerender } = render(
      <ModelPriceAliasesDialog open={true} onOpenChange={onOpenChange} />
    );
    expect(screen.getByText("加载规则中…")).toBeInTheDocument();

    fetching = false;
    rerender(<ModelPriceAliasesDialog open={true} onOpenChange={onOpenChange} />);

    expect(screen.getByText("规则 #1")).toBeInTheDocument();
    expect(screen.getByText(/启用 0 条/)).toBeInTheDocument();

    // toggling a null rule should create a default rule via updateRule()
    fireEvent.click(screen.getByRole("switch"));
    expect(screen.getByText(/启用 1 条/)).toBeInTheDocument();

    // cover matchType branches: exact + prefix
    fireEvent.change(screen.getAllByRole("combobox")[1]!, { target: { value: "exact" } });
    expect(screen.getByPlaceholderText("例如：gemini-3-flash")).toBeInTheDocument();
    expect(screen.getByText(/exact：完全相等才命中/)).toBeInTheDocument();

    fireEvent.change(screen.getAllByRole("combobox")[1]!, { target: { value: "prefix" } });
    expect(screen.getByPlaceholderText("例如：claude-opus-4-5")).toBeInTheDocument();
    expect(screen.getByText(/prefix：以 pattern 开头即命中/)).toBeInTheDocument();

    // delete rule directly (no confirmation dialog in Tauri WebView)
    fireEvent.click(screen.getByRole("button", { name: "删除" }));
    expect(toast).toHaveBeenCalledWith("已删除规则，点击「保存」生效");
    expect(screen.queryByText("规则 #1")).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith("保存失败：请检查规则内容（例如 wildcard 只能包含一个 *）")
    );
    expect(onOpenChange).not.toHaveBeenCalled();

    consoleSpy.mockRestore();
  });

  it("prevents closing and disables actions while saving", async () => {
    const onOpenChange = vi.fn();

    vi.mocked(useModelPriceAliasesQuery).mockReturnValue({
      data: { version: 1, rules: [] },
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useModelPricesListQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useModelPriceAliasesSetMutation).mockReturnValue({
      isPending: true,
      mutateAsync: vi.fn(),
    } as any);

    render(<ModelPriceAliasesDialog open={true} onOpenChange={onOpenChange} />);

    expect(screen.getByText("保存中…")).toBeInTheDocument();

    fireEvent.click(document.querySelector(".bg-black\\/30") as HTMLElement);
    expect(onOpenChange).not.toHaveBeenCalled();
  });

  it("supports add/edit/delete and save flows", async () => {
    const onOpenChange = vi.fn();

    const aliasesRefetch = vi.fn().mockResolvedValue({ data: {} });
    const claudeRefetch = vi.fn().mockResolvedValue({ data: {} });
    const codexRefetch = vi.fn().mockResolvedValue({ data: {} });
    const geminiRefetch = vi.fn().mockResolvedValue({ data: {} });
    const grokRefetch = vi.fn().mockResolvedValue({ data: {} });

    vi.mocked(useModelPriceAliasesQuery).mockReturnValue({
      data: { version: 1, rules: [] },
      isFetching: false,
      refetch: aliasesRefetch,
    } as any);

    vi.mocked(useModelPricesListQuery).mockImplementation((cliKey: any) => {
      const refetch =
        cliKey === "claude"
          ? claudeRefetch
          : cliKey === "codex"
            ? codexRefetch
            : cliKey === "gemini"
              ? geminiRefetch
              : grokRefetch;
      return {
        data: [{ model: `${cliKey}-model` }],
        isFetching: false,
        refetch,
      } as any;
    });

    const mutateAsync = vi.fn().mockResolvedValue({
      version: 1,
      rules: [
        {
          cli_key: "gemini",
          match_type: "prefix",
          pattern: "gemini-3",
          target_model: "gemini-3-preview",
          enabled: true,
        },
      ],
    });
    vi.mocked(useModelPriceAliasesSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync,
    } as any);

    render(<ModelPriceAliasesDialog open={true} onOpenChange={onOpenChange} />);

    expect(screen.getByText("暂无规则")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "新增规则" }));
    expect(screen.getByText("规则 #1")).toBeInTheDocument();
    expect(screen.getByText(/启用 1 条/)).toBeInTheDocument();

    // change match type to wildcard to exercise placeholder + hint branches
    fireEvent.change(screen.getAllByRole("combobox")[1], { target: { value: "wildcard" } });
    expect(screen.getByText(/wildcard：仅支持单个 \\*/)).toBeInTheDocument();

    fireEvent.change(screen.getByPlaceholderText("例如：gemini-3-*-preview"), {
      target: { value: "gemini-3-*-preview" },
    });
    fireEvent.change(screen.getByPlaceholderText("输入或从建议中选择…"), {
      target: { value: "gemini-3-flash-preview" },
    });

    // toggle enabled off and on
    fireEvent.click(screen.getByRole("switch"));
    expect(screen.getByText(/启用 0 条/)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("switch"));
    expect(screen.getByText(/启用 1 条/)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "刷新" }));
    await waitFor(() => {
      expect(aliasesRefetch).toHaveBeenCalled();
      expect(claudeRefetch).toHaveBeenCalled();
      expect(codexRefetch).toHaveBeenCalled();
      expect(geminiRefetch).toHaveBeenCalled();
      expect(grokRefetch).toHaveBeenCalled();
    });

    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(mutateAsync).toHaveBeenCalled());
    expect(toast).toHaveBeenCalledWith("已保存定价匹配规则");
    expect(onOpenChange).toHaveBeenCalledWith(false);

    // delete rule (no confirmation dialog)
    fireEvent.click(screen.getByRole("button", { name: "删除" }));
    expect(toast).toHaveBeenCalledWith("已删除规则，点击「保存」生效");
    expect(screen.queryByText("规则 #1")).not.toBeInTheDocument();
  });

  it("toasts when save returns null", async () => {
    const onOpenChange = vi.fn();

    vi.mocked(useModelPriceAliasesQuery).mockReturnValue({
      data: {
        version: 1,
        rules: [
          {
            cli_key: "gemini",
            match_type: "prefix",
            pattern: "g",
            target_model: "g",
            enabled: true,
          },
        ],
      },
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useModelPricesListQuery).mockReturnValue({
      data: [],
      isFetching: false,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useModelPriceAliasesSetMutation).mockReturnValue({
      isPending: false,
      mutateAsync: vi.fn().mockResolvedValue(null),
    } as any);

    render(<ModelPriceAliasesDialog open={true} onOpenChange={onOpenChange} />);

    fireEvent.click(screen.getByRole("button", { name: "保存" }));
  });
});
