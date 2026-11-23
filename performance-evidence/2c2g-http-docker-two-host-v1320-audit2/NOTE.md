# v1.3.20 审计轮证据说明

- 路由修正：v1.3.19 及更早的 catch-all 静态站会遮蔽更具体的反代路由。历史基线中 proxy-1k 88,490 RPS 与 static-1k 88,555 RPS 几乎同值，证明该“反代”成绩实为静态快路径误路由，不能作为反代性能证据；真机 1.814x 同口径存疑。
- v1320-audit：修正路由后、默认主动健康开启（反代快路径不合格）的真实反代成绩 proxy-1k 0.665x。
- v1320-audit2（最终口径）：bench 配置显式关闭 hot_reload/active_health/passive_health/retries，与 nginx 默认（无健康检查、无重试）对齐；static-1k 1.044x、static-64k 1.246x、proxy-1k 0.736x，0 错误。
- 环境噪声：nginx static-1k 在同机不同轮介于 70k–91k RPS，Docker Desktop CPU 放置偏斜明显；比率只作同轮对照。
- 结论：静态面稳定胜出；真实反代热路径（hyper 用户态回源）在 2C2G Docker bridge 下落后 nginx，列为后续专项优化，不在本轮宣称反代性能胜出。
- 边界：Docker bridge 双容器，非物理双机/跨主机生产证据；仅 HTTP/1 静态与反代三场景，非全协议混合负载。