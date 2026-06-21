# aio coding hub 社区插件系统开发计划

## 1. 目标与开发边界

### 1.1 总体目标

建设一套面向社区扩展的通用插件系统，使 aio coding hub 后续可以支持第三方开发者开发、发布、安装、配置、启用、禁用、升级、卸载和维护插件，并优先覆盖以下三类插件场景：

1. 提示词优化插件：在 CLI 请求进入 aio coding hub 网关后、发送给上游模型前，对请求体中的提示词内容进行结构化改写。
2. 安全检测插件：检测大模型返回内容中的高危操作、泄密风险和危险命令，并支持告警、脱敏、阻断等策略。
3. 脱敏插件：对请求、响应、日志和 GUI 展示内容中的敏感 URL、密钥、令牌、连接串等信息进行脱敏。

### 1.2 当前架构基础

当前项目已具备插件系统所需的若干基础能力：

- 三端 GUI：Tauri 2 + React + Vite + TypeScript。
- 后端宿主：Rust 主进程，内置 Axum 网关。
- 请求入口：`src-tauri/src/gateway/routes.rs`。
- 请求代理处理：`src-tauri/src/gateway/proxy/handler/mod.rs`。
- 上游发送：`src-tauri/src/gateway/proxy/handler/attempt/send.rs`。
- 响应处理：`src-tauri/src/gateway/proxy/handler/response/response_router.rs`。
- 流式处理：`src-tauri/src/gateway/streams/usage_tee.rs`。
- Tauri 命令注册：`src-tauri/src/commands/registry.rs`。
- 前端 IPC：`src/generated/bindings.ts` 与 `src/services/generatedIpc.ts`。
- 数据目录：`~/.aio-coding-hub`，由 `src-tauri/src/infra/app_paths.rs` 管理。
- 持久化：SQLite 与 `settings.json`。
- 构建发布：GitHub Actions + Tauri updater，覆盖 Windows x64、macOS Intel、macOS Apple Silicon、Linux x64。

### 1.3 明确开发边界

#### 范围内

- 插件 manifest 规范。
- 插件状态管理。
- 插件安装、启用、禁用、卸载、升级和回滚。
- 插件配置 schema 声明、GUI 渲染、校验和持久化。
- 插件权限声明、用户授权、运行时权限裁剪。
- 网关请求/响应 hook pipeline。
- 非流式响应和流式响应插件处理。
- 插件审计日志、错误隔离、超时、熔断。
- 声明式规则插件。
- 官方示例插件：提示词优化、安全检测、脱敏。
- 中期引入 WebAssembly 插件运行时。
- 中长期支持插件市场、GitHub release 安装、本地导入、离线安装。
- SDK、脚手架、开发文档和测试工具。

#### 范围外

- 短期不支持任意 JavaScript/TypeScript 插件直接执行。
- 短期不支持 Node.js/Deno 插件作为默认插件运行时。
- 不支持第三方原生动态库直接加载进 Rust 主进程。
- 不允许第三方插件直接运行在 Tauri WebView 中。
- 不承诺拦截 CLI 本地输入框或 CLI 内部 UI 事件。
- 不将当前 Skill 市场直接改造成插件运行系统。
- 不做企业级 RBAC、组织策略中心或商业插件结算系统，除非后续单独立项。

#### 关键假设

- 短期插件以低风险声明式规则为主。
- 第三方代码插件必须运行在可隔离、可授权、可熔断的运行时中。
- 插件系统优先服务网关、日志、配置和 GUI 管理，不侵入各 CLI 内部实现。
- 插件系统必须同时兼容 macOS、Windows、Linux。

## 2. 总体里程碑

| 阶段 | 名称 | 目标 | 主要交付物 | 优先级 |
| --- | --- | --- | --- | --- |
| M0 | 架构冻结 | 明确插件边界、API 草案和非目标 | RFC、manifest v1、权限草案、hook 草案 | P0 |
| M1 | 插件基础设施 | 支持无代码插件的安装、状态、配置、审计 | DB schema、Rust service、IPC、GUI 管理页 | P0 |
| M2 | 网关 Hook Pipeline | 插件可在请求/响应链路中读取、修改或阻断数据 | hook pipeline、上下文模型、权限裁剪、测试夹具 | P0 |
| M3 | 官方示例插件 | 用真实场景验证插件 API | 提示词优化、安全检测、脱敏插件 | P0 |
| M4 | 市场与分发 | 支持市场、本地、GitHub release、离线安装 | 插件包格式、market index、签名、更新、回滚 | P1 |
| M5 | 安全运行时 | 支持社区代码插件 | WASM runtime、独立进程 runtime 草案、资源限制 | P1 |
| M6 | 开发生态 | 支持社区持续开发和发布 | SDK、脚手架、调试工具、文档、兼容测试 | P2 |

## 3. M0：架构冻结与 RFC

### 3.1 目标

在写代码前冻结插件系统的核心边界、插件包格式、权限模型、hook 位置和短中长期路线，避免后续出现“为了一个插件场景临时开洞”的架构漂移。

### 3.2 范围内

- 新增插件系统 RFC 文档。
- 定义 manifest v1。
- 定义插件状态机。
- 定义 hook 名称、触发时机和修改权限。
- 定义权限名称和风险分级。
- 定义短期、中期、长期运行时路线。

### 3.3 范围外

- 不实现任何运行时代码。
- 不实现市场。
- 不实现 GUI。
- 不引入 WASM runtime。

### 3.4 任务清单

#### M0-T01：新增插件系统 RFC

- 文件建议：`docs/plugin-system-rfc.md`。
- 内容要求：
  - 插件系统目标。
  - 插件系统非目标。
  - 当前架构可承载的位置。
  - 运行时路线。
  - 安全原则。
  - 跨平台原则。
  - 与 Skill 市场的边界。
- 验收标准：
  - 文档明确说明短期不执行任意 JS/TS。
  - 文档明确说明提示词优化只能在网关请求阶段可靠实现。
  - 文档明确说明第三方代码不得直接进入主进程或 WebView。

#### M0-T02：定义 `plugin.json` manifest v1

- 文件建议：`docs/plugin-manifest-v1.md`。
- 必填字段：
  - `id`
  - `name`
  - `version`
  - `apiVersion`
  - `runtime`
  - `hooks`
  - `permissions`
  - `hostCompatibility`
- 可选字段：
  - `entry`
  - `configSchema`
  - `description`
  - `author`
  - `homepage`
  - `repository`
  - `license`
  - `checksum`
  - `signature`
- 验收标准：
  - 至少给出 3 个示例 manifest：提示词优化、安全检测、脱敏。
  - 明确插件 ID 命名规则，例如 `publisher.plugin-name`。
  - 明确 SemVer 版本要求。

#### M0-T03：定义插件状态机

- 状态：
  - `available`
  - `installed`
  - `enabled`
  - `disabled`
  - `update_available`
  - `incompatible`
  - `quarantined`
  - `uninstalled`
- 需要定义：
  - 允许的状态转换。
  - 自动转换条件。
  - 用户操作触发条件。
  - 错误状态恢复策略。
- 验收标准：
  - 状态转换图清晰。
  - 升级失败、签名失败、运行时崩溃均有对应状态。

#### M0-T04：定义 hook v1

- 初始 hook：
  - `gateway.request.received`
  - `gateway.request.afterBodyRead`
  - `gateway.request.beforeProviderResolution`
  - `gateway.request.beforeSend`
  - `gateway.response.headers`
  - `gateway.response.chunk`
  - `gateway.response.after`
  - `gateway.error`
  - `log.beforePersist`
- 每个 hook 必须定义：
  - 触发时机。
  - 可访问上下文。
  - 是否允许修改。
  - 默认超时。
  - 默认失败策略。
  - 可申请权限。
- 验收标准：
  - 请求类、响应类、流式响应类、日志类 hook 都覆盖。
  - 明确哪些 hook 支持修改 body。
  - 明确流式 hook 不提供无限完整响应。

#### M0-T05：定义权限模型 v1

- 初始权限：
  - `request.meta.read`
  - `request.header.read`
  - `request.header.readSensitive`
  - `request.header.write`
  - `request.body.read`
  - `request.body.write`
  - `response.header.read`
  - `response.header.write`
  - `response.body.read`
  - `response.body.write`
  - `stream.inspect`
  - `stream.modify`
  - `log.redact`
  - `plugin.storage`
  - `network.fetch`
  - `file.read`
  - `file.write`
  - `secret.read`
- 验收标准：
  - 每个权限有风险等级。
  - 高危权限需要二次授权。
  - 插件升级新增权限必须重新授权。

## 4. M1：插件基础设施

### 4.1 目标

建立插件系统的持久化、服务层、IPC 和 GUI 管理基础，使用户可以安装、查看、配置、启用、禁用和卸载无代码插件。

### 4.2 范围内

- 插件目录管理。
- SQLite 表结构。
- Rust domain/service/repository。
- Tauri commands。
- 前端插件列表页。
- 前端插件详情与配置页。
- 配置 schema 子集渲染。
- 审计日志基础能力。

### 4.3 范围外

- 不接入网关 hook。
- 不执行第三方代码。
- 不实现插件市场远程索引。
- 不引入 WASM。

### 4.4 后端任务

#### M1-T01：新增插件目录管理

- 涉及模块：
  - `src-tauri/src/infra/app_paths.rs`
- 新增路径：
  - `~/.aio-coding-hub/plugins/installed`
  - `~/.aio-coding-hub/plugins/cache`
  - `~/.aio-coding-hub/plugins/data`
  - `~/.aio-coding-hub/plugins/logs`
- 要求：
  - 路径必须跨平台。
  - 不拼接硬编码 `/`。
  - 所有插件文件操作必须限制在插件目录下。
- 验收标准：
  - macOS、Windows、Linux 下能得到合法路径。
  - 单测覆盖插件 ID 中非法路径片段，例如 `../`。

#### M1-T02：新增 SQLite schema

- 涉及模块：
  - `src-tauri/src/infra/db/migrations`
  - `src-tauri/src/infra/db/mod.rs`
- 建议表：
  - `plugins`
  - `plugin_versions`
  - `plugin_configs`
  - `plugin_permissions`
  - `plugin_audit_logs`
  - `plugin_market_sources`
  - `plugin_runtime_failures`
- 字段要求：
  - 插件 ID。
  - 当前版本。
  - 安装来源。
  - 状态。
  - manifest JSON。
  - 配置 JSON。
  - 授权权限 JSON。
  - 最近错误。
  - 创建和更新时间。
- 验收标准：
  - 迁移可从现有 latest schema 平滑升级。
  - 重复运行 ensure 不破坏已有数据。
  - SQLite schema 有回归测试。

#### M1-T03：新增插件 domain 类型

- 建议模块：
  - `src-tauri/src/domain/plugins.rs`
- 类型：
  - `PluginId`
  - `PluginManifest`
  - `PluginRuntime`
  - `PluginHook`
  - `PluginPermission`
  - `PluginStatus`
  - `PluginInstallSource`
  - `PluginConfigSchema`
  - `PluginAuditEvent`
- 要求：
  - 所有类型可序列化。
  - 与 Specta/Tauri IPC 类型生成兼容。
  - manifest 校验错误可结构化返回给前端。
- 验收标准：
  - manifest JSON 可反序列化。
  - 非法 runtime、非法 hook、非法权限能返回明确错误。

#### M1-T04：实现 manifest 校验器

- 校验内容：
  - ID 格式。
  - SemVer。
  - API version。
  - runtime 是否支持。
  - hook 是否已知。
  - 权限是否已知。
  - hook 与权限是否匹配。
  - config schema 是否可解析。
  - hostCompatibility 是否满足当前宿主版本。
- 验收标准：
  - 覆盖合法 manifest。
  - 覆盖缺字段 manifest。
  - 覆盖不兼容版本。
  - 覆盖声明未知权限。

#### M1-T05：实现插件 repository

- 建议模块：
  - `src-tauri/src/infra/plugins/repository.rs`
- 方法：
  - `list_plugins`
  - `get_plugin`
  - `insert_plugin`
  - `update_plugin_status`
  - `save_plugin_config`
  - `save_plugin_permissions`
  - `append_audit_log`
  - `record_runtime_failure`
- 验收标准：
  - CRUD 单测通过。
  - 数据库错误被映射为业务错误。

#### M1-T06：实现插件 service

- 建议模块：
  - `src-tauri/src/app/plugin_service.rs`
- 方法：
  - `list_plugins`
  - `get_plugin_detail`
  - `install_plugin_from_local_package`
  - `enable_plugin`
  - `disable_plugin`
  - `uninstall_plugin`
  - `save_plugin_config`
  - `grant_plugin_permissions`
  - `revoke_plugin_permission`
- 验收标准：
  - 启用插件前必须校验 manifest、配置和权限。
  - 禁用插件不删除配置。
  - 卸载插件默认保留审计记录。

#### M1-T07：新增 Tauri commands

- 涉及模块：
  - `src-tauri/src/commands/registry.rs`
  - 建议新增 `src-tauri/src/commands/plugins.rs`
- 命令：
  - `plugin_list`
  - `plugin_get`
  - `plugin_install_from_file`
  - `plugin_enable`
  - `plugin_disable`
  - `plugin_uninstall`
  - `plugin_save_config`
  - `plugin_grant_permissions`
  - `plugin_list_audit_logs`
- 验收标准：
  - 前端可通过生成 IPC 类型调用。
  - 命令参数和返回值有类型导出。

### 4.5 前端任务

#### M1-T08：新增插件服务封装

- 建议模块：
  - `src/services/plugins.ts`
  - `src/query/plugins.ts`
- 内容：
  - 列表查询。
  - 详情查询。
  - 启用/禁用 mutation。
  - 卸载 mutation。
  - 保存配置 mutation。
- 验收标准：
  - React Query key 命名稳定。
  - mutation 后插件列表和详情自动刷新。

#### M1-T09：新增插件列表页

- 建议文件：
  - `src/pages/PluginsPage.tsx`
- 展示字段：
  - 名称。
  - ID。
  - 当前版本。
  - 状态。
  - runtime。
  - 权限风险等级。
  - 是否可更新。
  - 最近错误。
- 操作：
  - 查看详情。
  - 启用。
  - 禁用。
  - 卸载。
  - 本地导入。
- 验收标准：
  - 空状态清晰。
  - loading、error、disabled 状态齐全。
  - 操作失败展示后端错误。

#### M1-T10：新增插件详情和权限展示

- 展示：
  - manifest 基本信息。
  - hooks。
  - permissions。
  - hostCompatibility。
  - 安装来源。
  - 审计日志摘要。
- 要求：
  - 高危权限使用明显提示。
  - 新增权限需要单独确认。
- 验收标准：
  - 用户能理解插件会读取或修改哪些数据。
  - 未授权权限不能启用插件。

#### M1-T11：实现配置 schema 表单

- 支持 JSON Schema 子集：
  - string。
  - number。
  - integer。
  - boolean。
  - enum。
  - array。
  - object。
  - password。
- 限制：
  - 不支持任意自定义前端组件。
  - 不支持插件提供 GUI 代码。
- 验收标准：
  - 前端校验必填、类型、枚举。
  - 后端保存前再次校验。
  - 敏感字段不回显明文。

## 5. M2：网关 Hook Pipeline

### 5.1 目标

在当前 Axum 网关链路中加入稳定的插件 hook pipeline，让插件可以在受权限控制的前提下读取、修改或阻断请求和响应。

### 5.2 范围内

- Hook pipeline。
- Hook 上下文。
- 权限裁剪。
- 超时策略。
- 错误隔离。
- 非流式响应处理。
- 流式 chunk 处理。
- 日志落库前脱敏 hook。

### 5.3 范围外

- 不开放第三方代码运行。
- 不支持插件直接访问数据库。
- 不支持插件自行发起网络请求。
- 不允许插件直接修改 provider 配置。

### 5.4 任务清单

#### M2-T01：设计 hook context 数据结构

- 建议模块：
  - `src-tauri/src/gateway/plugins/context.rs`
- 上下文类型：
  - `PluginRequestMeta`
  - `PluginRequestHeaders`
  - `PluginRequestBody`
  - `PluginResponseMeta`
  - `PluginResponseHeaders`
  - `PluginResponseBody`
  - `PluginStreamChunk`
  - `PluginTraceContext`
- 要求：
  - 敏感 header 默认不注入。
  - body 根据权限注入。
  - context 不暴露内部 Rust 类型引用。
- 验收标准：
  - 没有权限时插件看不到 body。
  - 没有敏感权限时插件看不到 Authorization。

#### M2-T02：实现权限裁剪器

- 建议模块：
  - `src-tauri/src/gateway/plugins/permissions.rs`
- 功能：
  - 按插件授权权限裁剪上下文。
  - 按 hook 类型限制可用权限。
  - 阻止插件返回未授权修改。
- 验收标准：
  - 未授权 `request.body.write` 的插件不能修改请求 body。
  - 未授权 `stream.modify` 的插件不能阻断流。

#### M2-T03：实现 hook pipeline 执行器

- 建议模块：
  - `src-tauri/src/gateway/plugins/pipeline.rs`
- 功能：
  - 按 hook 查找启用插件。
  - 按优先级排序。
  - 串行执行。
  - 记录每个插件耗时。
  - 捕获错误。
  - 执行超时。
- 默认超时：
  - 请求元数据 hook：50ms。
  - 请求 body hook：200ms。
  - 发送前 hook：300ms。
  - 响应 header hook：100ms。
  - 流式 chunk hook：20ms。
  - 完整响应 hook：300ms。
  - 日志 hook：100ms。
- 验收标准：
  - 单个插件超时不会拖垮整个请求。
  - 插件错误会进入审计日志。
  - 可配置 fail-open/fail-closed。

#### M2-T04：接入请求 body hook

- 接入位置：
  - `BodyReaderMiddleware` 后。
  - `gateway.request.afterBodyRead` 默认放在 `BodyReaderMiddleware` 后、`ModelInferenceMiddleware` 前。
  - `gateway.request.beforeProviderResolution` 必须放在 provider resolution 发生前，具体位置由 M2-T04 输出的最终链路图冻结。
  - 不允许在未记录、未测试的位置临时插入请求 body hook。
- Hook：
  - `gateway.request.afterBodyRead`
  - `gateway.request.beforeProviderResolution`
- 要求：
  - 支持修改 JSON body。
  - 支持保持原始 body bytes。
  - 修改后更新 content-length 或移除 content-length。
- 验收标准：
  - M2-T04 必须产出最终网关 hook 链路图，标明每个 hook 在现有 middleware/forwarder/response router 中的精确位置。
  - 插件可修改 OpenAI-compatible `messages`。
  - 修改后请求能正常转发。
  - 非 JSON body 不被错误破坏。

#### M2-T05：接入发送前 hook

- 接入位置：
  - `src-tauri/src/gateway/proxy/handler/attempt/send.rs`
- Hook：
  - `gateway.request.beforeSend`
- 要求：
  - 可读取 provider/upstream 摘要。
  - 可修改 header。
  - 可修改 body。
  - 高危修改写入审计日志。
- 验收标准：
  - header 注入和删除可测试。
  - body 修改后上游请求成功。

#### M2-T06：接入响应 header hook

- 接入位置：
  - `response_router.rs`
- Hook：
  - `gateway.response.headers`
- 要求：
  - 可读取 status、headers、content-type。
  - 可修改返回给 CLI 的非敏感 headers。
- 验收标准：
  - 插件可追加安全提示 header。
  - 不允许伪造内部 tracing header。

#### M2-T07：接入非流式响应 hook

- 接入位置：
  - `success_non_stream.rs`
- Hook：
  - `gateway.response.after`
- 要求：
  - 小响应可完整 body transform。
  - 大响应不得强制完整缓冲。
  - 超限响应只提供摘要或跳过 body。
- 验收标准：
  - 非流式 JSON 响应可被安全插件阻断。
  - 大响应不会造成内存激增。

#### M2-T08：接入流式响应 hook

- 接入位置：
  - `success_event_stream.rs`
  - `src-tauri/src/gateway/streams/usage_tee.rs`
- Hook：
  - `gateway.response.chunk`
- 要求：
  - 提供 chunk。
  - 提供固定大小滑动窗口。
  - 支持返回 pass、replace、block、warn。
  - 支持跨 chunk 敏感信息检测。
- 验收标准：
  - SSE 输出可被检测。
  - 跨 chunk token 可被识别。
  - 插件阻断后 CLI 收到合理错误事件。

#### M2-T09：接入错误 hook

- Hook：
  - `gateway.error`
- 要求：
  - 插件可收到错误类型、trace_id、provider、attempt。
  - 插件不能隐藏宿主错误。
- 验收标准：
  - 上游错误和插件错误都能被审计。

#### M2-T10：接入日志落库前 hook

- Hook：
  - `log.beforePersist`
- 要求：
  - 插件可脱敏 request/response/log fields。
  - 日志 hook 失败时使用宿主默认脱敏。
- 验收标准：
  - token 不以明文进入请求日志。
  - 脱敏插件异常时仍有默认兜底。

## 6. M3：官方示例插件

### 6.1 目标

用三个官方插件验证插件系统是否真正覆盖社区扩展场景。

### 6.2 范围内

- 三个官方插件均以声明式规则插件形式实现。
- 三个插件均可安装、配置、启用、禁用、卸载。
- 三个插件均有测试。

### 6.3 范围外

- 不引入第三方代码执行。
- 不支持插件自定义 GUI。
- 不支持插件联网更新规则。

### 6.4 任务清单

#### M3-T01：实现声明式规则 runtime

- 建议模块：
  - `src-tauri/src/app/plugins/rule_runtime.rs`
- 能力：
  - JSON path 定位。
  - regex 检测。
  - replace。
  - block。
  - warn。
  - append system/developer message。
- 限制：
  - 正则必须有长度限制和超时保护。
  - 不支持任意脚本表达式。
- 验收标准：
  - 规则插件无代码执行能力。
  - 恶意复杂正则不会卡死请求。

#### M3-T02：提示词优化插件

- Hook：
  - `gateway.request.afterBodyRead`
- 权限：
  - `request.body.read`
  - `request.body.write`
- 配置：
  - 优化模式：`append_instruction`、`rewrite_system_message`、`prepend_context`。
  - 是否只作用于指定模型。
  - 是否只作用于指定 CLI。
- 验收标准：
  - 能修改 `messages`。
  - 能修改 `input`。
  - 能修改 `prompt`。
  - 审计日志记录修改摘要。
  - 用户可关闭插件恢复原始请求。

#### M3-T03：安全检测插件

- Hook：
  - `gateway.response.chunk`
  - `gateway.response.after`
- 权限：
  - `response.body.read`
  - `stream.inspect`
  - 可选 `stream.modify`
- 配置：
  - 策略：`warn`、`block`、`redact`。
  - 检测类别：危险 shell、密钥泄露、数据外传、破坏性文件操作。
  - 阻断提示模板。
- 验收标准：
  - 非流式响应可检测。
  - 流式响应可检测。
  - 命中高危规则时按配置处理。
  - 高危命中写入审计日志。

#### M3-T04：脱敏插件

- Hook：
  - `gateway.request.beforeSend`
  - `gateway.response.chunk`
  - `gateway.response.after`
  - `log.beforePersist`
- 权限：
  - `request.body.read`
  - 可选 `request.body.write`
  - `response.body.read`
  - 可选 `response.body.write`
  - `log.redact`
- 配置：
  - 默认只脱敏日志和 GUI 展示。
  - 可显式开启发送上游前脱敏。
  - 可配置敏感类型。
  - 可配置保留前后字符数量。
- 验收标准：
  - Bearer token 被脱敏。
  - GitHub token 被脱敏。
  - URL query token 被脱敏。
  - 数据库连接串被脱敏。
  - 默认不破坏发送给上游的真实请求。

## 7. M4：插件安装、市场与更新

### 7.1 目标

提供可治理的插件分发机制，支持插件市场、本地导入、GitHub release 安装和离线安装。

### 7.2 范围内

- `.aio-plugin` 包格式。
- 插件市场 index。
- checksum 校验。
- 签名校验。
- 安装来源记录。
- 更新检测。
- 回滚。
- 撤销和隔离。

### 7.3 范围外

- 不做商业结算。
- 不做完整审核后台。
- 不支持直接执行 GitHub 仓库源码。

### 7.4 任务清单

#### M4-T01：定义 `.aio-plugin` 包格式

- 包内容：
  - `plugin.json`
  - `config.schema.json`
  - `README.md`
  - `rules/*.json`
  - 后续可选 `plugin.wasm`
- 要求：
  - 解压前检查包大小。
  - 解压时防止 zip slip。
  - 解压后再次校验 manifest。
- 验收标准：
  - 恶意路径文件无法写出插件目录。
  - 缺少 manifest 的包无法安装。

#### M4-T02：实现本地导入安装

- 流程：
  - 选择文件。
  - 复制到 cache。
  - 校验 checksum。
  - 解压到临时目录。
  - 校验 manifest。
  - 写入 DB。
  - 移动到 installed。
- 验收标准：
  - 合法包可安装。
  - 非法包安装失败且不留下半成品目录。

#### M4-T03：实现 market index

- index 字段：
  - plugin id。
  - versions。
  - download URL。
  - checksum。
  - signature。
  - compatibility。
  - risk labels。
  - revoked 标记。
- 验收标准：
  - 可列出市场插件。
  - 可识别可更新插件。
  - 不兼容插件不允许安装。

#### M4-T04：实现签名校验

- 签名算法：
  - Ed25519。
- 校验对象：
  - 插件包 checksum。
  - market index。
- 验收标准：
  - 签名错误无法安装。
  - checksum 不匹配无法安装。
  - revoked 插件自动隔离。

#### M4-T05：实现 GitHub release 安装

- 限制：
  - 只支持 release artifact。
  - 必须提供 checksum。
  - 不支持直接 clone 仓库源码并执行。
- 可借鉴模块：
  - 当前 Skill 市场的 repo cache 逻辑。
- 验收标准：
  - 能从固定 release 下载 `.aio-plugin`。
  - 下载失败、大小超限、checksum 不匹配均能失败回滚。

#### M4-T06：实现离线安装

- 流程：
  - 用户导入 `.aio-plugin`。
  - 用户导入或内置可信公钥。
  - 本地校验签名。
- 无签名包边界：
  - 仅允许在开发者模式或显式开启高风险安装开关时导入。
  - 默认安装后保持 `disabled`，不得自动启用。
  - 不允许申请 `request.header.readSensitive`、`network.fetch`、`file.read`、`file.write`、`secret.read` 等高危权限。
  - 不接入插件市场更新链路。
  - GUI 必须持续显示“未签名”风险标记。
- 验收标准：
  - 无网络情况下可安装合法包。
  - 无签名包必须显示高风险确认，默认不推荐启用。
  - 无签名包申请高危权限时安装失败。

#### M4-T07：实现更新与回滚

- 要求：
  - 更新前保存旧版本。
  - 新版本新增权限时必须重新授权。
  - 配置迁移失败自动回滚。
  - 插件运行失败可自动回滚到旧版本。
- 验收标准：
  - 更新失败不破坏当前可用版本。
  - 用户可手动回滚。

## 8. M5：安全插件运行时

### 8.1 目标

在权限和沙箱边界明确后，支持社区代码插件。

### 8.2 范围内

- WASM runtime。
- WASI capability 限制。
- 内存和时间限制。
- 独立进程插件技术预研。
- JSON-RPC 协议草案。

### 8.3 范围外

- 不默认支持 Node/Deno。
- 不支持主进程原生动态库插件。
- 不支持插件自带长期后台服务默认自启动。

### 8.4 任务清单

#### M5-T01：引入 WASM runtime

- 候选：
  - Wasmtime。
- 要求：
  - 限制内存。
  - 限制执行时间。
  - 禁用默认文件系统。
  - 禁用默认网络。
  - 只通过宿主导入函数访问授权能力。
- 验收标准：
  - WASM 插件可处理请求 body。
  - WASM 插件不能读取任意文件。
  - 死循环 WASM 会被终止。

#### M5-T02：定义 WASM ABI

- 内容：
  - hook 输入格式。
  - hook 输出格式。
  - 错误格式。
  - 日志接口。
  - 配置读取接口。
- 验收标准：
  - Rust 和 TypeScript SDK 能生成兼容插件。
  - ABI 有版本号。

#### M5-T03：实现 WASM 插件执行器

- 功能：
  - 加载 wasm。
  - 校验 manifest runtime。
  - 注入裁剪后的 context。
  - 执行 hook。
  - 解析返回值。
  - 记录耗时和错误。
- 验收标准：
  - 多插件串行执行稳定。
  - 单插件崩溃不影响其他插件。

#### M5-T04：独立进程插件预研和协议草案

- 边界：
  - 独立进程仅表示由宿主管理的短生命周期 worker 或按需复用 worker。
  - 独立进程不得注册为系统 daemon、登录项、计划任务或脱离宿主生命周期的后台服务。
  - 宿主必须能启动、超时终止、空闲回收和隔离该进程。
  - 该 PoC 不等于开放插件后台常驻任务。
- 协议：
  - JSON-RPC over stdio。
  - 每次请求带 trace_id。
  - 插件返回结构化 action。
- 进程控制：
  - 启动超时。
  - hook 超时。
  - 空闲回收。
  - 崩溃重启限制。
- 验收标准：
  - 形成设计文档。
  - 有最小 PoC，但不默认开放市场插件使用。
  - PoC 进程退出、超时、崩溃和空闲回收行为均可复现。

## 9. M6：SDK、脚手架、调试与文档

### 9.1 目标

让社区开发者能够稳定开发、测试、调试、打包和发布插件。

### 9.2 范围内

- SDK。
- 类型定义。
- 脚手架。
- 示例插件。
- 本地调试工具。
- 插件测试工具。
- 开发者文档。

### 9.3 范围外

- 不做插件商业化平台。
- 不提供插件托管计算服务。

### 9.4 任务清单

#### M6-T01：TypeScript SDK

- 包名建议：
  - `@aio-coding-hub/plugin-sdk`
- 内容：
  - manifest 类型。
  - hook context 类型。
  - hook result 类型。
  - 权限常量。
  - 测试 helper。
- 验收标准：
  - 示例插件可使用 SDK 类型通过编译。

#### M6-T02：Rust/WASM SDK

- 内容：
  - ABI 类型。
  - serde 类型。
  - helper macro。
  - 测试 fixture。
- 验收标准：
  - Rust 示例可编译为 WASM 并被宿主加载。

#### M6-T03：插件脚手架

- 命令建议：
  - `create-aio-plugin`
- 模板：
  - 声明式规则插件。
  - WASM prompt 插件。
  - WASM safety 插件。
  - WASM redactor 插件。
- 验收标准：
  - 用户能一条命令生成插件项目。
  - 生成项目包含测试和打包脚本。

#### M6-T04：本地开发模式

- 命令建议：
  - `aio plugin dev ./plugin`
- 能力：
  - manifest 校验。
  - fixture 回放。
  - hook 调试日志。
  - 权限模拟。
  - 配置模拟。
- 验收标准：
  - 开发者不安装插件也能本地调试 hook。

#### M6-T05：插件打包和发布工具

- 命令建议：
  - `aio plugin pack`
  - `aio plugin sign`
  - `aio plugin verify`
- 验收标准：
  - 可生成 `.aio-plugin`。
  - 可生成 checksum。
  - 可签名。
  - 可本地验证。

#### M6-T06：开发者文档

- 文档目录建议：
  - `docs/plugins/README.md`
  - `docs/plugins/developer-guide.md`
  - `docs/plugins/examples/privacy-filter.md`
  - `docs/plugins/reference/README.md`
  - `docs/plugins/reference/manifest.md`
  - `docs/plugins/reference/hooks.md`
  - `docs/plugins/reference/permissions.md`
  - `docs/plugins/reference/config-schema.md`
  - `docs/plugins/reference/declarative-rules.md`
  - `docs/plugins/reference/publishing.md`
  - `docs/plugins/reference/compatibility.md`
  - `docs/plugins/runtime/README.md`
  - `docs/plugins/runtime/streaming.md`
  - `docs/plugins/architecture/security.md`
- 验收标准：
  - 第三方开发者能按文档完成一个插件从开发到发布。

## 10. 测试计划

### 10.1 单元测试

- manifest 校验。
- 权限裁剪。
- 插件状态机。
- config schema 校验。
- hook pipeline 排序。
- hook 超时。
- hook 错误处理。
- 正则规则超时。
- 路径安全。

### 10.2 集成测试

- 本地插件安装、启用、禁用、卸载。
- 请求 body 修改。
- header 修改。
- 非流式响应修改。
- SSE chunk 检测。
- 日志落库前脱敏。
- 插件崩溃隔离。

### 10.3 端到端测试

- GUI 安装本地插件。
- GUI 配置插件。
- GUI 授权权限。
- 发起真实网关请求。
- 验证请求或响应被插件处理。
- 查看审计日志。

### 10.4 兼容性测试

- 插件 API v1 fixture。
- 低版本宿主不兼容插件。
- 高版本插件 API 被拒绝。
- 插件升级新增权限。
- 插件配置迁移。

### 10.5 跨平台测试

- macOS Intel：安装、启用、禁用、卸载本地插件，验证插件路径在用户数据目录下。
- macOS Apple Silicon：重复 macOS Intel 用例，并验证 WASM 插件在后续阶段不会依赖架构专属二进制。
- Windows x64：验证插件目录创建、路径分隔符、长路径、文件锁、更新时旧版本目录占用、卸载时进程占用。
- Linux x64：验证 AppImage、deb/rpm 安装形态下的数据目录、权限和插件缓存目录。
- Windows 文件锁：插件运行时占用文件时执行升级，预期失败回滚且旧版本继续可用。
- macOS 签名和 quarantine：下载插件包后校验签名，不依赖执行未签名原生二进制。
- Linux 不同安装包形态：确认插件目录不写入只读安装目录。

### 10.6 任务级测试矩阵

| 任务范围 | 最低验证命令 | 必备 fixture | 关键断言 |
| --- | --- | --- | --- |
| manifest/domain/repository | `cargo test plugins` | 合法 manifest、缺字段 manifest、未知权限 manifest、不兼容版本 manifest | 错误结构化返回，不写入非法插件 |
| DB migration | `cargo test db` | 旧版本空库、已有 provider/prompts/logs 的库 | 迁移可重复执行，不破坏既有数据 |
| Tauri commands | `cargo test commands` | 插件安装包、非法插件包 | IPC 返回类型稳定，失败错误可被 GUI 展示 |
| 前端插件列表 | `pnpm test` | 空列表、已安装、可更新、隔离状态 | loading/error/empty/normal 状态可渲染 |
| 配置 schema 表单 | `pnpm test` | string、boolean、enum、array、password schema | 前端校验与后端校验结果一致 |
| 请求 hook | `cargo test gateway_plugin_request` | OpenAI-compatible body、非 JSON body、大 body | body 修改正确，非 JSON 不破坏，content-length 正确处理 |
| 响应 hook | `cargo test gateway_plugin_response` | 非流式 JSON、小 body、大 body | 小响应可 transform，大响应不强制缓冲 |
| 流式 hook | `cargo test gateway_plugin_stream` | SSE chunk、跨 chunk token、阻断事件 | chunk 可检查，跨 chunk 可识别，阻断事件格式稳定 |
| 日志脱敏 | `cargo test plugin_log_redaction` | Authorization、Bearer token、URL token、数据库连接串 | 敏感值不落库，脱敏插件失败时宿主兜底 |
| 安装包安全 | `cargo test plugin_package_security` | zip slip 包、超大包、checksum 错误包、签名错误包 | 安装失败且不留下半成品目录 |
| WASM runtime | `cargo test plugin_wasm` | 合法 wasm、死循环 wasm、越权读文件 wasm | 超时终止，越权失败，主进程不崩溃 |
| GUI E2E | `pnpm test:e2e` 或项目既有 E2E 命令 | 本地插件包、权限授权流程 | 用户能完成安装、配置、启用、请求验证、卸载 |

### 10.7 测试闭环要求

- 每个 `Mx-Txx` 任务完成前必须在任务说明中写明对应测试命令。
- 如果任务级测试矩阵中的最低验证命令尚不存在，该任务必须先新增测试 target，或明确映射到项目现有命令；不得把矩阵命令作为不可执行占位符保留。
- P0/P1 任务完成前必须说明对应测试命令运行在哪个 CI job 中；如暂未接入 CI，必须创建后续 CI 接入任务并标记阻塞发布。
- 如果某任务暂时无法自动化测试，必须记录手工验证步骤和后续自动化补齐任务。
- P0 阶段不得只依赖手工测试完成验收。
- 每个 bug 修复必须补一个回归 fixture 或回归测试。
- 插件安全、权限、签名、路径穿越、流式阻断相关测试必须进入 CI。

## 11. 性能与稳定性计划

### 11.1 性能预算

- 插件系统不得显著增加应用冷启动时间。
- 插件列表和 manifest 可懒加载。
- 请求元数据 hook 默认预算 50ms。
- 请求 body hook 默认预算 200ms。
- 发送前 hook 默认预算 300ms。
- 流式 chunk hook 默认预算 20ms。
- 完整响应 hook 默认预算 300ms。

### 11.2 降级策略

- 装饰类插件默认 fail-open。
- 安全类插件可配置 fail-closed。
- 插件连续超时后自动熔断。
- 插件连续崩溃后进入 `quarantined`。
- 插件被隔离后 GUI 显示原因和恢复入口。

### 11.2.1 安全失败策略矩阵

| Hook | 默认失败策略 | 可否配置 | 失败时行为 | 必测场景 |
| --- | --- | --- | --- | --- |
| `gateway.request.received` | fail-open | 是 | 跳过插件，记录审计 | 插件超时不影响请求进入网关 |
| `gateway.request.afterBodyRead` | fail-open | 是 | 使用未修改请求继续，记录审计 | 提示词插件失败后原请求仍可发送 |
| `gateway.request.beforeProviderResolution` | fail-open | 是 | 忽略路由建议 | 插件失败不影响 provider 解析 |
| `gateway.request.beforeSend` | fail-open | 安全插件可设 fail-closed | 装饰类继续发送，安全类可阻断 | 发送前安全插件失败时按配置阻断 |
| `gateway.response.headers` | fail-open | 是 | 返回原始 headers | header 插件失败不破坏响应 |
| `gateway.response.chunk` | 安全类 fail-closed，非安全类 fail-open | 是 | 安全插件失败时终止流或降级为安全提示事件，不默认继续输出原始 chunk | 流式安全检测超时不会泄漏已命中高危内容 |
| `gateway.response.after` | 安全类 fail-closed，非安全类 fail-open | 是 | 安全插件失败时默认返回安全错误，不默认返回原始响应 | 非流式安全插件失败行为稳定 |
| `gateway.error` | fail-open | 否 | 不隐藏原始错误 | 错误 hook 失败仍返回宿主错误 |
| `log.beforePersist` | fail-closed-to-host-redaction | 否 | 插件失败时使用宿主内置脱敏后再落库 | 脱敏插件崩溃时 token 不落库 |

安全类插件必须在 manifest 中声明 `category: "security"` 或由官方/市场审核标记。用户可显式把安全插件降级为 fail-open，但 GUI 必须显示高风险提示，且每次降级生效都要写入审计日志。

### 11.3 可观测性

- 每次请求带 trace_id。
- 记录每个插件 hook 耗时。
- 记录插件修改摘要。
- 记录插件阻断事件。
- 记录插件错误和超时。
- 日志默认脱敏。

## 12. 安全计划

### 12.1 供应链安全

- 插件包 checksum 校验。
- 插件包签名校验。
- market index 签名校验。
- 支持 revoked 列表。
- 支持 quarantined 状态。

### 12.2 运行时安全

- 短期只支持声明式规则。
- 中期 WASM 必须禁用默认文件系统和网络。
- 独立进程插件必须最小环境变量启动。
- 不允许插件读取未授权请求体。
- 不允许插件读取未授权敏感 header。

### 12.3 数据安全

- 敏感配置不明文回显。
- 审计日志不保存敏感原文。
- token、Authorization、cookie 默认遮蔽。
- 插件私有数据隔离到 `plugins/data/<plugin-id>`。

## 13. 发布与迁移计划

### 13.1 Feature flag

- 插件系统初期应有全局 feature flag。
- 开发版默认开启。
- 稳定版可先隐藏市场入口，仅开放本地导入官方插件。

### 13.2 数据迁移

- 插件 DB migration 必须可重复执行。
- 插件配置 schema 变更必须提供迁移。
- 插件升级前保存配置快照。

### 13.2.1 插件配置迁移格式

- 插件 manifest 必须声明 `configVersion`。
- 插件升级如改变配置结构，必须提供从旧版本到新版本的迁移描述。
- 声明式插件只允许声明式迁移：
  - rename field。
  - set default。
  - remove field。
  - enum value mapping。
  - split string to array。
- WASM 插件后续可提供迁移函数，但必须在同一沙箱和超时限制中执行。
- 迁移前宿主保存配置快照。
- 迁移后宿主再次执行 schema 校验。
- 迁移失败时恢复旧配置、旧插件版本和旧启用状态。

### 13.2.2 插件 API 兼容策略

- 插件 API 版本使用独立于应用版本的 `apiVersion`。
- 同一 major API 内只能做向后兼容扩展。
- 删除字段或改变字段语义必须进入下一 major API。
- 宿主至少保留一个旧 major API 的只读兼容期，除非存在安全漏洞。
- 不兼容插件启动时自动标记为 `incompatible`，不得参与 hook pipeline。
- 市场 index 必须能标记 deprecated、vulnerable、revoked。

### 13.3 回滚

- 应用回滚后，不兼容插件自动禁用。
- 插件升级失败回滚旧版本。
- 插件启用失败不改变原状态。

### 13.4 阶段闸门与依赖关系

| 阶段 | 进入条件 | 退出条件 | 阻塞条件 | 可并行任务 |
| --- | --- | --- | --- | --- |
| M0 | 完成当前架构调研，确认插件目标和非目标 | RFC、manifest v1、hook v1、权限 v1 审核通过 | 无法确认 hook 边界或安全边界 | 无 |
| M1 | M0 文档冻结 | 本地无代码插件可安装、配置、启用、禁用、卸载，状态持久化 | DB schema 不稳定、IPC 类型无法生成 | 前端页面与后端 repository 可并行 |
| M2 | M1 插件状态和权限可用 | 请求、非流式响应、流式响应、日志 hook 均有测试 | 权限裁剪未完成、流式 hook 无测试 | 请求 hook 与日志 hook 可部分并行 |
| M3 | M2 hook pipeline 通过集成测试 | 三个官方插件均可安装、配置、运行、卸载 | 任一官方插件需要绕过权限模型 | 三个官方插件可并行，但共享规则 runtime |
| M4 | M1 安装基础稳定，M3 至少一个官方插件稳定 | 市场、本地、GitHub release、离线安装策略可用，签名和回滚通过测试 | 签名校验未完成、包安全未完成 | market index 与本地包格式可并行 |
| M5 | M2 权限裁剪稳定，M4 包格式支持 runtime artifact | WASM 插件可安全执行，越权/死循环测试通过 | WASM 无法限制文件/网络/时间/内存 | WASM ABI 与 SDK 可并行 |
| M6 | M3/M5 API 基本稳定 | SDK、脚手架、文档、调试工具可支撑第三方开发 | API 仍频繁破坏性变更 | 文档、SDK、示例可并行 |

M3 不得在 M2 的 `gateway.response.chunk` 流式测试完成前声明完成。M4 不得在签名校验完成前开放远程安装。M5 不得在权限裁剪和运行时资源限制通过测试前开放社区插件。

### 13.5 发布闭环

- 每个阶段完成后必须更新本计划或对应 Trellis 任务，记录实际偏差。
- 每个 P0 阶段必须有可复现测试命令。
- 每次发布前必须跑插件安全测试、路径安全测试、签名测试和日志脱敏测试。
- 发布后如发现插件安全漏洞，应通过 market revoked 列表隔离插件，并在 GUI 中提示用户。

## 14. 任务拆分建议

### 第一批任务

1. `plugin-system-rfc`
2. `plugin-manifest-v1`
3. `plugin-db-schema`
4. `plugin-domain-types`
5. `plugin-local-install`
6. `plugin-gui-management`

### 第二批任务

1. `plugin-hook-context`
2. `plugin-permission-enforcement`
3. `gateway-request-hooks`
4. `gateway-response-hooks`
5. `gateway-stream-hooks`
6. `log-redaction-hook`

### 第三批任务

1. `rule-plugin-runtime`
2. `official-prompt-optimizer-plugin`
3. `official-safety-detector-plugin`
4. `official-redactor-plugin`
5. `plugin-audit-log-ui`

### 第四批任务

1. `plugin-package-format`
2. `plugin-market-index`
3. `plugin-signature-verification`
4. `plugin-update-rollback`
5. `github-release-plugin-install`
6. `offline-plugin-install`

### 第五批任务

1. `wasm-runtime-design`
2. `wasm-runtime-host`
3. `wasm-plugin-abi`
4. `wasm-plugin-sdk`
5. `process-plugin-protocol-poc`

### 第六批任务

1. `plugin-typescript-sdk`
2. `plugin-scaffolder`
3. `plugin-dev-mode`
4. `plugin-pack-sign-verify-cli`
5. `plugin-developer-docs`
6. `plugin-compatibility-fixtures`

## 15. 总体验收标准

插件系统第一版完成时，应满足：

1. 用户可以在 GUI 中查看、安装、配置、启用、禁用和卸载插件。
2. 插件状态和版本信息持久化在 SQLite 中。
3. 插件配置可由 schema 渲染和校验。
4. 插件权限需要用户授权，未授权数据不会暴露给插件。
5. 插件可以在网关请求发送前修改请求 body 和 header。
6. 插件可以检测非流式响应。
7. 插件可以检测流式响应 chunk。
8. 插件可以在日志落库前脱敏。
9. 插件错误、超时和崩溃不会导致主应用崩溃。
10. 官方提示词优化、安全检测、脱敏三个插件可作为示例稳定运行。
11. 插件包安装有 checksum 校验。
12. manifest 的 `runtime` 字段、插件包格式和 hook ABI 已预留 WASM 运行时，并有兼容 fixture 证明声明式插件在该预留下不受影响。

## 16. 暂缓事项

以下事项暂缓，除非单独立项：

- 插件商业市场。
- 插件作者收益分成。
- 企业级策略管理。
- 插件自定义 GUI 组件。
- 插件后台常驻任务。
- 插件间依赖解析。
- Node.js 插件运行时。
- Deno 插件运行时。
- 原生动态库插件。
- 直接拦截 CLI 本地输入框。
