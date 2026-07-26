import type { UpdateMeta } from "../../hooks/useUpdateMeta";
import { SettingsAboutCard } from "./SettingsAboutCard";
import { SettingsDataManagementCard } from "./SettingsDataManagementCard";
import { SettingsDataSyncCard } from "./SettingsDataSyncCard";
import { SettingsDialogs } from "./SettingsDialogs";
import { useSettingsSidebar } from "./useSettingsSidebar";

export type SettingsSidebarProps = {
  updateMeta: UpdateMeta;
  requestLogRetentionDays?: number | null;
};

export function SettingsSidebar({
  updateMeta,
  requestLogRetentionDays = null,
}: SettingsSidebarProps) {
  const sidebar = useSettingsSidebar(updateMeta, requestLogRetentionDays);

  return (
    <>
      <div className="space-y-6 lg:col-span-4">
        <SettingsAboutCard {...sidebar.aboutCardProps} />

        <SettingsDataManagementCard {...sidebar.dataManagementCardProps} />

        <SettingsDataSyncCard {...sidebar.dataSyncCardProps} />
      </div>

      <SettingsDialogs {...sidebar.dialogsProps} />
    </>
  );
}
