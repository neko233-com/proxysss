# CDN 回源、安全下载与目录索引

proxysss 可以直接作为 CDN 的静态源站。公开资源使用源站令牌和缓存策略；私有资源增加短期签名 URL。源站校验发生在目录解析、文件缓存、`HEAD`、`Range`、`304` 之前。

## 公开资源作为 CDN 源站

```yaml
services:
  static_sites:
    - name: cdn
      path_prefix: /assets
      root: ./public
      index_files: [index.html, index.htm]
      autoindex: false
      hide_dotfiles: true
      cache_control: public, max-age=300
      security:
        origin_token: CHANGE_ME
        allowed_peers: [192.0.2.0/24, '2001:db8::/32']
      rate_limit:
        enabled: true
        zone: cdn-origin
        requests: 300
        window_ms: 1000
        burst: 100
```

`CHANGE_ME` 故意不满足校验：启动前必须替换成至少 32 字节的随机令牌。示例 IP 是文档地址，必须替换成你的 CDN 回源 IP/CIDR。CDN 回源时**覆盖** `X-Proxysss-Origin-Token`，不要把访客提交的同名 header 透传；回源使用 HTTPS。令牌用于鉴别回源方，不代表访客已获资源权限。

`allowed_peers` 检查 TCP 对端 IP，不信任 `X-Forwarded-For`、`X-Real-IP`。IPv4、IPv6、IPv4-mapped IPv6 均可处理。源站的 `remote_addr` 限流会把同一 CDN 出口合并计数，额度应按出口设置；访客防盗刷、限速和费用封顶还需在 CDN 边缘执行。下载内容已缓存在 CDN 时，请求不会抵达源站。

公开文件支持 `GET` / `HEAD`、MIME、大文件流式发送、单段及多段 `Range`、`206` / `416`、弱 `ETag`、`Last-Modified`、`If-None-Match`、`If-Modified-Since`。`If-Range` 日期必须精确匹配，且文件已至少 60 秒未修改；近期修改、日期不匹配或弱 ETag 均退回完整 `200`。配置 `cache_control` 后可明确控制公开资源缓存。未配置时沿用默认响应行为，不自动承诺 CDN TTL。

## 私有资源与短期签名 URL

在同一站点增加：

```yaml
security:
  origin_token: CHANGE_ME
  signed_url:
    enabled: true
    secret: CHANGE_ME
    max_ttl_secs: 300
```

两种密钥都至少 32 字节，分别生成；仅保存在服务端 YAML，配置展示和管理 API 会脱敏。`max_ttl_secs` 范围为 1–86400 秒，默认 300。签名 URL 与源站令牌同时配置时必须全部通过。独立私有下载服务器可以只配置 `signed_url`；此时不要给客户端源站令牌。

```bash
proxysss -c proxysss.yaml static-sign --site cdn --path /assets/bundle.bin --ttl-secs 120
```

输出相对 URL：`/assets/bundle.bin?expires=…&signature=…`。`--path` 使用 URL 编码后的绝对路径，不带 query/fragment；中文、空格、`#`、`?` 等应先编码。CLI 从配置读取密钥，密钥不会出现在命令参数中。签发算法供后端实现：

```text
message = "proxysss-static-v1\n" + site.name + "\n" + URI.path + "\n" + expires
signature = base64url_no_padding(HMAC_SHA256(secret_utf8, message_utf8))
```

`expires` 为 Unix 秒，必须是规范的十进制正整数且满足 `now < expires <= now + max_ttl_secs`。仅允许一个 `expires` 和一个 `signature`；重复、额外或编码别名参数都会拒绝。签名绑定站点名、编码后的路径和到期时间，`GET` / `HEAD` / `Range` 可复用；不绑定用户、设备或 IP，也不是一次性链接。链路中各节点时钟应同步。

所有签名响应和静态错误响应都发送 `Cache-Control: private, no-store` 与 `CDN-Cache-Control: no-store`。源站无法撤销已经命中的 CDN 缓存：若需要私有资源仍享受共享缓存，应由 CDN 在**每次访客请求、包括缓存命中时**验证其自身签名，然后只用源站令牌回源。不要对源站签名下载配置强制缓存、忽略 query、忽略 `no-store`；不要缓存鉴权失败响应。

## 像 nginx 一样浏览目录

```yaml
services:
  static_sites:
    - name: downloads
      path_prefix: /
      root: ./downloads
      index_files: [index.html, index.htm]
      autoindex: true
      hide_dotfiles: true
      autoindex_max_entries: 1000
```

显式配置 `/` 站点可以作为文件服务器根目录；未配置站点时 `/` 仍显示 `Welcome to proxysss`。优先返回 index 文件，目录路径缺少 `/` 时重定向并保留 query。没有 index 且 `autoindex: false` 返回 `403`。多个静态站点按 YAML 顺序取第一个匹配项，具体路径必须放在 `/` 之前。公开根目录不要包含私有站点的数据。

索引使用 HTML，显示文件名、字节数、GMT 修改时间，目录排在前面，提供站点内的上级目录链接；文件名经过 URL 编码和 HTML 转义。`HEAD` 不返回页面正文。索引响应不缓存，并设置 CSP、`nosniff`、`no-referrer`。本实现不声称支持 nginx 的 XML/JSON/JSONP 索引格式。

`autoindex_max_entries` 默认 1000、范围 1–10000，限制单次目录扫描条目，超过返回 `413`，不生成不完整索引。隐藏项也计入扫描上限。`hide_dotfiles: true` 同时禁止直接访问 `.env`、`.git` 等点文件/目录，并从预加载和索引中排除；显式改成 `false` 会公开这些文件，公开目录应独立存放。

禁止 `..`、反斜杠、控制字符、Windows ADS 冒号和结尾点/空格；文件、index 和符号链接的真实路径必须留在站点根目录内。文件系统根目录属于可信运维边界，应由部署账号维护，不应向访客开放任意文件/符号链接写入权限。目录索引与 `signed_url.enabled=true` 不能同时启用，目录授权不会自动签发子文件链接。

## 热重载、日志与脚本边界

站点安全配置随 YAML 热重载；新请求读取新策略，预热和 readiness 会重新执行。启用鉴权、站点限流、自定义缓存策略、热重载或管理更新时，使用完整静态处理链，避免连接缓存复用旧授权。普通无策略、不可变配置继续使用预加载的静态快路径。

访问日志保留路径、状态、对端地址和耗时，不记录签名 query 或源站令牌。业务 Referer/User-Agent/bot-score 规则由 CDN WAF 或脚本代理路由处理；原生 `static_sites` 不调用脚本路由钩子。Referer 可以伪造，不能替代身份认证。客户端 AES、manifest 校验、运行时防逆向不属于通用网关职责；短期链接也不能防止已获授权的客户端复制资源。

## 幂等验证与本地打包

Windows 运行 `test.cmd`，等价于 `scripts/verify-local.ps1 -Repeat 2`：格式、clippy、全量测试连续两轮，然后本地打包。`-SkipPackage` 可仅验证代码。测试结束（含失败）清理本轮测试沙箱；报告覆盖写入 `.tmp/verification/latest/`。单独 `cargo test` 的 Rust fixture 也位于 `.tmp/tests/`，新测试使用作用域清理；异常中断留下的旧 fixture 会在下次统一验证时清理。

构建缓存使用 `target/`、依赖缓存使用 `.cache/`，编译临时文件位于 `.tmp/toolchain/`。`scripts/package-local.ps1` 固定输出 `dist/proxysss-local.zip`，打包 staging 位于 `.tmp/package-local/` 并在完成或失败时清理。只打包明确列出的二进制、文档和示例，不带本地 YAML、令牌、证书、日志、Git、缓存和测试数据。验证/打包有进程锁，清理前检查绝对路径和链接边界。

这些是功能与幂等性检查，不是 Linux 性能发布证据；Linux 混合矩阵和跨主机性能门禁仍按项目原有要求执行。

参考：[nginx autoindex](https://nginx.org/en/docs/http/ngx_http_autoindex_module.html)、[Cloudflare 缓存规则](https://developers.cloudflare.com/cache/how-to/cache-rules/settings/)、[CloudFront 签名 URL](https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/private-content-signed-urls.html)。

## 多段下载与条件请求

热正文缓存使用不可变内存快照，单对象读取不超过 32 MiB，不直接 mmap 可变文件；就地覆盖/截断不会破坏已缓存正文或令 HTTPS 进程崩溃。现有响应可能在 2 秒 freshness 窗口及本次后台刷新完成前继续使用旧快照，删除或刷新失败后回到正常文件检查。推荐仍使用写新文件后原子替换，以避免读取过程中内容变化。

正文缓存使用原子预留的 256 MiB 上限；进程内全部静态快照（含正在填充、已淘汰但响应仍引用的正文）另受 320 MiB 硬上限约束。最后一个 Bytes 引用释放时归还预算，预算耗尽的冷下载立即回退流式读取，预热跳过该对象。缓存淘汰只扣减一次，填充协调锁随成功、失败或取消清理；缓存命中不重新争用预算。此上限不包括 socket、TLS、压缩输出与其他网关内存。

静态下载支持最多 16 个 Range 区间，合并重复、相邻和重叠部分；多个剩余区间返回 `multipart/byteranges`，每段带 Content-Range，总 Content-Length 精确计算。有效集合中的越界段被跳过，全部不可满足返回 416；未知单位、格式错误或超限集合忽略 Range，返回完整表示。HEAD 忽略 Range，不读取正文。普通 GET 快路径可能省略可选验证头；显式条件请求进入元数据验证路径。

If-Match / If-Unmodified-Since 优先于缓存验证和 Range，失败返回 412。元数据 ETag 是弱验证器，不能通过 If-Match 强比较，已存在文件可用 `*`；If-Range 保持精确日期且文件已稳定 60 秒的规则。单段按偏移读取，小范围不会整文件载入；多段持有一个文件描述符，按下游需求读取 64 KiB 有界池缓冲，空闲池最多 8 MiB。中途截断终止响应，取消释放文件与缓冲。部署资源推荐写新文件后原子替换。

Linux 完整大文件走原生 sendfile 时，HTTP 等待任务被取消会主动唤醒后台 owner，清理该作业持有的 socket 和文件描述符；不依赖停止读取的客户端再次发送数据或关闭连接。这不等于自动断开所有慢下载：尚未取消的响应继续受正常 socket 背压控制。

```bash
curl -i -H 'Range: bytes=0-15,1048576-1048591' http://127.0.0.1/assets/game.pak
curl -I -H 'Range: bytes=0-15' http://127.0.0.1/assets/game.pak
```

这些下载行为由原生静态服务实现，无需脚本 API。反向代理下载保留上游响应语义。Linux 混合负载、Windows 功能与发布验收见 [验证指南](VALIDATION.md)。
