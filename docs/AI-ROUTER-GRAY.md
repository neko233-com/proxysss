# ai-router 灰度接入

proxysss 只负责公网边缘层：TLS、限流、连接上限、健康检查和高性能流式转发。ai-router 负责用户、用户组、AI 提供商、分配、用量与费用统计。

## 拓扑

```text
公网 -> proxysss :443 -> ai-router 127.0.0.1:4080 -> sub2api / new-api / AI 提供商
```

将 `examples/ai-router-gray.example.yaml` 合并到现有配置后，先使用独立域名，例如 `ai-gray.example.com`，不要覆盖现有 sub2api 或 new-api 域名。模板关闭 metadata response headers，并明确移除 `x-admin-token`，避免管理员令牌被转发到上游。

## 灰度顺序

1. 启动 ai-router，设置随机的 `AI_ROUTER_ADMIN_TOKEN`，确认 `GET http://127.0.0.1:4080/health`。
2. 在 ai-router 管理后台添加上游 AI 提供商和用户分配。只为真实的上游服务配置 API 密钥。
3. 替换模板中的 proxysss 管理员和 ACME 占位值。管理面板保持 `127.0.0.1:7777`、`loopback_only: true`、`enable_write_ops: false`，不要把示例值用于生产。
4. 用灰度域名验证 `/v1/models`、非流式 Chat Completions、流式 Chat Completions、Responses、Embeddings、Images、Audio，以及超过限制时的 401/429 行为。
5. 用 OpenCode 指向灰度域名运行一次真实 `gpt-5.6 luna` 请求，并同时保留旧入口的日志作为对照。
6. 只有当上游账号不再返回 401、灰度请求成功且费用记录正确，才讨论入口切换。

已有 sub2api 和 new-api 不停止、不重启、不改端口。当前已知 OpenCode 验证阻塞为 Sub2API 唯一可用账号向上游返回 401，Sub2API 随后返回 502/no available accounts；这不能作为 ai-router 路由或 proxysss 网络故障结论。

## 检查

```sh
proxysss -config /etc/proxysss/proxysss.yaml check-config
curl -fsS https://ai-gray.example.com/health
curl -fsS -H "Authorization: Bearer $AI_ROUTER_USER_TOKEN" \
  https://ai-gray.example.com/v1/models
```

发布、tag、release 和生产入口切换全部由人工执行。不要启用 GitHub Actions 自动发布。
