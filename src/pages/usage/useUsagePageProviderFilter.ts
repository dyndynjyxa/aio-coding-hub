import { useEffect, useMemo, useState } from "react";
import { CLIS, cliShortLabel, type CliFilterKey } from "../../constants/clis";
import { useProvidersListQuery } from "../../query/providers";
import type { CliKey, ProviderSummary } from "../../services/providers/providers";

export type UsageProviderOption = {
  id: number;
  cliKey: CliKey;
  label: string;
};

const EMPTY_PROVIDERS: ProviderSummary[] = [];

function buildProviderOption(provider: ProviderSummary): UsageProviderOption {
  return {
    id: provider.id,
    cliKey: provider.cli_key,
    label: `${cliShortLabel(provider.cli_key)} · ${provider.name}`,
  };
}

function providersForCli(
  cliKey: CliFilterKey,
  providersByCli: Record<CliKey, ProviderSummary[]>
): ProviderSummary[] {
  if (cliKey === "all") {
    return CLIS.flatMap((cli) => providersByCli[cli.key]);
  }
  return providersByCli[cliKey];
}

export function useUsagePageProviderFilter(cliKey: CliFilterKey) {
  const [providerId, setProviderId] = useState<number | null>(null);

  const claudeProvidersQuery = useProvidersListQuery("claude");
  const codexProvidersQuery = useProvidersListQuery("codex");
  const geminiProvidersQuery = useProvidersListQuery("gemini");

  const providerOptions = useMemo(() => {
    const providersByCli = {
      claude: claudeProvidersQuery.data ?? EMPTY_PROVIDERS,
      codex: codexProvidersQuery.data ?? EMPTY_PROVIDERS,
      gemini: geminiProvidersQuery.data ?? EMPTY_PROVIDERS,
    } satisfies Record<CliKey, ProviderSummary[]>;

    return providersForCli(cliKey, providersByCli).map(buildProviderOption);
  }, [cliKey, claudeProvidersQuery.data, codexProvidersQuery.data, geminiProvidersQuery.data]);

  useEffect(() => {
    if (providerId == null) return;
    if (providerOptions.some((option) => option.id === providerId)) return;
    setProviderId(null);
  }, [providerId, providerOptions]);

  const providersLoading =
    claudeProvidersQuery.isFetching ||
    codexProvidersQuery.isFetching ||
    geminiProvidersQuery.isFetching;

  return { providerId, setProviderId, providerOptions, providersLoading };
}
