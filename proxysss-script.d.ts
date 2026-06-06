export type JsonPrimitive = string | number | boolean | null;
export type JsonValue =
  | JsonPrimitive
  | JsonValue[]
  | { [key: string]: JsonValue };

export interface RouteDecision {
  upstream: string;
  upstreams?: string[];
  affinity_key?: string;
  rewrite_path?: string;
  set_headers?: Record<string, string>;
  strip_headers?: string[];
  status?: number;
  content_type?: string;
}

export type RoutePatch = Partial<RouteDecision>;

export interface HttpContext {
  request_id: string;
  host: string;
  method: string;
  path: string;
  query?: string | null;
  scheme: string;
  version: string;
  remote_addr: string;
  player_id?: string | null;
  headers: Record<string, string>;
  body_len: number;
}

export interface StreamContext {
  request_id: string;
  listener: string;
  protocol: string;
  remote_addr: string;
  player_id?: string | null;
  first_packet_preview?: string | null;
  payload_len: number;
}

export interface GatewayMessage<TContext = HttpContext | StreamContext> {
  id: string;
  kind: "http" | "tcp" | "udp" | string;
  listener?: string | null;
  ctx: TContext;
}

export interface PluginSpec<TConfig = JsonValue> {
  name: string;
  module_path: string;
  priority?: number | null;
  enabled?: boolean | null;
  config?: TConfig;
}

export interface PluginInfo {
  name: string;
  module_path: string;
  priority: number;
  enabled: boolean;
  loaded_at?: string | null;
}

export interface LogPayload {
  message: GatewayMessage;
  route: RouteDecision;
}

export interface InitWorkerPayload<TConfig = JsonValue> {
  spec?: PluginSpec<TConfig>;
}

export interface ProxysssPlugin<TConfig = JsonValue> {
  name: string;
  priority?: number;
  enabled?: boolean;
  access?: (
    message: GatewayMessage<HttpContext>,
    current?: RoutePatch | null,
  ) => RoutePatch | void | null;
  balancer?: (
    message: GatewayMessage<HttpContext>,
    current?: RoutePatch | null,
  ) => RoutePatch | void | null;
  preread?: (
    message: GatewayMessage<StreamContext>,
    current?: RoutePatch | null,
  ) => RoutePatch | void | null;
  log?: (payload: LogPayload) => void;
  init_worker?: (payload: InitWorkerPayload<TConfig>) => void;
  onDispose?: () => void;
}

declare global {
  const PROXYSSS_ENV: Record<string, string>;
}
