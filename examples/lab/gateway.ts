// Lab gateway: routes TCP/UDP echo upstreams and keeps built-in welcome/health/static.

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
  if (buffer.length > 0) yield buffer.replace(/\r$/, "");
}

type RouteDecision = {
  upstream: string;
  upstreams?: string[];
  rewrite_path?: string;
  set_headers?: Record<string, string>;
  status?: number;
  content_type?: string;
};

type GatewayMessage = {
  id: string;
  kind: string;
  listener?: string | null;
  ctx: Record<string, unknown>;
};

function routeHttp(message: GatewayMessage): RouteDecision {
  const path = String(message.ctx.path ?? "/");
  const host = String(message.ctx.host ?? "localhost");

  if (path === "/" || path === "") {
    return { upstream: "proxysss://welcome" };
  }
  if (path === "/healthz") {
    return { upstream: "proxysss://healthz" };
  }
  if (path.startsWith("/static/")) {
    return { upstream: `proxysss://static/${path.slice("/static/".length)}` };
  }
  if (path.startsWith("/echo")) {
    return {
      upstream: "http://127.0.0.1:8081",
      rewrite_path: path.replace(/^\/echo/, "") || "/",
      set_headers: { "x-lab": "proxysss-echo" },
    };
  }
  if (path === "/admin" || path.startsWith("/admin/")) {
    return { upstream: "proxysss://admin" };
  }

  return {
    upstream: "proxysss://welcome",
    set_headers: { "x-lab-host": host },
  };
}

function routeStream(message: GatewayMessage): RouteDecision {
  const listener = message.listener ?? "";
  if (listener === "tcp-echo") {
    return { upstream: "127.0.0.1:7001" };
  }
  if (listener === "udp-echo") {
    return { upstream: "127.0.0.1:8101" };
  }
  if (listener === "ftp") {
    return { upstream: "127.0.0.1:2121" };
  }
  return { upstream: "127.0.0.1:7001" };
}

for await (const line of stdinLines()) {
  if (!line.trim()) continue;
  const request = JSON.parse(line);
  const id = request.id;
  const kind = request.kind;

  try {
    let route: RouteDecision;
    if (kind === "http") {
      route = routeHttp(request);
    } else if (kind === "tcp" || kind === "udp") {
      route = routeStream(request);
    } else {
      throw new Error(`unsupported kind: ${kind}`);
    }
    console.log(JSON.stringify({ id, ok: true, route }));
  } catch (error) {
    console.log(JSON.stringify({ id, ok: false, error: String(error) }));
  }
}
