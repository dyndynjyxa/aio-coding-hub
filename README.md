<div align="center">
  <img src="public/logo.jpg" width="120" alt="AIO Coding Hub Logo" />

# AIO Coding Hub

**本地 AI CLI 统一网关** — 让 Claude Code / Codex / Gemini CLI 请求走同一个入口

[![Release](https://img.shields.io/github/v/release/dyndynjyxa/aio-coding-hub?style=flat-square)](https://github.com/dyndynjyxa/aio-coding-hub/releases)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20|%20macOS%20|%20Linux-lightgrey?style=flat-square)](#安装)

简体中文 | [English](./README_EN.md)

</div>

> **致谢** — 本项目借鉴了 [cc-switch](https://github.com/farion1231/cc-switch)、[claude-code-hub](https://github.com/ding113/claude-code-hub)、[code-switch-R](https://github.com/Rogers-F/code-switch-R) 等优秀开源项目。

---

## 为什么需要它？

| 痛点 | AIO Coding Hub 的解决方案 |
|------|--------------------------|
| 每个 CLI 都要单独配置 API | **统一网关** — 所有 CLI 走 `127.0.0.1` 本机入口 |
| 上游不稳定时请求失败 | **智能 Failover** — 自动切换供应商，熔断保护 |
| 不同场景需要不同的供应商组合 | **排序模板** — 多套组合按 CLI 激活，一键切换 |
| 不知道用了多少 Token 和花了多少钱 | **全链路可观测** — Trace 追踪、用量统计、花费估算 |
| 不同项目需要不同的 Prompts / MCP 配置 | **工作区隔离** — 按项目管理 CLI 配置，一键切换 |

---

## 产品截图

### 首页 — 热力图、用量趋势、活跃 Session、请求日志

![首页](public/screenshots/home.png)

### 用量 — Token 统计、缓存命中率、耗时、花费排行

![用量](public/screenshots/usage.png)

### 模型验证 — 多维度渠道鉴别与供应商验证

![模型验证](public/screenshots/modelValidate.png)

---

## 核心功能

### 网关代理

- 单一入口代理 Claude Code / Codex / Gemini CLI 请求
- 首页每个 CLI 独立代理开关，一键启停
- 自定义模型名称映射
- SSE / JSON 响应自动修复

### 智能路由与容错

- 多供应商优先级排序 + 自动故障转移
- 熔断器模式（可配置阈值与恢复时间）
- Sticky Session 保持会话粘滞
- 排序模板：多套供应商组合，三个 CLI 各自激活
- 模板内拖拽排序、独立 enabled 开关、切换即时生效

### 用量与可观测

- Token 用量统计（按 CLI / 供应商 / 模型维度）
- 花费估算 + 模型价格自动同步
- 请求 Trace 与实时控制台日志
- 请求热力图（按时段分布）
- 缓存走势图：分供应商命中率折线，60% 预警线
- 可用率：供应商时间线点阵，15s 自动刷新

### 工作区管理

- 按项目隔离 Prompts、MCP、Skill 配置
- 工作区对比、克隆、切换与回滚
- 配置自动同步到各 CLI

### Skill 市场

- 从 Git 仓库发现并安装 Skill
- 仓库管理、过滤、排序
- 关联工作区批量管理

### CLI 管理

- Claude Code 设置直接编辑
- Codex config.toml 代码编辑器
- 环境变量冲突检测
- 本地 Session 历史浏览（项目 → 会话 → 消息）

### 模型验证

- 多维度验证模板（Token 截断、Extended Thinking 等）
- 跨供应商签名验证
- 批量验证 + 历史记录

### 其他

- 自动更新、开机自启、单实例
- 数据导入 / 导出 / 清空
- WSL 环境支持

---

## 安装

### 从 Release 下载（推荐）

前往 [Releases](https://github.com/dyndynjyxa/aio-coding-hub/releases) 下载对应平台安装包：

<!-- SUPPORT_MATRIX_RELEASE_DOWNLOAD:START -->
| 平台 | 官方发布安装包 |
| --- | --- |
| Windows x64 | `.msi` / `-portable.zip` |
| macOS Intel | `.zip` |
| macOS Apple Silicon | `.zip` |
| Linux x64 | `.deb` / `.AppImage` / `-wayland.AppImage` |
<!-- SUPPORT_MATRIX_RELEASE_DOWNLOAD:END -->

官方支持矩阵只覆盖上表 4 个目标。`mac:universal` 和 `win:arm64` 只保留本地构建命令，不进入 Release 产物和 `latest.json`。

<details>
<summary>Linux Arch / Wayland 用户</summary>

**推荐：AUR 软件包**（使用系统库，兼容性最好）

```bash
paru -S aio-coding-hub-bin
# 或
yay -S aio-coding-hub-bin
```

**AppImage 用户**

应用在 Wayland 下启动时会自动检测并注入 `WEBKIT_DISABLE_COMPOSITING_MODE=1` 以避免 EGL 冲突崩溃（见 [issue #93](https://github.com/dyndynjyxa/aio-coding-hub/issues/93)）。
若仍遇到白屏，可改用 Release 中附带的 `*-wayland.AppImage`（已剥离内置 EGL/Mesa 库，使用系统版本）：

```bash
# 或者手动对已有 AppImage 进行重打包
./scripts/repack-linux-appimage-wayland.sh aio-coding-hub-linux-amd64.AppImage
```

</details>

<details>
<summary>macOS 安全提示</summary>

若遇到"无法打开 / 来源未验证"提示：

```bash
sudo xattr -cr /Applications/"AIO Coding Hub.app"
```

</details>

### 从源码构建

<details>
<summary>前置条件</summary>

**通用要求：** Node.js 18+、pnpm、Rust 1.90+

**Windows：** [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)（勾选"使用 C++ 的桌面开发"）

**macOS：** `xcode-select --install`

**Linux (Ubuntu/Debian)：**
```bash
sudo apt-get update
sudo apt-get install -y libasound2-dev libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

</details>

```bash
git clone https://github.com/dyndynjyxa/aio-coding-hub.git
cd aio-coding-hub
pnpm install

# 开发模式
pnpm tauri:dev

# 构建（当前平台）
pnpm tauri:build

# 指定平台
```

<!-- SUPPORT_MATRIX_SOURCE_BUILD:START -->
| 分类 | 命令 | 说明 |
| --- | --- | --- |
| 官方支持 | `pnpm tauri:build:win:x64` | Windows x64；官方支持；进入 Release / updater 矩阵 |
| 官方支持 | `pnpm tauri:build:mac:x64` | macOS Intel；官方支持；进入 Release / updater 矩阵 |
| 官方支持 | `pnpm tauri:build:mac:arm64` | macOS Apple Silicon；官方支持；进入 Release / updater 矩阵 |
| 官方支持 | `pnpm tauri:build:linux:x64` | Linux x64；官方支持；进入 Release / updater 矩阵 |
| 本地构建 | `pnpm tauri:build:mac:universal` | macOS Universal；仅本地构建；不进入官方发布 / updater 矩阵 |
| 本地构建 | `pnpm tauri:build:win:arm64` | Windows ARM64；仅本地构建；不进入官方发布 / updater 矩阵 |
<!-- SUPPORT_MATRIX_SOURCE_BUILD:END -->

上表中的“官方支持”会进入 GitHub Release 和自动更新；“本地构建”只保留脚本，不承诺发布和更新。

---

## 快速开始

```
1. 供应商页 → 添加上游（官方 API / 自建代理 / 公司网关）
2. 首页 → 打开目标 CLI 的"代理"开关
3. 终端发起请求 → 在控制台 / 用量页查看 Trace 与统计
```

验证网关运行：

```bash
curl http://127.0.0.1:37123/health
# {"status":"ok"}
```

## 旁路关联审核（可选）

旁路关联审核用于在请求完成后，额外记录“用户输入”和“模型输出”之间的关联性与风险信号。它是**旁路记录**，不会阻断、改写、重试、路由或取消主请求。

### 启用方式

1. 进入 **设置** 页，找到 **旁路关联审核**。
2. 打开开关。
3. 选择审核用 Provider 和模型。
   - 当前后端支持直连 API Key 模式的 Claude / Codex provider。
   - OAuth、bridge、Gemini、CX2CC、禁用 provider、缺少 API Key 或缺少 Base URL 的 provider 会记录为未配置或不支持状态。
4. 选择模式、采样率、超时、输入/输出捕获上限。

### 查看结果

审核结果保存在请求日志里：

1. 打开首页的 **最近代理记录**，或进入 **代理记录** 页面。
2. 点击某条请求，打开 **代理记录详情**。
3. 在 **概览** tab 查看 **旁路关联审核** 卡片。
   - 可看到状态、风险、关联分、信号数、耗时、信号原因和证据。
4. 在 **原始数据** tab 展开 `association_audit_json`，可查看完整 JSON。

如果看不到 **旁路关联审核** 卡片，通常是因为该请求发生在功能启用前、功能处于 `off` / 关闭状态、没有产生审核结果，或异步审核尚未写入日志。等待几秒后刷新代理记录即可确认。

### 模式说明

| 模式 | 行为 |
| --- | --- |
| `off` | 不触发旁路审核，不改变主请求行为。 |
| `all` | 每条符合条件的已完成请求都会尝试调用审核 LLM。 |
| `sampled` | 按采样率决定是否调用审核 LLM；未命中时写入 `skipped` / `sampled_out`。 |
| `prefiltered` | 先做本地预筛，只有命中可疑信号或明显低关联的大段输出才调用审核 LLM；未命中时写入 `skipped` / `prefilter_no_signal`。 |

### 采样逻辑

`sampled` 模式使用确定性采样，不是每次随机：

- `sample_rate >= 100`：全部选中。
- `sample_rate == 0`：全部跳过。
- 其他值：对请求的 `trace_id` 做 hash，计算 `hash % 100 < sample_rate`。

因此，同一个 `trace_id` 的采样结果稳定；不同请求按采样率分布进入审核。

### 预筛逻辑

`prefiltered` 模式的预筛只决定“是否值得调用审核 LLM”，不直接给出风险结论。真正的语义判断仍由所选审核模型完成。

本地预筛会检查已经截断和脱敏后的响应快照：

- 命中高风险或无关内容标记时调用审核 LLM，例如命令执行、脚本、凭据、提示注入、回调、广告推广等相关片段。
- 当前实现中的典型标记包括：`curl`、`wget`、`powershell`、`bash`、`chmod`、`ssh `、`scp `、`eval(`、`document.cookie`、`<script`、`api_key`、`access_token`、`refresh_token`、`password`、`secret`、`ignore previous`、`system prompt`、`role:`、`bypass`、`callback`、`webhook`、`广告`、`服务器线路`、`优惠`、`购买`、`加群`、`推广`。
- 如果输出达到 1000 字符以上，并且请求与响应的粗略 token 重叠率低于 8%，也会调用审核 LLM。

粗略 token 重叠率的计算方式是：按非字母数字字符切分文本，只保留长度不少于 4 的 token，最多取 200 个 token，然后计算请求 token 中有多少也出现在响应里。

注意：预筛是成本控制和延迟控制手段，不是安全判决器；它的目标是减少明显无风险内容的审核调用，同时保留可疑或低关联输出给 LLM 判断。

---

## 技术栈

| 层级 | 技术 |
|------|------|
| **前端** | React 19 · TypeScript · Tailwind CSS · Vite |
| **状态管理** | TanStack Query · React Hooks |
| **桌面框架** | Tauri 2 |
| **后端** | Rust · Axum (HTTP Gateway) |
| **数据库** | SQLite (rusqlite) |
| **测试** | Vitest · Testing Library · MSW · Cargo Test |

---

## 质量保证

```bash
pnpm check:precommit       # 快速预提交检查（前端 + Rust check）
pnpm check:precommit:full  # 完整检查（格式 + clippy）
pnpm check:prepush         # 覆盖率 + 后端测试 + clippy
pnpm test:unit              # 前端单元测试
pnpm tauri:test             # 后端测试
```

---

## 不适用场景

- 公网部署 / 远程访问 / 多租户
- 企业级 RBAC 权限管理

> 本项目定位为 **单机桌面工具 + 本地网关**，所有数据保存在本机。

---

## 参与贡献

欢迎提交 Issue 和 PR！采用 [Conventional Commits](https://www.conventionalcommits.org/) 规范。

```bash
feat(ui): add usage heatmap
fix(gateway): handle timeout correctly
docs: update installation guide
```

---

## 许可证

[MIT License](LICENSE)

---

[![Stargazers over time](https://starchart.cc/dyndynjyxa/aio-coding-hub.svg?variant=adaptive)](https://starchart.cc/dyndynjyxa/aio-coding-hub)
