import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { toast } from "sonner";
import { HomeTokenCostPanel } from "../HomeTokenCostPanel";
import {
  useUsageDayDetailV1Query,
  useUsageFolderOptionsV1Query,
  useUsageLeaderboardV2Query,
  useUsageSummaryV2Query,
} from "../../../query/usage";
import { saveDesktopFilePath } from "../../../services/desktop/dialog";
import { usageLeaderboardCsvExport } from "../../../services/usage/usage";

vi.mock("sonner", () => ({
  toast: vi.fn(),
}));

vi.mock("../../../query/usage", async () => {
  const actual =
    await vi.importActual<typeof import("../../../query/usage")>("../../../query/usage");
  return {
    ...actual,
    useUsageDayDetailV1Query: vi.fn(),
    useUsageFolderOptionsV1Query: vi.fn(),
    useUsageSummaryV2Query: vi.fn(),
    useUsageLeaderboardV2Query: vi.fn(),
  };
});

vi.mock("../../../services/desktop/dialog", () => ({
  saveDesktopFilePath: vi.fn(),
}));

vi.mock("../../../services/usage/usage", async () => {
  const actual = await vi.importActual<typeof import("../../../services/usage/usage")>(
    "../../../services/usage/usage"
  );
  return {
    ...actual,
    usageLeaderboardCsvExport: vi.fn(),
  };
});

function makeDayDetail(day = "2026-04-16") {
  return {
    day,
    folders: [
      {
        key: "/Users/demo/aio-coding-hub",
        name: "aio-coding-hub",
        folder_path: "/Users/demo/aio-coding-hub",
        requests_total: 4,
        requests_success: 4,
        requests_failed: 0,
        total_tokens: 4200,
        io_total_tokens: 3600,
        input_tokens: 2400,
        output_tokens: 1200,
        cache_creation_input_tokens: 200,
        cache_read_input_tokens: 400,
        avg_duration_ms: 900,
        avg_ttfb_ms: 210,
        avg_output_tokens_per_second: 102.4,
        cost_usd: 0.42,
      },
    ],
    hours: Array.from({ length: 24 }, (_, hour) => ({
      hour,
      requests_total: hour === 13 ? 4 : 0,
      total_tokens: hour === 13 ? 4200 : 0,
      io_total_tokens: hour === 13 ? 3600 : 0,
    })),
  };
}

function makeSortableDayDetail(day = "2026-04-16") {
  return {
    day,
    folders: [
      {
        key: "/Users/demo/alpha",
        name: "Alpha app",
        folder_path: "/Users/demo/alpha",
        requests_total: 2,
        requests_success: 2,
        requests_failed: 0,
        total_tokens: 200,
        io_total_tokens: 50,
        input_tokens: 30,
        output_tokens: 20,
        cache_creation_input_tokens: 60,
        cache_read_input_tokens: 90,
        avg_duration_ms: 800,
        avg_ttfb_ms: 180,
        avg_output_tokens_per_second: 70,
        cost_usd: 0.2,
      },
      {
        key: "/Users/demo/bravo",
        name: "Bravo app",
        folder_path: "/Users/demo/bravo",
        requests_total: 5,
        requests_success: 4,
        requests_failed: 1,
        total_tokens: 500,
        io_total_tokens: 450,
        input_tokens: 250,
        output_tokens: 200,
        cache_creation_input_tokens: 20,
        cache_read_input_tokens: 30,
        avg_duration_ms: 1200,
        avg_ttfb_ms: 250,
        avg_output_tokens_per_second: 95,
        cost_usd: 0.1,
      },
      {
        key: "/Users/demo/charlie",
        name: "Charlie app",
        folder_path: "/Users/demo/charlie",
        requests_total: 3,
        requests_success: 3,
        requests_failed: 0,
        total_tokens: 300,
        io_total_tokens: 100,
        input_tokens: 60,
        output_tokens: 40,
        cache_creation_input_tokens: 120,
        cache_read_input_tokens: 80,
        avg_duration_ms: 900,
        avg_ttfb_ms: 210,
        avg_output_tokens_per_second: 80,
        cost_usd: 0.4,
      },
    ],
    hours: Array.from({ length: 24 }, (_, hour) => ({
      hour,
      requests_total: hour === 13 ? 4 : 0,
      total_tokens: hour === 13 ? 4200 : 0,
      io_total_tokens: hour === 13 ? 3600 : 0,
    })),
  };
}

function controlLabel(element: Element) {
  return element.getAttribute("aria-label") ?? element.textContent?.replace(/\s+/g, " ").trim();
}

function expectTableRowsInOrder(table: HTMLElement, names: string[]) {
  const rows = within(table).getAllByRole("row").slice(1);
  expect(rows).toHaveLength(names.length);
  names.forEach((name, index) => {
    expect(within(rows[index]).getByText(name)).toBeInTheDocument();
  });
}

function expectTableRanks(table: HTMLElement, ranks: string[]) {
  const rows = within(table).getAllByRole("row").slice(1);
  expect(rows.map((row) => row.querySelector("td")?.textContent?.trim())).toEqual(ranks);
}

function clickSortableHeader(table: HTMLElement, name: RegExp | string) {
  const header = within(table).getByRole("columnheader", { name });
  fireEvent.click(within(header).getByRole("button", { name }));
  return header;
}

function localTimeMs(year: number, monthIndex: number, day: number, hour: number, minute: number) {
  return new Date(year, monthIndex, day, hour, minute, 0, 0).getTime();
}

function mockSingleProviderUsageForExport() {
  vi.mocked(useUsageSummaryV2Query).mockReturnValue({
    data: {
      requests_total: 1,
      requests_with_usage: 1,
      requests_success: 1,
      requests_failed: 0,
      cost_covered_success: 1,
      total_duration_ms: 1000,
      avg_duration_ms: 1000,
      avg_ttfb_ms: 200,
      avg_output_tokens_per_second: 80,
      input_tokens: 100,
      output_tokens: 50,
      io_total_tokens: 150,
      total_tokens: 150,
      cache_read_input_tokens: 0,
      cache_creation_input_tokens: 0,
      cache_creation_5m_input_tokens: 0,
    },
    isLoading: false,
    isFetching: false,
    error: null,
    refetch: vi.fn(),
  } as any);
  vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
    data: [
      {
        key: "provider",
        name: "Provider",
        requests_total: 1,
        requests_success: 1,
        requests_failed: 0,
        total_tokens: 150,
        io_total_tokens: 150,
        input_tokens: 100,
        output_tokens: 50,
        cache_creation_input_tokens: 0,
        cache_read_input_tokens: 0,
        total_duration_ms: 1000,
        first_request_created_at_ms: null,
        last_request_created_at_ms: null,
        avg_duration_ms: 1000,
        avg_ttfb_ms: 200,
        avg_output_tokens_per_second: 80,
        cost_usd: 0.01,
      },
    ],
    isLoading: false,
    isFetching: false,
    error: null,
    refetch: vi.fn(),
  } as any);
}

describe("components/home/HomeTokenCostPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    window.localStorage.removeItem("homeUsageDayStartHour");
    vi.mocked(saveDesktopFilePath).mockResolvedValue("/tmp/home-usage.csv");
    vi.mocked(usageLeaderboardCsvExport).mockResolvedValue(true);
    vi.mocked(useUsageDayDetailV1Query).mockReturnValue({
      data: null,
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageFolderOptionsV1Query).mockReturnValue({
      data: [
        {
          key: "/Users/demo/aio-coding-hub",
          name: "aio-coding-hub",
          folder_path: "/Users/demo/aio-coding-hub",
          requests_total: 6,
          total_tokens: 6200,
        },
        {
          key: "__unknown__",
          name: "未知文件夹",
          folder_path: null,
          requests_total: 2,
          total_tokens: 800,
        },
      ],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("prefers real data over dev preview fallback and can switch to model view", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 24,
        requests_with_usage: 24,
        requests_success: 21,
        requests_failed: 3,
        cost_covered_success: 17,
        total_duration_ms: 125_000,
        avg_duration_ms: 1200,
        avg_ttfb_ms: 320,
        avg_output_tokens_per_second: 88.4,
        input_tokens: 12000,
        output_tokens: 6000,
        io_total_tokens: 18000,
        total_tokens: 22500,
        cache_read_input_tokens: 3000,
        cache_creation_input_tokens: 1500,
        cache_creation_5m_input_tokens: 800,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useUsageLeaderboardV2Query).mockImplementation(
      (scope) =>
        ({
          data:
            scope === "provider"
              ? [
                  {
                    key: "provider-1",
                    name: "OpenAI 主供应商",
                    requests_total: 10,
                    requests_success: 10,
                    requests_failed: 0,
                    total_tokens: 12000,
                    io_total_tokens: 10000,
                    input_tokens: 7000,
                    output_tokens: 3000,
                    cache_creation_input_tokens: 500,
                    cache_read_input_tokens: 1500,
                    total_duration_ms: 62_000,
                    avg_duration_ms: 1000,
                    avg_ttfb_ms: 260,
                    avg_output_tokens_per_second: 96.2,
                    cost_usd: 1.2,
                  },
                ]
              : scope === "model"
                ? [
                    {
                      key: "model-1",
                      name: "gpt-5.4",
                      requests_total: 8,
                      requests_success: 8,
                      requests_failed: 0,
                      total_tokens: 9000,
                      io_total_tokens: 7600,
                      input_tokens: 4800,
                      output_tokens: 2800,
                      cache_creation_input_tokens: 300,
                      cache_read_input_tokens: 1100,
                      total_duration_ms: 44_000,
                      avg_duration_ms: 920,
                      avg_ttfb_ms: 240,
                      avg_output_tokens_per_second: 101.5,
                      cost_usd: 0.9,
                    },
                  ]
                : [
                    {
                      key: "2026-04-16",
                      name: "2026-04-16",
                      requests_total: 6,
                      requests_success: 6,
                      requests_failed: 0,
                      total_tokens: 6200,
                      io_total_tokens: 5200,
                      input_tokens: 3400,
                      output_tokens: 1800,
                      cache_creation_input_tokens: 200,
                      cache_read_input_tokens: 800,
                      total_duration_ms: 15_691_200,
                      first_request_created_at_ms: localTimeMs(2026, 3, 16, 8, 0),
                      last_request_created_at_ms: localTimeMs(2026, 3, 16, 23, 34),
                      avg_duration_ms: 880,
                      avg_ttfb_ms: 210,
                      avg_output_tokens_per_second: 102.4,
                      cost_usd: 0.6,
                    },
                  ],
          isLoading: false,
          isFetching: false,
          error: null,
          refetch: vi.fn(),
        }) as any
    );

    render(<HomeTokenCostPanel devPreviewEnabled={true} />);

    const settingsGroup = screen.getByRole("group", { name: "用量筛选设置" });
    const settingLabels = Array.from(settingsGroup.querySelectorAll("button,input,select")).map(
      controlLabel
    );
    expect(settingLabels).toEqual(["全部文件夹", "过滤转接重复用量", "统计日开始"]);
    const dayStartSelect = screen.getByLabelText("统计日开始") as HTMLSelectElement;
    expect(dayStartSelect.value).toBe("0");
    expect(Array.from(dayStartSelect.options).map((option) => option.textContent)).toEqual([
      "00:00",
      "01:00",
      "02:00",
      "03:00",
      "04:00",
      "05:00",
      "06:00",
      "07:00",
      "08:00",
      "09:00",
    ]);
    const rangeGroup = screen.getByRole("group", { name: "用量时间范围" });
    const rangeLabels = Array.from(rangeGroup.querySelectorAll("button,input")).map(controlLabel);
    expect(rangeLabels).toEqual([
      "今天",
      "昨天",
      "最近 3 天",
      "最近 7 天",
      "最近 15 天",
      "最近 30 天",
      "当月",
      "开始日期",
      "结束日期",
      "自定义",
    ]);

    const cachedTotalCard = screen.getByText("含缓存总 Token");
    const inputOutputTokenCard = screen.getAllByText("输入+输出 Token")[0];
    const totalCostCard = screen.getAllByText("总花费")[0];
    const totalDurationCard = screen.getByText("请求总耗时");
    const successCard = screen.getByText("成功请求");
    const cacheHitRateCard = screen.getByText("缓存命中率");
    const providerCountCard = screen.getByText("供应商数");

    expect(screen.getAllByText("总花费")).toHaveLength(2);
    expect(screen.getByText("OpenAI 主供应商")).toBeInTheDocument();
    expect(screen.getByText("18.0K")).toBeInTheDocument();
    expect(screen.getByText("2m5s")).toBeInTheDocument();
    expect(screen.getAllByText("$1.20")).toHaveLength(2);
    const providerRow = screen.getByText("OpenAI 主供应商").closest("tr");
    expect(providerRow).toBeTruthy();
    expect(within(providerRow as HTMLElement).getByText("12K")).toBeInTheDocument();
    expect(within(providerRow as HTMLElement).getByText("10K")).toBeInTheDocument();
    expect(within(providerRow as HTMLElement).getByText("1m2s")).toBeInTheDocument();
    expect(within(providerRow as HTMLElement).getByLabelText("10K/16.7%")).toBeInTheDocument();
    expect(within(providerRow as HTMLElement).getByText("55.6%")).toBeInTheDocument();
    expect(within(providerRow as HTMLElement).getByText("-")).toBeInTheDocument();
    expect(
      within(screen.getByRole("table", { name: "用量排行榜" })).queryByRole("progressbar")
    ).not.toBeInTheDocument();
    expect(within(providerRow as HTMLElement).queryByLabelText("2K/16.7%")).not.toBeInTheDocument();
    expect(screen.getByText("18.2%")).toBeInTheDocument();
    expect(screen.queryByText("成本覆盖率")).not.toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /总Token/ })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /输入\+出\/缓存率/ })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /总耗时/ })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /请求数\/成功率/ })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: /首末请求\/统计日占比/ })).toBeInTheDocument();
    expect(screen.queryByRole("columnheader", { name: /缓存情况/ })).not.toBeInTheDocument();
    expect(screen.queryByRole("columnheader", { name: "成功率" })).not.toBeInTheDocument();
    expect(screen.queryByRole("columnheader", { name: /平均输出速度/ })).not.toBeInTheDocument();
    expect(screen.queryByText("（含缓存/缓存/命中率）")).not.toBeInTheDocument();
    expect(screen.queryByText("Token 明细")).not.toBeInTheDocument();
    expect(screen.queryByText("含缓存总量 / 缓存量 / 缓存命中率")).not.toBeInTheDocument();
    expect(screen.queryByText("平均耗时")).not.toBeInTheDocument();
    expect(
      cachedTotalCard.compareDocumentPosition(inputOutputTokenCard) &
        Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();
    expect(
      inputOutputTokenCard.compareDocumentPosition(totalCostCard) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();
    expect(
      totalCostCard.compareDocumentPosition(totalDurationCard) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();
    expect(
      totalDurationCard.compareDocumentPosition(successCard) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();
    expect(
      successCard.compareDocumentPosition(cacheHitRateCard) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();
    expect(
      cacheHitRateCard.compareDocumentPosition(providerCountCard) & Node.DOCUMENT_POSITION_FOLLOWING
    ).toBeTruthy();
    expect(screen.queryByText("OpenAI Primary")).not.toBeInTheDocument();
    expect(screen.queryByText("总请求 24 / 失败 3")).not.toBeInTheDocument();
    expect(screen.queryByText("总 Token、缓存占比、总花费。")).not.toBeInTheDocument();
    expect(screen.queryByText("今天 · 1 个供应商")).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /日期详情/ })).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "模型" }));

    expect(screen.getByText("gpt-5.4")).toBeInTheDocument();
    expect(screen.getAllByText("$0.90")).toHaveLength(2);
    const modelRow = screen.getByText("gpt-5.4").closest("tr");
    expect(modelRow).toBeTruthy();
    expect(within(modelRow as HTMLElement).getByText("-")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /日期详情/ })).not.toBeInTheDocument();
    expect(vi.mocked(useUsageLeaderboardV2Query)).toHaveBeenLastCalledWith(
      "model",
      "daily",
      expect.objectContaining({
        startTs: null,
        endTs: null,
        cliKey: null,
        providerId: null,
        limit: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      undefined
    );

    fireEvent.click(screen.getByRole("tab", { name: "日期" }));

    expect(screen.getByText("2026-04-16")).toBeInTheDocument();
    expect(screen.getByLabelText("08:00-23:34/18.2%")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "展开 2026-04-16 日期详情" })).toBeInTheDocument();
    expect(vi.mocked(useUsageLeaderboardV2Query)).toHaveBeenLastCalledWith(
      "day",
      "daily",
      expect.objectContaining({
        startTs: null,
        endTs: null,
        cliKey: null,
        providerId: null,
        limit: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      undefined
    );
  });

  it("persists the home usage statistics day start hour and passes it to queries", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 1,
        requests_with_usage: 1,
        requests_success: 1,
        requests_failed: 0,
        cost_covered_success: 1,
        total_duration_ms: 1_000,
        avg_duration_ms: 1_000,
        avg_ttfb_ms: 200,
        avg_output_tokens_per_second: 80,
        input_tokens: 100,
        output_tokens: 50,
        io_total_tokens: 150,
        total_tokens: 180,
        cache_read_input_tokens: 30,
        cache_creation_input_tokens: 0,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockImplementation(
      (scope) =>
        ({
          data:
            scope === "day"
              ? [
                  {
                    key: "2026-04-16",
                    name: "2026-04-16",
                    requests_total: 1,
                    requests_success: 1,
                    requests_failed: 0,
                    total_tokens: 180,
                    io_total_tokens: 150,
                    input_tokens: 100,
                    output_tokens: 50,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 30,
                    total_duration_ms: 1_000,
                    first_request_created_at_ms: localTimeMs(2026, 3, 16, 9, 0),
                    last_request_created_at_ms: localTimeMs(2026, 3, 16, 9, 1),
                    avg_duration_ms: 1_000,
                    avg_ttfb_ms: 200,
                    avg_output_tokens_per_second: 80,
                    cost_usd: 0.1,
                  },
                ]
              : [],
          isLoading: false,
          isFetching: false,
          error: null,
          refetch: vi.fn(),
        }) as any
    );

    render(<HomeTokenCostPanel />);

    const dayStartSelect = screen.getByLabelText("统计日开始") as HTMLSelectElement;
    expect(dayStartSelect.value).toBe("0");
    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith(
      "daily",
      expect.objectContaining({ dayStartHour: 0 }),
      undefined
    );

    fireEvent.click(screen.getByRole("tab", { name: "日期" }));
    fireEvent.click(screen.getByRole("button", { name: "展开 2026-04-16 日期详情" }));
    expect(vi.mocked(useUsageDayDetailV1Query)).toHaveBeenLastCalledWith(
      expect.objectContaining({ day: "2026-04-16", dayStartHour: 0 }),
      expect.objectContaining({ enabled: true })
    );

    fireEvent.change(dayStartSelect, { target: { value: "7" } });

    expect(dayStartSelect.value).toBe("7");
    expect(window.localStorage.getItem("homeUsageDayStartHour")).toBe("7");
    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith(
      "daily",
      expect.objectContaining({ dayStartHour: 7 }),
      undefined
    );
    expect(vi.mocked(useUsageLeaderboardV2Query)).toHaveBeenLastCalledWith(
      "day",
      "daily",
      expect.objectContaining({ dayStartHour: 7 }),
      undefined
    );
    expect(vi.mocked(useUsageFolderOptionsV1Query)).toHaveBeenLastCalledWith(
      "daily",
      expect.objectContaining({ dayStartHour: 7 }),
      expect.objectContaining({ enabled: true })
    );
    expect(vi.mocked(useUsageDayDetailV1Query)).toHaveBeenLastCalledWith(
      expect.objectContaining({ day: "", dayStartHour: 7 }),
      expect.objectContaining({ enabled: false })
    );
  });

  it("formats date request windows across midnight with next-day text for non-midnight boundaries", () => {
    window.localStorage.setItem("homeUsageDayStartHour", "5");
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 2,
        requests_with_usage: 2,
        requests_success: 2,
        requests_failed: 0,
        cost_covered_success: 2,
        total_duration_ms: 26_388_000,
        avg_duration_ms: 1_000,
        avg_ttfb_ms: 200,
        avg_output_tokens_per_second: 80,
        input_tokens: 100,
        output_tokens: 50,
        io_total_tokens: 150,
        total_tokens: 180,
        cache_read_input_tokens: 30,
        cache_creation_input_tokens: 0,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockImplementation(
      (scope) =>
        ({
          data:
            scope === "day"
              ? [
                  {
                    key: "2026-04-17",
                    name: "2026-04-17",
                    requests_total: 1,
                    requests_success: 1,
                    requests_failed: 0,
                    total_tokens: 180,
                    io_total_tokens: 150,
                    input_tokens: 100,
                    output_tokens: 50,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 30,
                    total_duration_ms: 11_088_000,
                    first_request_created_at_ms: localTimeMs(2026, 3, 17, 9, 0),
                    last_request_created_at_ms: localTimeMs(2026, 3, 17, 20, 0),
                    avg_duration_ms: 1_000,
                    avg_ttfb_ms: 200,
                    avg_output_tokens_per_second: 80,
                    cost_usd: 0.1,
                  },
                  {
                    key: "2026-04-16",
                    name: "2026-04-16",
                    requests_total: 1,
                    requests_success: 1,
                    requests_failed: 0,
                    total_tokens: 180,
                    io_total_tokens: 150,
                    input_tokens: 100,
                    output_tokens: 50,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 30,
                    total_duration_ms: 15_300_000,
                    first_request_created_at_ms: localTimeMs(2026, 3, 16, 9, 0),
                    last_request_created_at_ms: localTimeMs(2026, 3, 17, 2, 0),
                    avg_duration_ms: 1_000,
                    avg_ttfb_ms: 200,
                    avg_output_tokens_per_second: 80,
                    cost_usd: 0.1,
                  },
                ]
              : [],
          isLoading: false,
          isFetching: false,
          error: null,
          refetch: vi.fn(),
        }) as any
    );

    render(<HomeTokenCostPanel />);

    fireEvent.click(screen.getByRole("tab", { name: "日期" }));

    expect(screen.getByLabelText("09:00-20:00/12.8%")).toBeInTheDocument();
    expect(screen.getByLabelText("09:00-次日02:00/17.7%")).toBeInTheDocument();
  });

  it("sorts the leaderboard by clicked headers without changing usage query params", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 18,
        requests_with_usage: 18,
        requests_success: 16,
        requests_failed: 2,
        cost_covered_success: 16,
        total_duration_ms: 12_000,
        avg_duration_ms: 1000,
        avg_ttfb_ms: 240,
        avg_output_tokens_per_second: 75,
        input_tokens: 490,
        output_tokens: 950,
        io_total_tokens: 1440,
        total_tokens: 2000,
        cache_read_input_tokens: 240,
        cache_creation_input_tokens: 410,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [
        {
          key: "alpha",
          name: "Alpha Provider",
          requests_total: 3,
          requests_success: 3,
          requests_failed: 0,
          total_tokens: 300,
          io_total_tokens: 100,
          input_tokens: 70,
          output_tokens: 30,
          cache_creation_input_tokens: 50,
          cache_read_input_tokens: 150,
          total_duration_ms: 3_000,
          avg_duration_ms: 1000,
          avg_ttfb_ms: 200,
          avg_output_tokens_per_second: 10,
          cost_usd: 0.3,
        },
        {
          key: "bravo",
          name: "Bravo Provider",
          requests_total: 10,
          requests_success: 8,
          requests_failed: 2,
          total_tokens: 1000,
          io_total_tokens: 990,
          input_tokens: 100,
          output_tokens: 890,
          cache_creation_input_tokens: 0,
          cache_read_input_tokens: 10,
          total_duration_ms: 10_000,
          avg_duration_ms: 900,
          avg_ttfb_ms: 180,
          avg_output_tokens_per_second: 20,
          cost_usd: 1,
        },
        {
          key: "charlie",
          name: "Charlie Provider",
          requests_total: 5,
          requests_success: 5,
          requests_failed: 0,
          total_tokens: 700,
          io_total_tokens: 200,
          input_tokens: 120,
          output_tokens: 80,
          cache_creation_input_tokens: 300,
          cache_read_input_tokens: 200,
          total_duration_ms: 5_000,
          avg_duration_ms: 1100,
          avg_ttfb_ms: 260,
          avg_output_tokens_per_second: 5,
          cost_usd: 0.7,
        },
      ],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel />);

    const table = screen.getByRole("table", { name: "用量排行榜" });
    const rankHeader = within(table).getByRole("columnheader", { name: "排名" });
    expect(within(rankHeader).queryByRole("button")).not.toBeInTheDocument();
    expectTableRowsInOrder(table, ["Alpha Provider", "Bravo Provider", "Charlie Provider"]);
    expectTableRanks(table, ["1", "2", "3"]);

    clickSortableHeader(table, /总Token/);
    expectTableRowsInOrder(table, ["Bravo Provider", "Charlie Provider", "Alpha Provider"]);
    expectTableRanks(table, ["1", "2", "3"]);
    expect(within(table).getByRole("columnheader", { name: /总Token/ })).toHaveAttribute(
      "aria-sort",
      "descending"
    );

    clickSortableHeader(table, /总Token/);
    expectTableRowsInOrder(table, ["Alpha Provider", "Charlie Provider", "Bravo Provider"]);
    expectTableRanks(table, ["1", "2", "3"]);
    expect(within(table).getByRole("columnheader", { name: /总Token/ })).toHaveAttribute(
      "aria-sort",
      "ascending"
    );

    clickSortableHeader(table, /输入\+出\/缓存率/);
    expectTableRowsInOrder(table, ["Bravo Provider", "Charlie Provider", "Alpha Provider"]);
    expect(within(table).getByRole("columnheader", { name: /输入\+出\/缓存率/ })).toHaveAttribute(
      "aria-sort",
      "descending"
    );
    expect(within(table).getByRole("columnheader", { name: /总Token/ })).toHaveAttribute(
      "aria-sort",
      "none"
    );

    clickSortableHeader(table, "供应商");
    expectTableRowsInOrder(table, ["Charlie Provider", "Bravo Provider", "Alpha Provider"]);
    expectTableRanks(table, ["1", "2", "3"]);

    expect(vi.mocked(useUsageLeaderboardV2Query)).toHaveBeenLastCalledWith(
      "provider",
      "daily",
      expect.objectContaining({
        startTs: null,
        endTs: null,
        cliKey: null,
        providerId: null,
        limit: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      undefined
    );
  });

  it("exports the current sorted leaderboard rows as split-column csv", async () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 6,
        requests_with_usage: 6,
        requests_success: 5,
        requests_failed: 1,
        cost_covered_success: 5,
        total_duration_ms: 120_000,
        avg_duration_ms: 1000,
        avg_ttfb_ms: 240,
        avg_output_tokens_per_second: 75,
        input_tokens: 2600,
        output_tokens: 1200,
        io_total_tokens: 3800,
        total_tokens: 5000,
        cache_read_input_tokens: 800,
        cache_creation_input_tokens: 600,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [
        {
          key: "alpha",
          name: "Alpha",
          requests_total: 2,
          requests_success: 2,
          requests_failed: 0,
          total_tokens: 1000,
          io_total_tokens: 800,
          input_tokens: 600,
          output_tokens: 200,
          cache_creation_input_tokens: 100,
          cache_read_input_tokens: 300,
          total_duration_ms: 30_000,
          first_request_created_at_ms: null,
          last_request_created_at_ms: null,
          avg_duration_ms: 1000,
          avg_ttfb_ms: 200,
          avg_output_tokens_per_second: 10,
          cost_usd: 0.01,
        },
        {
          key: "beta",
          name: "Beta",
          requests_total: 4,
          requests_success: 3,
          requests_failed: 1,
          total_tokens: 4000,
          io_total_tokens: 3000,
          input_tokens: 2000,
          output_tokens: 1000,
          cache_creation_input_tokens: 500,
          cache_read_input_tokens: 500,
          total_duration_ms: 90_000,
          first_request_created_at_ms: null,
          last_request_created_at_ms: null,
          avg_duration_ms: 1000,
          avg_ttfb_ms: 200,
          avg_output_tokens_per_second: 10,
          cost_usd: 0.02,
        },
      ],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel />);

    const table = screen.getByRole("table", { name: "用量排行榜" });
    clickSortableHeader(table, /总Token/);
    fireEvent.click(screen.getByRole("button", { name: "导出 CSV" }));

    await waitFor(() => {
      expect(saveDesktopFilePath).toHaveBeenCalledWith(
        expect.objectContaining({
          title: "导出用量排行 CSV",
          defaultPath: expect.stringMatching(
            /^aio-coding-hub-home-usage-provider-\d{8}-\d{6}\.csv$/
          ),
          filters: [{ name: "CSV", extensions: ["csv"] }],
          canCreateDirectories: true,
        })
      );
    });
    await waitFor(() => expect(usageLeaderboardCsvExport).toHaveBeenCalledTimes(1));

    const [filePath, csv] = vi.mocked(usageLeaderboardCsvExport).mock.calls[0] ?? [];
    expect(filePath).toBe("/tmp/home-usage.csv");
    expect(csv).toContain(
      "\uFEFF排名,供应商,总Token,输入+出,缓存率,总花费,总耗时,请求数,成功率,Token 占比,首末请求,统计日占比\r\n"
    );
    expect(csv).not.toContain("输入+出/缓存率");
    expect(csv).not.toContain("请求数/成功率");
    expect(csv).not.toContain("最早最晚/请求占比");
    expect(csv).toContain("1,Beta,4K,3K,16.7%,$0.02,1m30s,4,75%,78.9%,-,-\r\n");
    expect(csv).toContain("2,Alpha,1K,800,30%,$0.01,30s,2,100%,21.1%,-,-\r\n");
    expect(String(csv).indexOf("1,Beta")).toBeLessThan(String(csv).indexOf("2,Alpha"));
    await waitFor(() => expect(toast).toHaveBeenCalledWith("用量排行 CSV 已导出"));
  });

  it("does not export csv when the save dialog is cancelled", async () => {
    mockSingleProviderUsageForExport();
    vi.mocked(saveDesktopFilePath).mockResolvedValueOnce(null);

    render(<HomeTokenCostPanel />);
    fireEvent.click(screen.getByRole("button", { name: "导出 CSV" }));

    await waitFor(() => expect(saveDesktopFilePath).toHaveBeenCalledTimes(1));
    expect(usageLeaderboardCsvExport).not.toHaveBeenCalled();
  });

  it("shows a toast when csv export fails", async () => {
    mockSingleProviderUsageForExport();
    vi.mocked(usageLeaderboardCsvExport).mockRejectedValueOnce(new Error("disk denied"));

    render(<HomeTokenCostPanel />);
    fireEvent.click(screen.getByRole("button", { name: "导出 CSV" }));

    await waitFor(() =>
      expect(toast).toHaveBeenCalledWith(expect.stringContaining("导出 CSV 失败：disk denied"))
    );
  });

  it("expands a day row and fetches folder plus hourly detail", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 6,
        requests_with_usage: 6,
        requests_success: 6,
        requests_failed: 0,
        cost_covered_success: 6,
        avg_duration_ms: 880,
        avg_ttfb_ms: 210,
        avg_output_tokens_per_second: 102.4,
        input_tokens: 3400,
        output_tokens: 1800,
        io_total_tokens: 5200,
        total_tokens: 6200,
        cache_read_input_tokens: 800,
        cache_creation_input_tokens: 200,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useUsageLeaderboardV2Query).mockImplementation(
      (scope) =>
        ({
          data:
            scope === "day"
              ? [
                  {
                    key: "2026-04-16",
                    name: "2026-04-16",
                    requests_total: 6,
                    requests_success: 6,
                    requests_failed: 0,
                    total_tokens: 6200,
                    io_total_tokens: 5200,
                    input_tokens: 3400,
                    output_tokens: 1800,
                    cache_creation_input_tokens: 200,
                    cache_read_input_tokens: 800,
                    avg_duration_ms: 880,
                    avg_ttfb_ms: 210,
                    avg_output_tokens_per_second: 102.4,
                    cost_usd: 0.6,
                  },
                ]
              : [],
          isLoading: false,
          isFetching: false,
          error: null,
          refetch: vi.fn(),
        }) as any
    );
    vi.mocked(useUsageDayDetailV1Query).mockReturnValue({
      data: makeDayDetail("2026-04-16"),
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel />);

    fireEvent.click(screen.getByRole("tab", { name: "日期" }));
    fireEvent.click(screen.getByRole("button", { name: "展开 2026-04-16 日期详情" }));

    expect(vi.mocked(useUsageDayDetailV1Query)).toHaveBeenLastCalledWith(
      {
        day: "2026-04-16",
        cliKey: null,
        providerId: null,
        folderLimit: 8,
        folderKeys: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      },
      expect.objectContaining({ enabled: true })
    );
    expect(screen.getByText("文件夹 Token 明细")).toBeInTheDocument();
    expect(screen.getByText("aio-coding-hub")).toBeInTheDocument();
    expect(screen.getByText("/Users/demo/aio-coding-hub")).toBeInTheDocument();
    const detailTable = screen.getByRole("table", { name: "日期文件夹用量明细" });
    const detailHeaders = within(detailTable)
      .getAllByRole("columnheader")
      .map((header) => header.textContent);
    expect(detailHeaders).toEqual(["文件夹", "总Token", "输入+输出", "缓存情况", "花费"]);
    expect(within(detailTable).getByText("4.2K")).toBeInTheDocument();
    expect(within(detailTable).getByText("3.6K")).toBeInTheDocument();
    expect(within(detailTable).getByLabelText("600/13.3%")).toBeInTheDocument();
    expect(
      within(detailTable).queryByRole("columnheader", { name: "请求数" })
    ).not.toBeInTheDocument();
    expect(
      within(detailTable).queryByRole("columnheader", { name: "成功率" })
    ).not.toBeInTheDocument();
    expect(
      within(detailTable).queryByRole("columnheader", { name: "平均耗时" })
    ).not.toBeInTheDocument();
    expect(screen.getByText("24 小时分布")).toBeInTheDocument();
    expect(screen.getByText("最早 13:00 · 最晚 13:00")).toBeInTheDocument();
    expect(screen.getAllByTestId("day-hour-bar")).toHaveLength(24);
  });

  it("orders expanded day hourly buckets by an explicitly configured non-midnight statistics day boundary", () => {
    window.localStorage.setItem("homeUsageDayStartHour", "5");
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 2,
        requests_with_usage: 2,
        requests_success: 2,
        requests_failed: 0,
        cost_covered_success: 2,
        total_duration_ms: 4_000,
        avg_duration_ms: 1000,
        avg_ttfb_ms: 200,
        avg_output_tokens_per_second: 100,
        input_tokens: 200,
        output_tokens: 100,
        io_total_tokens: 300,
        total_tokens: 400,
        cache_read_input_tokens: 100,
        cache_creation_input_tokens: 0,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockImplementation(
      (scope) =>
        ({
          data:
            scope === "day"
              ? [
                  {
                    key: "2026-04-16",
                    name: "2026-04-16",
                    requests_total: 2,
                    requests_success: 2,
                    requests_failed: 0,
                    total_tokens: 400,
                    io_total_tokens: 300,
                    input_tokens: 200,
                    output_tokens: 100,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 100,
                    total_duration_ms: 4_000,
                    first_request_created_at_ms: localTimeMs(2026, 3, 16, 9, 0),
                    last_request_created_at_ms: localTimeMs(2026, 3, 17, 2, 0),
                    avg_duration_ms: 1000,
                    avg_ttfb_ms: 200,
                    avg_output_tokens_per_second: 100,
                    cost_usd: 0.1,
                  },
                ]
              : [],
          isLoading: false,
          isFetching: false,
          error: null,
          refetch: vi.fn(),
        }) as any
    );
    vi.mocked(useUsageDayDetailV1Query).mockReturnValue({
      data: {
        day: "2026-04-16",
        folders: [],
        hours: Array.from({ length: 24 }, (_, hour) => ({
          hour,
          requests_total: hour === 2 || hour === 9 ? 1 : 0,
          total_tokens: hour === 2 ? 100 : hour === 9 ? 300 : 0,
          io_total_tokens: hour === 2 ? 80 : hour === 9 ? 220 : 0,
        })),
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel />);

    fireEvent.click(screen.getByRole("tab", { name: "日期" }));
    fireEvent.click(screen.getByRole("button", { name: "展开 2026-04-16 日期详情" }));

    expect(screen.getByText("最早 09:00 · 最晚 次日02:00")).toBeInTheDocument();
    expect(screen.getByText("05")).toBeInTheDocument();
    expect(screen.getByText("11")).toBeInTheDocument();
    expect(screen.getByText("17")).toBeInTheDocument();
    expect(screen.getByText("23")).toBeInTheDocument();
    expect(screen.getByText("次日04")).toBeInTheDocument();
    const hourBars = screen.getAllByTestId("day-hour-bar");
    expect(hourBars[4].parentElement).toHaveAttribute("title", "09:00 · 300 · 1 次请求");
    expect(hourBars[21].parentElement).toHaveAttribute("title", "次日02:00 · 100 · 1 次请求");
  });

  it("sorts day detail folders independently by clicked headers", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 6,
        requests_with_usage: 6,
        requests_success: 6,
        requests_failed: 0,
        cost_covered_success: 6,
        avg_duration_ms: 880,
        avg_ttfb_ms: 210,
        avg_output_tokens_per_second: 102.4,
        input_tokens: 3400,
        output_tokens: 1800,
        io_total_tokens: 5200,
        total_tokens: 6200,
        cache_read_input_tokens: 800,
        cache_creation_input_tokens: 200,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockImplementation(
      (scope) =>
        ({
          data:
            scope === "day"
              ? [
                  {
                    key: "2026-04-16",
                    name: "2026-04-16",
                    requests_total: 6,
                    requests_success: 6,
                    requests_failed: 0,
                    total_tokens: 6200,
                    io_total_tokens: 5200,
                    input_tokens: 3400,
                    output_tokens: 1800,
                    cache_creation_input_tokens: 200,
                    cache_read_input_tokens: 800,
                    avg_duration_ms: 880,
                    avg_ttfb_ms: 210,
                    avg_output_tokens_per_second: 102.4,
                    cost_usd: 0.6,
                  },
                ]
              : [],
          isLoading: false,
          isFetching: false,
          error: null,
          refetch: vi.fn(),
        }) as any
    );
    vi.mocked(useUsageDayDetailV1Query).mockReturnValue({
      data: makeSortableDayDetail("2026-04-16"),
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel />);

    fireEvent.click(screen.getByRole("tab", { name: "日期" }));
    fireEvent.click(screen.getByRole("button", { name: "展开 2026-04-16 日期详情" }));

    const detailTable = screen.getByRole("table", { name: "日期文件夹用量明细" });
    expectTableRowsInOrder(detailTable, ["Alpha app", "Bravo app", "Charlie app"]);

    clickSortableHeader(detailTable, /总Token/);
    expectTableRowsInOrder(detailTable, ["Bravo app", "Charlie app", "Alpha app"]);
    expect(within(detailTable).getByRole("columnheader", { name: /总Token/ })).toHaveAttribute(
      "aria-sort",
      "descending"
    );

    clickSortableHeader(detailTable, /总Token/);
    expectTableRowsInOrder(detailTable, ["Alpha app", "Charlie app", "Bravo app"]);
    expect(within(detailTable).getByRole("columnheader", { name: /总Token/ })).toHaveAttribute(
      "aria-sort",
      "ascending"
    );

    clickSortableHeader(detailTable, /输入\+输出/);
    expectTableRowsInOrder(detailTable, ["Bravo app", "Charlie app", "Alpha app"]);

    clickSortableHeader(detailTable, /缓存情况/);
    expectTableRowsInOrder(detailTable, ["Charlie app", "Alpha app", "Bravo app"]);
    expect(within(detailTable).getByRole("columnheader", { name: /缓存情况/ })).toHaveAttribute(
      "aria-sort",
      "descending"
    );
    expect(within(detailTable).getByRole("columnheader", { name: /输入\+输出/ })).toHaveAttribute(
      "aria-sort",
      "none"
    );

    clickSortableHeader(detailTable, "花费");
    expectTableRowsInOrder(detailTable, ["Charlie app", "Alpha app", "Bravo app"]);

    clickSortableHeader(detailTable, "文件夹");
    expectTableRowsInOrder(detailTable, ["Charlie app", "Bravo app", "Alpha app"]);
  });

  it("passes selected folder keys to home usage queries and day detail", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 6,
        requests_with_usage: 6,
        requests_success: 6,
        requests_failed: 0,
        cost_covered_success: 6,
        avg_duration_ms: 880,
        avg_ttfb_ms: 210,
        avg_output_tokens_per_second: 102.4,
        input_tokens: 3400,
        output_tokens: 1800,
        io_total_tokens: 5200,
        total_tokens: 6200,
        cache_read_input_tokens: 800,
        cache_creation_input_tokens: 200,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockImplementation(
      (scope) =>
        ({
          data:
            scope === "day"
              ? [
                  {
                    key: "2026-04-16",
                    name: "2026-04-16",
                    requests_total: 6,
                    requests_success: 6,
                    requests_failed: 0,
                    total_tokens: 6200,
                    io_total_tokens: 5200,
                    input_tokens: 3400,
                    output_tokens: 1800,
                    cache_creation_input_tokens: 200,
                    cache_read_input_tokens: 800,
                    avg_duration_ms: 880,
                    avg_ttfb_ms: 210,
                    avg_output_tokens_per_second: 102.4,
                    cost_usd: 0.6,
                  },
                ]
              : [
                  {
                    key: "provider-1",
                    name: "OpenAI 主供应商",
                    requests_total: 6,
                    requests_success: 6,
                    requests_failed: 0,
                    total_tokens: 6200,
                    io_total_tokens: 5200,
                    input_tokens: 3400,
                    output_tokens: 1800,
                    cache_creation_input_tokens: 200,
                    cache_read_input_tokens: 800,
                    avg_duration_ms: 880,
                    avg_ttfb_ms: 210,
                    avg_output_tokens_per_second: 102.4,
                    cost_usd: 0.6,
                  },
                ],
          isLoading: false,
          isFetching: false,
          error: null,
          refetch: vi.fn(),
        }) as any
    );

    render(<HomeTokenCostPanel />);

    expect(vi.mocked(useUsageFolderOptionsV1Query)).toHaveBeenLastCalledWith(
      "daily",
      {
        startTs: null,
        endTs: null,
        cliKey: null,
        providerId: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      },
      expect.objectContaining({ enabled: true })
    );

    fireEvent.click(screen.getByRole("button", { name: /全部文件夹/ }));
    fireEvent.click(screen.getByRole("checkbox", { name: /aio-coding-hub/ }));

    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith(
      "daily",
      expect.objectContaining({
        folderKeys: ["/Users/demo/aio-coding-hub"],
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      undefined
    );
    expect(vi.mocked(useUsageLeaderboardV2Query)).toHaveBeenLastCalledWith(
      "provider",
      "daily",
      expect.objectContaining({
        folderKeys: ["/Users/demo/aio-coding-hub"],
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      undefined
    );

    fireEvent.click(screen.getByRole("tab", { name: "日期" }));
    fireEvent.click(screen.getByRole("button", { name: "展开 2026-04-16 日期详情" }));

    expect(vi.mocked(useUsageDayDetailV1Query)).toHaveBeenLastCalledWith(
      expect.objectContaining({
        day: "2026-04-16",
        folderKeys: ["/Users/demo/aio-coding-hub"],
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      expect.objectContaining({ enabled: true })
    );
  });

  it("can include cx2cc gateway bridge usage when the filter is disabled", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 6,
        requests_with_usage: 6,
        requests_success: 6,
        requests_failed: 0,
        cost_covered_success: 6,
        avg_duration_ms: 880,
        avg_ttfb_ms: 210,
        avg_output_tokens_per_second: 102.4,
        input_tokens: 3400,
        output_tokens: 1800,
        io_total_tokens: 5200,
        total_tokens: 6200,
        cache_read_input_tokens: 800,
        cache_creation_input_tokens: 200,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockImplementation(
      (scope) =>
        ({
          data:
            scope === "day"
              ? [
                  {
                    key: "2026-04-16",
                    name: "2026-04-16",
                    requests_total: 6,
                    requests_success: 6,
                    requests_failed: 0,
                    total_tokens: 6200,
                    io_total_tokens: 5200,
                    input_tokens: 3400,
                    output_tokens: 1800,
                    cache_creation_input_tokens: 200,
                    cache_read_input_tokens: 800,
                    avg_duration_ms: 880,
                    avg_ttfb_ms: 210,
                    avg_output_tokens_per_second: 102.4,
                    cost_usd: 0.6,
                  },
                ]
              : [
                  {
                    key: "provider-1",
                    name: "OpenAI 主供应商",
                    requests_total: 6,
                    requests_success: 6,
                    requests_failed: 0,
                    total_tokens: 6200,
                    io_total_tokens: 5200,
                    input_tokens: 3400,
                    output_tokens: 1800,
                    cache_creation_input_tokens: 200,
                    cache_read_input_tokens: 800,
                    avg_duration_ms: 880,
                    avg_ttfb_ms: 210,
                    avg_output_tokens_per_second: 102.4,
                    cost_usd: 0.6,
                  },
                ],
          isLoading: false,
          isFetching: false,
          error: null,
          refetch: vi.fn(),
        }) as any
    );
    vi.mocked(useUsageDayDetailV1Query).mockReturnValue({
      data: makeDayDetail(),
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel />);

    fireEvent.click(screen.getByRole("switch", { name: "过滤转接重复用量" }));

    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith(
      "daily",
      expect.objectContaining({
        excludeCx2CcGatewayBridge: false,
        dayStartHour: 0,
      }),
      undefined
    );
    expect(vi.mocked(useUsageLeaderboardV2Query)).toHaveBeenLastCalledWith(
      "provider",
      "daily",
      expect.objectContaining({
        excludeCx2CcGatewayBridge: false,
        dayStartHour: 0,
      }),
      undefined
    );
    expect(vi.mocked(useUsageFolderOptionsV1Query)).toHaveBeenLastCalledWith(
      "daily",
      expect.objectContaining({
        excludeCx2CcGatewayBridge: false,
        dayStartHour: 0,
      }),
      expect.objectContaining({ enabled: true })
    );

    fireEvent.click(screen.getByRole("tab", { name: "日期" }));
    fireEvent.click(screen.getByRole("button", { name: "展开 2026-04-16 日期详情" }));

    expect(vi.mocked(useUsageDayDetailV1Query)).toHaveBeenLastCalledWith(
      expect.objectContaining({
        day: "2026-04-16",
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: false,
      }),
      expect.objectContaining({ enabled: true })
    );
  });

  it("shows an inline loading state while day detail is loading", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 1,
        requests_with_usage: 1,
        requests_success: 1,
        requests_failed: 0,
        cost_covered_success: 1,
        avg_duration_ms: 880,
        avg_ttfb_ms: 210,
        avg_output_tokens_per_second: 102.4,
        input_tokens: 3400,
        output_tokens: 1800,
        io_total_tokens: 5200,
        total_tokens: 6200,
        cache_read_input_tokens: 800,
        cache_creation_input_tokens: 200,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [
        {
          key: "2026-04-16",
          name: "2026-04-16",
          requests_total: 1,
          requests_success: 1,
          requests_failed: 0,
          total_tokens: 6200,
          io_total_tokens: 5200,
          input_tokens: 3400,
          output_tokens: 1800,
          cache_creation_input_tokens: 200,
          cache_read_input_tokens: 800,
          avg_duration_ms: 880,
          avg_ttfb_ms: 210,
          avg_output_tokens_per_second: 102.4,
          cost_usd: 0.6,
        },
      ],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageDayDetailV1Query).mockReturnValue({
      data: null,
      isLoading: true,
      isFetching: true,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel />);

    fireEvent.click(screen.getByRole("tab", { name: "日期" }));
    fireEvent.click(screen.getByRole("button", { name: "展开 2026-04-16 日期详情" }));

    expect(screen.getByText("加载日期详情中…")).toBeInTheDocument();
  });

  it("shows loading skeletons, disabled folder selector, and empty leaderboard copy", () => {
    vi.mocked(useUsageFolderOptionsV1Query).mockReturnValue({
      data: [],
      isLoading: true,
      isFetching: true,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: null,
      isLoading: true,
      isFetching: true,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [],
      isLoading: true,
      isFetching: true,
      error: null,
      refetch: vi.fn(),
    } as any);

    const { rerender } = render(<HomeTokenCostPanel />);

    expect(screen.queryByText("含缓存总 Token")).not.toBeInTheDocument();
    expect(screen.getByText("加载用量中…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /全部文件夹/ })).not.toBeDisabled();
    expect(screen.getByRole("button", { name: "导出 CSV" })).toBeDisabled();

    vi.mocked(useUsageFolderOptionsV1Query).mockReturnValue({
      data: [],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: null,
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    rerender(<HomeTokenCostPanel />);

    expect(screen.getByRole("button", { name: /全部文件夹/ })).toBeDisabled();
    expect(screen.getByText("当前时间范围暂无用量数据。")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "导出 CSV" })).toBeDisabled();
  });

  it("shows pending custom range copy and invalid range toast without querying", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 1,
        requests_with_usage: 1,
        requests_success: 1,
        requests_failed: 0,
        cost_covered_success: 1,
        avg_duration_ms: 100,
        avg_ttfb_ms: 50,
        avg_output_tokens_per_second: 80,
        input_tokens: 100,
        output_tokens: 50,
        io_total_tokens: 150,
        total_tokens: 150,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [
        {
          key: "provider",
          name: "Provider",
          requests_total: 1,
          requests_success: 1,
          requests_failed: 0,
          total_tokens: 150,
          io_total_tokens: 150,
          input_tokens: 100,
          output_tokens: 50,
          cache_creation_input_tokens: 0,
          cache_read_input_tokens: 0,
          avg_duration_ms: 100,
          avg_ttfb_ms: 50,
          avg_output_tokens_per_second: 80,
          cost_usd: 0.01,
        },
      ],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel />);

    fireEvent.change(screen.getByLabelText("开始日期"), { target: { value: "2026-04-30" } });
    fireEvent.change(screen.getByLabelText("结束日期"), { target: { value: "2026-04-01" } });
    fireEvent.click(screen.getByRole("button", { name: "自定义" }));

    expect(screen.getByRole("button", { name: "自定义" })).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByText("Provider")).toBeInTheDocument();
  });

  it("shows day detail empty and error states", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 1,
        requests_with_usage: 1,
        requests_success: 1,
        requests_failed: 0,
        cost_covered_success: 1,
        avg_duration_ms: 880,
        avg_ttfb_ms: 210,
        avg_output_tokens_per_second: 102.4,
        input_tokens: 3400,
        output_tokens: 1800,
        io_total_tokens: 5200,
        total_tokens: 6200,
        cache_read_input_tokens: 800,
        cache_creation_input_tokens: 200,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [
        {
          key: "2026-04-16",
          name: "2026-04-16",
          requests_total: 1,
          requests_success: 1,
          requests_failed: 0,
          total_tokens: 6200,
          io_total_tokens: 5200,
          input_tokens: 3400,
          output_tokens: 1800,
          cache_creation_input_tokens: 200,
          cache_read_input_tokens: 800,
          avg_duration_ms: 880,
          avg_ttfb_ms: 210,
          avg_output_tokens_per_second: 102.4,
          cost_usd: 0.6,
        },
      ],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageDayDetailV1Query)
      .mockReturnValueOnce({
        data: null,
        isLoading: false,
        isFetching: false,
        error: null,
        refetch: vi.fn(),
      } as any)
      .mockReturnValue({
        data: null,
        isLoading: false,
        isFetching: false,
        error: new Error("detail failed"),
        refetch: vi.fn(),
      } as any);

    render(<HomeTokenCostPanel />);

    fireEvent.click(screen.getByRole("tab", { name: "日期" }));
    fireEvent.click(screen.getByRole("button", { name: "展开 2026-04-16 日期详情" }));

    expect(screen.getByText(/日期详情加载失败：.*detail failed/)).toBeInTheDocument();
  });

  it("maps range filters to the expected usage query periods and bounds", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 3, 16, 10, 0, 0));

    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: null,
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel />);

    fireEvent.click(screen.getByRole("button", { name: "昨天" }));

    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith(
      "custom",
      expect.objectContaining({
        startTs: Math.floor(new Date(2026, 3, 15, 0, 0, 0).getTime() / 1000),
        endTs: Math.floor(new Date(2026, 3, 16, 0, 0, 0).getTime() / 1000),
        cliKey: null,
        providerId: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      undefined
    );
    expect(vi.mocked(useUsageLeaderboardV2Query)).toHaveBeenLastCalledWith(
      "provider",
      "custom",
      expect.objectContaining({
        startTs: Math.floor(new Date(2026, 3, 15, 0, 0, 0).getTime() / 1000),
        endTs: Math.floor(new Date(2026, 3, 16, 0, 0, 0).getTime() / 1000),
        cliKey: null,
        providerId: null,
        limit: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      undefined
    );

    fireEvent.click(screen.getByRole("button", { name: "最近 3 天" }));

    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith(
      "custom",
      expect.objectContaining({
        startTs: Math.floor(new Date(2026, 3, 14, 0, 0, 0).getTime() / 1000),
        endTs: Math.floor(new Date(2026, 3, 17, 0, 0, 0).getTime() / 1000),
        cliKey: null,
        providerId: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      undefined
    );
    expect(vi.mocked(useUsageLeaderboardV2Query)).toHaveBeenLastCalledWith(
      "provider",
      "custom",
      expect.objectContaining({
        startTs: Math.floor(new Date(2026, 3, 14, 0, 0, 0).getTime() / 1000),
        endTs: Math.floor(new Date(2026, 3, 17, 0, 0, 0).getTime() / 1000),
        cliKey: null,
        providerId: null,
        limit: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      undefined
    );

    fireEvent.click(screen.getByRole("button", { name: "最近 15 天" }));

    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith(
      "custom",
      expect.objectContaining({
        startTs: Math.floor(new Date(2026, 3, 2, 0, 0, 0).getTime() / 1000),
        endTs: Math.floor(new Date(2026, 3, 17, 0, 0, 0).getTime() / 1000),
        cliKey: null,
        providerId: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      undefined
    );
    expect(vi.mocked(useUsageLeaderboardV2Query)).toHaveBeenLastCalledWith(
      "provider",
      "custom",
      expect.objectContaining({
        startTs: Math.floor(new Date(2026, 3, 2, 0, 0, 0).getTime() / 1000),
        endTs: Math.floor(new Date(2026, 3, 17, 0, 0, 0).getTime() / 1000),
        cliKey: null,
        providerId: null,
        limit: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      undefined
    );

    fireEvent.click(screen.getByRole("button", { name: "最近 7 天" }));

    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith(
      "weekly",
      {
        startTs: null,
        endTs: null,
        cliKey: null,
        providerId: null,
        folderKeys: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      },
      undefined
    );

    fireEvent.click(screen.getByRole("button", { name: "当月" }));

    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith(
      "monthly",
      {
        startTs: null,
        endTs: null,
        cliKey: null,
        providerId: null,
        folderKeys: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      },
      undefined
    );
  });

  it("supports custom date range on the home usage panel", () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date(2026, 3, 16, 10, 0, 0));

    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 1,
        requests_with_usage: 1,
        requests_success: 1,
        requests_failed: 0,
        cost_covered_success: 1,
        avg_duration_ms: 1000,
        avg_ttfb_ms: 200,
        avg_output_tokens_per_second: 100,
        input_tokens: 1000,
        output_tokens: 500,
        io_total_tokens: 1500,
        total_tokens: 1800,
        cache_read_input_tokens: 300,
        cache_creation_input_tokens: 0,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [
        {
          key: "previous-provider",
          name: "Previous Provider",
          requests_total: 1,
          requests_success: 1,
          requests_failed: 0,
          total_tokens: 1800,
          io_total_tokens: 1500,
          input_tokens: 1000,
          output_tokens: 500,
          cache_creation_input_tokens: 0,
          cache_read_input_tokens: 300,
          avg_duration_ms: 1000,
          avg_ttfb_ms: 200,
          avg_output_tokens_per_second: 100,
          cost_usd: 0.1,
        },
      ],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel />);

    expect(screen.getByLabelText("开始日期")).toBeInTheDocument();
    expect(screen.getByLabelText("结束日期")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "自定义" })).toHaveAttribute("aria-pressed", "false");
    expect(screen.getByText("Previous Provider")).toBeInTheDocument();
    expect(
      vi.mocked(useUsageSummaryV2Query).mock.calls.some(([period]) => period === "custom")
    ).toBe(false);
    expect(vi.mocked(useUsageFolderOptionsV1Query)).toHaveBeenLastCalledWith(
      "daily",
      expect.objectContaining({
        startTs: null,
        endTs: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      expect.objectContaining({ enabled: true })
    );
    expect(vi.mocked(useUsageDayDetailV1Query)).toHaveBeenLastCalledWith(
      expect.objectContaining({
        day: "",
        cliKey: null,
        providerId: null,
        folderLimit: 8,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      expect.objectContaining({ enabled: false })
    );
    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith(
      "daily",
      expect.objectContaining({
        startTs: null,
        endTs: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      undefined
    );

    fireEvent.change(screen.getByLabelText("开始日期"), { target: { value: "2026-04-01" } });
    fireEvent.change(screen.getByLabelText("结束日期"), { target: { value: "2026-04-30" } });
    fireEvent.click(screen.getByRole("button", { name: "自定义" }));

    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith(
      "custom",
      expect.objectContaining({
        startTs: Math.floor(new Date(2026, 3, 1, 0, 0, 0).getTime() / 1000),
        endTs: Math.floor(new Date(2026, 4, 1, 0, 0, 0).getTime() / 1000),
        cliKey: null,
        providerId: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      undefined
    );
    expect(vi.mocked(useUsageLeaderboardV2Query)).toHaveBeenLastCalledWith(
      "provider",
      "custom",
      expect.objectContaining({
        startTs: Math.floor(new Date(2026, 3, 1, 0, 0, 0).getTime() / 1000),
        endTs: Math.floor(new Date(2026, 4, 1, 0, 0, 0).getTime() / 1000),
        cliKey: null,
        providerId: null,
        limit: null,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      undefined
    );
    expect(screen.getByRole("button", { name: "自定义" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.queryByText("2026-04-01 至 2026-04-30")).not.toBeInTheDocument();
    expect(screen.queryByText(/点击"应用"/)).not.toBeInTheDocument();
  });

  it("sorts the leaderboard by remaining metric headers and sparse values", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 7,
        requests_with_usage: 7,
        requests_success: 6,
        requests_failed: 1,
        cost_covered_success: 6,
        total_duration_ms: 4_000,
        avg_duration_ms: 1000,
        avg_ttfb_ms: 200,
        avg_output_tokens_per_second: 100,
        input_tokens: 300,
        output_tokens: 100,
        io_total_tokens: 400,
        total_tokens: 500,
        cache_read_input_tokens: 100,
        cache_creation_input_tokens: 0,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [
        {
          key: "alpha",
          name: "Alpha Provider",
          requests_total: 0,
          requests_success: 0,
          requests_failed: 0,
          total_tokens: 0,
          io_total_tokens: 0,
          input_tokens: 0,
          output_tokens: 0,
          cache_creation_input_tokens: 0,
          cache_read_input_tokens: 0,
          total_duration_ms: 0,
          avg_duration_ms: 0,
          avg_ttfb_ms: 0,
          avg_output_tokens_per_second: null,
          cost_usd: null,
        },
        {
          key: "bravo",
          name: "Bravo Provider",
          requests_total: 5,
          requests_success: 4,
          requests_failed: 1,
          total_tokens: 200,
          io_total_tokens: 100,
          input_tokens: 50,
          output_tokens: 50,
          cache_creation_input_tokens: 20,
          cache_read_input_tokens: 80,
          total_duration_ms: 5_000,
          avg_duration_ms: 900,
          avg_ttfb_ms: 180,
          avg_output_tokens_per_second: 40,
          cost_usd: 0.5,
        },
        {
          key: "charlie",
          name: "Charlie Provider",
          requests_total: 2,
          requests_success: 2,
          requests_failed: 0,
          total_tokens: 300,
          io_total_tokens: 300,
          input_tokens: 250,
          output_tokens: 50,
          cache_creation_input_tokens: 0,
          cache_read_input_tokens: 0,
          total_duration_ms: 2_000,
          avg_duration_ms: 700,
          avg_ttfb_ms: 120,
          avg_output_tokens_per_second: 80,
          cost_usd: 0.2,
        },
      ],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel />);

    const table = screen.getByRole("table", { name: "用量排行榜" });
    expectTableRowsInOrder(table, ["Alpha Provider", "Bravo Provider", "Charlie Provider"]);

    clickSortableHeader(table, /输入\+出\/缓存率/);
    expectTableRowsInOrder(table, ["Charlie Provider", "Bravo Provider", "Alpha Provider"]);

    clickSortableHeader(table, /总花费/);
    expectTableRowsInOrder(table, ["Bravo Provider", "Charlie Provider", "Alpha Provider"]);

    clickSortableHeader(table, /总耗时/);
    expectTableRowsInOrder(table, ["Bravo Provider", "Charlie Provider", "Alpha Provider"]);

    clickSortableHeader(table, /请求数/);
    expectTableRowsInOrder(table, ["Bravo Provider", "Charlie Provider", "Alpha Provider"]);

    clickSortableHeader(table, /Token 占比/);
    expectTableRowsInOrder(table, ["Charlie Provider", "Bravo Provider", "Alpha Provider"]);

    clickSortableHeader(table, "供应商");
    expectTableRowsInOrder(table, ["Charlie Provider", "Bravo Provider", "Alpha Provider"]);
    clickSortableHeader(table, "供应商");
    expectTableRowsInOrder(table, ["Alpha Provider", "Bravo Provider", "Charlie Provider"]);
    expect(within(table).getByRole("columnheader", { name: "供应商" })).toHaveAttribute(
      "aria-sort",
      "ascending"
    );
  });

  it("clears and toggles folder filters from the multi-select", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 1,
        requests_with_usage: 1,
        requests_success: 1,
        requests_failed: 0,
        cost_covered_success: 1,
        avg_duration_ms: 1000,
        avg_ttfb_ms: 200,
        avg_output_tokens_per_second: 100,
        input_tokens: 1000,
        output_tokens: 500,
        io_total_tokens: 1500,
        total_tokens: 1800,
        cache_read_input_tokens: 300,
        cache_creation_input_tokens: 0,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [
        {
          key: "provider-1",
          name: "Provider",
          requests_total: 1,
          requests_success: 1,
          requests_failed: 0,
          total_tokens: 1800,
          io_total_tokens: 1500,
          input_tokens: 1000,
          output_tokens: 500,
          cache_creation_input_tokens: 0,
          cache_read_input_tokens: 300,
          avg_duration_ms: 1000,
          avg_ttfb_ms: 200,
          avg_output_tokens_per_second: 100,
          cost_usd: 0.1,
        },
      ],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel />);

    fireEvent.click(screen.getByRole("button", { name: /全部文件夹/ }));
    expect(screen.getByRole("button", { name: "清空文件夹筛选" })).toBeDisabled();

    fireEvent.click(screen.getByRole("checkbox", { name: /aio-coding-hub/ }));
    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith(
      "daily",
      expect.objectContaining({ folderKeys: ["/Users/demo/aio-coding-hub"] }),
      undefined
    );

    fireEvent.click(screen.getByRole("checkbox", { name: /aio-coding-hub/ }));
    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith(
      "daily",
      expect.objectContaining({ folderKeys: null }),
      undefined
    );

    fireEvent.click(screen.getByRole("checkbox", { name: /aio-coding-hub/ }));
    fireEvent.click(screen.getByRole("checkbox", { name: /未知文件夹/ }));
    expect(screen.getByRole("button", { name: /2 个文件夹/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "清空文件夹筛选" })).toBeEnabled();

    fireEvent.click(screen.getByRole("button", { name: "清空文件夹筛选" }));
    expect(vi.mocked(useUsageSummaryV2Query)).toHaveBeenLastCalledWith(
      "daily",
      expect.objectContaining({ folderKeys: null }),
      undefined
    );
  });

  it("renders empty day detail folders and inactive hourly range", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 1,
        requests_with_usage: 1,
        requests_success: 1,
        requests_failed: 0,
        cost_covered_success: 1,
        avg_duration_ms: 880,
        avg_ttfb_ms: 210,
        avg_output_tokens_per_second: 102.4,
        input_tokens: 3400,
        output_tokens: 1800,
        io_total_tokens: 5200,
        total_tokens: 6200,
        cache_read_input_tokens: 800,
        cache_creation_input_tokens: 200,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockImplementation(
      (scope) =>
        ({
          data:
            scope === "day"
              ? [
                  {
                    key: "2026-04-16",
                    name: "2026-04-16",
                    requests_total: 1,
                    requests_success: 1,
                    requests_failed: 0,
                    total_tokens: 6200,
                    io_total_tokens: 5200,
                    input_tokens: 3400,
                    output_tokens: 1800,
                    cache_creation_input_tokens: 200,
                    cache_read_input_tokens: 800,
                    avg_duration_ms: 880,
                    avg_ttfb_ms: 210,
                    avg_output_tokens_per_second: 102.4,
                    cost_usd: 0.6,
                  },
                ]
              : [],
          isLoading: false,
          isFetching: false,
          error: null,
          refetch: vi.fn(),
        }) as any
    );
    vi.mocked(useUsageDayDetailV1Query).mockReturnValue({
      data: {
        day: "2026-04-16",
        folders: [],
        hours: Array.from({ length: 24 }, (_, hour) => ({
          hour,
          requests_total: 0,
          total_tokens: 0,
          io_total_tokens: 0,
        })),
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel />);

    fireEvent.click(screen.getByRole("tab", { name: "日期" }));
    fireEvent.click(screen.getByRole("button", { name: "展开 2026-04-16 日期详情" }));

    expect(screen.getByText("当天暂无可展示的文件夹用量。")).toBeInTheDocument();
    expect(screen.getByText("最早 — · 最晚 —")).toBeInTheDocument();
    expect(screen.getAllByTestId("day-hour-bar")).toHaveLength(24);
  });

  it("renders preview rows when dev preview is enabled and queries are empty", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: null,
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel devPreviewEnabled={true} />);

    expect(screen.getByText("OpenAI Primary")).toBeInTheDocument();
    expect(screen.getByText("Gemini Mirror")).toBeInTheDocument();
    expect(screen.getByText("99.0K")).toBeInTheDocument();
    expect(screen.getByText("$3.36")).toBeInTheDocument();
    const previewProviderRow = screen.getByText("OpenAI Primary").closest("tr");
    expect(previewProviderRow).toBeTruthy();
    expect(within(previewProviderRow as HTMLElement).getByText("49.2K")).toBeInTheDocument();
    expect(within(previewProviderRow as HTMLElement).getByText("42K")).toBeInTheDocument();
    expect(
      within(previewProviderRow as HTMLElement).getByLabelText("42K/13.1%")
    ).toBeInTheDocument();
    expect(screen.getByText("17.0%")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("tab", { name: "模型" }));

    expect(screen.getByText("gpt-5.4")).toBeInTheDocument();
    expect(screen.getByText("claude-3.7-sonnet")).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: "最近 30 天" }));

    expect(screen.getByText("3.0M")).toBeInTheDocument();
    expect(screen.getByText("$100.80")).toBeInTheDocument();
  });

  it("uses synthetic day detail in dev preview without enabling backend detail query", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: null,
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);
    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel devPreviewEnabled={true} />);

    fireEvent.click(screen.getByRole("tab", { name: "日期" }));
    const firstDayButton = screen.getAllByRole("button", { name: /展开 .* 日期详情/ })[0];
    fireEvent.click(firstDayButton);

    expect(vi.mocked(useUsageDayDetailV1Query)).toHaveBeenLastCalledWith(
      expect.objectContaining({
        cliKey: null,
        providerId: null,
        folderLimit: 8,
        dayStartHour: 0,
        excludeCx2CcGatewayBridge: true,
      }),
      expect.objectContaining({ enabled: false })
    );
    expect(screen.getByText("文件夹 Token 明细")).toBeInTheDocument();
    expect(screen.getByText("aio-coding-hub")).toBeInTheDocument();
    expect(screen.getByText("24 小时分布")).toBeInTheDocument();
    expect(screen.getAllByTestId("day-hour-bar")).toHaveLength(24);
  });

  it("renders cache hit-rate per row (not the old token-share ratio)", () => {
    // Row picked so the cache-token ratio and cache hit-rate diverge sharply.
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 5,
        requests_with_usage: 5,
        requests_success: 5,
        requests_failed: 0,
        cost_covered_success: 5,
        avg_duration_ms: 1000,
        avg_ttfb_ms: 200,
        avg_output_tokens_per_second: 100,
        input_tokens: 1000,
        output_tokens: 6000,
        io_total_tokens: 7000,
        total_tokens: 16000,
        cache_read_input_tokens: 9000,
        cache_creation_input_tokens: 0,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [
        {
          key: "provider-cache",
          name: "Cache Hit Provider",
          requests_total: 5,
          requests_success: 5,
          requests_failed: 0,
          total_tokens: 16000,
          io_total_tokens: 7000,
          input_tokens: 1000,
          output_tokens: 6000,
          cache_creation_input_tokens: 0,
          cache_read_input_tokens: 9000,
          avg_duration_ms: 1000,
          avg_ttfb_ms: 200,
          avg_output_tokens_per_second: 100,
          cost_usd: 0.5,
        },
      ],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel />);

    expect(screen.getByLabelText("7K/90%")).toBeInTheDocument();
    expect(screen.queryByText(/56\.3%/)).not.toBeInTheDocument();
    expect(screen.getByText("90.0%")).toBeInTheDocument();
  });

  it("falls back to dashes when a row has no cache data", () => {
    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: {
        requests_total: 1,
        requests_with_usage: 1,
        requests_success: 1,
        requests_failed: 0,
        cost_covered_success: 1,
        avg_duration_ms: 100,
        avg_ttfb_ms: 50,
        avg_output_tokens_per_second: 80,
        input_tokens: 0,
        output_tokens: 0,
        io_total_tokens: 0,
        total_tokens: 0,
        cache_read_input_tokens: 0,
        cache_creation_input_tokens: 0,
        cache_creation_5m_input_tokens: 0,
      },
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [
        {
          key: "provider-empty",
          name: "Empty Provider",
          requests_total: 1,
          requests_success: 1,
          requests_failed: 0,
          total_tokens: 0,
          io_total_tokens: 0,
          input_tokens: 0,
          output_tokens: 0,
          cache_creation_input_tokens: 0,
          cache_read_input_tokens: 0,
          avg_duration_ms: 100,
          avg_ttfb_ms: 50,
          avg_output_tokens_per_second: 80,
          cost_usd: 0,
        },
      ],
      isLoading: false,
      isFetching: false,
      error: null,
      refetch: vi.fn(),
    } as any);

    render(<HomeTokenCostPanel />);

    expect(screen.getByLabelText("0/—")).toBeInTheDocument();
  });

  it("retries summary and leaderboard queries from the error card", () => {
    const refetchSummary = vi.fn();
    const refetchLeaderboard = vi.fn();

    vi.mocked(useUsageSummaryV2Query).mockReturnValue({
      data: null,
      isLoading: false,
      isFetching: false,
      error: new Error("summary failed"),
      refetch: refetchSummary,
    } as any);

    vi.mocked(useUsageLeaderboardV2Query).mockReturnValue({
      data: [],
      isLoading: false,
      isFetching: false,
      error: new Error("leaderboard failed"),
      refetch: refetchLeaderboard,
    } as any);

    render(<HomeTokenCostPanel />);

    fireEvent.click(screen.getByRole("button", { name: "重试" }));

    expect(refetchSummary).toHaveBeenCalled();
    expect(refetchLeaderboard).toHaveBeenCalled();
  });
});
