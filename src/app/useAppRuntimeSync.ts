import { useGatewayQuerySync } from "../hooks/useGatewayQuerySync";
import { useSettingsRuntimeBridge } from "./useSettingsRuntimeBridge";

export function useAppRuntimeSync() {
  useGatewayQuerySync();
  useSettingsRuntimeBridge();
}
