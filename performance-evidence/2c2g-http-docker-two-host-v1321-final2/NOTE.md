# v1.3.21 2 CPU 双容器证据说明

- 本目录是本轮发布使用的最终证据（`2026-11-23 22:08`，3 轮 × 2s 预热 + 5s 测量，64 条预连接，0 错误）。
- 二进制为 v1.3.21 发布候选：raw 反代/SSE/WS 车道默认启用 + 逐请求内存黑名单/monitoring 复查 + 有界重试回落 + 按路由 `forward_headers` 写入 `X-Real-IP`/`X-Forwarded-*`/`Forwarded` + AI/SSE 定长响应保持 keep-alive。
- 结果：static-1k `68,626 vs 85,206`（1.242x），static-64k `38,635 vs 53,776`（1.392x），proxy-1k `48,272 vs 49,475`（1.025x）；proxy p50 `1.013 ms vs 1.153 ms`。
- 范围：Docker bridge 双容器（server 容器内同时跑 nginx 1.28 作为被测网关和 19081 后端；client 独立容器），每个容器 2 CPU / 2 GiB cgroup。不是物理双机、不是跨主机生产证据，也不能外推全协议混合负载。
- 历史对照：`../2c2g-http-docker-two-host-v1321-final/` 是加入逐请求准入与转发头之前的中间版（proxy 1.029x），不代表发布语义；`../2c2g-http-docker-two-host-v1321-audit1/` 反代 1.132x 亦为中间版；`../2c2g-http-docker-two-host-v1320-audit2/` 是 v1.3.20 修复路由后的 0.736x 基线。
- 4 CPU 入口（脚本 `-Cpus 4`）在 nginx-proxy 第三轮于 harness 客户端侧挂死（已完成两轮、0 错误），已中止并清理容器；本轮不提供 4 CPU 数据，也不将其作为证据。
