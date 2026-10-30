# 文档索引

本页供维护者和 agent 定位文档；面向使用者的入口是 [中文文档站](index.html)。先读当前主题的参考页，再按需阅读教程、决策或特定版本的历史证据。不要把历史测试结果当作当前版本的验收结论。

## 使用与配置

| 目标 | 人类阅读入口 | 维护与检索入口 | 类型 |
| --- | --- | --- | --- |
| 安装、快速开始、CLI | [首页](index.html) | [项目 README](../README.md) | 教程 |
| 配置与监控 | [配置指南](configuration.html) | [CONFIGURATION.md](CONFIGURATION.md) | 参考 |
| 文件服务与 CDN 源站 | [CDN 下载](cdn-origin.html) | [CDN-ORIGIN.md](CDN-ORIGIN.md) | 参考 |
| 脚本与插件 | [脚本指南](ts-how-to-use.html) | [ts-how-to-use.md](../ts-how-to-use.md)、[类型声明](../proxysss-script.d.ts) | 教程及参考 |
| 从 nginx 迁移 | [nginx 迁移](nginx-to-proxysss.html) | [nginx-to-proxysss.md](../nginx-to-proxysss.md) | 教程 |
| 从 Caddy 迁移 | [Caddy 迁移](caddy-to-proxysss.html) | [caddy-to-proxysss.md](../caddy-to-proxysss.md) | 教程 |
| 火山证书权限 | — | [火山引擎自动化所需权限](火山引擎-自动化所需权限.md) | 参考 |
| 安全策略与调优 | [安全与性能](security-performance.html) | [SECURITY-PERFORMANCE.md](SECURITY-PERFORMANCE.md)、[SECURITY.md](SECURITY.md)、[PRODUCTION-HARDENING.md](PRODUCTION-HARDENING.md) | 参考 |
| AI 路由示例 | — | [AI-ROUTER-GRAY.md](AI-ROUTER-GRAY.md) | 教程 |
| agent 管理接口 | — | [AGENT-API.md](AGENT-API.md) | 参考 |

## 开发、验证与历史

| 目标 | 入口 | 类型 |
| --- | --- | --- |
| 文档维护规则 | [AGENTS.md](AGENTS.md)、[根规则](../AGENTS.md) | 常驻规则 |
| 当前架构 | [HTML](architecture.html)、[ARCHITECTURE.md](ARCHITECTURE.md) | 参考 |
| 验证和发布 | [HTML](validation.html)、[VALIDATION.md](VALIDATION.md)、[发布证据](../performance-evidence/README.md) | 流程及参考 |
| Linux 性能设计取舍 | [decisions/linux-performance.md](decisions/linux-performance.md) | 决策 |
| nginx 对比说明 | [HTML](benchmark-linux.html)、[BENCHMARK-ubuntu24-vs-nginx.md](BENCHMARK-ubuntu24-vs-nginx.md) | 性能参考 |
| 后续改进 | [IMPROVEMENT-BACKLOG.md](IMPROVEMENT-BACKLOG.md) | 待办 |
| v1.3.9 验证记录 | [HTML](release-1.3.9.html)、[RELEASE-1.3.9.md](RELEASE-1.3.9.md) | 历史证据 |
| v1.3.10 验证记录 | [HTML](release-1.3.10.html)、[RELEASE-1.3.10.md](RELEASE-1.3.10.md) | 历史证据 |
| 版本变化 | [CHANGELOG.md](../CHANGELOG.md) | 历史记录 |

`site.css`、`site.js` 和 `404.html` 是文档站资源，不承载独立产品契约。维护规则参考 [DeepSeek Harness 文档规范](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/AGENTS.md)，但使用本项目的 HTML、Markdown、示例和检查工具，不引入其运行时。
