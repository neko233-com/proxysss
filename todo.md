# proxysss 待办事项

更新日期：2026-06-09

## 状态：本周期功能已完成

v1.0.0 已发布。后续维护项：

- FTP：完整 nginx ftp 模块指令级 parity（当前 transfer hooks + per-user 策略已实现）。
- Wildcard ACME：继续在各文档/init 输出保持 `acme_dns_external` + `acme.sh` 边界一致。
- Caddyfile 适配器（可选，非当前目标）。

## 常用检查命令

```bash
proxysss config explain
proxysss config capabilities
proxysss config nginx-parity --format yaml
cargo test
cargo fmt --all
cargo clippy -- -D warnings
```
