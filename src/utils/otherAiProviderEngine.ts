import { normalizeModelList } from "./otherAiModels";

export type OtherAiDiscoverySource =
  | "cli"
  | "api"
  | "fallback"
  | "existing";

export type OtherAiModelRegistryEntry = {
  id: string;
  source: OtherAiDiscoverySource;
  discoveredAt: number;
  validated: boolean;
};

type DiscoverOtherAiModelsParams = {
  providerType: string;
  apiKey: string;
  cliCommand: string;
  prefersCli: boolean;
  env: Record<string, string> | null;
  existingModels: string[];
  fallbackModels: string[];
  listViaApi: (provider: string, apiKey: string) => Promise<string[]>;
  listViaCli: (
    provider: string,
    command: string,
    env?: Record<string, string> | null,
  ) => Promise<string[]>;
};

export type DiscoverOtherAiModelsResult = {
  models: string[];
  source: OtherAiDiscoverySource;
  error: string | null;
  registry: OtherAiModelRegistryEntry[];
};

function toRegistry(
  models: string[],
  source: OtherAiDiscoverySource,
  now = Date.now(),
): OtherAiModelRegistryEntry[] {
  return models.map((id) => ({
    id,
    source,
    discoveredAt: now,
    validated: false,
  }));
}

export async function discoverOtherAiModels(
  params: DiscoverOtherAiModelsParams,
): Promise<DiscoverOtherAiModelsResult> {
  const canUseCli = params.cliCommand.length > 0;
  const canUseApi = params.apiKey.length > 0;
  const existing = normalizeModelList(params.existingModels);
  const fallback = normalizeModelList(params.fallbackModels);
  const now = Date.now();

  let error: string | null = null;
  const runCli = async () => {
    const models = await params.listViaCli(
      params.providerType,
      params.cliCommand,
      params.env,
    );
    return normalizeModelList(models);
  };
  const runApi = async () => {
    const models = await params.listViaApi(params.providerType, params.apiKey);
    return normalizeModelList(models);
  };

  const tryDiscovery = async (
    source: OtherAiDiscoverySource,
    operation: () => Promise<string[]>,
  ): Promise<DiscoverOtherAiModelsResult | null> => {
    try {
      const models = await operation();
      if (models.length > 0) {
        return {
          models,
          source,
          error: null,
          registry: toRegistry(models, source, now),
        };
      }
      error = "Provider returned no models.";
      return null;
    } catch (caught) {
      error = caught instanceof Error ? caught.message : String(caught);
      return null;
    }
  };

  if (canUseCli && (params.prefersCli || !canUseApi)) {
    const cli = await tryDiscovery("cli", runCli);
    if (cli) {
      return cli;
    }
    if (canUseApi) {
      const api = await tryDiscovery("api", runApi);
      if (api) {
        return api;
      }
    }
  } else if (canUseApi) {
    const api = await tryDiscovery("api", runApi);
    if (api) {
      return api;
    }
    if (canUseCli) {
      const cli = await tryDiscovery("cli", runCli);
      if (cli) {
        return cli;
      }
    }
  } else if (canUseCli) {
    const cli = await tryDiscovery("cli", runCli);
    if (cli) {
      return cli;
    }
  }

  if (existing.length > 0) {
    return {
      models: existing,
      source: "existing",
      error,
      registry: toRegistry(existing, "existing", now),
    };
  }

  if (fallback.length > 0) {
    return {
      models: fallback,
      source: "fallback",
      error,
      registry: toRegistry(fallback, "fallback", now),
    };
  }

  return { models: [], source: "existing", error, registry: [] };
}
