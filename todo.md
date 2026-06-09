# proxysss 待办事项

更新日期：2026-06-09

## 剩余工作

### FTP — partial
- 状态：已实现 `command_allow`/`command_deny`、控制/数据通道结构化日志；仍为 nginx parity partial。
- 剩余：transfer 级 hook、更细粒度 per-user 策略。

### Auto HTTPS — partial
- 状态：内置 managed ACME（HTTP-01/TLS-ALPN-01）为默认路径；wildcard 明确走 `acme_dns_external` + `acme.sh`。
- 剩余：在各文档与 init 输出中持续保持边界一致（已同步 README、CONFIGURATION、AGENT-API、SECURITY、AGENTS、architecture.html）。

### On-demand TLS — missing（Caddy parity）
- 首请求触发、策略门控的证书签发尚未实现。

### 发布规范
- 下一 release tag 前确认 `Cargo.toml` 版本与 `CHANGELOG.md` 段落匹配。
- GitHub Actions 继续使用 Node.js 24 兼容 artifact actions（`upload-artifact@v6` 等）。

## 已完成（本周期）

- `docs/architecture.html` 交互式架构可视化。
- `AGENTS.md` 要求维护 `architecture.html`。
- Compression、cache/proxy cache、rate limiting 从 partial → supported。
- Cache：`vary_headers`、`key_prefix`、`stale_while_revalidate_secs` 后台刷新。
- Rate limit：leaky bucket + `services.rate_limit.stream` TCP 共享区策略。
- Parity 漂移防护测试（README/AGENTS/init 模板/内建 docs）。
- gRPC：HTTP/2 透明转发已支持，无额外缺口（见 capabilities 与 CONFIGURATION）。

## 常用检查命令

```bash
proxysss config explain
proxysss config capabilities
proxysss config watched-scripts
proxysss config routes
proxysss config reload-plan
proxysss config nginx-parity --format yaml
cargo test
cargo fmt --all
```
