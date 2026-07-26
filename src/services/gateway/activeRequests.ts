import { commands, type ActiveRequestSnapshotItem } from "../../generated/bindings";
import { invokeGeneratedIpc, mapGeneratedCommandResponse } from "../generatedIpc";

export type ActiveRequest = ActiveRequestSnapshotItem;

export async function activeRequestLogsSnapshot() {
  return invokeGeneratedIpc<ActiveRequest[]>({
    title: "读取进行中请求失败",
    cmd: "active_request_logs_snapshot",
    invoke: async () =>
      mapGeneratedCommandResponse(await commands.activeRequestLogsSnapshot(), (rows) => rows),
  });
}
