/**
 * JSON primitive value supported by proxysss script payloads.
 * proxysss 脚本载荷里允许出现的 JSON 基础值。
 */
export type JsonPrimitive = string | number | boolean | null;

/**
 * Recursive JSON value supported by proxysss script runtime inputs/outputs.
 * proxysss 脚本运行时输入输出支持的递归 JSON 值。
 */
export type JsonValue =
  | JsonPrimitive
  | JsonValue[]
  | { [key: string]: JsonValue };

/**
 * Final route decision shape returned to the Rust gateway runtime.
 * 返回给 Rust 网关运行时的最终路由决策结构。
 */
export interface RouteDecision {
  /** Primary upstream target. / 主上游目标。 */
  upstream: string;
  /** Optional upstream pool for load balancing. / 可选上游池，用于负载均衡。 */
  upstreams?: string[];
  /** Optional sticky key for affinity routing. / 可选亲和路由键。 */
  affinity_key?: string;
  /** Optional rewritten path sent downstream. / 发往下游的可选改写路径。 */
  rewrite_path?: string;
  /** Extra headers to inject. / 要额外注入的 header。 */
  set_headers?: Record<string, string>;
  /** Headers to strip before proxying. / 代理前需要移除的 header。 */
  strip_headers?: string[];
  /** Optional synthetic HTTP status. / 可选的合成 HTTP 状态码。 */
  status?: number;
  /** Optional synthetic content type. / 可选的合成内容类型。 */
  content_type?: string;
}

/**
 * Partial route patch used while plugins merge decisions in pipeline stages.
 * 插件在流水线阶段里逐步合并决策时使用的局部路由补丁。
 */
export type RoutePatch = Partial<RouteDecision>;

/**
 * HTTP context passed to `access` / `balancer` hooks.
 * 传给 `access` / `balancer` hook 的 HTTP 上下文。
 */
export interface HttpContext {
  /** Unique request id. / 唯一请求 ID。 */
  request_id: string;
  /** Host header / authority. / Host 头或 authority。 */
  host: string;
  /** HTTP method. / HTTP 方法。 */
  method: string;
  /** Request path without query. / 不带 query 的请求路径。 */
  path: string;
  /** Raw query string if present. / 若存在则为原始 query 字符串。 */
  query?: string | null;
  /** Request scheme such as http / https. / 请求 scheme，例如 http / https。 */
  scheme: string;
  /** HTTP protocol version label. / HTTP 协议版本标签。 */
  version: string;
  /** Remote client socket address. / 客户端远端 socket 地址。 */
  remote_addr: string;
  /** Extracted affinity/player key if any. / 若存在则为提取出的亲和或玩家标识。 */
  player_id?: string | null;
  /** Normalized request headers. / 规范化后的请求头。 */
  headers: Record<string, string>;
  /** Buffered request body length in bytes. / 已缓冲请求体字节长度。 */
  body_len: number;
}

/**
 * Stream context passed to `preread` hooks.
 * 传给 `preread` hook 的流量上下文。
 */
export interface StreamContext {
  /** Unique request/session id. / 唯一请求或会话 ID。 */
  request_id: string;
  /** Listener name from config. / 配置中的监听器名称。 */
  listener: string;
  /** Protocol label such as tcp / udp. / 协议标签，例如 tcp / udp。 */
  protocol: string;
  /** Remote client socket address. / 客户端远端 socket 地址。 */
  remote_addr: string;
  /** Extracted affinity/player key if any. / 若存在则为提取出的亲和或玩家标识。 */
  player_id?: string | null;
  /** Optional preview of first packet bytes. / 首包内容的可选预览。 */
  first_packet_preview?: string | null;
  /** Buffered payload length in bytes. / 已缓冲 payload 字节长度。 */
  payload_len: number;
}

/**
 * Generic message envelope passed into script hooks.
 * 传入脚本 hook 的通用消息包。
 */
export interface GatewayMessage<TContext = HttpContext | StreamContext> {
  /** Unique message/request id. / 唯一消息或请求 ID。 */
  id: string;
  /** Message kind. / 消息类型。 */
  kind: "http" | "tcp" | "udp" | string;
  /** Optional listener name. / 可选监听器名称。 */
  listener?: string | null;
  /** Hook-specific context payload. / 针对 hook 的上下文载荷。 */
  ctx: TContext;
}

/**
 * Plugin spec shape visible in `init_worker` and admin plugin operations.
 * `init_worker` 和管理端插件操作里可见的插件规格结构。
 */
export interface PluginSpec<TConfig = JsonValue> {
  /** Stable plugin name. / 稳定插件名称。 */
  name: string;
  /** Module path as registered by gateway. / 网关注册时使用的模块路径。 */
  module_path: string;
  /** Optional priority override. / 可选优先级覆写。 */
  priority?: number | null;
  /** Optional enabled flag. / 可选启用开关。 */
  enabled?: boolean | null;
  /** Plugin configuration payload. / 插件配置载荷。 */
  config?: TConfig;
}

/**
 * Runtime plugin info returned by admin/plugin listing surfaces.
 * 管理端/插件列表返回的运行时插件信息。
 */
export interface PluginInfo {
  name: string;
  module_path: string;
  priority: number;
  enabled: boolean;
  loaded_at?: string | null;
}

/**
 * Payload passed to the `log` hook.
 * 传给 `log` hook 的载荷。
 */
export interface LogPayload {
  message: GatewayMessage;
  route: RouteDecision;
}

/**
 * Payload passed to `init_worker`.
 * 传给 `init_worker` 的载荷。
 */
export interface InitWorkerPayload<TConfig = JsonValue> {
  spec?: PluginSpec<TConfig>;
}

/**
 * Full plugin contract supported by proxysss embedded TS runtime.
 * proxysss 内嵌 TS 运行时支持的完整插件契约。
 */
export interface ProxysssPlugin<TConfig = JsonValue> {
  /** Plugin name. / 插件名称。 */
  name: string;
  /** Higher value runs earlier. / 数值越大越先执行。 */
  priority?: number;
  /** Whether this plugin participates in pipeline. / 该插件是否参与流水线。 */
  enabled?: boolean;
  /**
   * HTTP route hook.
   * HTTP 路由 hook。
   */
  access?: (
    message: GatewayMessage<HttpContext>,
    current?: RoutePatch | null,
  ) => RoutePatch | void | null;
  /**
   * HTTP upstream/balancing hook.
   * HTTP 上游选择/均衡 hook。
   */
  balancer?: (
    message: GatewayMessage<HttpContext>,
    current?: RoutePatch | null,
  ) => RoutePatch | void | null;
  /**
   * TCP/UDP preread hook.
   * TCP/UDP 预读阶段 hook。
   */
  preread?: (
    message: GatewayMessage<StreamContext>,
    current?: RoutePatch | null,
  ) => RoutePatch | void | null;
  /**
   * Observability/log hook.
   * 观测/日志 hook。
   */
  log?: (payload: LogPayload) => void;
  /**
   * Called when worker loads the plugin.
   * Worker 加载插件时调用。
   */
  init_worker?: (payload: InitWorkerPayload<TConfig>) => void;
  /**
   * Called before plugin unload/reload disposal.
   * 插件卸载或重载前调用。
   */
  onDispose?: () => void;
}

declare global {
  /**
   * Environment variables exposed by proxysss embedded runtime.
   * proxysss 内嵌运行时暴露的环境变量映射。
   */
  const PROXYSSS_ENV: Record<string, string>;
}
