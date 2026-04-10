import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useState } from "react";
import { toast } from "sonner";
import { BaseUrlEditor } from "../BaseUrlEditor";
import type { BaseUrlRow } from "../types";
import { baseUrlPingMs } from "../../../services/providers/providers";

vi.mock("sonner", () => ({ toast: vi.fn() }));

vi.mock("../../../services/providers/providers", async () => {
  const actual = await vi.importActual<typeof import("../../../services/providers/providers")>(
    "../../../services/providers/providers"
  );
  return { ...actual, baseUrlPingMs: vi.fn() };
});

function TestWrapper({ initial }: { initial: BaseUrlRow[] }) {
  const [rows, setRows] = useState<BaseUrlRow[]>(initial);
  const [pingingAll, setPingingAll] = useState(false);
  const newRow = (url = ""): BaseUrlRow => ({
    id: String(rows.length + 1),
    url,
    ping: { status: "idle" },
  });

  return (
    <BaseUrlEditor
      rows={rows}
      setRows={setRows}
      pingingAll={pingingAll}
      setPingingAll={setPingingAll}
      newRow={newRow}
    />
  );
}

describe("pages/providers/BaseUrlEditor", () => {
  it("pings base urls and handles empty/tauri-only/error cases", async () => {
    vi.mocked(baseUrlPingMs).mockResolvedValueOnce(123);
    vi.mocked(baseUrlPingMs).mockResolvedValueOnce(null);
    vi.mocked(baseUrlPingMs).mockRejectedValueOnce(new Error("boom"));

    render(<TestWrapper initial={[{ id: "1", url: "", ping: { status: "idle" } }]} />);

    fireEvent.click(screen.getByRole("button", { name: "Ping" }));
    expect(vi.mocked(toast)).toHaveBeenCalledWith("Base URL 不能为空");

    fireEvent.change(screen.getByPlaceholderText("https://api.openai.com"), {
      target: { value: "https://example.com" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Ping" }));
    await waitFor(() => expect(screen.getByText("123ms")).toBeInTheDocument());

    fireEvent.click(screen.getByRole("button", { name: "Ping" }));
    await waitFor(() => expect(vi.mocked(baseUrlPingMs)).toHaveBeenCalledTimes(2));

    fireEvent.click(screen.getByRole("button", { name: "Ping" }));
    await waitFor(() => expect(screen.getByText("失败")).toBeInTheDocument());
  });

  it("supports adding and pinging all rows", async () => {
    vi.mocked(baseUrlPingMs).mockResolvedValue(10);

    render(<TestWrapper initial={[{ id: "1", url: "https://a", ping: { status: "idle" } }]} />);

    fireEvent.click(screen.getByRole("button", { name: "+ 添加" }));
    const inputs = screen.getAllByPlaceholderText("https://api.openai.com");
    fireEvent.change(inputs[1]!, { target: { value: "https://b" } });

    fireEvent.click(screen.getByRole("button", { name: "全部 Ping" }));
    await waitFor(() => expect(screen.getAllByText(/ms$/).length).toBeGreaterThanOrEqual(2));
  });
});
