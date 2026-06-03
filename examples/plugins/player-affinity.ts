const LOGIN_BACKENDS = [
  "http://127.0.0.1:8088",
  "http://127.0.0.1:8089",
  "http://127.0.0.1:8090",
];

const TCP_LOGIN_BACKENDS = [
  "127.0.0.1:7001",
  "127.0.0.1:7002",
  "127.0.0.1:7003",
];

const UDP_REALTIME_BACKENDS = [
  "127.0.0.1:8101",
  "127.0.0.1:8102",
];

function isLoginPath(path: string): boolean {
  return path.startsWith("/sdk/login") || path.startsWith("/sdk/register");
}

export default {
  name: "player-affinity",
  priority: 100,
  enabled: true,

  access(message: { ctx: { path?: string; player_id?: string } }) {
    const path = message.ctx.path ?? "";
    if (!isLoginPath(path)) {
      return;
    }

    return {
      upstream: LOGIN_BACKENDS[0],
      upstreams: LOGIN_BACKENDS,
      affinity_key: message.ctx.player_id,
      set_headers: {
        "x-proxysss-plugin": "player-affinity",
      },
    };
  },

  preread(message: { listener?: string; ctx: { player_id?: string } }) {
    if (message.listener === "game-login") {
      return {
        upstream: TCP_LOGIN_BACKENDS[0],
        upstreams: TCP_LOGIN_BACKENDS,
        affinity_key: message.ctx.player_id,
      };
    }

    if (message.listener === "game-realtime") {
      return {
        upstream: UDP_REALTIME_BACKENDS[0],
        upstreams: UDP_REALTIME_BACKENDS,
        affinity_key: message.ctx.player_id,
      };
    }
  },
};
