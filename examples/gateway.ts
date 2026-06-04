// @ts-nocheck

declare const Deno: any;

const textEncoder = new TextEncoder();
const textDecoder = new TextDecoder();

async function* stdinLines(): AsyncGenerator<string> {
  let buffer = "";
  for await (const chunk of Deno.stdin.readable) {
    buffer += textDecoder.decode(chunk, { stream: true });

    let lineBreak = buffer.indexOf("\n");
    while (lineBreak >= 0) {
      const line = buffer.slice(0, lineBreak).replace(/\r$/, "");
      buffer = buffer.slice(lineBreak + 1);
      yield line;
      lineBreak = buffer.indexOf("\n");
    }
  }

  buffer += textDecoder.decode();
  if (buffer.length > 0) {
    yield buffer.replace(/\r$/, "");
  }
}

type RouteDecision = {
  upstream: string;
  upstreams?: string[];
  affinity_key?: string;
  rewrite_path?: string;
  set_headers?: Record<string, string>;
  strip_headers?: string[];
};

type GatewayMessage = {
  id: string;
  kind: "http" | "tcp" | "udp" | "plugin_load" | "plugin_unload" | "plugin_list";
  listener?: string | null;
  ctx: {
    request_id?: string;
    host?: string;
    path?: string;
    method?: string;
    remote_addr?: string;
    player_id?: string;
    first_packet_preview?: string;
    payload_len?: number;
    name?: string;
    module_path?: string;
    priority?: number;
    enabled?: boolean;
    config?: Record<string, unknown>;
  };
};

type PluginSpec = {
  name: string;
  module_path: string;
  priority?: number;
  enabled?: boolean;
  config?: Record<string, unknown>;
};

type PluginRuntime = {
  name?: string;
  priority?: number;
  enabled?: boolean;
  init_worker?: (ctx: { spec: PluginSpec }) => unknown | Promise<unknown>;
  onDispose?: () => unknown | Promise<unknown>;
  access?: (message: GatewayMessage, current?: RouteDecision) => RouteDecision | void | Promise<RouteDecision | void>;
  balancer?: (message: GatewayMessage, current?: RouteDecision) => RouteDecision | void | Promise<RouteDecision | void>;
  preread?: (message: GatewayMessage, current?: RouteDecision) => RouteDecision | void | Promise<RouteDecision | void>;
  log?: (ctx: { message: GatewayMessage; route: RouteDecision }) => unknown | Promise<unknown>;
};

type PluginRecord = {
  name: string;
  modulePath: string;
  priority: number;
  enabled: boolean;
  loadedAt: string;
  runtime: PluginRuntime;
};

const plugins = new Map<string, PluginRecord>();

function fallbackHttpRoute(message: GatewayMessage): RouteDecision {
  const path = message.ctx.path ?? "";
  const playerId = message.ctx.player_id;

  if (path === "/" || path === "/index.html" || path === "/docs") {
    return {
      upstream: "proxysss://welcome",
    };
  }

  if (path === "/admin") {
    return {
      upstream: "proxysss://admin",
    };
  }

  if (path.startsWith("/sdk/login") || path.startsWith("/sdk/register")) {
    return {
      upstream: "http://127.0.0.1:8088",
      upstreams: [
        "http://127.0.0.1:8088",
        "http://127.0.0.1:8089",
        "http://127.0.0.1:8090",
      ],
      affinity_key: playerId,
      set_headers: {
        "x-gateway": "proxysss",
      },
    };
  }

  if (message.ctx.host === "api.local") {
    return {
      upstream: "http://127.0.0.1:8081",
      set_headers: {
        "x-gateway": "proxysss",
      },
    };
  }

  return {
    upstream: "http://127.0.0.1:8080",
    set_headers: {
      "x-gateway": "proxysss",
    },
  };
}

function fallbackTcpRoute(message: GatewayMessage): RouteDecision {
  if (message.listener === "game-login") {
    return {
      upstream: "127.0.0.1:7001",
      upstreams: ["127.0.0.1:7001", "127.0.0.1:7002", "127.0.0.1:7003"],
      affinity_key: message.ctx.player_id,
    };
  }

  return { upstream: "127.0.0.1:9000" };
}

function fallbackUdpRoute(message: GatewayMessage): RouteDecision {
  if (message.listener === "game-realtime") {
    return {
      upstream: "127.0.0.1:8101",
      upstreams: ["127.0.0.1:8101", "127.0.0.1:8102"],
      affinity_key: message.ctx.player_id,
    };
  }

  return { upstream: "127.0.0.1:9999" };
}

function orderedPlugins(): PluginRecord[] {
  return Array.from(plugins.values())
    .filter((record) => record.enabled)
    .sort((a, b) => b.priority - a.priority);
}

function mergeRoute(current: RouteDecision | undefined, next: RouteDecision | void): RouteDecision | undefined {
  if (!next) {
    return current;
  }

  if (!current) {
    return normalizeRoute(next);
  }

  return normalizeRoute({
    ...current,
    ...next,
    set_headers: {
      ...(current.set_headers ?? {}),
      ...(next.set_headers ?? {}),
    },
    strip_headers: dedupe([...(current.strip_headers ?? []), ...(next.strip_headers ?? [])]),
    upstreams: next.upstreams ?? current.upstreams,
  });
}

function normalizeRoute(route: RouteDecision): RouteDecision {
  const upstream = route.upstream?.trim();
  if (!upstream) {
    throw new Error("route.upstream is required");
  }

  const upstreams = (route.upstreams ?? [])
    .map((item) => item.trim())
    .filter((item) => item.length > 0);

  if (upstreams.length > 0 && !upstreams.includes(upstream)) {
    upstreams.unshift(upstream);
  }

  return {
    ...route,
    upstream,
    upstreams,
    set_headers: route.set_headers ?? {},
    strip_headers: route.strip_headers ?? [],
  };
}

function dedupe(values: string[]): string[] {
  return Array.from(new Set(values));
}

async function runHttpPipeline(message: GatewayMessage): Promise<RouteDecision> {
  let route: RouteDecision | undefined;

  for (const plugin of orderedPlugins()) {
    if (plugin.runtime.access) {
      route = mergeRoute(route, await plugin.runtime.access(message, route));
    }
  }

  for (const plugin of orderedPlugins()) {
    if (plugin.runtime.balancer) {
      route = mergeRoute(route, await plugin.runtime.balancer(message, route));
    }
  }

  route = route ?? fallbackHttpRoute(message);
  route = normalizeRoute(route);

  for (const plugin of orderedPlugins()) {
    if (plugin.runtime.log) {
      await plugin.runtime.log({ message, route });
    }
  }

  return route;
}

async function runStreamPipeline(message: GatewayMessage, fallback: (message: GatewayMessage) => RouteDecision): Promise<RouteDecision> {
  let route: RouteDecision | undefined;

  for (const plugin of orderedPlugins()) {
    if (plugin.runtime.preread) {
      route = mergeRoute(route, await plugin.runtime.preread(message, route));
    }
  }

  route = route ?? fallback(message);
  route = normalizeRoute(route);

  for (const plugin of orderedPlugins()) {
    if (plugin.runtime.log) {
      await plugin.runtime.log({ message, route });
    }
  }

  return route;
}

function resolveModule(modulePath: string): string {
  if (modulePath.startsWith("file:") || modulePath.startsWith("http://") || modulePath.startsWith("https://")) {
    return modulePath;
  }

  const normalized = modulePath.replace(/\\/g, "/");
  if (/^[a-zA-Z]:\//.test(normalized)) {
    return `file:///${normalized}`;
  }

  const cwd = String(Deno.cwd()).replace(/\\/g, "/");
  const base = cwd.startsWith("/") ? `file://${cwd}/` : `file:///${cwd}/`;
  return new URL(normalized, base).href;
}

async function loadPlugin(spec: PluginSpec): Promise<Record<string, unknown>> {
  if (!spec.name?.trim()) {
    throw new Error("plugin name is required");
  }

  if (!spec.module_path?.trim()) {
    throw new Error("plugin module_path is required");
  }

  const moduleUrl = `${resolveModule(spec.module_path)}?v=${Date.now()}`;
  const imported = await import(moduleUrl);
  const runtime = (imported.default ?? imported.plugin ?? imported) as PluginRuntime;

  const record: PluginRecord = {
    name: spec.name,
    modulePath: spec.module_path,
    priority: spec.priority ?? runtime.priority ?? 0,
    enabled: spec.enabled ?? runtime.enabled ?? true,
    loadedAt: new Date().toISOString(),
    runtime,
  };

  plugins.set(spec.name, record);

  if (runtime.init_worker) {
    await runtime.init_worker({ spec });
  }

  return {
    name: record.name,
    module_path: record.modulePath,
    priority: record.priority,
    enabled: record.enabled,
    loaded_at: record.loadedAt,
  };
}

async function unloadPlugin(name: string): Promise<Record<string, unknown>> {
  const record = plugins.get(name);
  if (!record) {
    throw new Error(`plugin not found: ${name}`);
  }

  if (record.runtime.onDispose) {
    await record.runtime.onDispose();
  }

  plugins.delete(name);

  return {
    name,
    unloaded: true,
  };
}

function listPlugins(): Record<string, unknown>[] {
  return Array.from(plugins.values())
    .sort((a, b) => b.priority - a.priority)
    .map((record) => ({
      name: record.name,
      module_path: record.modulePath,
      priority: record.priority,
      enabled: record.enabled,
      loaded_at: record.loadedAt,
    }));
}

async function processMessage(message: GatewayMessage): Promise<Record<string, unknown>> {
  switch (message.kind) {
    case "http": {
      const route = await runHttpPipeline(message);
      return {
        id: message.id,
        ok: true,
        route,
      };
    }
    case "tcp": {
      const route = await runStreamPipeline(message, fallbackTcpRoute);
      return {
        id: message.id,
        ok: true,
        route,
      };
    }
    case "udp": {
      const route = await runStreamPipeline(message, fallbackUdpRoute);
      return {
        id: message.id,
        ok: true,
        route,
      };
    }
    case "plugin_load": {
      const data = await loadPlugin({
        name: message.ctx.name ?? "",
        module_path: message.ctx.module_path ?? "",
        priority: message.ctx.priority,
        enabled: message.ctx.enabled,
        config: message.ctx.config,
      });

      return {
        id: message.id,
        ok: true,
        data,
      };
    }
    case "plugin_unload": {
      const data = await unloadPlugin(message.ctx.name ?? "");
      return {
        id: message.id,
        ok: true,
        data,
      };
    }
    case "plugin_list": {
      return {
        id: message.id,
        ok: true,
        plugins: listPlugins(),
      };
    }
    default:
      throw new Error(`unsupported kind: ${message.kind}`);
  }
}

for await (const line of stdinLines()) {
  const trimmed = line.trim();
  if (!trimmed) {
    continue;
  }

  let message: GatewayMessage;
  try {
    message = JSON.parse(trimmed) as GatewayMessage;
  } catch (error) {
    const payload = {
      id: "unknown",
      ok: false,
      error: `invalid json: ${String(error)}`,
    };
    await Deno.stdout.write(textEncoder.encode(`${JSON.stringify(payload)}\n`));
    continue;
  }

  try {
    const payload = await processMessage(message);
    await Deno.stdout.write(textEncoder.encode(`${JSON.stringify(payload)}\n`));
  } catch (error) {
    const payload = {
      id: message.id,
      ok: false,
      error: String(error),
    };
    await Deno.stdout.write(textEncoder.encode(`${JSON.stringify(payload)}\n`));
  }
}
