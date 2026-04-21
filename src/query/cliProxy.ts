import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import type { CliKey } from "../services/providers/providers";
import {
  cliProxySetEnabled,
  cliProxyStatusAll,
  type CliProxyStatus,
} from "../services/cli/cliProxy";
import { cliProxyKeys } from "./keys";

export function useCliProxyStatusAllQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: cliProxyKeys.statusAll(),
    queryFn: () => cliProxyStatusAll(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useCliProxySetEnabledMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (input: { cliKey: CliKey; enabled: boolean }) =>
      cliProxySetEnabled({ cli_key: input.cliKey, enabled: input.enabled }),
    onMutate: (input) => {
      queryClient.cancelQueries({ queryKey: cliProxyKeys.statusAll() });
      const prev = queryClient.getQueryData<CliProxyStatus[] | null>(cliProxyKeys.statusAll());

      queryClient.setQueryData<CliProxyStatus[] | null>(cliProxyKeys.statusAll(), (cur) => {
        if (!cur) return cur;
        const exists = cur.some((row) => row.cli_key === input.cliKey);
        if (!exists) {
          return [
            {
              cli_key: input.cliKey,
              enabled: input.enabled,
              base_origin: null,
              applied_to_current_gateway: input.enabled ? true : null,
            },
            ...cur,
          ];
        }
        return cur.map((row) =>
          row.cli_key === input.cliKey
            ? {
                ...row,
                enabled: input.enabled,
                applied_to_current_gateway: input.enabled ? true : null,
              }
            : row
        );
      });

      return { prev };
    },
    onError: (_err, _input, ctx) => {
      if (ctx?.prev !== undefined) {
        queryClient.setQueryData(cliProxyKeys.statusAll(), ctx.prev);
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: cliProxyKeys.statusAll() });
    },
  });
}
