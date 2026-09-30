# Codex Responses WebSocket 实施与验证记录

> 日期：2026-09-27。AIO 源码基线：`420e9958`。
>
> 对应 [开发 Spec](./codex-responses-websocket-development-spec.md)。前半部分是 M0 协议证据，末尾单列实际 AIO 集成证据；两者不相互替代。默认开关保持关闭。

## 验证方法

使用 [本地协议探针](../scripts/codex-responses-ws-probe.mjs)，启动真实 CLI、临时配置目录与工作目录、仅接受固定 Responses 请求的 loopback mock。工具只向临时计数文件写入一行。记录协议关联元数据、请求形状、次数及断言，完整 prompt、凭据、工具参数和输出不进入报告。

必须用探针记录的实际启动器和版本归属实验。终端 `codex --version` 为 0.156.0，但最初 Node 子进程经另一条 PATH 启动了 0.144.4；这批结果不能充当 0.156.0 验收。后续显式指定二进制重跑。

## 实验结果

真实 `codex-cli 0.156.0`、Node `v24.15.0`、macOS arm64 的 11 个场景已通过。实际启动器是 Volta 中 `@openai/codex` 的 `bin/codex.js`；当时探针对同一个解析后的可执行路径执行版本检查和请求实验，并精确核对实验目标；该约束只说明历史证据的版本归属，不应变成用户使用或后续探针运行的版本门槛。不能把终端另一路 PATH 的输出当作实际客户端凭据。

| 场景                      | 实际观测和断言                                                              |
| ------------------------- | --------------------------------------------------------------------------- |
| `upgrade-http`            | 确实发生一次 Upgrade、收到 426，然后一次 HTTP 完整请求成功                  |
| `ws-tool`                 | 预热后同一 WS 完成生成与工具结果增量；工具执行一次                          |
| `context-retry`           | 工具增量收到上下文错误后，重连 WS，以 5 项完整 input 重发且无旧引用         |
| `context-http`            | 增量错误后重连握手 426，转 HTTP 完整重发                                    |
| `context-http-no-retries` | `stream_max_retries=0` 仍会转 HTTP 完整重发；不能将其称为“禁用所有自动恢复” |
| `context-http-tools`      | HTTP 恢复后第二个工具轮继续 HTTP，无新增 WS；两个工具各执行一次             |
| `context-status-400`      | 目标版本对带 status 400 的特定上下文错误仍执行恢复                          |
| `prewarm`                 | 观察到真实 `generate=false`，后续正常生成                                   |
| `prewarm-http`            | 预热错误并关闭、重连 426，随后 HTTP 完整生成；无伪造成功引用、无预热误生成  |
| `context-two-tools`       | 同 turn 两次独立恢复，完整 input 分别 5/7 项；两个工具各一次                |
| `business-400`            | 普通 `invalid_prompt` 400 终止，未自动重放                                  |

7 次完整恢复均在内存核对原输入、工具名称/参数及结果；同时验证实际 create body 的 owner 元数据一致，以及逐项 canonical JSON 的 SHA256 累计摘要和条目数相同。报告仅保存布尔值、计数和合成会话标识，不保存正文或摘要。这证明这些实测请求可以用摘要核对历史，不证明所有 Responses item、取消/迟到请求、跨供应商预算承接已经正确。

实际测试最大 create 为约 33 KiB，最大 HTTP body 约 33 KiB，最大模拟输出事件 347 bytes。这些是小型合成工具链的数据，不能据此宣称大型工具输出和多模态资源上限已经验证。

最终阶段的报告保存在测试机 `/tmp/aio-ws-probe-0156-turn-state-final.json`。可重现命令：

```sh
node scripts/codex-responses-ws-probe.mjs --codex /absolute/path/to/codex --report /tmp/aio-ws-probe-report.json
```

当时的探针另已检查缺少二进制、版本不符、相对二进制路径和 SIGTERM 清理；后续移除精确版本拒绝，仅记录显式选定二进制的实际版本。Windows CRLF 计数路径已修正；尚未实跑 Windows/Linux/WSL，不能列为通过。

## 恢复关联与预热边界

- `x-client-request-id` 是 thread 标识；`turn_id` 被同 turn 多个生成共用。握手可能是 prewarm 的空 turn，必须以各条 create body 内的 `client_metadata.x-codex-turn-metadata` 为准；HTTP 使用相应请求头。
- 已验证 owner 的 session/thread/window/context-window/turn 在自动 WS 和 HTTP 完整重发时保持一致。没有发现独立且跨重试稳定的客户端 generation ID。
- 摘要应从客户端入站、插件改写前的 input 和最终交给客户端的 output item 构建；不能用 provider 侧改写结果代替。客户端会规范化部分 item，不能把 mock 中摘要相等外推到所有类型。
- owner、串行认领、完整摘要/计数可以排除已测工具轮混淆，取消后恢复同 turn、新客户端实例和迟到请求仍需独立判断。已实测 AIO 发出的 `response.metadata.headers.x-codex-turn-state` nonce 被目标 CLI 在后续 WS body/HTTP header 回传。客户端 OnceLock 保留第一次值，新 ModelClientSession 重置；结合 owner、严格历史增长和原子单次认领，形成受控恢复的归属条件；运行时核验实际字段，不用版本号代替归属校验。
- 已证明预热失败后由下一次握手 426 促成 HTTP 安全回退。101 后直接发送“status 426”的 WS error 不是同一个协议行为。若采用已测方案，需明确该会话此后走 HTTP，不能在 UI 继续显示客户端 WS。

## 代理与 TLS 的源码核查

设计基线的 reqwest 为 0.12.28、Axum 为 0.7.9。实现已开启 Axum ws，并使用 tokio-tungstenite 0.24 codec；上游握手复用原 reqwest HTTP/1.1 Upgrade、代理/DNS/证书配置，不额外创建绕过代理的 WS TCP/TLS 路径。

[http_client.rs](../src-tauri/src/gateway/http_client.rs) 的 `get_no_redirect` 可复用现有代理、DNS 和证书策略做 HTTP/1.1 Upgrade；reqwest `Response::upgrade()` 返回实现 Tokio AsyncRead/AsyncWrite 的升级流。codec 仅包装 reqwest 已升级连接；连接器单测覆盖握手校验、错误状态、帧和超时。不能从脱敏的 current proxy URL 重新拼代理凭据，也不能认为热替换 reqwest client 会自动关闭已升级连接。

这是源码核查与 loopback 连接器测试，尚不是跨平台企业证书/VPN 实网验收。

## 实际 AIO 集成证据

[路由集成测试](../src-tauri/src/gateway/responses_ws/integration_tests.rs) 使用真实 `build_router`、临时 SQLite/settings、真实 loopback listener 和假供应商。普通 HTTP、关闭时 426、WS 成功、同家 HTTP 降级、冷却、A 失败切 B HTTP、输出后断流/错误禁止换家、JSON 完成和 incomplete 均有独立断言。

`real_codex_cli_rebuilds_context_then_fails_over_without_repeating_tool` 另外启动真实 `codex-cli 0.156.0`：AIO → A 上游 WS → 工具调用 → 上下文丢失 → CLI 全量重发 → A 失败 → B HTTP 完成。实际执行已通过；工具计数文件只有一行，B 收到原 input 和配对的 tool call/output，未收到旧 `previous_response_id` 或本地 nonce。CLI 使用临时 HOME/CODEX_HOME/工作目录，不读取用户的真实认证或改写其配置。

```sh
AIO_CODEX_WS_TEST_CLI=/absolute/path/to/codex   cargo test --locked --manifest-path src-tauri/Cargo.toml --lib   real_codex_cli_rebuilds_context -- --ignored --test-threads=1
```

此用例默认 ignored，必须显式提供目标 CLI 才能算执行；普通 `cargo test` 的 ignored 数量不能算作通过。

[事件门控测试](../src-tauri/src/gateway/responses_ws/gate.rs) 验证中性前缀错误不提交、提交后 EOF 明确失败、空完成/incomplete/failed 分离、JSON 补全 done 事件不重复、分块顺序与上游路由 token 剥离。[状态测试](../src-tauri/src/gateway/responses_ws/state.rs) 验证归属冲突、双认领、TTL/预算、迟到终态、冷却单探测和资源准入；这些是网关约束，不把 prompt 摘要当独立生成 ID。

固定源码补充：`rust-v0.156.0` 对应提交 `fe74a774532af67b5a4a3dec03ce9469e17f89af`，protocol `models.rs` 的 SHA256 为 `d34f7b4f81f189e3446f27bb58de90e9c3d6d473ba3fa9a9aaa64371b23ab671`；未知 item/字段投影拒绝恢复。

## 冻结的运行边界

- 普通 WS 按实际协议接入，不以版本或 User-Agent 白名单准入；缺恢复元数据不阻止普通请求。自动恢复仍核验 owner、nonce、完整历史及原预算，错误本地恢复 nonce 不得退回普通请求绕过校验。
- 全局开关和 provider 能力均默认 false；首版上游 WS 仅原生 Codex API-key provider，其他路径使用既有 HTTP。
- 单帧/事件/受控恢复 input 为 4 MiB；首内容前 1 MiB/256 事件；输出队列 4 MiB/256 事件；codec 写窗口 8 MiB；WS 分支 fixer 1 MiB。
- 固定共享准入预留 256 MiB，每个 WS/受控 HTTP 上下文 128 MiB，最后一个持有者释放后归还；有效并发至多 2。它限制受控 WS 缓冲窗口和并发，不是进程 RSS 或既有插件执行器 heap 的承诺。
- 新 WS 资源不足在升级前回 426；HTTP 恢复申请资源先于原子认领，拒绝不会消费恢复资格。普通 HTTP 不受新资源池限制。
- 建连最多 5 秒，且不突破当前 attempt 剩余期限；WS 冷却 60 秒，过期仅一个探测；空闲连接 60 秒；待恢复记录 30 秒/最多 128 个。
- 无共享 WS 会话池、完整历史缓存、响应 Replay 或 WS-only 模式。取消不换供应商；应用退出关闭连接；关闭开关允许已接受生成收尾，不接受新 create。

## 验收范围

已在 macOS arm64 的 loopback 环境验证。Windows、Linux、WSL 实机、系统睡眠唤醒、VPN 和企业 CA/代理组合仍需发布前实测。WSL TOML/manifest/串行同步逻辑有自动测试，不等同于 WSL 网络实机验收。跨平台和 UI 视觉验收未执行项不能填写为 T01–T42 全通过。

## 首轮实现自动检查（2026-09-27）

以下分组存在覆盖重叠，不相加为独立用例总数。所有检查均针对未提交工作区；未提交、未推送，也未修改测试外的真实 CLI 配置。

| 检查                                                         | 结果                                                                                      |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------------------- |
| `cargo test --lib gateway::responses_ws -- --test-threads=1` | 48 通过、1 默认忽略；包括 13 个真实路由场景、全部供应商失败、取消与关闭、资源/门控/连接器 |
| 显式 `real_codex_cli_rebuilds_context`                       | 1 通过；真实 0.156.0，工具恰好一次，恢复及跨家后累计 attempt 不超过原预算                 |
| native CLI proxy / WSL config / WSL commands                 | 分别 70 / 47 / 8 通过；均为自动逻辑/文件测试                                              |
| 配置导入运行态                                               | 6 通过，包含 true→false 与同步失败反馈                                                    |
| 共享 HTTP/SSE、ResponseFixer、Gemini、CX2CC                  | 204 项定向回归通过；HTTP client 另 28 项通过                                              |
| 相关前端                                                     | 17 文件 285 项通过；最后新增握手/事件状态展示后，对应 2 文件 37 项再次通过                |
| `pnpm typecheck` / `pnpm lint`                               | 通过                                                                                      |
| `cargo clippy --all-targets --locked -- -D warnings`         | 通过                                                                                      |
| `cargo fmt -- --check` / `git diff --check`                  | 通过                                                                                      |
| `pnpm tauri:gen-types` / `pnpm check:generated-bindings`     | 生成后再校验通过，包含同工作区已有的其他合法 IPC 变更                                     |
| 错误码/支持矩阵/spec 链接检查                                | 通过；三份本文档另显式检查本地链接及 Prettier                                             |
| 禁止参考名称                                                 | 已扫描新增和修改的实际源码/测试/脚本，无匹配                                              |

该源码快照的只读复审当时未发现新增可复现 P0/P1；后续功能审查另有发现，修复见下节。首轮检查后补充了全部供应商失败、插件在输出后阻断仍保持本地归因、代理热重载连接隔离、握手/事件状态区分与对应回归，并重新执行 WS/真实 CLI/Clippy 检查。平台实机限制仍按上节保留。

## 审查修复后的复验（2026-09-27）

本次修复跨 turn continuation、compaction/WS 配置及备份冲突、恢复期限误拒绝、坏 HTTP SSE 错误归因、插件标记正文误判、明确能力错误降级、incomplete 与同家降级展示、WSL TOML 根键解析及 CLI 版本提示。真实插件集成还复现并修复了共享 beforeSend 重试累积改写的问题。逐项文件与修复说明见 [复审记录 §8](./codex-responses-websocket-spec-review.md#8-功能审查修复与二次复验2026-09-27)。

- WS 综合：59 通过、2 默认忽略；两个 ignored 均为需显式提供 CLI 的真实协议用例，已另行执行。
- 真实 Codex CLI 0.156.0：2 通过，均经过生产 router。新增连续两个用户 turn，第二 turn 再工具增量：仅一条上游 WS、三个正式生成、工具执行一次；原上下文恢复→A 失败→B HTTP 用例继续通过。
- 恢复 deadline 路由：TTL 内正确 nonce/历史认领不刷新旧期限；A 无新 HTTP 发送，timeout 标记 upstream_sent=false；随后 B 一次 HTTP 成功。
- 真实插件集成：2 通过；afterBodyRead/beforeSend/chunk 次数、非幂等改写、WS→HTTP、A→B、普通 HTTP 503 同家重试均有生产链断言。扩大回归发现内部 rectifier 需保留首轮脱敏，已改为修复实际发送的正文，原脱敏测试断言未改；rectifier 47 项、gzip 11 项回归通过。内部 repair 保持原逐 attempt hook 合同，普通重试/降级不累积改写。
- 配置/WSL：native cli_proxy 86 项、Codex 配置 36 项、WSL 逻辑 48 项通过，涵盖两种设置顺序 × 三种原 WS 值、两个受管别名、未知嵌套字段、冲突拒绝及根键语法。扩大回归发现严格解析阻断旧重复表归并，已调整为复用旧去重后完整校验，旧测试保持不变并补 OpenAI 别名反例；这些仍属于逻辑/文件测试。
- 前端全量：296 文件、2518 项通过；随后新增持久化日志回灌两种场景，对应 projection 文件 24 项通过。实时、列表、详情均覆盖 incomplete，同家降级不误报换家。

- 最终全网关回归：`cargo test --lib gateway:: -- --test-threads=1`，931 通过、0 失败、4 忽略；其中两个真实 CLI 用例随后显式重跑，2 通过，另外两个为既有插件性能 smoke，本次未执行。包含上述 WS 与共享 HTTP/SSE、CX2CC、Gemini、插件、rectifier 及事件契约测试。
- 最终静态检查：`pnpm typecheck`、`pnpm lint`、`cargo clippy --all-targets --locked --offline -- -D warnings`、Rust fmt、前端 Prettier、生成 bindings 校验、diff/新增文件空白检查通过；三份文档本地链接/锚点与实际源码禁止名称扫描通过。
- 二次独立只读复审：跨 turn/nonce、恢复 TTL/旧预算承接、实时事件与持久化回灌、incomplete 与同家降级展示未发现新增有效风险。HTTP 200/attempt success 仍描述传输结果，不再被当成逻辑完整终态。

以上测试分组存在重叠，不能相加作为独立总数。测试使用临时目录、合成认证和 loopback 上游；未提交、未推送。Windows/Linux/WSL 实机、睡眠唤醒、VPN/企业 CA/代理组合及 UI 视觉验收仍待执行，不宣称 T01–T42 全部完成。

## 版本兼容性修正后的复验（2026-09-27）

这是移除精确版本门禁、拆分普通 WS 与恢复身份后的验证快照；前节数字保留为历史记录。普通请求只按实际协议接入，`Generation.identity: Option<RecoveryIdentity>` 不取消提交门控、尝试预算或 provider failover。恢复仍检查 owner、nonce、历史、请求约束与原预算。

本轮实际客户端为桌面内置 `codex-cli 0.158.0-alpha.2.1` 与独立安装的 `codex-cli 0.156.0`，各自显式选择二进制执行，未将 shell 版本推定为桌面进程版本。两版本均通过以下两个真实生产 router 用例：

- `real_codex_cli_two_user_turns_and_tool_increment_keep_the_same_ws_context`：连续两个用户 turn，随后工具增量；同一上游 WS 上完成三个正式生成，工具执行一次。
- `real_codex_cli_rebuilds_context_then_fails_over_without_repeating_tool`：上游上下文失效后完整重建、供应商 A 失败后 B HTTP 完成，工具执行一次，继续使用原预算。

| 检查                                                    | 本轮结果与范围                                                                                                                                                            |
| ------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 全网关 `cargo test --lib gateway:: -- --test-threads=1` | 932 通过、0 失败、4 默认忽略；其中两个真实 CLI 用例已分别显式执行，另外两个为既有插件性能 smoke，本轮未执行                                                               |
| 上述全网关中的 `responses_ws`                           | 66 通过、2 默认忽略；包含版本/缺失 UA 准入、普通请求无恢复身份、同 socket 续接、nonce 冲突、历史完整性、降级/切家及提交后禁止重放                                         |
| 桌面内置 CLI `0.158.0-alpha.2.1` 经 AIO router          | 2 通过、0 失败                                                                                                                                                            |
| 独立 CLI `0.156.0` 经 AIO router                        | 2 通过、0 失败                                                                                                                                                            |
| 桌面内置 CLI 协议探针                                   | 11 场景通过；该探针是 CLI 直接连接假上游，与前两行的 AIO router 用例分别记账                                                                                              |
| CodexTab 定向前端用例                                   | 26 通过，包含新提示与设置保存行为                                                                                                                                         |
| 静态检查                                                | `pnpm typecheck`、`pnpm lint`、`cargo clippy --all-targets -- -D warnings`、Rust fmt、相关 Prettier、`pnpm check:generated-bindings`、diff 空白及实际源码禁用名称扫描通过 |

安全边界另有回归：同 socket 可省略已持有 nonce 的回传，完整 input 仍须严格扩展已完成历史；Upgrade header 与 body 的本地 nonce 不一致时拒绝，首个正式生成后不再沿用旧 header nonce。未知历史字段允许首个普通请求及有效 `previous_response_id` 续接；丢弃引用后的全量重建仍须证明完整历史，无法证明即失败，不能以普通协议准入绕过恢复校验。

测试机证据：`/tmp/aio-ws-version-gateway.log`、`/tmp/aio-ws-real-cli-desktop-final.log`、`/tmp/aio-ws-real-cli-standalone-final.log`、`/tmp/aio-ws-desktop-probe-all.json`。分组存在重叠，不相加作为独立用例总数。macOS arm64 本地打包已通过：`pnpm tauri:build:mac:arm64` 退出 0，产物位于 `src-tauri/target/aarch64-apple-darwin/release/bundle/macos/AIO Coding Hub.app`，构建日志为 `/tmp/aio-ws-version-build.log`。尚未替换或重启用户正在运行的应用。

本轮未向远程生产供应商发送 WS 生成请求，未修改服务器配置。Windows/Linux/WSL 实机、VPN/企业证书、睡眠唤醒及 UI 视觉验收仍未完成；loopback 测试不代替这些验收，也不保证任意客户端的自动恢复行为。
