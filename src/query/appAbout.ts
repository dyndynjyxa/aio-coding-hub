import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { appAboutGet, type AppAboutInfo } from "../services/app/appAbout";
import { appAboutKeys } from "./keys";

export function useAppAboutQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: appAboutKeys.get(),
    queryFn: () => appAboutGet(),
    enabled: options?.enabled ?? true,
    staleTime: Infinity,
    placeholderData: keepPreviousData,
  });
}

export function isAppAboutAvailable(about: AppAboutInfo | null) {
  return about != null;
}
