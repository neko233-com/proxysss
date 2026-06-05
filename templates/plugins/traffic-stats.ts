type GatewayMessage = {
  id: string;
  kind: "http" | "tcp" | "udp" | string;
  listener?: string | null;
  ctx: {
    method?: string;
    path?: string;
    host?: string;
    payload_len?: number;
    body_len?: number;
    remote_addr?: string;
  };
};

type RouteDecision = {
  upstream: string;
  upstreams?: string[];
};

const stats = {
  total: 0,
  http: 0,
  tcp: 0,
  udp: 0,
  errors: 0,
  bytes: 0,
};

function log(level: "debug" | "info" | "warn" | "error", event: Record<string, unknown>) {
  const payload = {
    level,
    plugin: "traffic-stats",
    ts: new Date().toISOString(),
    ...event,
  };

  const line = JSON.stringify(payload);
  if (level === "error" || level === "warn") {
    console.error(line);
  } else {
    console.log(line);
  }
}

export default {
  name: "traffic-stats",
  priority: -100,
  enabled: true,

  log({ message, route }: { message: GatewayMessage; route: RouteDecision }) {
    stats.total += 1;
    if (message.kind === "http") stats.http += 1;
    if (message.kind === "tcp") stats.tcp += 1;
    if (message.kind === "udp") stats.udp += 1;
    stats.bytes += message.ctx.body_len ?? message.ctx.payload_len ?? 0;

    if (!route.upstream) {
      stats.errors += 1;
      log("error", { message_id: message.id, error: "missing upstream", stats });
      return;
    }

    log("info", {
      message_id: message.id,
      kind: message.kind,
      listener: message.listener,
      host: message.ctx.host,
      method: message.ctx.method,
      path: message.ctx.path,
      upstream: route.upstream,
      stats,
    });
  },
};
