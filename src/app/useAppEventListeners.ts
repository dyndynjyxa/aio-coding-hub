import { useAsyncListener } from "../hooks/useAsyncListener";
import { listenAppHeartbeat } from "../services/app/appHeartbeat";
import { listenGatewayEvents } from "../services/gateway/gatewayEvents";
import { listenNoticeEvents } from "../services/notification/noticeEvents";
import { listenTaskCompleteNotifyEvents } from "../services/notification/taskCompleteNotifyEvents";

export function useAppEventListeners() {
  useAsyncListener(listenAppHeartbeat, "listenAppHeartbeat", "应用心跳监听初始化失败");
  useAsyncListener(listenGatewayEvents, "listenGatewayEvents", "网关事件监听初始化失败");
  useAsyncListener(listenNoticeEvents, "listenNoticeEvents", "通知事件监听初始化失败");
  useAsyncListener(
    listenTaskCompleteNotifyEvents,
    "listenTaskCompleteNotifyEvents",
    "任务结束提醒监听初始化失败"
  );
}
