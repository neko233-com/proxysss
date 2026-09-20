# 文档维护规则

先读根目录 [AGENTS.md](../AGENTS.md)。本文件只规定 `docs/` 的组织和维护方式。

## 内容归属

| 入口 | 负责内容 |
| --- | --- |
| 根 `AGENTS.md` | 每次工作都必须遵守的产品、性能和发布约束；链接到详细说明 |
| [架构](ARCHITECTURE.md) / [HTML](architecture.html) | 当前模块职责、请求流程、资源所有权、扩展点与重载边界 |
| [配置](CONFIGURATION.md) / [HTML](configuration.html) | 字段语义、默认值和可执行配置示例 |
| [CDN 下载](CDN-ORIGIN.md) / [HTML](cdn-origin.html) | 源站授权、HTTP 下载语义、资源限制与运维方式 |
| [验证指南](VALIDATION.md) / [HTML](validation.html) | Linux 优先的功能、混合性能、Windows 适配和发布证据流程 |
| `decisions/` | 取舍、已拒绝实验和复验要求；历史数据不能充当当前发布证据 |
| `skills/`（仓库根目录） | 可复用操作流程，不重复定义产品行为 |

每项事实只设一个主要维护位置，其余入口给出短说明和链接。Markdown 与对应 HTML 表达同一行为；Markdown 供 agent 检索，HTML 面向用户。官方说明中文优先，README.md 的英文入口按根规则保留。

## 修改顺序

1. 确认事实来自当前源码、命令输出或有提交标识的实测工件。
2. 在所属主题中描述行为、限制、失败情况和资源释放；架构页只概述并链接，不放实验过程。
3. 同步 HTML、内置文档和受影响的配置示例、CLI 能力输出；脚本 API 变化还要同步声明与脚本指南。
4. nginx/Caddy 迁移教程先编辑根目录 Markdown，再用 PowerShell 7.2+ 执行 `pwsh -File scripts/render-migration-docs.ps1` 生成对应 HTML 正文；页面外壳、导航与样式单独维护。生成器记录源文档与正文摘要，禁止直接手改生成区。
5. 运行 `go run scripts/verify-docs.go` 检查文档配对、生成内容摘要、导航链接和架构入口。Docker 功能验证同时运行检查器的修改/缺失/重复摘要测试。页面视觉/交互验证仍只用应用内 Browser。

配置、架构、CDN 和 nginx/Caddy 迁移教程的 YAML 通过实际 `GatewayConfig` 解析、`validate` 和逐字段序列化回读；每组 Markdown/HTML 保持相同示例多重集合。`src/config/documentation_tests.rs` 还遍历全部公开 HTML、渲染后的内置 /docs，以及 examples/templates 的网关 YAML（排除插件元数据）。未知字段现在会导致配置加载失败，逐字段回读继续检查写错值、非规范别名和示例不一致。script/plugins 等根节点也须参加检查，不能声称不存在的认证、缓存或路由字段已生效。主配置使用当前规范字段，支持的别名由专门解码测试保护。CDN 的 CHANGE_ME 是明确要求替换的密钥占位符：测试先确认原配置被拒绝，再仅替换站点 origin_token/signed_url.secret 为本次随机测试值进行校验；不得删除部署者必须填写密钥的保护。

教程按读者实际操作顺序组织；参考页按查询主题组织。不要把旧参数旁追加“当前修订”当成同步维护，应替换过期叙述，把原有取舍移到决策记录。文档不能把“目标”“支持能力”“本次验证通过”写成同一件事。

本分层参考 [DeepSeek Harness 文档规范](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/AGENTS.md)，按 proxysss 的 Rust 单进程网关、中文 HTML 文档和本地验证流程调整。
