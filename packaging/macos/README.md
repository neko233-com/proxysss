# Mac 内网 BuildWorld 入口

`gamer-buildworld.plist` 是 `192.168.110.42` 的单机 LaunchDaemon 配置，绝对路径和用户名仅适用于这台 Mac。它以 `gamer` 运行，开机启动并在进程退出时重启；系统服务清单由 root 持有。网关配置来自 `examples/macos-buildworld.example.yaml`，监听 `8888` 并将请求转给本机 BuildWorld `8080`。系统服务文件和网关配置都不包含凭据。

当前使用官方 `v1.3.12` macOS ARM64 二进制；官方压缩包 SHA-256 为 `6234af3557c609301b8efc91bae93f46c06c1ddc3efef1ffa53943902d3d2160`，二进制 SHA-256 为 `07076bf08546dd32518506b16c1e28ace4e13945f2531d6e43d10c5f7dea984a`。配置目录由服务账号持有；二进制和系统服务清单由 root 持有。变更前先校验配置及现有连接，监听地址变更需要重启服务。

现场验收检查实际的 `/api/health`、认证 WebSocket 握手和 `launchctl print system/com.neko233.proxysss.buildworld`，不能只以进程存在作为成功依据。配置热重载已启用；公网 TLS `20101` 和静态目录 `50200` 仍需分别解决受信任证书及 macOS 隐私授权后验收。
