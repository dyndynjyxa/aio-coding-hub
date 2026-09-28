import { render, screen } from "@testing-library/react";
import type { RefObject } from "react";
import { describe, expect, it, vi } from "vitest";
import type { UsageDataPanelProps } from "../UsageDataPanel";
import { UsageDataPanel } from "../UsageDataPanel";

vi.mock("../UsageDataPanelContent", () => ({
  UsageDataPanelContent: ({
    contentRef,
    overlayOpen,
    activeStale,
    tableTab,
  }: UsageDataPanelProps & {
    contentRef: RefObject<HTMLDivElement | null>;
    overlayOpen: boolean;
    activeStale: boolean;
  }) => (
    <div
      ref={contentRef}
      data-testid="usage-panel-content"
      data-active-stale={String(activeStale)}
      data-overlay-open={String(overlayOpen)}
      data-table-tab={tableTab}
    />
  ),
}));

const BASE_PROPS: UsageDataPanelProps = {
  tableTab: "usage",
  onChangeTableTab: vi.fn(),
  scope: "cli",
  onChangeScope: vi.fn(),
  loading: false,
  dataLoading: false,
  cacheTrendLoading: false,
  metricsTrendLoading: false,
  dataStale: false,
  cacheTrendStale: false,
  metricsTrendStale: false,
  errorText: null,
  tableTitle: "Usage",
  summary: null,
  rows: [],
  totalCostUsd: 0,
  cacheTrendRows: [],
  cacheTrendProviderCount: 0,
  metricsTrendRows: [],
  metricsTrendProviderCount: 0,
  providerSelectValue: "all",
  providerOptions: [],
  onProviderIdChange: vi.fn(),
  providersLoading: false,
  period: "daily",
  customApplied: null,
  customPending: false,
  availabilityData: null,
  availabilityLoading: false,
  availabilityRefreshing: false,
  onRefreshAvailability: vi.fn(),
};

function renderPanel(overrides: Partial<UsageDataPanelProps> = {}) {
  render(<UsageDataPanel {...BASE_PROPS} {...overrides} />);
  return screen.getByTestId("usage-panel-content");
}

describe("pages/usage/UsageDataPanel", () => {
  it.each([
    {
      name: "keeps overlay closed when custom range is not pending",
      overrides: {
        customPending: false,
        rows: [{ key: "cli:claude" }],
        dataStale: true,
      },
      overlayOpen: false,
      activeStale: true,
    },
    {
      name: "opens overlay for stale usage rows while custom range is pending",
      overrides: {
        customPending: true,
        rows: [{ key: "cli:claude" }],
        dataStale: true,
      },
      overlayOpen: true,
      activeStale: true,
    },
    {
      name: "keeps overlay closed for empty usage data",
      overrides: {
        customPending: true,
        rows: [],
        summary: null,
      },
      overlayOpen: false,
      activeStale: false,
    },
    {
      name: "opens overlay for cache trend rows and uses cache stale state",
      overrides: {
        tableTab: "cacheTrend",
        customPending: true,
        cacheTrendRows: [{ provider_id: 1 }],
        cacheTrendStale: true,
      },
      overlayOpen: true,
      activeStale: true,
    },
    {
      name: "keeps overlay closed for empty cache trend data",
      overrides: {
        tableTab: "cacheTrend",
        customPending: true,
        cacheTrendRows: [],
        cacheTrendStale: false,
      },
      overlayOpen: false,
      activeStale: false,
    },
    {
      name: "opens overlay for metrics trend rows and uses metrics stale state",
      overrides: {
        tableTab: "metricsTrend",
        customPending: true,
        metricsTrendRows: [{ key: "codex:1" }],
        metricsTrendStale: true,
      },
      overlayOpen: true,
      activeStale: true,
    },
    {
      name: "keeps overlay closed for empty metrics trend data",
      overrides: {
        tableTab: "metricsTrend",
        customPending: true,
        metricsTrendRows: [],
        metricsTrendStale: false,
      },
      overlayOpen: false,
      activeStale: false,
    },
    {
      name: "opens overlay for availability providers and uses refresh state",
      overrides: {
        tableTab: "availability",
        customPending: true,
        availabilityData: { providers: [{ providerId: 1, providerName: "Provider A" }] } as any,
        availabilityRefreshing: true,
      },
      overlayOpen: true,
      activeStale: true,
    },
    {
      name: "keeps overlay closed before availability data loads",
      overrides: {
        tableTab: "availability",
        customPending: true,
        availabilityData: null,
        availabilityRefreshing: false,
      },
      overlayOpen: false,
      activeStale: false,
    },
  ])("$name", ({ overrides, overlayOpen, activeStale }) => {
    const content = renderPanel(overrides as Partial<UsageDataPanelProps>);

    expect(content).toHaveAttribute("data-overlay-open", String(overlayOpen));
    expect(content).toHaveAttribute("data-active-stale", String(activeStale));
    if (overlayOpen) {
      expect(document.querySelector("output")).toBeInstanceOf(HTMLOutputElement);
    } else {
      expect(document.querySelector("output")).toBeNull();
    }
  });
});
