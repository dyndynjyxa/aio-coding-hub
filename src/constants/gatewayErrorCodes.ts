// Usage:
// - Import canonical gateway error-code constants to avoid scattered string literals.
// - Keep frontend labels/parsing aligned with Rust gateway error definitions.

export const GatewayErrorCodes = {
  ALL_PROVIDERS_UNAVAILABLE: "GW_ALL_PROVIDERS_UNAVAILABLE",
  UPSTREAM_ALL_FAILED: "GW_UPSTREAM_ALL_FAILED",
  NO_ENABLED_PROVIDER: "GW_NO_ENABLED_PROVIDER",
  UPSTREAM_TIMEOUT: "GW_UPSTREAM_TIMEOUT",
  UPSTREAM_CONNECT_FAILED: "GW_UPSTREAM_CONNECT_FAILED",
  UPSTREAM_5XX: "GW_UPSTREAM_5XX",
  UPSTREAM_4XX: "GW_UPSTREAM_4XX",
  UPSTREAM_READ_ERROR: "GW_UPSTREAM_READ_ERROR",
  UPSTREAM_BODY_READ_ERROR: "GW_UPSTREAM_BODY_READ_ERROR",
  STREAM_ERROR: "GW_STREAM_ERROR",
  STREAM_ABORTED: "GW_STREAM_ABORTED",
  STREAM_IDLE_TIMEOUT: "GW_STREAM_IDLE_TIMEOUT",
  REQUEST_ABORTED: "GW_REQUEST_ABORTED",
  INTERNAL_ERROR: "GW_INTERNAL_ERROR",
  BODY_TOO_LARGE: "GW_BODY_TOO_LARGE",
  LARGE_BODY_MISSING_MODEL: "GW_LARGE_BODY_MISSING_MODEL",
  INVALID_CLI_KEY: "GW_INVALID_CLI_KEY",
  INVALID_BASE_URL: "GW_INVALID_BASE_URL",
  PORT_IN_USE: "GW_PORT_IN_USE",
  RESPONSE_BUILD_ERROR: "GW_RESPONSE_BUILD_ERROR",
  PROVIDER_RATE_LIMITED: "GW_PROVIDER_RATE_LIMITED",
  PROVIDER_CIRCUIT_OPEN: "GW_PROVIDER_CIRCUIT_OPEN",
  CLI_PROXY_DISABLED: "GW_CLI_PROXY_DISABLED",
  CLI_PROXY_GUARD_ERROR: "GW_CLI_PROXY_GUARD_ERROR",
  HTTP_CLIENT_INIT: "GW_HTTP_CLIENT_INIT",
  ATTEMPT_LOG_CHANNEL_CLOSED: "GW_ATTEMPT_LOG_CHANNEL_CLOSED",
  ATTEMPT_LOG_ENQUEUE_TIMEOUT: "GW_ATTEMPT_LOG_ENQUEUE_TIMEOUT",
  ATTEMPT_LOG_DROPPED: "GW_ATTEMPT_LOG_DROPPED",
  REQUEST_LOG_CHANNEL_CLOSED: "GW_REQUEST_LOG_CHANNEL_CLOSED",
  REQUEST_LOG_ENQUEUE_TIMEOUT: "GW_REQUEST_LOG_ENQUEUE_TIMEOUT",
  REQUEST_LOG_WRITE_THROUGH_ON_BACKPRESSURE: "GW_REQUEST_LOG_WRITE_THROUGH_ON_BACKPRESSURE",
  REQUEST_LOG_WRITE_THROUGH_RATE_LIMITED: "GW_REQUEST_LOG_WRITE_THROUGH_RATE_LIMITED",
  REQUEST_LOG_DROPPED: "GW_REQUEST_LOG_DROPPED",
  FAKE_200: "GW_FAKE_200",
} as const;

export type GatewayErrorCode = (typeof GatewayErrorCodes)[keyof typeof GatewayErrorCodes];

export type GatewayErrorDescription = { desc: string; suggestion: string };

export const GatewayErrorShortLabels = {
  [GatewayErrorCodes.ALL_PROVIDERS_UNAVAILABLE]: "全部不可用",
  [GatewayErrorCodes.UPSTREAM_ALL_FAILED]: "全部失败",
  [GatewayErrorCodes.NO_ENABLED_PROVIDER]: "无供应商",
  [GatewayErrorCodes.UPSTREAM_TIMEOUT]: "上游超时",
  [GatewayErrorCodes.UPSTREAM_CONNECT_FAILED]: "连接失败",
  [GatewayErrorCodes.UPSTREAM_5XX]: "上游5XX",
  [GatewayErrorCodes.UPSTREAM_4XX]: "上游4XX",
  [GatewayErrorCodes.UPSTREAM_READ_ERROR]: "读取错误",
  [GatewayErrorCodes.UPSTREAM_BODY_READ_ERROR]: "响应体读取失败",
  [GatewayErrorCodes.STREAM_ERROR]: "流错误",
  [GatewayErrorCodes.STREAM_ABORTED]: "流中断",
  [GatewayErrorCodes.STREAM_IDLE_TIMEOUT]: "流空闲超时",
  [GatewayErrorCodes.REQUEST_ABORTED]: "请求中断",
  [GatewayErrorCodes.INTERNAL_ERROR]: "内部错误",
  [GatewayErrorCodes.BODY_TOO_LARGE]: "请求过大",
  [GatewayErrorCodes.LARGE_BODY_MISSING_MODEL]: "缺少 model",
  [GatewayErrorCodes.INVALID_CLI_KEY]: "无效CLI",
  [GatewayErrorCodes.INVALID_BASE_URL]: "无效URL",
  [GatewayErrorCodes.PORT_IN_USE]: "端口占用",
  [GatewayErrorCodes.RESPONSE_BUILD_ERROR]: "响应构建错误",
  [GatewayErrorCodes.PROVIDER_RATE_LIMITED]: "供应商限额",
  [GatewayErrorCodes.PROVIDER_CIRCUIT_OPEN]: "供应商熔断",
  [GatewayErrorCodes.FAKE_200]: "假200",
} satisfies Partial<Record<GatewayErrorCode, string>>;

export function getGatewayErrorShortLabel(errorCode: string) {
  return GatewayErrorShortLabels[errorCode as keyof typeof GatewayErrorShortLabels] ?? errorCode;
}

export const GatewayErrorDescriptions = {
  GW_ALL_PROVIDERS_UNAVAILABLE: {
    desc: "所有 Provider 均不可用",
    suggestion:
      "所有配置的 Provider 都处于熔断或冷却状态。请检查 Provider 列表和各 Provider 的服务状态。",
  },
  GW_UPSTREAM_ALL_FAILED: {
    desc: "所有 Provider 尝试均失败",
    suggestion: "已尝试所有可用 Provider 但都失败了。请检查各 Provider 的 API Key 和服务状态。",
  },
  GW_NO_ENABLED_PROVIDER: {
    desc: "没有已启用的 Provider",
    suggestion: "当前 CLI 没有启用任何 Provider。请前往 Provider 管理页面启用至少一个 Provider。",
  },
  GW_UPSTREAM_TIMEOUT: {
    desc: "上游服务响应超时",
    suggestion: "Provider 响应时间过长。请检查 Provider 服务状态，或考虑在设置中增加超时时间。",
  },
  GW_UPSTREAM_CONNECT_FAILED: {
    desc: "无法连接到上游服务",
    suggestion: "Provider 不可达。请检查网络连接和 Provider 的 Base URL 是否正确。",
  },
  GW_UPSTREAM_5XX: {
    desc: "上游服务返回服务端错误 (5xx)",
    suggestion: "Provider 内部错误。通常是 Provider 侧的临时故障，系统会自动尝试其他 Provider。",
  },
  GW_UPSTREAM_4XX: {
    desc: "上游服务返回客户端错误 (4xx)",
    suggestion: "请求被 Provider 拒绝。可能是 API Key 无效、请求格式错误或权限不足。",
  },
  GW_UPSTREAM_READ_ERROR: {
    desc: "读取上游响应失败",
    suggestion: "从 Provider 接收数据时发生错误。可能是网络不稳定或 Provider 异常断开连接。",
  },
  GW_UPSTREAM_BODY_READ_ERROR: {
    desc: "读取上游响应体失败",
    suggestion: "在读取 Provider 返回的响应内容时发生错误。可能是响应数据不完整或网络中断。",
  },
  GW_STREAM_ERROR: {
    desc: "流式响应传输错误",
    suggestion: "SSE 流在传输过程中异常中断。可能是网络不稳定导致。",
  },
  GW_STREAM_ABORTED: {
    desc: "流式响应被中断",
    suggestion: "SSE 流被客户端或网络中断。如果是用户主动取消操作，则属正常行为。",
  },
  GW_STREAM_IDLE_TIMEOUT: {
    desc: "流式响应空闲超时",
    suggestion: "SSE 流长时间无数据传输。可能是上游 Provider 处理异常卡住。",
  },
  GW_REQUEST_ABORTED: {
    desc: "请求被中断",
    suggestion: "客户端（CLI 工具）主动取消了请求，或因总超时被网关主动终止。",
  },
  GW_INTERNAL_ERROR: {
    desc: "网关内部错误",
    suggestion: "网关自身发生了意外错误。请查看日志文件获取更多信息。",
  },
  GW_BODY_TOO_LARGE: {
    desc: "请求体过大",
    suggestion: "发送的请求内容超过了网关允许的最大尺寸。请减小请求内容。",
  },
  GW_LARGE_BODY_MISSING_MODEL: {
    desc: "大请求体缺少 model 字段",
    suggestion:
      "请求体超过了诊断阈值且未声明 model。常见原因：上游客户端或中间代理截断了 body、以非 JSON 方式发送、或漏掉了 model 字段。请检查请求体完整性与格式。",
  },
  GW_INVALID_CLI_KEY: {
    desc: "无效的 CLI Key",
    suggestion: "请求中的 CLI Key 无法识别。支持的 CLI Key 包括 claude、codex、gemini。",
  },
  GW_INVALID_BASE_URL: {
    desc: "无效的 Base URL",
    suggestion: "Provider 的 Base URL 格式不正确。请检查 Provider 配置中的 URL。",
  },
  GW_PORT_IN_USE: {
    desc: "端口被占用",
    suggestion:
      "首选端口已被其他程序占用。网关已自动选择可用端口启动。如需固定端口，请在设置中修改并确保该端口未被占用。",
  },
  GW_RESPONSE_BUILD_ERROR: {
    desc: "构建响应失败",
    suggestion: "网关在构建 HTTP 响应时发生内部错误。请查看日志文件获取更多信息。",
  },
  GW_PROVIDER_RATE_LIMITED: {
    desc: "Provider 速率限制",
    suggestion: "该 Provider 返回了 429 (Too Many Requests)。请稍后再试或切换到其他 Provider。",
  },
  GW_PROVIDER_CIRCUIT_OPEN: {
    desc: "Provider 已熔断",
    suggestion: "该 Provider 因连续失败已被熔断，请求已自动跳过。熔断将在设定时间后自动恢复。",
  },
  GW_CLI_PROXY_DISABLED: {
    desc: "CLI Proxy 未启用",
    suggestion: "该 CLI 的代理功能未启用。请在设置中启用对应 CLI 的代理。",
  },
  GW_CLI_PROXY_GUARD_ERROR: {
    desc: "CLI Proxy 守卫错误",
    suggestion: "CLI Proxy 在处理请求时发生内部错误。请重试或查看日志。",
  },
  GW_HTTP_CLIENT_INIT: {
    desc: "HTTP 客户端初始化失败",
    suggestion: "无法创建 HTTP 客户端。可能是系统资源不足或 TLS 配置问题。",
  },
  GW_ATTEMPT_LOG_CHANNEL_CLOSED: {
    desc: "尝试日志通道已关闭",
    suggestion: "内部日志通道异常关闭。通常在应用关闭过程中出现，可忽略。",
  },
  GW_ATTEMPT_LOG_ENQUEUE_TIMEOUT: {
    desc: "尝试日志入队超时",
    suggestion: "日志写入队列已满。通常在高并发场景下出现，不影响请求处理。",
  },
  GW_ATTEMPT_LOG_DROPPED: {
    desc: "尝试日志被丢弃",
    suggestion: "部分尝试日志因队列压力被丢弃。不影响请求处理。",
  },
  GW_REQUEST_LOG_CHANNEL_CLOSED: {
    desc: "请求日志通道已关闭",
    suggestion: "内部日志通道异常关闭。通常在应用关闭过程中出现，可忽略。",
  },
  GW_REQUEST_LOG_ENQUEUE_TIMEOUT: {
    desc: "请求日志入队超时",
    suggestion: "日志写入队列已满。通常在高并发场景下出现，不影响请求处理。",
  },
  GW_REQUEST_LOG_WRITE_THROUGH_ON_BACKPRESSURE: {
    desc: "请求日志在背压下直写",
    suggestion: "日志队列压力过大，已切换为同步写入。可能导致轻微延迟。",
  },
  GW_REQUEST_LOG_WRITE_THROUGH_RATE_LIMITED: {
    desc: "请求日志直写被限速",
    suggestion: "日志同步写入频率过高已被限速。不影响请求处理。",
  },
  GW_REQUEST_LOG_DROPPED: {
    desc: "请求日志被丢弃",
    suggestion: "部分请求日志因队列压力被丢弃。不影响请求处理。",
  },
  GW_FAKE_200: {
    desc: "上游返回伪成功响应",
    suggestion:
      "上游 Provider 返回了 HTTP 200 但响应体包含错误内容。已自动标记为失败并更新熔断器状态。",
  },
} satisfies Record<GatewayErrorCode, GatewayErrorDescription>;
