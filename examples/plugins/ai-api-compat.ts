type ProviderRule = {
  name: string;
  upstream: string;
  match_host?: string;
  path_prefix?: string;
  rewrite_base_path?: string;
  provider?: string;
  add_headers?: Record<string, string>;
  strip_headers?: string[];
};

type PluginConfig = {
  header_prefix?: string;
  rules?: ProviderRule[];
};

type GatewayMessage = {
  ctx: {
    host?: string;
    path?: string;
    method?: string;
  };
};

const DEFAULT_CONFIG: PluginConfig = {
  header_prefix: "proxysss-",
  rules: [],
};

let runtimeConfig: PluginConfig = { ...DEFAULT_CONFIG };

function normalizePrefix(prefix?: string): string {
  const value = prefix?.trim() || "proxysss-";
  return value.endsWith("-") ? value : `${value}-`;
}

function hostMatches(rule: ProviderRule, host?: string): boolean {
  if (!rule.match_host) return true;
  return (host ?? "").toLowerCase() === rule.match_host.toLowerCase();
}

function pathMatches(rule: ProviderRule, path?: string): boolean {
  if (!rule.path_prefix) return true;
  return (path ?? "").startsWith(rule.path_prefix);
}

function rewritePath(rule: ProviderRule, path?: string): string | undefined {
  if (!path) return path;
  if (!rule.path_prefix || !rule.rewrite_base_path) return path;
  const suffix = path.slice(rule.path_prefix.length) || "/";
  const base = rule.rewrite_base_path.endsWith("/") ? rule.rewrite_base_path.slice(0, -1) : rule.rewrite_base_path;
  return `${base}${suffix.startsWith("/") ? suffix : `/${suffix}`}`;
}

export default {
  name: "ai-api-compat",
  priority: 220,
  enabled: false,

  init_worker({ spec }: { spec?: { config?: PluginConfig } }) {
    runtimeConfig = { ...DEFAULT_CONFIG, ...(spec?.config ?? {}) };
    runtimeConfig.header_prefix = normalizePrefix(runtimeConfig.header_prefix);
  },

  access(message: GatewayMessage, current?: { set_headers?: Record<string, string>; strip_headers?: string[] }) {
    const rule = (runtimeConfig.rules ?? []).find(
      (candidate) => hostMatches(candidate, message.ctx.host) && pathMatches(candidate, message.ctx.path),
    );
    if (!rule) return;

    const prefix = normalizePrefix(runtimeConfig.header_prefix);
    const headers = {
      ...(current?.set_headers ?? {}),
      [`${prefix}ai-route`]: rule.name,
      [`${prefix}ai-provider`]: rule.provider ?? rule.name,
      [`${prefix}ai-original-path`]: message.ctx.path ?? "/",
      [`${prefix}ai-method`]: message.ctx.method ?? "GET",
      ...(rule.add_headers ?? {}),
    };

    return {
      upstream: rule.upstream,
      rewrite_path: rewritePath(rule, message.ctx.path),
      set_headers: headers,
      strip_headers: [...(current?.strip_headers ?? []), ...(rule.strip_headers ?? [])],
    };
  },
};
