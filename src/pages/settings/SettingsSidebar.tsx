import type { UpdateMeta } from "../../hooks/useUpdateMeta";
import { SettingsAboutCard } from "./SettingsAboutCard";
import { SettingsDataManagementCard } from "./SettingsDataManagementCard";
import { SettingsDataSyncCard } from "./SettingsDataSyncCard";
import { SettingsDialogs } from "./SettingsDialogs";
import { useSettingsSidebar } from "./useSettingsSidebar";

export type SettingsSidebarProps = {
  updateMeta: UpdateMeta;
};

export function SettingsSidebar({ updateMeta }: SettingsSidebarProps) {
  const sidebar = useSettingsSidebar(updateMeta);

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
