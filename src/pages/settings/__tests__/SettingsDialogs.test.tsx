import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { SettingsDialogs } from "../SettingsDialogs";

vi.mock("../../../components/settings/ModelPriceAliasesDialog", () => ({
  ModelPriceAliasesDialog: () => <div>aliases-dialog</div>,
}));

describe("pages/settings/SettingsDialogs", () => {
  it("prevents closing clear request logs dialog while in progress", () => {
    const setClearOpen = vi.fn();
    const setClearing = vi.fn();

    render(
      <SettingsDialogs
        modelPriceAliasesDialogOpen={false}
        setModelPriceAliasesDialogOpen={vi.fn()}
        clearRequestLogsDialogOpen={true}
        setClearRequestLogsDialogOpen={setClearOpen}
        clearingRequestLogs={true}
        setClearingRequestLogs={setClearing}
        clearRequestLogs={vi.fn().mockResolvedValue(undefined)}
        resetAllDialogOpen={false}
        setResetAllDialogOpen={vi.fn()}
        resettingAll={false}
        setResettingAll={vi.fn()}
        resetAllData={vi.fn().mockResolvedValue(undefined)}
        configImportDialogOpen={false}
        setConfigImportDialogOpen={vi.fn()}
        importingConfig={false}
        setImportingConfig={vi.fn()}
        pendingConfigBundle={null}
        confirmConfigImport={vi.fn().mockResolvedValue(undefined)}
      />
    );

    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(setClearOpen).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "取消" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "清理中…" })).toBeDisabled();
  });

  it("closes clear request logs dialog and resets pending flag when dismissed", () => {
    const setClearOpen = vi.fn();
    const setClearing = vi.fn();

    render(
      <SettingsDialogs
        modelPriceAliasesDialogOpen={false}
        setModelPriceAliasesDialogOpen={vi.fn()}
        clearRequestLogsDialogOpen={true}
        setClearRequestLogsDialogOpen={setClearOpen}
        clearingRequestLogs={false}
        setClearingRequestLogs={setClearing}
        clearRequestLogs={vi.fn().mockResolvedValue(undefined)}
        resetAllDialogOpen={false}
        setResetAllDialogOpen={vi.fn()}
        resettingAll={false}
        setResettingAll={vi.fn()}
        resetAllData={vi.fn().mockResolvedValue(undefined)}
        configImportDialogOpen={false}
        setConfigImportDialogOpen={vi.fn()}
        importingConfig={false}
        setImportingConfig={vi.fn()}
        pendingConfigBundle={null}
        confirmConfigImport={vi.fn().mockResolvedValue(undefined)}
      />
    );

    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });

    expect(setClearOpen).toHaveBeenCalledWith(false);
    expect(setClearing).toHaveBeenCalledWith(false);
  });

  it("prevents closing reset all dialog while in progress", () => {
    const setResetOpen = vi.fn();
    const setResetting = vi.fn();

    render(
      <SettingsDialogs
        modelPriceAliasesDialogOpen={false}
        setModelPriceAliasesDialogOpen={vi.fn()}
        clearRequestLogsDialogOpen={false}
        setClearRequestLogsDialogOpen={vi.fn()}
        clearingRequestLogs={false}
        setClearingRequestLogs={vi.fn()}
        clearRequestLogs={vi.fn().mockResolvedValue(undefined)}
        resetAllDialogOpen={true}
        setResetAllDialogOpen={setResetOpen}
        resettingAll={true}
        setResettingAll={setResetting}
        resetAllData={vi.fn().mockResolvedValue(undefined)}
        configImportDialogOpen={false}
        setConfigImportDialogOpen={vi.fn()}
        importingConfig={false}
        setImportingConfig={vi.fn()}
        pendingConfigBundle={null}
        confirmConfigImport={vi.fn().mockResolvedValue(undefined)}
      />
    );

    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });
    expect(setResetOpen).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "取消" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "清理中…" })).toBeDisabled();
  });

  it("closes reset all dialog and resets pending flag when dismissed", () => {
    const setResetOpen = vi.fn();
    const setResetting = vi.fn();

    render(
      <SettingsDialogs
        modelPriceAliasesDialogOpen={false}
        setModelPriceAliasesDialogOpen={vi.fn()}
        clearRequestLogsDialogOpen={false}
        setClearRequestLogsDialogOpen={vi.fn()}
        clearingRequestLogs={false}
        setClearingRequestLogs={vi.fn()}
        clearRequestLogs={vi.fn().mockResolvedValue(undefined)}
        resetAllDialogOpen={true}
        setResetAllDialogOpen={setResetOpen}
        resettingAll={false}
        setResettingAll={setResetting}
        resetAllData={vi.fn().mockResolvedValue(undefined)}
        configImportDialogOpen={false}
        setConfigImportDialogOpen={vi.fn()}
        importingConfig={false}
        setImportingConfig={vi.fn()}
        pendingConfigBundle={null}
        confirmConfigImport={vi.fn().mockResolvedValue(undefined)}
      />
    );

    fireEvent.keyDown(screen.getByRole("dialog"), { key: "Escape" });

    expect(setResetOpen).toHaveBeenCalledWith(false);
    expect(setResetting).toHaveBeenCalledWith(false);
  });
});
