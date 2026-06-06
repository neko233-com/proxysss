/// <reference path="../proxysss-script.d.ts" />

type GeoRule = {
  cidr?: string;
  exact?: string;
  country?: string;
  country_code?: string;
  city?: string;
  source?: string;
  confidence?: string;
};

type GeoConfig = {
  header_prefix?: string;
  include_city?: boolean;
  include_private_defaults?: boolean;
  rules?: GeoRule[];
};

type GatewayMessage = {
  ctx: {
    remote_addr?: string;
  };
};

const DEFAULT_CONFIG: GeoConfig = {
  header_prefix: "proxysss-",
  include_city: true,
  include_private_defaults: true,
  rules: [],
};

let runtimeConfig: GeoConfig = { ...DEFAULT_CONFIG };

function normalizePrefix(prefix?: string): string {
  const value = prefix?.trim() || "proxysss-";
  return value.endsWith("-") ? value : `${value}-`;
}

function extractIp(remoteAddr?: string): string {
  if (!remoteAddr) return "";
  if (remoteAddr.startsWith("[")) {
    const end = remoteAddr.indexOf("]");
    return end >= 0 ? remoteAddr.slice(1, end) : remoteAddr;
  }
  const lastColon = remoteAddr.lastIndexOf(":");
  const firstColon = remoteAddr.indexOf(":");
  if (lastColon > 0 && firstColon === lastColon) {
    return remoteAddr.slice(0, lastColon);
  }
  return remoteAddr;
}

function ipv4ToNumber(ip: string): number | null {
  const parts = ip.split(".");
  if (parts.length !== 4) return null;
  let value = 0;
  for (const part of parts) {
    if (!/^\d+$/.test(part)) return null;
    const octet = Number(part);
    if (octet < 0 || octet > 255) return null;
    value = (value << 8) + octet;
  }
  return value >>> 0;
}

function matchRule(ip: string, rule: GeoRule): boolean {
  if (rule.exact && rule.exact === ip) return true;
  if (!rule.cidr) return false;

  const [base, prefixText] = rule.cidr.split("/");
  const ipNum = ipv4ToNumber(ip);
  const baseNum = ipv4ToNumber(base);
  const prefix = Number(prefixText ?? "32");
  if (ipNum === null || baseNum === null || Number.isNaN(prefix) || prefix < 0 || prefix > 32) {
    return false;
  }
  const mask = prefix === 0 ? 0 : (~((1 << (32 - prefix)) - 1)) >>> 0;
  return (ipNum & mask) === (baseNum & mask);
}

function classifyPrivate(ip: string): GeoRule | undefined {
  if (ip === "127.0.0.1" || ip === "::1") {
    return {
      country: "Loopback",
      country_code: "LO",
      city: "localhost",
      source: "builtin",
      confidence: "high",
    };
  }
  if (ip.startsWith("10.") || ip.startsWith("192.168.") || /^172\.(1[6-9]|2\d|3[0-1])\./.test(ip)) {
    return {
      country: "Private Network",
      country_code: "PR",
      city: "private",
      source: "builtin",
      confidence: "medium",
    };
  }
  if (ip.startsWith("169.254.") || ip.startsWith("fe80:")) {
    return {
      country: "Link Local",
      country_code: "LL",
      city: "link-local",
      source: "builtin",
      confidence: "medium",
    };
  }
  return undefined;
}

function resolveGeo(ip: string): GeoRule {
  for (const rule of runtimeConfig.rules ?? []) {
    if (matchRule(ip, rule)) {
      return {
        country: rule.country ?? "Unknown",
        country_code: rule.country_code ?? "ZZ",
        city: rule.city,
        source: rule.source ?? "custom_rule",
        confidence: rule.confidence ?? "high",
      };
    }
  }

  if (runtimeConfig.include_private_defaults !== false) {
    const builtin = classifyPrivate(ip);
    if (builtin) return builtin;
  }

  return {
    country: "Unknown",
    country_code: "ZZ",
    source: "unmapped",
    confidence: "low",
  };
}

export default {
  name: "geo-headers",
  priority: 180,
  enabled: false,

  init_worker({ spec }: { spec?: { config?: GeoConfig } }) {
    runtimeConfig = {
      ...DEFAULT_CONFIG,
      ...(spec?.config ?? {}),
    };
    runtimeConfig.header_prefix = normalizePrefix(runtimeConfig.header_prefix);
  },

  access(message: GatewayMessage, current?: { set_headers?: Record<string, string> }) {
    const clientIp = extractIp(message.ctx.remote_addr);
    if (!clientIp) return;

    const geo = resolveGeo(clientIp);
    const prefix = normalizePrefix(runtimeConfig.header_prefix);
    const headers = {
      ...(current?.set_headers ?? {}),
      [`${prefix}client-ip`]: clientIp,
      [`${prefix}geo-country`]: geo.country ?? "Unknown",
      [`${prefix}geo-country-code`]: geo.country_code ?? "ZZ",
      [`${prefix}geo-source`]: geo.source ?? "unknown",
      [`${prefix}geo-confidence`]: geo.confidence ?? "low",
    } as Record<string, string>;

    if (runtimeConfig.include_city !== false && geo.city) {
      headers[`${prefix}geo-city`] = geo.city;
    }

    return {
      set_headers: headers,
    };
  },
};
