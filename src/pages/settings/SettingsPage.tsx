// Usage: App settings and gateway controls. Backend commands: `settings_*`, `gateway_*`, `cli_proxy_*`, `model_prices_*`, `usage_*`, `app_data_*`.

import { useGatewayMeta } from "../../hooks/useGatewayMeta";
import { useUpdateMeta } from "../../hooks/useUpdateMeta";
import { PageHeader } from "../../ui/PageHeader";
import { SettingsMainColumn } from "./SettingsMainColumn";
import { SettingsSidebar } from "./SettingsSidebar";
import { useSettingsPersistence } from "./useSettingsPersistence";
import { useSystemNotification } from "./useSystemNotification";

export function SettingsPage() {
  const { gateway, gatewayAvailable } = useGatewayMeta();
  const updateMeta = useUpdateMeta();
  const about = updateMeta.about;

  const persistence = useSettingsPersistence({ gateway, about });
  const notice = useSystemNotification();

  return (
    <div className="flex flex-col gap-6">
      <PageHeader title="设置" />
      <div className="grid grid-cols-1 gap-6 lg:grid-cols-12 lg:items-start">
        <SettingsMainColumn
          gateway={gateway}
          gatewayAvailable={gatewayAvailable}
          settingsReady={persistence.settingsReady}
          port={persistence.port}
          setPort={persistence.setPort}
          commitNumberField={persistence.commitNumberField}
          autoStart={persistence.autoStart}
          setAutoStart={persistence.setAutoStart}
          trayEnabled={persistence.trayEnabled}
          setTrayEnabled={persistence.setTrayEnabled}
          logRetentionDays={persistence.logRetentionDays}
          setLogRetentionDays={persistence.setLogRetentionDays}
          requestPersist={persistence.requestPersist}
          noticePermissionStatus={notice.noticePermissionStatus}
          requestingNoticePermission={notice.requestingNoticePermission}
          sendingNoticeTest={notice.sendingNoticeTest}
          requestSystemNotificationPermission={notice.requestSystemNotificationPermission}
          sendSystemNotificationTest={notice.sendSystemNotificationTest}
        />

        <SettingsSidebar updateMeta={updateMeta} />
      </div>
    </div>
  );
}
