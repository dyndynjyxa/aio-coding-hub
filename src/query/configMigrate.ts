import { useMutation, useQueryClient } from "@tanstack/react-query";
import { configExport, configImport } from "../services/app/configMigrate";
import {
  cliProxyKeys,
  gatewayKeys,
  mcpKeys,
  promptsKeys,
  providersKeys,
  settingsKeys,
  skillsKeys,
  sortModesKeys,
  workspacesKeys,
  wslKeys,
} from "./keys";

async function invalidateImportedConfigQueries(
  queryClient: ReturnType<typeof useQueryClient>
) {
  await Promise.all([
    queryClient.invalidateQueries({ queryKey: settingsKeys.all }),
    queryClient.invalidateQueries({ queryKey: gatewayKeys.all }),
    queryClient.invalidateQueries({ queryKey: providersKeys.all }),
    queryClient.invalidateQueries({ queryKey: sortModesKeys.all }),
    queryClient.invalidateQueries({ queryKey: workspacesKeys.all }),
    queryClient.invalidateQueries({ queryKey: promptsKeys.all }),
    queryClient.invalidateQueries({ queryKey: mcpKeys.all }),
    queryClient.invalidateQueries({ queryKey: skillsKeys.all }),
    queryClient.invalidateQueries({ queryKey: wslKeys.all }),
    queryClient.invalidateQueries({ queryKey: cliProxyKeys.all }),
  ]);
}

export function useConfigExportMutation() {
  return useMutation({
    mutationFn: (input: { filePath: string }) => configExport(input.filePath),
  });
}

export function useConfigImportMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { filePath: string }) => configImport(input.filePath),
    onSuccess: async (result) => {
      if (!result) return;
      await invalidateImportedConfigQueries(queryClient);
    },
  });
}
