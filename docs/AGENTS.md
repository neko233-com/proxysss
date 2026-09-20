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
4. 运行 `go run scripts/verify-docs.go` 检查文档配对、导航链接和架构入口。页面视觉/交互验证仍只用应用内 Browser。

教程按读者实际操作顺序组织；参考页按查询主题组织。不要把旧参数旁追加“当前修订”当成同步维护，应替换过期叙述，把原有取舍移到决策记录。文档不能把“目标”“支持能力”“本次验证通过”写成同一件事。

本分层参考 [DeepSeek Harness 文档规范](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/AGENTS.md)，按 proxysss 的 Rust 单进程网关、中文 HTML 文档和本地验证流程调整。
