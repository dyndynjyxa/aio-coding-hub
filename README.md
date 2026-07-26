<div align="center">
  <img src="public/logo.jpg" width="120" alt="AIO Coding Hub Logo" />

# AIO Coding Hub

**本地 AI CLI 统一网关** — 让 Claude Code / Codex / Gemini CLI 请求走同一个入口

[![Release](https://img.shields.io/github/v/release/dyndynjyxa/aio-coding-hub?style=flat-square)](https://github.com/dyndynjyxa/aio-coding-hub/releases)
[![Downloads](https://img.shields.io/github/downloads/dyndynjyxa/aio-coding-hub/total?style=flat-square)](https://github.com/dyndynjyxa/aio-coding-hub/releases)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20|%20macOS%20|%20Linux-lightgrey?style=flat-square)](#安装)
[![Tauri](https://img.shields.io/badge/built%20with-Tauri%202-24C8DB?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app/)

简体中文 | [English](./README_EN.md)

[安装](#安装) · [快速开始](#快速开始) · [核心功能](#核心功能) · [工作原理](#工作原理) · [FAQ](#faq) · [参与贡献](#参与贡献)

</div>

---

## 为什么需要它？

| 痛点 | AIO Coding Hub 的解决方案 |
|------|--------------------------|
| 每个 CLI 都要单独配置 API | **统一网关** — 所有 CLI 走 `127.0.0.1` 本机入口 |
| 上游不稳定时请求失败 | **智能 Failover** — 自动切换供应商，熔断保护 |
| 不同场景需要不同的供应商组合 | **排序模板** — 多套组合按 CLI 激活，一键切换 |
| 不知道用了多少 Token 和花了多少钱 | **全链路可观测** — Trace 追踪、用量统计、花费估算 |
| 不同项目需要不同的 Prompts / MCP 配置 | **工作区隔离** — 按项目管理 CLI 配置，一键切换 |

> 本项目定位为 **单机桌面工具 + 本地网关**：网关只监听 `127.0.0.1`，所有数据保存在本机，不做公网部署、远程访问和多租户。

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

### 🔀 网关代理

- 单一入口代理 Claude Code / Codex / Gemini CLI 请求
- 首页每个 CLI 独立代理开关，一键启停
- 自定义模型名称映射
- SSE / JSON 响应自动修复

### 🛡️ 智能路由与容错

- 多供应商优先级排序 + 自动故障转移
- 熔断器模式（可配置阈值与恢复时间）
- Sticky Session 保持会话粘滞
- 排序模板：多套供应商组合，三个 CLI 各自激活
- 模板内拖拽排序、独立 enabled 开关、切换即时生效

### 📊 用量与可观测

- Token 用量统计（按 CLI / 供应商 / 模型维度）
- 花费估算 + 模型价格自动同步
- 请求 Trace 与实时控制台日志
- 请求热力图（按时段分布）
- 缓存走势图：分供应商命中率折线，60% 预警线
- 可用率：供应商时间线点阵，15s 自动刷新

### 🗂️ 工作区管理

- 按项目隔离 Prompts、MCP、Skill 配置
- 工作区对比、克隆、切换与回滚
- 配置自动同步到各 CLI

### 🧩 Skill 市场

- 从 Git 仓库发现并安装 Skill
- 仓库管理、过滤、排序
- 关联工作区批量管理

### 🔌 插件系统

- 官方内置插件：Privacy Filter
- Extension Host 插件：命令、Provider 扩展值、网关 hook、协议桥骨架、宿主渲染 UI
- 插件权限、配置 schema、审计日志、启用 / 禁用 / 卸载
- SDK 与脚手架：`@aio-coding-hub/plugin-sdk`、`create-aio-plugin`

插件作者应从 [插件开发手册](docs/plugins/README.md) 开始。社区插件统一使用 Extension Host；旧的预发布规则 / WASM / 进程运行时只作为不支持的迁移历史处理。

### 🖥️ CLI 管理

- Claude Code 设置直接编辑
- Codex config.toml 代码编辑器
- 环境变量冲突检测
- 本地 Session 历史浏览（项目 → 会话 → 消息）

### ✅ 模型验证

- 多维度验证模板（Token 截断、Extended Thinking 等）
- 跨供应商签名验证
- 批量验证 + 历史记录

### ⚙️ 其他

- 自动更新、开机自启、单实例
- 数据导入 / 导出 / 清空
- WSL 环境支持

---

## 安装

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

### macOS

**方式一：Homebrew（推荐）**

```bash
brew tap dyndynjyxa/aio-coding-hub
brew install --cask aio-coding-hub
```

后续升级：

```bash
brew update
brew upgrade --cask aio-coding-hub
```

**方式二：手动下载**

从 [Releases](https://github.com/dyndynjyxa/aio-coding-hub/releases) 下载对应芯片的 `.zip`（Apple Silicon 选 `arm`，Intel 选 `intel`），解压后把 `AIO Coding Hub.app` 拖入「应用程序」文件夹。

> [!IMPORTANT]
> **首次打开提示"已损坏"或"无法验证开发者"？**
>
> 当前 macOS 安装包**未经 Apple 开发者证书签名与公证**，Gatekeeper 会拦截首次启动。任选一种方式处理：
>
> **① 移除隔离属性（推荐，一条命令）**
>
> ```bash
> sudo xattr -cr "/Applications/AIO Coding Hub.app"
> ```
>
> **② 系统设置放行**
>
> 首次双击被拦截后，打开「系统设置 → 隐私与安全性」，在页面底部点击「仍要打开」。
>
> **③ 本地自签名（可选，一劳永逸）**
>
> 用 ad-hoc 签名替换掉无效签名，之后系统升级也不会再提示：
>
> ```bash
> sudo codesign --force --deep --sign - "/Applications/AIO Coding Hub.app"
> ```
>
> 以上处理只需在首次安装或手动覆盖安装后执行一次。

### Windows

从 [Releases](https://github.com/dyndynjyxa/aio-coding-hub/releases) 下载：

- `.msi` — 标准安装包，支持自动更新
- `-portable.zip` — 免安装便携版，解压即用

### Linux

从 [Releases](https://github.com/dyndynjyxa/aio-coding-hub/releases) 下载 `.deb`（Debian / Ubuntu）或 `.AppImage`（通用）。

**Arch Linux（AUR，推荐）** — 使用系统库，兼容性最好：

```bash
paru -S aio-coding-hub-bin
# 或
yay -S aio-coding-hub-bin
```

<details>
<summary>Wayland 白屏 / 启动崩溃排查</summary>

应用在 Wayland 下启动时会自动检测并注入 `WEBKIT_DISABLE_COMPOSITING_MODE=1` 以避免 EGL 冲突崩溃（见 [issue #93](https://github.com/dyndynjyxa/aio-coding-hub/issues/93)）。
若仍遇到白屏，可改用 Release 中附带的 `*-wayland.AppImage`（已剥离内置 EGL/Mesa 库，使用系统版本）：

```bash
# 或者手动对已有 AppImage 进行重打包
./scripts/repack-linux-appimage-wayland.sh aio-coding-hub-linux-amd64.AppImage
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

1. **添加供应商** — 打开「供应商」页，添加上游（官方 API / 自建代理 / 公司网关）
2. **打开代理** — 首页打开目标 CLI 的「代理」开关，请求即经由本机网关转发
3. **照常使用 CLI** — 在终端正常使用 Claude Code / Codex / Gemini CLI
4. **查看统计** — 在控制台 / 用量页查看 Trace、Token 用量与花费

验证网关运行：

```bash
curl http://127.0.0.1:37123/health
# {"status":"ok"}
```

---

## 工作原理

```
 Claude Code ──┐
 Codex        ─┼──▶  AIO Coding Hub 网关 (127.0.0.1:37123)  ──▶  供应商 A（优先级 1）
 Gemini CLI  ──┘     排序模板 · 熔断器 · Failover · 用量计量      ├▶  供应商 B（优先级 2）
                                                                └▶  供应商 C（优先级 3）
```

三个 CLI 的请求统一进入本机网关；网关按当前激活的排序模板选择供应商，失败时自动熔断并切换到下一个，同时记录 Trace、Token 用量与花费。

---

## FAQ

**macOS 提示"已损坏，无法打开"或"无法验证开发者"？**

安装包未经 Apple 签名公证，属预期行为。参见 [macOS 安装说明](#macos)，执行 `sudo xattr -cr "/Applications/AIO Coding Hub.app"` 即可。

**网关端口是多少？如何确认网关在运行？**

默认监听 `127.0.0.1:37123`。执行 `curl http://127.0.0.1:37123/health`，返回 `{"status":"ok"}` 即正常。

**我的 API Key 和请求数据会上传吗？**

不会。网关只监听本机回环地址，所有配置与统计数据保存在本地 SQLite 数据库中。

**Linux Wayland 下白屏或启动崩溃？**

参见 [Linux 安装说明](#linux) 中的 Wayland 排查折叠块，或改用 `*-wayland.AppImage`。

**哪些平台有自动更新？**

官方支持矩阵内的 4 个目标（Windows x64、macOS Intel / Apple Silicon、Linux x64）进入 Release 与 updater 通道；`mac:universal`、`win:arm64` 仅提供本地构建脚本。

---

## 插件开发文档

插件系统面向社区扩展，社区插件统一使用 Extension Host。开发入口：

- [插件开发总览](docs/plugins/README.md)
- [插件开发总指南](docs/plugins/developer-guide.md)
- [Plugin SDK](docs/plugins/reference/sdk.md)
- [官方示例插件](docs/plugins/examples/privacy-filter.md)
- [插件 API 参考](docs/plugins/reference/README.md)
- [Manifest v1 规范](docs/plugin-manifest-v1.md)

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

## 参与贡献

欢迎提交 Issue 和 PR！采用 [Conventional Commits](https://www.conventionalcommits.org/) 规范。

```bash
feat(ui): add usage heatmap
fix(gateway): handle timeout correctly
docs: update installation guide
```

提交 PR 前请本地跑一遍检查：

```bash
pnpm check:precommit       # 快速预提交检查（前端 + Rust check）
pnpm check:precommit:full  # 完整检查（格式 + clippy）
pnpm check:prepush         # 覆盖率 + 后端测试 + clippy
pnpm test:unit             # 前端单元测试
pnpm tauri:test            # 后端测试
```

---

## 致谢

本项目借鉴了以下优秀开源项目：

- [cc-switch](https://github.com/farion1231/cc-switch)
- [claude-code-hub](https://github.com/ding113/claude-code-hub)
- [code-switch-R](https://github.com/Rogers-F/code-switch-R)

---

## 许可证

[MIT License](LICENSE)

## Star History

<a href="https://www.star-history.com/?repos=dyndynjyxa%2Faio-coding-hub&type=timeline&legend=top-left">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/chart?repos=dyndynjyxa/aio-coding-hub&type=timeline&theme=dark&legend=top-left&sealed_token=jaHZDCqFyRK8pFxbpl9LPEq1w0XeHW5ZgEPgz0v-lA3dld9oQHaLo4PhBHoeCCHj0x2SQ4rFcl01feYvK7sW_pbwG6MhN3N1-v9AURRKgU-CuPtkj795d-0XP1dSGdzM1LNi2C5U3O4xiecboJnF3JihPmYap63V23nedzYGqGK_NSpNAN0xU97EoRrO" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/chart?repos=dyndynjyxa/aio-coding-hub&type=timeline&legend=top-left&sealed_token=jaHZDCqFyRK8pFxbpl9LPEq1w0XeHW5ZgEPgz0v-lA3dld9oQHaLo4PhBHoeCCHj0x2SQ4rFcl01feYvK7sW_pbwG6MhN3N1-v9AURRKgU-CuPtkj795d-0XP1dSGdzM1LNi2C5U3O4xiecboJnF3JihPmYap63V23nedzYGqGK_NSpNAN0xU97EoRrO" />
   <img alt="Star History Chart" src="https://api.star-history.com/chart?repos=dyndynjyxa/aio-coding-hub&type=timeline&legend=top-left&sealed_token=jaHZDCqFyRK8pFxbpl9LPEq1w0XeHW5ZgEPgz0v-lA3dld9oQHaLo4PhBHoeCCHj0x2SQ4rFcl01feYvK7sW_pbwG6MhN3N1-v9AURRKgU-CuPtkj795d-0XP1dSGdzM1LNi2C5U3O4xiecboJnF3JihPmYap63V23nedzYGqGK_NSpNAN0xU97EoRrO" />
 </picture>
</a>
