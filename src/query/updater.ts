import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { updaterCheck } from "../services/app/updater";
import { updaterKeys } from "./keys";

export function useUpdaterCheckQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: updaterKeys.check(),
    queryFn: () => updaterCheck(),
    enabled: options?.enabled ?? false,
    staleTime: 1000 * 60 * 30,
    placeholderData: keepPreviousData,
  });
}
