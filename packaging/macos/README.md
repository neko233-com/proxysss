# Mac 内网 BuildWorld 入口

`gamer-buildworld.plist` 是 `192.168.110.42` 的单机 LaunchDaemon 配置，绝对路径和用户名仅适用于这台 Mac。它以 `gamer` 运行，开机启动并在进程退出时重启；系统服务清单由 root 持有。网关配置来自 `examples/macos-buildworld.example.yaml`，监听 `8888` 并将请求转给本机 BuildWorld `8080`。系统服务文件和网关配置都不包含凭据。

当前使用官方 `v1.3.12` macOS ARM64 二进制；官方压缩包 SHA-256 为 `6234af3557c609301b8efc91bae93f46c06c1ddc3efef1ffa53943902d3d2160`，二进制 SHA-256 为 `07076bf08546dd32518506b16c1e28ace4e13945f2531d6e43d10c5f7dea984a`。配置目录由服务账号持有；二进制和系统服务清单由 root 持有。变更前先校验配置及现有连接，监听地址变更需要重启服务。

`gamer-static-share.plist` 和 `examples/macos-static-share.example.yaml` 承载同一 Mac 的 `50200` 文件服务，数据目录位于服务账号的 `~/.local/share/proxysss-share`。历史桌面目录作为指向该数据目录的路径别名，既有上传路径继续使用同一份文件，不存在双写。迁移目录前确认同一文件系统、无活动句柄，迁移后验证目录浏览和实际文件的 Range 下载；系统服务只运行 ProxySSS。

现场验收检查实际的 `/api/health`、认证 WebSocket 握手、文件服务以及对应的 `launchctl` 状态，不能只以进程存在作为成功依据。配置热重载已启用；公网 TLS `20101` 仍需受信任证书与实际端到端验收。
