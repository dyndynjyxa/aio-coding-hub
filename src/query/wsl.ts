import { keepPreviousData, useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  wslConfigStatusGet,
  wslConfigureClients,
  wslDetect,
  wslHostAddressGet,
  normalizeWslDistros,
  type WslDetection,
  type WslDistroConfigStatus,
  type WslConfigureReport,
} from "../services/app/wsl";
import { wslKeys } from "./keys";

export function useWslDetectionQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: wslKeys.detection(),
    queryFn: () => wslDetect(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useWslHostAddressQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: wslKeys.hostAddress(),
    queryFn: () => wslHostAddressGet(),
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useWslConfigStatusQuery(distros: string[], options?: { enabled?: boolean }) {
  const normalizedDistros = normalizeWslDistros(distros) ?? [];

  return useQuery({
    queryKey: wslKeys.configStatus(normalizedDistros),
    queryFn: () => {
      if (normalizedDistros.length === 0) return null;
      return wslConfigStatusGet(normalizedDistros);
    },
    enabled: normalizedDistros.length > 0 && (options?.enabled ?? true),
    placeholderData: keepPreviousData,
  });
}

export type WslOverview = {
  detection: WslDetection | null;
  hostIp: string | null;
  statusRows: WslDistroConfigStatus[] | null;
};

export function useWslOverviewQuery(options?: { enabled?: boolean }) {
  return useQuery({
    queryKey: wslKeys.overview(),
    queryFn: async (): Promise<WslOverview> => {
      const det = await wslDetect();
      if (!det) {
        return { detection: null, hostIp: null, statusRows: null };
      }
      const normalizedDistros = normalizeWslDistros(det.distros) ?? [];
      const detection = { ...det, distros: normalizedDistros };
      if (!detection.detected || normalizedDistros.length === 0) {
        return { detection, hostIp: null, statusRows: null };
      }

      const [ip, statuses] = await Promise.all([
        wslHostAddressGet().catch(() => null),
        wslConfigStatusGet(normalizedDistros).catch(() => null),
      ]);
      return { detection, hostIp: ip ?? null, statusRows: statuses ?? null };
    },
    enabled: options?.enabled ?? true,
    placeholderData: keepPreviousData,
  });
}

export function useWslConfigureClientsMutation() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => wslConfigureClients(),
    onSuccess: (_report: WslConfigureReport | null) => {
      queryClient.invalidateQueries({ queryKey: wslKeys.all });
    },
  });
}
