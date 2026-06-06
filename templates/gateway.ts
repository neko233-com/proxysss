/// <reference path="./proxysss-script.d.ts" />

// proxysss main gateway script — embedded TypeScript engine (proxysss + TS = nginx + Lua).
//
// This file is the "house" router. It is loaded by the in-process TypeScript
// engine as the lowest-priority fallback: it only runs when no plugin in the
// plugins/ directory produced a routing decision. Plugins always run first and
// can override anything here.
//
// There is no external `deno`/`node` runtime — proxysss transpiles this file to
// JavaScript in-process and executes it inside a sandboxed QuickJS engine with a
// hard per-call timeout and memory limit. A bug here (throw / infinite loop)
// is reported to the error log and never affects native/YAML proxy traffic.
//
// The default export is a plugin object. Supported hooks:
//   access(message, current?)  -> RouteDecision | void   (HTTP routing)
//   balancer(message, current?)-> RouteDecision | void   (HTTP upstream choice)
//   preread(message, current?) -> RouteDecision | void   (TCP/UDP routing)
//   log({ message, route })    -> void                    (observability)
//   init_worker({ spec })      -> void                    (on load)
//   onDispose()                -> void                    (on unload/reload)

type RouteDecision = {
  upstream: string;
  upstreams?: string[];
  affinity_key?: string;
  rewrite_path?: string;
  set_headers?: Record<string, string>;
  strip_headers?: string[];
  status?: number;
  content_type?: string;
};

type GatewayMessage = {
  id: string;
  kind: "http" | "tcp" | "udp" | string;
  listener?: string | null;
  ctx: {
    host?: string;
    path?: string;
    method?: string;
    player_id?: string;
    remote_addr?: string;
  };
};

const HTTP_LOGIN_BACKENDS = [
  "http://127.0.0.1:8088",
  "http://127.0.0.1:8089",
  "http://127.0.0.1:8090",
];

const TCP_LOGIN_BACKENDS = ["127.0.0.1:7001", "127.0.0.1:7002", "127.0.0.1:7003"];
const UDP_REALTIME_BACKENDS = ["127.0.0.1:8101", "127.0.0.1:8102"];

function isLoginPath(path: string): boolean {
  return path.startsWith("/sdk/login") || path.startsWith("/sdk/register");
}

function fallbackHttpRoute(message: GatewayMessage): RouteDecision {
  const path = message.ctx.path ?? "";
  const playerId = message.ctx.player_id;

  if (path === "/" || path === "/index.html" || path === "/docs") {
    return { upstream: "proxysss://welcome" };
  }
  if (path === "/healthz") {
    return { upstream: "proxysss://healthz" };
  }
  if (path.startsWith("/static/")) {
    return { upstream: `proxysss://static/${path.slice("/static/".length)}` };
  }
  if (path === "/admin") {
    return { upstream: "proxysss://admin" };
  }
  if (isLoginPath(path)) {
    return {
      upstream: HTTP_LOGIN_BACKENDS[0],
      upstreams: HTTP_LOGIN_BACKENDS,
      affinity_key: playerId,
      set_headers: { "x-gateway": "proxysss" },
    };
  }
  if (message.ctx.host === "api.local") {
    return {
      upstream: "http://127.0.0.1:8081",
      set_headers: { "x-gateway": "proxysss" },
    };
  }
  return {
    upstream: "http://127.0.0.1:8080",
    set_headers: { "x-gateway": "proxysss" },
  };
}

function fallbackStreamRoute(message: GatewayMessage): RouteDecision {
  if (message.kind === "tcp") {
    if (message.listener === "tcp-affinity-demo") {
      return {
        upstream: TCP_LOGIN_BACKENDS[0],
        upstreams: TCP_LOGIN_BACKENDS,
        affinity_key: message.ctx.player_id,
      };
    }
    return { upstream: "127.0.0.1:9000" };
  }

  if (message.listener === "udp-affinity-demo") {
    return {
      upstream: UDP_REALTIME_BACKENDS[0],
      upstreams: UDP_REALTIME_BACKENDS,
      affinity_key: message.ctx.player_id,
    };
  }
  return { upstream: "127.0.0.1:9999" };
}

export default {
  name: "gateway",
  // Lowest priority: plugins always win. (The engine also runs this script only
  // as the fallback when no plugin produced a route.)
  priority: -1_000_000,
  enabled: true,

  access(message: GatewayMessage): RouteDecision {
    return fallbackHttpRoute(message);
  },

  preread(message: GatewayMessage): RouteDecision {
    return fallbackStreamRoute(message);
  },
};
