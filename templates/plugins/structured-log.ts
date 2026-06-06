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

function emit(
  level: "debug" | "info" | "warn" | "error",
  event: Record<string, unknown>,
) {
  const payload = {
    level,
    plugin: "structured-log",
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
  name: "structured-log",
  priority: -150,
  enabled: false,

  log({ message, route }: { message: GatewayMessage; route: RouteDecision }) {
    if (!route.upstream) {
      emit("error", {
        message_id: message.id,
        kind: message.kind,
        error: "missing upstream",
      });
      return;
    }

    emit("info", {
      message_id: message.id,
      kind: message.kind,
      listener: message.listener,
      host: message.ctx.host,
      method: message.ctx.method,
      path: message.ctx.path,
      remote_addr: message.ctx.remote_addr,
      upstream: route.upstream,
      bytes: message.ctx.body_len ?? message.ctx.payload_len ?? 0,
    });
  },
};
