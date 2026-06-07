<div align="center">
  <img src="public/logo.jpg" width="120" alt="AIO Coding Hub Logo" />

# AIO Coding Hub

**Local AI CLI Unified Gateway** — Route Claude Code / Codex / Gemini CLI through a single entry point

[![Release](https://img.shields.io/github/v/release/dyndynjyxa/aio-coding-hub?style=flat-square)](https://github.com/dyndynjyxa/aio-coding-hub/releases)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows%20|%20macOS%20|%20Linux-lightgrey?style=flat-square)](#installation)

[简体中文](./README.md) | English

</div>

> **Credits** — Inspired by [cc-switch](https://github.com/farion1231/cc-switch), [claude-code-hub](https://github.com/ding113/claude-code-hub), and [code-switch-R](https://github.com/Rogers-F/code-switch-R).

---

## Why?

| Problem | How AIO Coding Hub Solves It |
|---------|------------------------------|
| Each CLI needs separate API config | **Unified gateway** — all CLIs route through `127.0.0.1` |
| Upstream goes down, requests fail | **Smart failover** — auto-switch providers with circuit breaker |
| Different scenarios need different provider sets | **Sort templates** — multiple sets, per-CLI activation |
| No idea how many tokens or how much it costs | **Full observability** — trace, usage stats, cost estimation |
| Different projects need different Prompts / MCP configs | **Workspace isolation** — per-project CLI config, one-click switch |

---

## Screenshots

### Home — Heatmap, usage trends, active sessions, request logs

![Home](public/screenshots/home.png)

### Usage — Token stats, cache hit rate, latency, cost leaderboard

![Usage](public/screenshots/usage.png)

### Model Validation — Multi-dimensional channel verification

![Model Validation](public/screenshots/modelValidate.png)

---

## Features

### Gateway Proxy

- Single entry point for Claude Code / Codex / Gemini CLI
- Per-CLI proxy toggle on Home, one-click on/off
- Custom model name mapping
- Auto-fix for SSE / JSON responses

### Smart Routing & Resilience

- Multi-provider priority ordering + automatic failover
- Circuit breaker (configurable threshold & recovery time)
- Sticky session for consistent provider routing
- Sort templates: multiple provider sets, activated per CLI
- Drag-to-reorder, per-provider toggle, instant switching

### Usage & Observability

- Token usage analytics (by CLI / provider / model)
- Cost estimation + auto-synced model pricing
- Request trace & real-time console logs
- Request heatmap (time-of-day distribution)
- Cache trend chart: per-provider hit rate, 60% warning line
- Availability: provider timeline dots, 15s auto-refresh

### Workspace Management

- Per-project isolation for Prompts, MCP, and Skill configs
- Workspace compare, clone, switch & rollback
- Auto-sync configs to each CLI

### Skill Market

- Discover and install Skills from Git repositories
- Repository management, filtering, and sorting
- Batch management linked to workspaces

### CLI Management

- Direct editing of Claude Code settings
- CodeMirror editor for Codex config.toml
- Environment variable conflict detection
- Local session history browser (project → session → messages)

### Model Validation

- Multi-dimensional validation templates (token truncation, Extended Thinking, etc.)
- Cross-provider signature verification
- Batch validation + history

### More

- Auto-update, autostart, single instance
- Data import / export / reset
- WSL support

---

## Installation

### Download from Releases (Recommended)

Go to [Releases](https://github.com/dyndynjyxa/aio-coding-hub/releases) and download for your platform:

<!-- SUPPORT_MATRIX_RELEASE_DOWNLOAD:START -->
| Platform | Official release packages |
| --- | --- |
| Windows x64 | `.msi` / `-portable.zip` |
| macOS Intel | `.zip` |
| macOS Apple Silicon | `.zip` |
| Linux x64 | `.deb` / `.AppImage` / `-wayland.AppImage` |
<!-- SUPPORT_MATRIX_RELEASE_DOWNLOAD:END -->

The official support matrix only covers those four targets. `mac:universal` and `win:arm64` remain local build scripts and do not ship in Release assets or `latest.json`.

<details>
<summary>Linux Arch / Wayland users</summary>

**Recommended: AUR package** (uses system libraries, best compatibility)

```bash
paru -S aio-coding-hub-bin
# or
yay -S aio-coding-hub-bin
```

**AppImage users**

The app automatically detects Wayland sessions and sets `WEBKIT_DISABLE_COMPOSITING_MODE=1`
to prevent EGL display initialisation crashes (see [issue #93](https://github.com/dyndynjyxa/aio-coding-hub/issues/93)).
If you still see a blank white window, use the `*-wayland.AppImage` artifact from the Release page
(bundled EGL/Mesa libraries stripped; system versions are used instead):

```bash
# Or manually repack an existing AppImage
./scripts/repack-linux-appimage-wayland.sh aio-coding-hub-linux-amd64.AppImage
```

</details>

<details>
<summary>macOS security note</summary>

If you see "can't be opened / unverified developer":

```bash
sudo xattr -cr /Applications/"AIO Coding Hub.app"
```

</details>

### Build from Source

<details>
<summary>Prerequisites</summary>

**General:** Node.js 18+, pnpm, Rust 1.90+

**Windows:** [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) (select "Desktop development with C++")

**macOS:** `xcode-select --install`

**Linux (Ubuntu/Debian):**
```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf
```

</details>

```bash
git clone https://github.com/dyndynjyxa/aio-coding-hub.git
cd aio-coding-hub
pnpm install

# Development
pnpm tauri:dev

# Build (current platform)
pnpm tauri:build

# Platform-specific
```

<!-- SUPPORT_MATRIX_SOURCE_BUILD:START -->
| Scope | Command | Notes |
| --- | --- | --- |
| Official | `pnpm tauri:build:win:x64` | Windows x64; Official; included in Release / updater matrix |
| Official | `pnpm tauri:build:mac:x64` | macOS Intel; Official; included in Release / updater matrix |
| Official | `pnpm tauri:build:mac:arm64` | macOS Apple Silicon; Official; included in Release / updater matrix |
| Official | `pnpm tauri:build:linux:x64` | Linux x64; Official; included in Release / updater matrix |
| Local only | `pnpm tauri:build:mac:universal` | macOS Universal; Local build only; excluded from the official release / updater matrix |
| Local only | `pnpm tauri:build:win:arm64` | Windows ARM64; Local build only; excluded from the official release / updater matrix |
<!-- SUPPORT_MATRIX_SOURCE_BUILD:END -->

Only the "Official" rows above feed GitHub Releases and auto-update. The "Local only" rows keep local build flexibility without claiming shipped support.

---

## Quick Start

```
1. Providers page → Add upstream (official API / self-hosted proxy / company gateway)
2. Home page → Toggle "Proxy" switch for target CLI
3. Run CLI in terminal → View trace & stats in Console / Usage page
```

Verify the gateway is running:

```bash
curl http://127.0.0.1:37123/health
# {"status":"ok"}
```

## Association Audit (Optional)

Association Audit records advisory signals about whether the model output is related to the user input. It runs as a passive sidecar after the main request finishes. It does **not** block, rewrite, retry, reroute, or cancel the main request.

### Enable it

1. Open **Settings** and find **Association Audit**.
2. Turn it on.
3. Select the provider and model used for audit calls.
   - The current backend supports direct API-key Claude / Codex providers.
   - OAuth, bridge, Gemini, CX2CC, disabled providers, missing API keys, or missing base URLs are recorded as not-configured or unsupported audit states.
4. Choose mode, sample rate, timeout, and input/output capture limits.

### View results

Audit results are stored in request logs:

1. Open **Recent proxy records** on Home, or go to the **Proxy Records** page.
2. Click a request to open **Proxy Record Details**.
3. In the **Overview** tab, look for the **Association Audit** card.
   - It shows status, risk, association score, signal count, duration, signal reasons, and evidence.
4. In the **Raw Data** tab, expand `association_audit_json` to inspect the full JSON.

If the audit card is missing, the request may have happened before the feature was enabled, the feature may be off, no audit result may have been produced, or the async audit write may not have finished yet. Wait a few seconds and refresh the proxy records to confirm.

### Modes

| Mode | Behavior |
| --- | --- |
| `off` | Does not trigger association audit and does not change main request behavior. |
| `all` | Attempts an audit LLM call for every eligible completed request. |
| `sampled` | Uses the sample rate to decide whether to call the audit LLM; skipped requests are recorded as `skipped` / `sampled_out`. |
| `prefiltered` | Runs a local prefilter first. It calls the audit LLM only when suspicious markers or long low-overlap output are detected; skipped requests are recorded as `skipped` / `prefilter_no_signal`. |

### Sampling logic

`sampled` mode uses deterministic sampling, not per-run randomness:

- `sample_rate >= 100`: every request is selected.
- `sample_rate == 0`: every request is skipped.
- Other values: hash the request `trace_id`, then check `hash % 100 < sample_rate`.

This makes the sampling result stable for the same `trace_id`, while different requests are distributed according to the configured sample rate.

### Prefilter logic

`prefiltered` mode only decides whether an audit LLM call is worth making. It does not emit the final risk judgment. Semantic judgment still belongs to the selected audit model.

The local prefilter checks the redacted and bounded response snapshot:

- It calls the audit LLM when high-risk or unrelated-content markers appear, such as command execution, scripts, credentials, prompt injection, callbacks, or promotional content.
- Current marker examples include: `curl`, `wget`, `powershell`, `bash`, `chmod`, `ssh `, `scp `, `eval(`, `document.cookie`, `<script`, `api_key`, `access_token`, `refresh_token`, `password`, `secret`, `ignore previous`, `system prompt`, `role:`, `bypass`, `callback`, `webhook`, `广告`, `服务器线路`, `优惠`, `购买`, `加群`, `推广`.
- It also calls the audit LLM when the output is at least 1000 characters long and the rough token overlap between request and response is below 8%.

Rough token overlap is computed by splitting text on non-alphanumeric characters, keeping tokens with at least 4 characters, taking up to 200 tokens, and checking how many request tokens also appear in the response.

Prefiltering is a cost and latency control mechanism, not a security decision engine. Its purpose is to skip obviously low-risk outputs while still sending suspicious or low-association outputs to the audit LLM.

---

## Tech Stack

| Layer | Technology |
|-------|------------|
| **Frontend** | React 19 · TypeScript · Tailwind CSS · Vite |
| **State** | TanStack Query · React Hooks |
| **Desktop** | Tauri 2 |
| **Backend** | Rust · Axum (HTTP Gateway) |
| **Database** | SQLite (rusqlite) |
| **Testing** | Vitest · Testing Library · MSW · Cargo Test |

---

## Quality Assurance

```bash
pnpm check:precommit       # Quick pre-commit (frontend + Rust check)
pnpm check:precommit:full  # Full check (formatting + clippy)
pnpm check:prepush         # Coverage + backend tests + clippy
pnpm test:unit              # Frontend unit tests
pnpm tauri:test             # Backend tests
```

---

## Not Designed For

- Public deployment / remote access / multi-tenant
- Enterprise RBAC

> This is a **local desktop tool + local gateway**. All data stays on your machine.

---

## Contributing

Issues and PRs welcome! We follow [Conventional Commits](https://www.conventionalcommits.org/).

```bash
feat(ui): add usage heatmap
fix(gateway): handle timeout correctly
docs: update installation guide
```

---

## License

[MIT License](LICENSE)

---

[![Stargazers over time](https://starchart.cc/dyndynjyxa/aio-coding-hub.svg?variant=adaptive)](https://starchart.cc/dyndynjyxa/aio-coding-hub)
