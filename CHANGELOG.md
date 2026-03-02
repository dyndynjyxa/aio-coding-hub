# Changelog

## [0.30.8](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.30.7...aio-coding-hub-v0.30.8) (2026-03-02)


### Features

* **skills:** 添加 Skill 返回本机已安装功能 ([84cfd0b](https://github.com/dyndynjyxa/aio-coding-hub/commit/84cfd0be09c5c38dbd9613533062a3daef6c57df))


### Bug Fixes

* **skills:** 优化符号链接目录的移除逻辑 ([2ef9928](https://github.com/dyndynjyxa/aio-coding-hub/commit/2ef9928eb3d1d4fa23cb22e2035384e7be2a01f6))

## [0.30.7](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.30.6...aio-coding-hub-v0.30.7) (2026-03-01)


### Features

* **gateway:** 添加决策链以丰富失败重试事件细节 ([262e98c](https://github.com/dyndynjyxa/aio-coding-hub/commit/262e98cb0ac974c5452e037ac51d5730625d1ef2))
* **mcp:** 添加 env 和 header 键值对格式校验 ([7d698ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/7d698ca04f1cc9f9c0d420f6d7fec2a53f2d643a))


### Bug Fixes

* **gateway:** 修正 provider 重新排序时清除会话绑定逻辑 ([f03c02e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f03c02e5332fe84496a029b2e5d130b558cfad31))

## [0.30.6](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.30.5...aio-coding-hub-v0.30.6) (2026-03-01)


### Features

* **claudeModelValidation:** 优化模型验证对话框和多轮验证流程支持 ([3f6c031](https://github.com/dyndynjyxa/aio-coding-hub/commit/3f6c03102db7aaeaa2d8399ebdac1cc4787c2a84))

## [0.30.5](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.30.4...aio-coding-hub-v0.30.5) (2026-02-28)


### Features

* **cli-manager:** 对齐 CCH 基础配置开关 ([55779ef](https://github.com/dyndynjyxa/aio-coding-hub/commit/55779efce6964a17b423869af77f3ebff567d62c))
* **config:** 新增计划模式推理强度配置项 ([6f5da87](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f5da87696af5c90054d6ca5be0f25e0a91eba40))
* **gateway:** 对齐 CCH 基础配置整流 ([a1c1b5c](https://github.com/dyndynjyxa/aio-coding-hub/commit/a1c1b5c2b730b89184d59fac207d252e9d415c93))

## [0.30.4](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.30.3...aio-coding-hub-v0.30.4) (2026-02-27)


### Bug Fixes

* **heartbeat_watchdog:** 优化窗口重载的错误处理逻辑 ([6188e6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6188e6c29135c0126490882ceb54302d112ec79c))

## [0.30.3](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.30.2...aio-coding-hub-v0.30.3) (2026-02-25)


### Features

* **cli-sessions:** P0批次1 - 安全校验/React Query/Router state/虚拟化依赖 ([53c9d43](https://github.com/dyndynjyxa/aio-coding-hub/commit/53c9d4320566b03ff5e655ad2442d46a67f77572))
* **cli-sessions:** P0批次2 - 应用安全校验和状态管理重构 ([2dd098f](https://github.com/dyndynjyxa/aio-coding-hub/commit/2dd098fd39d0d45040b83a0cbd80cb13d4a29352))
* **cli-sessions:** P0批次3 - 三个列表虚拟化 ([d48fccd](https://github.com/dyndynjyxa/aio-coding-hub/commit/d48fccd775086bd11c6cf01147fedf0a3c44e86a))
* **cli-sessions:** P1批次 - 布局一致性和Shell转义安全 ([84723f3](https://github.com/dyndynjyxa/aio-coding-hub/commit/84723f377e5088d2f80574be84e6fa819038bb36))
* **codex:** add multi_agent feature toggle ([f56a201](https://github.com/dyndynjyxa/aio-coding-hub/commit/f56a2016bb535812e1a00edb0056e0faa6f44957))


### Bug Fixes

* **cli-sessions:** 修复SessionsPage Card容器flex布局 ([2d2a048](https://github.com/dyndynjyxa/aio-coding-hub/commit/2d2a0481f9811160223f9753d6edec5a28dcfc6c))
* **components:** 修复ClaudeModelValidationDialog点击事件阻塞问题 ([9bd5f53](https://github.com/dyndynjyxa/aio-coding-hub/commit/9bd5f53f38d01fc312b979e3620b5e0badaba585))
* **sessions:** 剥离替换字符并调整按钮样式 ([3586a16](https://github.com/dyndynjyxa/aio-coding-hub/commit/3586a165ceb8ba3afc42a34fdf4228c71b18cce5))
* 修复clippy redundant_pattern_matching警告 ([b0274a6](https://github.com/dyndynjyxa/aio-coding-hub/commit/b0274a663396a2958e470e2167933470bf34a70a))

## [0.30.2](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.30.1...aio-coding-hub-v0.30.2) (2026-02-23)


### Bug Fixes

* **router:** 修复侧边栏快速点击导航卡死问题 ([8b1f862](https://github.com/dyndynjyxa/aio-coding-hub/commit/8b1f862542b8bd94770ab5f937772a53a4bd1d22))
* **tauri:** 修复 Cargo.lock 同步及 CLI 代理切换后 MCP 同步问题 ([1854b00](https://github.com/dyndynjyxa/aio-coding-hub/commit/1854b00205b4b3a257b984b5a6c5271067b2545e))

## [0.30.1](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.30.0...aio-coding-hub-v0.30.1) (2026-02-23)


### Features

* **settings:** 新增任务结束提醒开关和相关功能 ([2fb4022](https://github.com/dyndynjyxa/aio-coding-hub/commit/2fb4022ff2f1078238ced25188960f19da188260))
* **taskCompleteNotifyEvents:** 改进任务完成通知的请求追踪与静默期控制 ([e3f03d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/e3f03d4bf2276db371e170ee4739cd379b6d3fe5))

## [0.30.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.29.1...aio-coding-hub-v0.30.0) (2026-02-23)


### ⚠ BREAKING CHANGES

* **infra:** request_attempt_logs 表不再创建或写入

### Features

* add TextEvidenceSection component for improved output display in ClaudeModelValidationResultPanel ([47be119](https://github.com/dyndynjyxa/aio-coding-hub/commit/47be119a83c365b3e7b41f22308be7550ecaede5))
* **app:** add LogsPage route and enhance CLI manager settings ([6796715](https://github.com/dyndynjyxa/aio-coding-hub/commit/679671526be935129211978b3598a3962f7e2a8e))
* **app:** 优化页面路由懒加载和加载反馈体验 ([504256e](https://github.com/dyndynjyxa/aio-coding-hub/commit/504256e11ed517797405f8bb4ab3d353607cb1ae))
* **app:** 添加应用心跳机制与监听支持 ([df78171](https://github.com/dyndynjyxa/aio-coding-hub/commit/df781712d16a2f5ccc99e37a687d39cb4e4f56c5))
* **cache-anomaly-monitor:** implement cache anomaly monitoring feature ([59d69d1](https://github.com/dyndynjyxa/aio-coding-hub/commit/59d69d1528d888a84d71038873321f3273d4ded1))
* **charts:** migrate from ECharts to Recharts for improved charting capabilities ([b747b61](https://github.com/dyndynjyxa/aio-coding-hub/commit/b747b61a8dd587043a7e8de01a5f9b3d64ead7ae))
* **claude-validation:** add signature and caching roundtrip probes ([15badee](https://github.com/dyndynjyxa/aio-coding-hub/commit/15badee08b0c14f71695e6e71f0b165e4844371c))
* **ClaudeModelValidationDialog:** 添加综合协议验证功能与UI展示 ([b6ede30](https://github.com/dyndynjyxa/aio-coding-hub/commit/b6ede30e61e19f9af69e8d7153cbc711b3e3a180))
* **ClaudeModelValidation:** enhance cross-provider validation and UI feedback ([bf83c7e](https://github.com/dyndynjyxa/aio-coding-hub/commit/bf83c7e03c7edf78795cd51a943c01a88e0b17d7))
* **ClaudeModelValidation:** enhance output token validation and error handling ([d245288](https://github.com/dyndynjyxa/aio-coding-hub/commit/d245288d7a4937ca7b0213ebd79d9c0d5e3c76b4))
* **ClaudeModelValidation:** implement cross-provider signature validation and enhance request handling ([2e102d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/2e102d4f3fd2745e4480a5884272baeafe66b6d0))
* **cli-manager:** add Codex and Gemini tabs for CLI management ([6f1ee01](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f1ee011fdef3aa323e35dfe537e6ea3fc03e087))
* **cli-manager:** add collaboration features to CodexTab ([f6ab4e4](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6ab4e48ebdc5cdafa132e1513c25be01964b81d))
* **cli-manager:** add experimental agent teams setting and update related tests ([6e1dd0c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6e1dd0c82a19f7fa40728fa7fa6d7f23605dfab0))
* **cli-manager:** add experimental MCP CLI features and max output tokens management ([84ac464](https://github.com/dyndynjyxa/aio-coding-hub/commit/84ac4645e7f4eb1e6235b85d7ca63f16b04c8ee6))
* **cli-manager:** add TOML configuration support for Codex ([826737a](https://github.com/dyndynjyxa/aio-coding-hub/commit/826737a89b74076d5bd90d960e876fb86a12e1cd))
* **cli-manager:** enhance Claude and WSL settings management ([3aacb6a](https://github.com/dyndynjyxa/aio-coding-hub/commit/3aacb6ad4a05b238a80064529445de398b1978b5))
* **cli-manager:** enhance Claude settings management and UI ([cbc1160](https://github.com/dyndynjyxa/aio-coding-hub/commit/cbc11605ee2cc64b9b08c51e4b33b70b72d168fa))
* **cli-manager:** enhance CodexTab with sandbox mode management ([759c19d](https://github.com/dyndynjyxa/aio-coding-hub/commit/759c19d02553f929f5a91c6051a0134e198e298e))
* **cli-manager:** enhance WSL settings and improve CLI manager performance ([be50456](https://github.com/dyndynjyxa/aio-coding-hub/commit/be50456432ef1f00cf1567c664a168779a23fc58))
* **cli-manager:** update GeneralTab and cache anomaly monitoring logic ([c84a1eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/c84a1eb6c312c938ba067551e7b3303bba761edf))
* **cli:** add CLI proxy startup recovery feature ([eb40a6f](https://github.com/dyndynjyxa/aio-coding-hub/commit/eb40a6f03d9a3b50b501795727a06b7ce9013fc1))
* **CliManager:** add response fixer configuration limits and UI inputs ([0023ad6](https://github.com/dyndynjyxa/aio-coding-hub/commit/0023ad69abf91f48a5144250e20b53ea0b2e24bf))
* **cli:** 添加 Claude 终端启动命令生成及剪贴板管理插件支持 ([ba3d3c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba3d3c20d8d1d40accc0d97780c4dae54d48c71a))
* **components:** enhance HomeRequestLogsPanel and RealtimeTraceCards with new formatting utilities ([e6a3550](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6a3550636e4278100a24d009e49ade1056733bc))
* **ConsolePage:** revamp console log display and functionality ([1d28397](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d28397e88c0b6d43a4d73b348c49c93cb18efde))
* **console:** 改进控制台界面并增加日志过滤和搜索功能 ([93badf8](https://github.com/dyndynjyxa/aio-coding-hub/commit/93badf8766c9a415c821c3f14084183d18d31687))
* **core:** improve workspace imports and startup resilience ([1d7dbf0](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d7dbf0b73cde2b78a70be945c6fd8e248b30c1d))
* **date-range:** refactor custom date range handling in HomeCostPanel and UsagePage ([95b1ad1](https://github.com/dyndynjyxa/aio-coding-hub/commit/95b1ad11b16a0f3e9b187b4883ef85a367d712e0))
* enhance provider model configuration with support for model whitelisting and mapping ([4f44510](https://github.com/dyndynjyxa/aio-coding-hub/commit/4f445106fefa10badae230de52c9fee09bd2486f))
* **failover-loop:** refactor context management for improved clarity and usability ([b96df01](https://github.com/dyndynjyxa/aio-coding-hub/commit/b96df013317c77ca610bb640a43b2d60058e6847))
* **failover-loop:** refactor error handling and logging in failover loop ([ef2662c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef2662c335ae4057c591b531b0784c86cd73951e))
* **gateway:** 支持会话绑定提供者的查询与恢复 ([cbfb17e](https://github.com/dyndynjyxa/aio-coding-hub/commit/cbfb17e385f6ee4689148c8630f56bed0c281c95))
* **gateway:** 支持强制指定请求的Provider并更新Claude终端启动逻辑 ([904a9aa](https://github.com/dyndynjyxa/aio-coding-hub/commit/904a9aa6805681fb5642e350105803ce01ae998b))
* **home:** add provider limit usage overview tab ([c224748](https://github.com/dyndynjyxa/aio-coding-hub/commit/c224748c590d1b242df045a7e81667a623d0ec0b))
* **home:** add provider limit usage overview tab ([6473253](https://github.com/dyndynjyxa/aio-coding-hub/commit/64732536a25e140193242e9829aacb2cce15f05d))
* **home:** add window start timestamps for provider limit usage ([69a91a2](https://github.com/dyndynjyxa/aio-coding-hub/commit/69a91a21567b6a2c9af2ac879936dd75a6004e37))
* **home:** enhance HomeCostPanel with data-testid attributes and update tests ([b1d23d2](https://github.com/dyndynjyxa/aio-coding-hub/commit/b1d23d2575afb25d3314911b0c6488a872f56b46))
* **home:** enhance status handling with failover support ([5aab8a9](https://github.com/dyndynjyxa/aio-coding-hub/commit/5aab8a92c4440062d18a3ecf495375d11a461660))
* **home:** enhance UI components and improve layout consistency ([70d9655](https://github.com/dyndynjyxa/aio-coding-hub/commit/70d9655cf1a740676e81bcb0a1e204b81abdd1c9))
* **home:** implement CLI proxy environment conflict checks ([aceff42](https://github.com/dyndynjyxa/aio-coding-hub/commit/aceff428537261c5132b5cb4ebdcb22d16efec82))
* **home:** implement CLI proxy environment conflict checks ([4c014ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/4c014ca5ab04dab7aa5ca38799d69d1a8eb3cc3e))
* **home:** implement window foreground detection for usage heatmap refresh ([4e66f35](https://github.com/dyndynjyxa/aio-coding-hub/commit/4e66f359f198ddddc52b6cd4c0ab8cdb59630a27))
* init ([7e30c40](https://github.com/dyndynjyxa/aio-coding-hub/commit/7e30c40727d50980bcd43c2f275419a74fa3b148))
* integrate PageHeader component across multiple pages for consistent UI ([330da27](https://github.com/dyndynjyxa/aio-coding-hub/commit/330da276f9ef8e91744a9534d59590a3a6fec5ff))
* mac最小化优化 ([1f11a5d](https://github.com/dyndynjyxa/aio-coding-hub/commit/1f11a5df9623f8cab90bb40f5c499c5d3569f01d))
* **mcp:** implement batch import for local skills and add global error reporting ([51e053c](https://github.com/dyndynjyxa/aio-coding-hub/commit/51e053c81f1db00696f0cb968860033deee12cce))
* **model-prices:** add model price alias rules ([60cbcc1](https://github.com/dyndynjyxa/aio-coding-hub/commit/60cbcc1c65ff025e79313facaf27e625a3de9997))
* **provider-editor:** enhance ProviderEditorDialog with limit configuration cards ([cc14a00](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc14a009303129793ceec840933155dcfad775d8))
* **providers:** collapse model mapping editors ([4672961](https://github.com/dyndynjyxa/aio-coding-hub/commit/4672961c8facbd27d715a762864c2bf4f32ac932))
* **providers:** 添加供应商标签功能，支持分类筛选 ([7671732](https://github.com/dyndynjyxa/aio-coding-hub/commit/76717329a119f9a65dc2f3a9dffe0a887063b0bd))
* **request-end:** enhance request logging and event emission ([809a684](https://github.com/dyndynjyxa/aio-coding-hub/commit/809a6843a2c2eb952bbb00f7c56694c908d99bd1))
* **request-end:** refactor request logging and event emission ([e56909e](https://github.com/dyndynjyxa/aio-coding-hub/commit/e56909e374c26608407a1d63d32c9d55f914b355))
* **responsive:** enhance layout and styling for improved mobile experience ([e22483a](https://github.com/dyndynjyxa/aio-coding-hub/commit/e22483a8da8c35822c6e5b0532aa1461cb68cf95))
* **route:** 支持展示同一 provider 的连续尝试次数及跳过状态 ([6659157](https://github.com/dyndynjyxa/aio-coding-hub/commit/6659157a12c3809b1e4d8de7bf95b95afe6ed383))
* **settings:** update Claude settings and remove deprecated options ([49e055a](https://github.com/dyndynjyxa/aio-coding-hub/commit/49e055aeb176819cb56a925aecbd51f362008762))
* **settings:** 新增缓存异常监测功能开关 ([ea681c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ea681c155346da3057f25e25867ab75a818f4157))
* **SkillsMarketPage:** enhance UI with tab selection and external links ([2849017](https://github.com/dyndynjyxa/aio-coding-hub/commit/2849017554128279822fef9b667d8ec166a08432))
* **sort-modes:** add toggle functionality for enabling/disabling providers in sort modes and update related services ([e6cd992](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6cd99241f5c86da3f62afebefeffdf33e62b8f0))
* **tauri:** add WSL support and listen modes ([a357007](https://github.com/dyndynjyxa/aio-coding-hub/commit/a35700753e9633493f6e939d1700ce979d635c93))
* **tauri:** update error handling and add thiserror dependency ([10d918a](https://github.com/dyndynjyxa/aio-coding-hub/commit/10d918a0a0a746b4d25c1c236f88c0110ec664c3))
* **tests:** add comprehensive unit tests for various components ([98fb022](https://github.com/dyndynjyxa/aio-coding-hub/commit/98fb022d1487e593e1fe50a1e9d02592671a4944))
* **theme:** add dark mode and native window theme sync ([588a373](https://github.com/dyndynjyxa/aio-coding-hub/commit/588a37311c456bd2182ac1676c8e9b309157353f))
* **ui:** align CLI manager with network and WSL settings ([ae5b5fc](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae5b5fc99330b55872e1c30da6e653d7433b7d48))
* **ui:** 统一 Loading/Empty/Error 状态反馈组件 ([b27f64a](https://github.com/dyndynjyxa/aio-coding-hub/commit/b27f64a79a1ad655a37992527794759f9f8c5370))
* **updater:** display release notes in update dialog ([b891ed7](https://github.com/dyndynjyxa/aio-coding-hub/commit/b891ed75a1b67a9df281917ffa94e684fb372664))
* **usage:** add cache rate trend functionality to UsagePage ([5535e7d](https://github.com/dyndynjyxa/aio-coding-hub/commit/5535e7d3480237284037ef841d2878bfa6a180f5))
* **usage:** add cost tracking to usage statistics ([765ea8a](https://github.com/dyndynjyxa/aio-coding-hub/commit/765ea8ae06efe2cc39c6eac7e95dccfbba4de541))
* **usage:** add summary stat cards with filter layout optimization ([f6c9206](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6c9206ba6e0b424748ba05f0ac749884d5bf0d3))
* **workspaces:** add WorkspacesPage route and enhance CLI manager settings ([2784072](https://github.com/dyndynjyxa/aio-coding-hub/commit/2784072e26748aa21f679c245f74cf3063f4177c))
* **workspaces:** enhance MCP and Prompts management with workspace support ([791ee6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/791ee6c88b06c16c596b82dcf7956ea5b98dc18d))
* **workspaces:** improve workspace switching and management functionality ([496c9c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/496c9c35daa23be4d2c44f0653bb11bcb5206e1f))
* **wsl:** 支持WSL宿主机地址模式自动检测与自定义 ([f809e86](https://github.com/dyndynjyxa/aio-coding-hub/commit/f809e86ab3690214ba2407c14e4dee1febd9ab14))
* **wsl:** 添加启动时自动检测和配置 WSL 环境功能 ([707bd55](https://github.com/dyndynjyxa/aio-coding-hub/commit/707bd5518fd9881715ec5f3df0ef9250970570bb))


### Bug Fixes

* **ci:** add explicit permissions to release-please job ([8c77c04](https://github.com/dyndynjyxa/aio-coding-hub/commit/8c77c041b0d6f5081abf6ea9a39e1031dc56ad75))
* **ci:** ensure consistent use of RELEASE_PLEASE_TOKEN in release workflow for GitHub actions ([f8a439d](https://github.com/dyndynjyxa/aio-coding-hub/commit/f8a439d213346a907dfc7355b6b6caf4b4194799))
* **ci:** quote FALLBACK_NOTES value in release workflow ([536896c](https://github.com/dyndynjyxa/aio-coding-hub/commit/536896ca4072f48002e8c83b91d78164ed40384f))
* **ci:** revert release-please job to original config ([0d1afad](https://github.com/dyndynjyxa/aio-coding-hub/commit/0d1afade58707c53554f7680dd7455d6ef547187))
* **ci:** update release workflow to consistently use RELEASE_PLEASE_TOKEN for GitHub actions ([2c45c46](https://github.com/dyndynjyxa/aio-coding-hub/commit/2c45c461ddf157a4b781c146a0ebed5cd9dc1a44))
* **ci:** update release workflow to use RELEASE_PLEASE_TOKEN for GitHub actions ([48ec3ce](https://github.com/dyndynjyxa/aio-coding-hub/commit/48ec3ce92e28702ee9920eaa42d2f7374b07be31))
* **ci:** use PAT for release-please to fix permission issue ([a2919f2](https://github.com/dyndynjyxa/aio-coding-hub/commit/a2919f2446d8fcb1c4c7ac8dc6a1b4f63f705157))
* **ci:** 修复CI流程中正则表达式重复问题 ([609f2ea](https://github.com/dyndynjyxa/aio-coding-hub/commit/609f2ea657034a58b6f175b995e05431829f6dd9))
* **ci:** 修复正则表达式转义字符问题 ([ba805c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba805c3b2f147f153cbe39042b078547eb36da5a))
* **ci:** 修正发布流程中更新器 JSON 文件参数名称 ([9a4c0f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/9a4c0f063bd645e3286e67f1607334a84a5cb6fd))
* **ClaudeModelValidationDialog:** adjust dialog max-width for responsive design ([11bdfd0](https://github.com/dyndynjyxa/aio-coding-hub/commit/11bdfd0df1b51923a561211f6cc6d5383eedf76c))
* **cli_manager:** 修复 run_in_login_shell 的平台兼容性处理 ([9f81433](https://github.com/dyndynjyxa/aio-coding-hub/commit/9f814337d08f36c08e37486f95b9c9da62880fc1))
* **codex-tab:** sandbox_mode danger-full-access 选择不生效及高级配置不回显 ([3943401](https://github.com/dyndynjyxa/aio-coding-hub/commit/3943401524c8ef6a179e65e8ee62fd2ac4f784cb))
* **deps:** 升级 react-router-dom 7.11→7.13 修复 XSS 漏洞 ([c4db630](https://github.com/dyndynjyxa/aio-coding-hub/commit/c4db630b4478cab2f1180f03b324cfc0af9d42e3))
* **domain:** 修复 SSE 流错误处理逻辑 ([4446939](https://github.com/dyndynjyxa/aio-coding-hub/commit/4446939c457735727f96dce0b09c91ae6b616c1d))
* **gateway:** finalize failed requests and align error codes ([871de9c](https://github.com/dyndynjyxa/aio-coding-hub/commit/871de9c9c9e06ed6cb03026ceb21dfc4b0ba18d3))
* **gateway:** reject forwarding when CLI proxy disabled ([c9edd10](https://github.com/dyndynjyxa/aio-coding-hub/commit/c9edd10cd2f41ef86c8c4c8a3ca2262c8bcb09ef))
* **gateway:** 调整上游连接超时与错误切换逻辑 ([a6870ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6870ae14b6b5bb8edce814ca37da2000e84f6b4))
* **home:** update HomeCostPanel with accessibility labels and improve test assertions ([0bda6c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/0bda6c1ce8aec6920b30810882f3a804c62ab732))
* **proxy:** 允许 resolve_session_bound_provider_id 函数有多个参数 ([705457e](https://github.com/dyndynjyxa/aio-coding-hub/commit/705457eac5092ccd98a0f3523ea1986c67dd9d9c))
* **skills:** 移除批量导入功能并改为刷新本机列表 ([45b9618](https://github.com/dyndynjyxa/aio-coding-hub/commit/45b961847b458f72f2bc062bdcdfef93c8722da8))
* **tests:** update RealtimeTraceCards test to reflect token display format change ([f43633e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f43633e5381f44447bbf8aa64c1065d222687c59))
* **theme:** 修复 dark mode 切换闪烁回退问题 ([ac9500c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac9500c04a108c5bc15921353b691394a689ac54))
* **usage:** align cache creation ttl to 5m only ([8d28bcd](https://github.com/dyndynjyxa/aio-coding-hub/commit/8d28bcd2f5d7f8d6bac1a7f65f974c04c5fce337))
* **wsl:** 修复 WSL 脚本错误消息编码和路径写入问题 ([bccd5c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/bccd5c93110641898e04cd1cde031df604af637b))
* **wsl:** 修复WSL适配器IPv4解析和状态脚本处理 ([06ae9b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/06ae9b1f462572550f327c3afe4034179b1da491))


### Code Refactoring

* **infra:** 移除 request_attempt_logs 独立表，改用 request_logs.attempts_json 派生 ([3fb2627](https://github.com/dyndynjyxa/aio-coding-hub/commit/3fb2627407e1b9b57758a8a40d3a7d7393278e1b))

## [0.29.1](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.29.0...aio-coding-hub-v0.29.1) (2026-02-23)


### Features

* **ClaudeModelValidationDialog:** 添加综合协议验证功能与UI展示 ([b6ede30](https://github.com/dyndynjyxa/aio-coding-hub/commit/b6ede30e61e19f9af69e8d7153cbc711b3e3a180))
* **gateway:** 支持会话绑定提供者的查询与恢复 ([cbfb17e](https://github.com/dyndynjyxa/aio-coding-hub/commit/cbfb17e385f6ee4689148c8630f56bed0c281c95))


### Bug Fixes

* **ci:** 修正发布流程中更新器 JSON 文件参数名称 ([9a4c0f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/9a4c0f063bd645e3286e67f1607334a84a5cb6fd))
* **proxy:** 允许 resolve_session_bound_provider_id 函数有多个参数 ([705457e](https://github.com/dyndynjyxa/aio-coding-hub/commit/705457eac5092ccd98a0f3523ea1986c67dd9d9c))

## [0.29.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.28.2...aio-coding-hub-v0.29.0) (2026-02-22)


### ⚠ BREAKING CHANGES

* **infra:** request_attempt_logs 表不再创建或写入

### Features

* add TextEvidenceSection component for improved output display in ClaudeModelValidationResultPanel ([47be119](https://github.com/dyndynjyxa/aio-coding-hub/commit/47be119a83c365b3e7b41f22308be7550ecaede5))
* **app:** add LogsPage route and enhance CLI manager settings ([6796715](https://github.com/dyndynjyxa/aio-coding-hub/commit/679671526be935129211978b3598a3962f7e2a8e))
* **app:** 优化页面路由懒加载和加载反馈体验 ([504256e](https://github.com/dyndynjyxa/aio-coding-hub/commit/504256e11ed517797405f8bb4ab3d353607cb1ae))
* **app:** 添加应用心跳机制与监听支持 ([df78171](https://github.com/dyndynjyxa/aio-coding-hub/commit/df781712d16a2f5ccc99e37a687d39cb4e4f56c5))
* **cache-anomaly-monitor:** implement cache anomaly monitoring feature ([59d69d1](https://github.com/dyndynjyxa/aio-coding-hub/commit/59d69d1528d888a84d71038873321f3273d4ded1))
* **charts:** migrate from ECharts to Recharts for improved charting capabilities ([b747b61](https://github.com/dyndynjyxa/aio-coding-hub/commit/b747b61a8dd587043a7e8de01a5f9b3d64ead7ae))
* **claude-validation:** add signature and caching roundtrip probes ([15badee](https://github.com/dyndynjyxa/aio-coding-hub/commit/15badee08b0c14f71695e6e71f0b165e4844371c))
* **ClaudeModelValidation:** enhance cross-provider validation and UI feedback ([bf83c7e](https://github.com/dyndynjyxa/aio-coding-hub/commit/bf83c7e03c7edf78795cd51a943c01a88e0b17d7))
* **ClaudeModelValidation:** enhance output token validation and error handling ([d245288](https://github.com/dyndynjyxa/aio-coding-hub/commit/d245288d7a4937ca7b0213ebd79d9c0d5e3c76b4))
* **ClaudeModelValidation:** implement cross-provider signature validation and enhance request handling ([2e102d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/2e102d4f3fd2745e4480a5884272baeafe66b6d0))
* **cli-manager:** add Codex and Gemini tabs for CLI management ([6f1ee01](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f1ee011fdef3aa323e35dfe537e6ea3fc03e087))
* **cli-manager:** add collaboration features to CodexTab ([f6ab4e4](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6ab4e48ebdc5cdafa132e1513c25be01964b81d))
* **cli-manager:** add experimental agent teams setting and update related tests ([6e1dd0c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6e1dd0c82a19f7fa40728fa7fa6d7f23605dfab0))
* **cli-manager:** add experimental MCP CLI features and max output tokens management ([84ac464](https://github.com/dyndynjyxa/aio-coding-hub/commit/84ac4645e7f4eb1e6235b85d7ca63f16b04c8ee6))
* **cli-manager:** add TOML configuration support for Codex ([826737a](https://github.com/dyndynjyxa/aio-coding-hub/commit/826737a89b74076d5bd90d960e876fb86a12e1cd))
* **cli-manager:** enhance Claude and WSL settings management ([3aacb6a](https://github.com/dyndynjyxa/aio-coding-hub/commit/3aacb6ad4a05b238a80064529445de398b1978b5))
* **cli-manager:** enhance Claude settings management and UI ([cbc1160](https://github.com/dyndynjyxa/aio-coding-hub/commit/cbc11605ee2cc64b9b08c51e4b33b70b72d168fa))
* **cli-manager:** enhance CodexTab with sandbox mode management ([759c19d](https://github.com/dyndynjyxa/aio-coding-hub/commit/759c19d02553f929f5a91c6051a0134e198e298e))
* **cli-manager:** enhance WSL settings and improve CLI manager performance ([be50456](https://github.com/dyndynjyxa/aio-coding-hub/commit/be50456432ef1f00cf1567c664a168779a23fc58))
* **cli-manager:** update GeneralTab and cache anomaly monitoring logic ([c84a1eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/c84a1eb6c312c938ba067551e7b3303bba761edf))
* **cli:** add CLI proxy startup recovery feature ([eb40a6f](https://github.com/dyndynjyxa/aio-coding-hub/commit/eb40a6f03d9a3b50b501795727a06b7ce9013fc1))
* **CliManager:** add response fixer configuration limits and UI inputs ([0023ad6](https://github.com/dyndynjyxa/aio-coding-hub/commit/0023ad69abf91f48a5144250e20b53ea0b2e24bf))
* **cli:** 添加 Claude 终端启动命令生成及剪贴板管理插件支持 ([ba3d3c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba3d3c20d8d1d40accc0d97780c4dae54d48c71a))
* **components:** enhance HomeRequestLogsPanel and RealtimeTraceCards with new formatting utilities ([e6a3550](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6a3550636e4278100a24d009e49ade1056733bc))
* **ConsolePage:** revamp console log display and functionality ([1d28397](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d28397e88c0b6d43a4d73b348c49c93cb18efde))
* **console:** 改进控制台界面并增加日志过滤和搜索功能 ([93badf8](https://github.com/dyndynjyxa/aio-coding-hub/commit/93badf8766c9a415c821c3f14084183d18d31687))
* **core:** improve workspace imports and startup resilience ([1d7dbf0](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d7dbf0b73cde2b78a70be945c6fd8e248b30c1d))
* **date-range:** refactor custom date range handling in HomeCostPanel and UsagePage ([95b1ad1](https://github.com/dyndynjyxa/aio-coding-hub/commit/95b1ad11b16a0f3e9b187b4883ef85a367d712e0))
* enhance provider model configuration with support for model whitelisting and mapping ([4f44510](https://github.com/dyndynjyxa/aio-coding-hub/commit/4f445106fefa10badae230de52c9fee09bd2486f))
* **failover-loop:** refactor context management for improved clarity and usability ([b96df01](https://github.com/dyndynjyxa/aio-coding-hub/commit/b96df013317c77ca610bb640a43b2d60058e6847))
* **failover-loop:** refactor error handling and logging in failover loop ([ef2662c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef2662c335ae4057c591b531b0784c86cd73951e))
* **gateway:** 支持强制指定请求的Provider并更新Claude终端启动逻辑 ([904a9aa](https://github.com/dyndynjyxa/aio-coding-hub/commit/904a9aa6805681fb5642e350105803ce01ae998b))
* **home:** add provider limit usage overview tab ([c224748](https://github.com/dyndynjyxa/aio-coding-hub/commit/c224748c590d1b242df045a7e81667a623d0ec0b))
* **home:** add provider limit usage overview tab ([6473253](https://github.com/dyndynjyxa/aio-coding-hub/commit/64732536a25e140193242e9829aacb2cce15f05d))
* **home:** add window start timestamps for provider limit usage ([69a91a2](https://github.com/dyndynjyxa/aio-coding-hub/commit/69a91a21567b6a2c9af2ac879936dd75a6004e37))
* **home:** enhance HomeCostPanel with data-testid attributes and update tests ([b1d23d2](https://github.com/dyndynjyxa/aio-coding-hub/commit/b1d23d2575afb25d3314911b0c6488a872f56b46))
* **home:** enhance status handling with failover support ([5aab8a9](https://github.com/dyndynjyxa/aio-coding-hub/commit/5aab8a92c4440062d18a3ecf495375d11a461660))
* **home:** enhance UI components and improve layout consistency ([70d9655](https://github.com/dyndynjyxa/aio-coding-hub/commit/70d9655cf1a740676e81bcb0a1e204b81abdd1c9))
* **home:** implement CLI proxy environment conflict checks ([aceff42](https://github.com/dyndynjyxa/aio-coding-hub/commit/aceff428537261c5132b5cb4ebdcb22d16efec82))
* **home:** implement CLI proxy environment conflict checks ([4c014ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/4c014ca5ab04dab7aa5ca38799d69d1a8eb3cc3e))
* **home:** implement window foreground detection for usage heatmap refresh ([4e66f35](https://github.com/dyndynjyxa/aio-coding-hub/commit/4e66f359f198ddddc52b6cd4c0ab8cdb59630a27))
* init ([7e30c40](https://github.com/dyndynjyxa/aio-coding-hub/commit/7e30c40727d50980bcd43c2f275419a74fa3b148))
* integrate PageHeader component across multiple pages for consistent UI ([330da27](https://github.com/dyndynjyxa/aio-coding-hub/commit/330da276f9ef8e91744a9534d59590a3a6fec5ff))
* mac最小化优化 ([1f11a5d](https://github.com/dyndynjyxa/aio-coding-hub/commit/1f11a5df9623f8cab90bb40f5c499c5d3569f01d))
* **mcp:** implement batch import for local skills and add global error reporting ([51e053c](https://github.com/dyndynjyxa/aio-coding-hub/commit/51e053c81f1db00696f0cb968860033deee12cce))
* **model-prices:** add model price alias rules ([60cbcc1](https://github.com/dyndynjyxa/aio-coding-hub/commit/60cbcc1c65ff025e79313facaf27e625a3de9997))
* **provider-editor:** enhance ProviderEditorDialog with limit configuration cards ([cc14a00](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc14a009303129793ceec840933155dcfad775d8))
* **providers:** collapse model mapping editors ([4672961](https://github.com/dyndynjyxa/aio-coding-hub/commit/4672961c8facbd27d715a762864c2bf4f32ac932))
* **providers:** 添加供应商标签功能，支持分类筛选 ([7671732](https://github.com/dyndynjyxa/aio-coding-hub/commit/76717329a119f9a65dc2f3a9dffe0a887063b0bd))
* **request-end:** enhance request logging and event emission ([809a684](https://github.com/dyndynjyxa/aio-coding-hub/commit/809a6843a2c2eb952bbb00f7c56694c908d99bd1))
* **request-end:** refactor request logging and event emission ([e56909e](https://github.com/dyndynjyxa/aio-coding-hub/commit/e56909e374c26608407a1d63d32c9d55f914b355))
* **responsive:** enhance layout and styling for improved mobile experience ([e22483a](https://github.com/dyndynjyxa/aio-coding-hub/commit/e22483a8da8c35822c6e5b0532aa1461cb68cf95))
* **route:** 支持展示同一 provider 的连续尝试次数及跳过状态 ([6659157](https://github.com/dyndynjyxa/aio-coding-hub/commit/6659157a12c3809b1e4d8de7bf95b95afe6ed383))
* **settings:** update Claude settings and remove deprecated options ([49e055a](https://github.com/dyndynjyxa/aio-coding-hub/commit/49e055aeb176819cb56a925aecbd51f362008762))
* **settings:** 新增缓存异常监测功能开关 ([ea681c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ea681c155346da3057f25e25867ab75a818f4157))
* **SkillsMarketPage:** enhance UI with tab selection and external links ([2849017](https://github.com/dyndynjyxa/aio-coding-hub/commit/2849017554128279822fef9b667d8ec166a08432))
* **sort-modes:** add toggle functionality for enabling/disabling providers in sort modes and update related services ([e6cd992](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6cd99241f5c86da3f62afebefeffdf33e62b8f0))
* **tauri:** add WSL support and listen modes ([a357007](https://github.com/dyndynjyxa/aio-coding-hub/commit/a35700753e9633493f6e939d1700ce979d635c93))
* **tauri:** update error handling and add thiserror dependency ([10d918a](https://github.com/dyndynjyxa/aio-coding-hub/commit/10d918a0a0a746b4d25c1c236f88c0110ec664c3))
* **tests:** add comprehensive unit tests for various components ([98fb022](https://github.com/dyndynjyxa/aio-coding-hub/commit/98fb022d1487e593e1fe50a1e9d02592671a4944))
* **theme:** add dark mode and native window theme sync ([588a373](https://github.com/dyndynjyxa/aio-coding-hub/commit/588a37311c456bd2182ac1676c8e9b309157353f))
* **ui:** align CLI manager with network and WSL settings ([ae5b5fc](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae5b5fc99330b55872e1c30da6e653d7433b7d48))
* **ui:** 统一 Loading/Empty/Error 状态反馈组件 ([b27f64a](https://github.com/dyndynjyxa/aio-coding-hub/commit/b27f64a79a1ad655a37992527794759f9f8c5370))
* **updater:** display release notes in update dialog ([b891ed7](https://github.com/dyndynjyxa/aio-coding-hub/commit/b891ed75a1b67a9df281917ffa94e684fb372664))
* **usage:** add cache rate trend functionality to UsagePage ([5535e7d](https://github.com/dyndynjyxa/aio-coding-hub/commit/5535e7d3480237284037ef841d2878bfa6a180f5))
* **usage:** add cost tracking to usage statistics ([765ea8a](https://github.com/dyndynjyxa/aio-coding-hub/commit/765ea8ae06efe2cc39c6eac7e95dccfbba4de541))
* **usage:** add summary stat cards with filter layout optimization ([f6c9206](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6c9206ba6e0b424748ba05f0ac749884d5bf0d3))
* **workspaces:** add WorkspacesPage route and enhance CLI manager settings ([2784072](https://github.com/dyndynjyxa/aio-coding-hub/commit/2784072e26748aa21f679c245f74cf3063f4177c))
* **workspaces:** enhance MCP and Prompts management with workspace support ([791ee6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/791ee6c88b06c16c596b82dcf7956ea5b98dc18d))
* **workspaces:** improve workspace switching and management functionality ([496c9c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/496c9c35daa23be4d2c44f0653bb11bcb5206e1f))
* **wsl:** 支持WSL宿主机地址模式自动检测与自定义 ([f809e86](https://github.com/dyndynjyxa/aio-coding-hub/commit/f809e86ab3690214ba2407c14e4dee1febd9ab14))
* **wsl:** 添加启动时自动检测和配置 WSL 环境功能 ([707bd55](https://github.com/dyndynjyxa/aio-coding-hub/commit/707bd5518fd9881715ec5f3df0ef9250970570bb))


### Bug Fixes

* **ci:** add explicit permissions to release-please job ([8c77c04](https://github.com/dyndynjyxa/aio-coding-hub/commit/8c77c041b0d6f5081abf6ea9a39e1031dc56ad75))
* **ci:** ensure consistent use of RELEASE_PLEASE_TOKEN in release workflow for GitHub actions ([f8a439d](https://github.com/dyndynjyxa/aio-coding-hub/commit/f8a439d213346a907dfc7355b6b6caf4b4194799))
* **ci:** quote FALLBACK_NOTES value in release workflow ([536896c](https://github.com/dyndynjyxa/aio-coding-hub/commit/536896ca4072f48002e8c83b91d78164ed40384f))
* **ci:** revert release-please job to original config ([0d1afad](https://github.com/dyndynjyxa/aio-coding-hub/commit/0d1afade58707c53554f7680dd7455d6ef547187))
* **ci:** update release workflow to consistently use RELEASE_PLEASE_TOKEN for GitHub actions ([2c45c46](https://github.com/dyndynjyxa/aio-coding-hub/commit/2c45c461ddf157a4b781c146a0ebed5cd9dc1a44))
* **ci:** update release workflow to use RELEASE_PLEASE_TOKEN for GitHub actions ([48ec3ce](https://github.com/dyndynjyxa/aio-coding-hub/commit/48ec3ce92e28702ee9920eaa42d2f7374b07be31))
* **ci:** use PAT for release-please to fix permission issue ([a2919f2](https://github.com/dyndynjyxa/aio-coding-hub/commit/a2919f2446d8fcb1c4c7ac8dc6a1b4f63f705157))
* **ci:** 修复CI流程中正则表达式重复问题 ([609f2ea](https://github.com/dyndynjyxa/aio-coding-hub/commit/609f2ea657034a58b6f175b995e05431829f6dd9))
* **ci:** 修复正则表达式转义字符问题 ([ba805c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba805c3b2f147f153cbe39042b078547eb36da5a))
* **ClaudeModelValidationDialog:** adjust dialog max-width for responsive design ([11bdfd0](https://github.com/dyndynjyxa/aio-coding-hub/commit/11bdfd0df1b51923a561211f6cc6d5383eedf76c))
* **cli_manager:** 修复 run_in_login_shell 的平台兼容性处理 ([9f81433](https://github.com/dyndynjyxa/aio-coding-hub/commit/9f814337d08f36c08e37486f95b9c9da62880fc1))
* **codex-tab:** sandbox_mode danger-full-access 选择不生效及高级配置不回显 ([3943401](https://github.com/dyndynjyxa/aio-coding-hub/commit/3943401524c8ef6a179e65e8ee62fd2ac4f784cb))
* **deps:** 升级 react-router-dom 7.11→7.13 修复 XSS 漏洞 ([c4db630](https://github.com/dyndynjyxa/aio-coding-hub/commit/c4db630b4478cab2f1180f03b324cfc0af9d42e3))
* **domain:** 修复 SSE 流错误处理逻辑 ([4446939](https://github.com/dyndynjyxa/aio-coding-hub/commit/4446939c457735727f96dce0b09c91ae6b616c1d))
* **gateway:** finalize failed requests and align error codes ([871de9c](https://github.com/dyndynjyxa/aio-coding-hub/commit/871de9c9c9e06ed6cb03026ceb21dfc4b0ba18d3))
* **gateway:** reject forwarding when CLI proxy disabled ([c9edd10](https://github.com/dyndynjyxa/aio-coding-hub/commit/c9edd10cd2f41ef86c8c4c8a3ca2262c8bcb09ef))
* **gateway:** 调整上游连接超时与错误切换逻辑 ([a6870ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6870ae14b6b5bb8edce814ca37da2000e84f6b4))
* **home:** update HomeCostPanel with accessibility labels and improve test assertions ([0bda6c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/0bda6c1ce8aec6920b30810882f3a804c62ab732))
* **skills:** 移除批量导入功能并改为刷新本机列表 ([45b9618](https://github.com/dyndynjyxa/aio-coding-hub/commit/45b961847b458f72f2bc062bdcdfef93c8722da8))
* **tests:** update RealtimeTraceCards test to reflect token display format change ([f43633e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f43633e5381f44447bbf8aa64c1065d222687c59))
* **theme:** 修复 dark mode 切换闪烁回退问题 ([ac9500c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac9500c04a108c5bc15921353b691394a689ac54))
* **usage:** align cache creation ttl to 5m only ([8d28bcd](https://github.com/dyndynjyxa/aio-coding-hub/commit/8d28bcd2f5d7f8d6bac1a7f65f974c04c5fce337))
* **wsl:** 修复 WSL 脚本错误消息编码和路径写入问题 ([bccd5c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/bccd5c93110641898e04cd1cde031df604af637b))
* **wsl:** 修复WSL适配器IPv4解析和状态脚本处理 ([06ae9b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/06ae9b1f462572550f327c3afe4034179b1da491))


### Code Refactoring

* **infra:** 移除 request_attempt_logs 独立表，改用 request_logs.attempts_json 派生 ([3fb2627](https://github.com/dyndynjyxa/aio-coding-hub/commit/3fb2627407e1b9b57758a8a40d3a7d7393278e1b))

## [0.28.2](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.28.1...aio-coding-hub-v0.28.2) (2026-02-22)


### Features

* **app:** 添加应用心跳机制与监听支持 ([df78171](https://github.com/dyndynjyxa/aio-coding-hub/commit/df781712d16a2f5ccc99e37a687d39cb4e4f56c5))
* **wsl:** 支持WSL宿主机地址模式自动检测与自定义 ([f809e86](https://github.com/dyndynjyxa/aio-coding-hub/commit/f809e86ab3690214ba2407c14e4dee1febd9ab14))
* **wsl:** 添加启动时自动检测和配置 WSL 环境功能 ([707bd55](https://github.com/dyndynjyxa/aio-coding-hub/commit/707bd5518fd9881715ec5f3df0ef9250970570bb))


### Bug Fixes

* **wsl:** 修复 WSL 脚本错误消息编码和路径写入问题 ([bccd5c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/bccd5c93110641898e04cd1cde031df604af637b))
* **wsl:** 修复WSL适配器IPv4解析和状态脚本处理 ([06ae9b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/06ae9b1f462572550f327c3afe4034179b1da491))

## [0.28.1](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.28.0...aio-coding-hub-v0.28.1) (2026-02-16)


### Bug Fixes

* **domain:** 修复 SSE 流错误处理逻辑 ([4446939](https://github.com/dyndynjyxa/aio-coding-hub/commit/4446939c457735727f96dce0b09c91ae6b616c1d))

## [0.28.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.27.0...aio-coding-hub-v0.28.0) (2026-02-13)


### ⚠ BREAKING CHANGES

* **infra:** request_attempt_logs 表不再创建或写入

### Features

* add TextEvidenceSection component for improved output display in ClaudeModelValidationResultPanel ([47be119](https://github.com/dyndynjyxa/aio-coding-hub/commit/47be119a83c365b3e7b41f22308be7550ecaede5))
* **app:** add LogsPage route and enhance CLI manager settings ([6796715](https://github.com/dyndynjyxa/aio-coding-hub/commit/679671526be935129211978b3598a3962f7e2a8e))
* **app:** 优化页面路由懒加载和加载反馈体验 ([504256e](https://github.com/dyndynjyxa/aio-coding-hub/commit/504256e11ed517797405f8bb4ab3d353607cb1ae))
* **cache-anomaly-monitor:** implement cache anomaly monitoring feature ([59d69d1](https://github.com/dyndynjyxa/aio-coding-hub/commit/59d69d1528d888a84d71038873321f3273d4ded1))
* **charts:** migrate from ECharts to Recharts for improved charting capabilities ([b747b61](https://github.com/dyndynjyxa/aio-coding-hub/commit/b747b61a8dd587043a7e8de01a5f9b3d64ead7ae))
* **claude-validation:** add signature and caching roundtrip probes ([15badee](https://github.com/dyndynjyxa/aio-coding-hub/commit/15badee08b0c14f71695e6e71f0b165e4844371c))
* **ClaudeModelValidation:** enhance cross-provider validation and UI feedback ([bf83c7e](https://github.com/dyndynjyxa/aio-coding-hub/commit/bf83c7e03c7edf78795cd51a943c01a88e0b17d7))
* **ClaudeModelValidation:** enhance output token validation and error handling ([d245288](https://github.com/dyndynjyxa/aio-coding-hub/commit/d245288d7a4937ca7b0213ebd79d9c0d5e3c76b4))
* **ClaudeModelValidation:** implement cross-provider signature validation and enhance request handling ([2e102d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/2e102d4f3fd2745e4480a5884272baeafe66b6d0))
* **cli-manager:** add Codex and Gemini tabs for CLI management ([6f1ee01](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f1ee011fdef3aa323e35dfe537e6ea3fc03e087))
* **cli-manager:** add collaboration features to CodexTab ([f6ab4e4](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6ab4e48ebdc5cdafa132e1513c25be01964b81d))
* **cli-manager:** add experimental agent teams setting and update related tests ([6e1dd0c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6e1dd0c82a19f7fa40728fa7fa6d7f23605dfab0))
* **cli-manager:** add experimental MCP CLI features and max output tokens management ([84ac464](https://github.com/dyndynjyxa/aio-coding-hub/commit/84ac4645e7f4eb1e6235b85d7ca63f16b04c8ee6))
* **cli-manager:** add TOML configuration support for Codex ([826737a](https://github.com/dyndynjyxa/aio-coding-hub/commit/826737a89b74076d5bd90d960e876fb86a12e1cd))
* **cli-manager:** enhance Claude and WSL settings management ([3aacb6a](https://github.com/dyndynjyxa/aio-coding-hub/commit/3aacb6ad4a05b238a80064529445de398b1978b5))
* **cli-manager:** enhance Claude settings management and UI ([cbc1160](https://github.com/dyndynjyxa/aio-coding-hub/commit/cbc11605ee2cc64b9b08c51e4b33b70b72d168fa))
* **cli-manager:** enhance CodexTab with sandbox mode management ([759c19d](https://github.com/dyndynjyxa/aio-coding-hub/commit/759c19d02553f929f5a91c6051a0134e198e298e))
* **cli-manager:** enhance WSL settings and improve CLI manager performance ([be50456](https://github.com/dyndynjyxa/aio-coding-hub/commit/be50456432ef1f00cf1567c664a168779a23fc58))
* **cli-manager:** update GeneralTab and cache anomaly monitoring logic ([c84a1eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/c84a1eb6c312c938ba067551e7b3303bba761edf))
* **cli:** add CLI proxy startup recovery feature ([eb40a6f](https://github.com/dyndynjyxa/aio-coding-hub/commit/eb40a6f03d9a3b50b501795727a06b7ce9013fc1))
* **CliManager:** add response fixer configuration limits and UI inputs ([0023ad6](https://github.com/dyndynjyxa/aio-coding-hub/commit/0023ad69abf91f48a5144250e20b53ea0b2e24bf))
* **cli:** 添加 Claude 终端启动命令生成及剪贴板管理插件支持 ([ba3d3c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba3d3c20d8d1d40accc0d97780c4dae54d48c71a))
* **components:** enhance HomeRequestLogsPanel and RealtimeTraceCards with new formatting utilities ([e6a3550](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6a3550636e4278100a24d009e49ade1056733bc))
* **ConsolePage:** revamp console log display and functionality ([1d28397](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d28397e88c0b6d43a4d73b348c49c93cb18efde))
* **console:** 改进控制台界面并增加日志过滤和搜索功能 ([93badf8](https://github.com/dyndynjyxa/aio-coding-hub/commit/93badf8766c9a415c821c3f14084183d18d31687))
* **core:** improve workspace imports and startup resilience ([1d7dbf0](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d7dbf0b73cde2b78a70be945c6fd8e248b30c1d))
* **date-range:** refactor custom date range handling in HomeCostPanel and UsagePage ([95b1ad1](https://github.com/dyndynjyxa/aio-coding-hub/commit/95b1ad11b16a0f3e9b187b4883ef85a367d712e0))
* enhance provider model configuration with support for model whitelisting and mapping ([4f44510](https://github.com/dyndynjyxa/aio-coding-hub/commit/4f445106fefa10badae230de52c9fee09bd2486f))
* **failover-loop:** refactor context management for improved clarity and usability ([b96df01](https://github.com/dyndynjyxa/aio-coding-hub/commit/b96df013317c77ca610bb640a43b2d60058e6847))
* **failover-loop:** refactor error handling and logging in failover loop ([ef2662c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef2662c335ae4057c591b531b0784c86cd73951e))
* **gateway:** 支持强制指定请求的Provider并更新Claude终端启动逻辑 ([904a9aa](https://github.com/dyndynjyxa/aio-coding-hub/commit/904a9aa6805681fb5642e350105803ce01ae998b))
* **home:** add provider limit usage overview tab ([c224748](https://github.com/dyndynjyxa/aio-coding-hub/commit/c224748c590d1b242df045a7e81667a623d0ec0b))
* **home:** add provider limit usage overview tab ([6473253](https://github.com/dyndynjyxa/aio-coding-hub/commit/64732536a25e140193242e9829aacb2cce15f05d))
* **home:** add window start timestamps for provider limit usage ([69a91a2](https://github.com/dyndynjyxa/aio-coding-hub/commit/69a91a21567b6a2c9af2ac879936dd75a6004e37))
* **home:** enhance HomeCostPanel with data-testid attributes and update tests ([b1d23d2](https://github.com/dyndynjyxa/aio-coding-hub/commit/b1d23d2575afb25d3314911b0c6488a872f56b46))
* **home:** enhance status handling with failover support ([5aab8a9](https://github.com/dyndynjyxa/aio-coding-hub/commit/5aab8a92c4440062d18a3ecf495375d11a461660))
* **home:** enhance UI components and improve layout consistency ([70d9655](https://github.com/dyndynjyxa/aio-coding-hub/commit/70d9655cf1a740676e81bcb0a1e204b81abdd1c9))
* **home:** implement CLI proxy environment conflict checks ([aceff42](https://github.com/dyndynjyxa/aio-coding-hub/commit/aceff428537261c5132b5cb4ebdcb22d16efec82))
* **home:** implement CLI proxy environment conflict checks ([4c014ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/4c014ca5ab04dab7aa5ca38799d69d1a8eb3cc3e))
* **home:** implement window foreground detection for usage heatmap refresh ([4e66f35](https://github.com/dyndynjyxa/aio-coding-hub/commit/4e66f359f198ddddc52b6cd4c0ab8cdb59630a27))
* init ([7e30c40](https://github.com/dyndynjyxa/aio-coding-hub/commit/7e30c40727d50980bcd43c2f275419a74fa3b148))
* integrate PageHeader component across multiple pages for consistent UI ([330da27](https://github.com/dyndynjyxa/aio-coding-hub/commit/330da276f9ef8e91744a9534d59590a3a6fec5ff))
* mac最小化优化 ([1f11a5d](https://github.com/dyndynjyxa/aio-coding-hub/commit/1f11a5df9623f8cab90bb40f5c499c5d3569f01d))
* **mcp:** implement batch import for local skills and add global error reporting ([51e053c](https://github.com/dyndynjyxa/aio-coding-hub/commit/51e053c81f1db00696f0cb968860033deee12cce))
* **model-prices:** add model price alias rules ([60cbcc1](https://github.com/dyndynjyxa/aio-coding-hub/commit/60cbcc1c65ff025e79313facaf27e625a3de9997))
* **provider-editor:** enhance ProviderEditorDialog with limit configuration cards ([cc14a00](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc14a009303129793ceec840933155dcfad775d8))
* **providers:** collapse model mapping editors ([4672961](https://github.com/dyndynjyxa/aio-coding-hub/commit/4672961c8facbd27d715a762864c2bf4f32ac932))
* **providers:** 添加供应商标签功能，支持分类筛选 ([7671732](https://github.com/dyndynjyxa/aio-coding-hub/commit/76717329a119f9a65dc2f3a9dffe0a887063b0bd))
* **request-end:** enhance request logging and event emission ([809a684](https://github.com/dyndynjyxa/aio-coding-hub/commit/809a6843a2c2eb952bbb00f7c56694c908d99bd1))
* **request-end:** refactor request logging and event emission ([e56909e](https://github.com/dyndynjyxa/aio-coding-hub/commit/e56909e374c26608407a1d63d32c9d55f914b355))
* **responsive:** enhance layout and styling for improved mobile experience ([e22483a](https://github.com/dyndynjyxa/aio-coding-hub/commit/e22483a8da8c35822c6e5b0532aa1461cb68cf95))
* **route:** 支持展示同一 provider 的连续尝试次数及跳过状态 ([6659157](https://github.com/dyndynjyxa/aio-coding-hub/commit/6659157a12c3809b1e4d8de7bf95b95afe6ed383))
* **settings:** update Claude settings and remove deprecated options ([49e055a](https://github.com/dyndynjyxa/aio-coding-hub/commit/49e055aeb176819cb56a925aecbd51f362008762))
* **settings:** 新增缓存异常监测功能开关 ([ea681c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ea681c155346da3057f25e25867ab75a818f4157))
* **SkillsMarketPage:** enhance UI with tab selection and external links ([2849017](https://github.com/dyndynjyxa/aio-coding-hub/commit/2849017554128279822fef9b667d8ec166a08432))
* **sort-modes:** add toggle functionality for enabling/disabling providers in sort modes and update related services ([e6cd992](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6cd99241f5c86da3f62afebefeffdf33e62b8f0))
* **tauri:** add WSL support and listen modes ([a357007](https://github.com/dyndynjyxa/aio-coding-hub/commit/a35700753e9633493f6e939d1700ce979d635c93))
* **tauri:** update error handling and add thiserror dependency ([10d918a](https://github.com/dyndynjyxa/aio-coding-hub/commit/10d918a0a0a746b4d25c1c236f88c0110ec664c3))
* **tests:** add comprehensive unit tests for various components ([98fb022](https://github.com/dyndynjyxa/aio-coding-hub/commit/98fb022d1487e593e1fe50a1e9d02592671a4944))
* **theme:** add dark mode and native window theme sync ([588a373](https://github.com/dyndynjyxa/aio-coding-hub/commit/588a37311c456bd2182ac1676c8e9b309157353f))
* **ui:** align CLI manager with network and WSL settings ([ae5b5fc](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae5b5fc99330b55872e1c30da6e653d7433b7d48))
* **ui:** 统一 Loading/Empty/Error 状态反馈组件 ([b27f64a](https://github.com/dyndynjyxa/aio-coding-hub/commit/b27f64a79a1ad655a37992527794759f9f8c5370))
* **updater:** display release notes in update dialog ([b891ed7](https://github.com/dyndynjyxa/aio-coding-hub/commit/b891ed75a1b67a9df281917ffa94e684fb372664))
* **usage:** add cache rate trend functionality to UsagePage ([5535e7d](https://github.com/dyndynjyxa/aio-coding-hub/commit/5535e7d3480237284037ef841d2878bfa6a180f5))
* **usage:** add cost tracking to usage statistics ([765ea8a](https://github.com/dyndynjyxa/aio-coding-hub/commit/765ea8ae06efe2cc39c6eac7e95dccfbba4de541))
* **usage:** add summary stat cards with filter layout optimization ([f6c9206](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6c9206ba6e0b424748ba05f0ac749884d5bf0d3))
* **workspaces:** add WorkspacesPage route and enhance CLI manager settings ([2784072](https://github.com/dyndynjyxa/aio-coding-hub/commit/2784072e26748aa21f679c245f74cf3063f4177c))
* **workspaces:** enhance MCP and Prompts management with workspace support ([791ee6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/791ee6c88b06c16c596b82dcf7956ea5b98dc18d))
* **workspaces:** improve workspace switching and management functionality ([496c9c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/496c9c35daa23be4d2c44f0653bb11bcb5206e1f))


### Bug Fixes

* **ci:** add explicit permissions to release-please job ([8c77c04](https://github.com/dyndynjyxa/aio-coding-hub/commit/8c77c041b0d6f5081abf6ea9a39e1031dc56ad75))
* **ci:** ensure consistent use of RELEASE_PLEASE_TOKEN in release workflow for GitHub actions ([f8a439d](https://github.com/dyndynjyxa/aio-coding-hub/commit/f8a439d213346a907dfc7355b6b6caf4b4194799))
* **ci:** quote FALLBACK_NOTES value in release workflow ([536896c](https://github.com/dyndynjyxa/aio-coding-hub/commit/536896ca4072f48002e8c83b91d78164ed40384f))
* **ci:** revert release-please job to original config ([0d1afad](https://github.com/dyndynjyxa/aio-coding-hub/commit/0d1afade58707c53554f7680dd7455d6ef547187))
* **ci:** update release workflow to consistently use RELEASE_PLEASE_TOKEN for GitHub actions ([2c45c46](https://github.com/dyndynjyxa/aio-coding-hub/commit/2c45c461ddf157a4b781c146a0ebed5cd9dc1a44))
* **ci:** update release workflow to use RELEASE_PLEASE_TOKEN for GitHub actions ([48ec3ce](https://github.com/dyndynjyxa/aio-coding-hub/commit/48ec3ce92e28702ee9920eaa42d2f7374b07be31))
* **ci:** use PAT for release-please to fix permission issue ([a2919f2](https://github.com/dyndynjyxa/aio-coding-hub/commit/a2919f2446d8fcb1c4c7ac8dc6a1b4f63f705157))
* **ci:** 修复CI流程中正则表达式重复问题 ([609f2ea](https://github.com/dyndynjyxa/aio-coding-hub/commit/609f2ea657034a58b6f175b995e05431829f6dd9))
* **ci:** 修复正则表达式转义字符问题 ([ba805c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba805c3b2f147f153cbe39042b078547eb36da5a))
* **ClaudeModelValidationDialog:** adjust dialog max-width for responsive design ([11bdfd0](https://github.com/dyndynjyxa/aio-coding-hub/commit/11bdfd0df1b51923a561211f6cc6d5383eedf76c))
* **cli_manager:** 修复 run_in_login_shell 的平台兼容性处理 ([9f81433](https://github.com/dyndynjyxa/aio-coding-hub/commit/9f814337d08f36c08e37486f95b9c9da62880fc1))
* **codex-tab:** sandbox_mode danger-full-access 选择不生效及高级配置不回显 ([3943401](https://github.com/dyndynjyxa/aio-coding-hub/commit/3943401524c8ef6a179e65e8ee62fd2ac4f784cb))
* **deps:** 升级 react-router-dom 7.11→7.13 修复 XSS 漏洞 ([c4db630](https://github.com/dyndynjyxa/aio-coding-hub/commit/c4db630b4478cab2f1180f03b324cfc0af9d42e3))
* **gateway:** finalize failed requests and align error codes ([871de9c](https://github.com/dyndynjyxa/aio-coding-hub/commit/871de9c9c9e06ed6cb03026ceb21dfc4b0ba18d3))
* **gateway:** reject forwarding when CLI proxy disabled ([c9edd10](https://github.com/dyndynjyxa/aio-coding-hub/commit/c9edd10cd2f41ef86c8c4c8a3ca2262c8bcb09ef))
* **gateway:** 调整上游连接超时与错误切换逻辑 ([a6870ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6870ae14b6b5bb8edce814ca37da2000e84f6b4))
* **home:** update HomeCostPanel with accessibility labels and improve test assertions ([0bda6c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/0bda6c1ce8aec6920b30810882f3a804c62ab732))
* **skills:** 移除批量导入功能并改为刷新本机列表 ([45b9618](https://github.com/dyndynjyxa/aio-coding-hub/commit/45b961847b458f72f2bc062bdcdfef93c8722da8))
* **tests:** update RealtimeTraceCards test to reflect token display format change ([f43633e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f43633e5381f44447bbf8aa64c1065d222687c59))
* **theme:** 修复 dark mode 切换闪烁回退问题 ([ac9500c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac9500c04a108c5bc15921353b691394a689ac54))
* **usage:** align cache creation ttl to 5m only ([8d28bcd](https://github.com/dyndynjyxa/aio-coding-hub/commit/8d28bcd2f5d7f8d6bac1a7f65f974c04c5fce337))


### Code Refactoring

* **infra:** 移除 request_attempt_logs 独立表，改用 request_logs.attempts_json 派生 ([3fb2627](https://github.com/dyndynjyxa/aio-coding-hub/commit/3fb2627407e1b9b57758a8a40d3a7d7393278e1b))

## [0.27.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.26.0...aio-coding-hub-v0.27.0) (2026-02-13)


### ⚠ BREAKING CHANGES

* **infra:** request_attempt_logs 表不再创建或写入

### Code Refactoring

* **infra:** 移除 request_attempt_logs 独立表，改用 request_logs.attempts_json 派生 ([3fb2627](https://github.com/dyndynjyxa/aio-coding-hub/commit/3fb2627407e1b9b57758a8a40d3a7d7393278e1b))

## [0.26.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.25.0...aio-coding-hub-v0.26.0) (2026-02-13)


### Features

* **console:** 改进控制台界面并增加日志过滤和搜索功能 ([93badf8](https://github.com/dyndynjyxa/aio-coding-hub/commit/93badf8766c9a415c821c3f14084183d18d31687))
* **providers:** 添加供应商标签功能，支持分类筛选 ([7671732](https://github.com/dyndynjyxa/aio-coding-hub/commit/76717329a119f9a65dc2f3a9dffe0a887063b0bd))


### Bug Fixes

* **cli_manager:** 修复 run_in_login_shell 的平台兼容性处理 ([9f81433](https://github.com/dyndynjyxa/aio-coding-hub/commit/9f814337d08f36c08e37486f95b9c9da62880fc1))

## [0.25.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.24.1...aio-coding-hub-v0.25.0) (2026-02-12)


### Features

* **cli:** 添加 Claude 终端启动命令生成及剪贴板管理插件支持 ([ba3d3c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba3d3c20d8d1d40accc0d97780c4dae54d48c71a))
* **gateway:** 支持强制指定请求的Provider并更新Claude终端启动逻辑 ([904a9aa](https://github.com/dyndynjyxa/aio-coding-hub/commit/904a9aa6805681fb5642e350105803ce01ae998b))


### Bug Fixes

* **ci:** 修复正则表达式转义字符问题 ([ba805c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba805c3b2f147f153cbe39042b078547eb36da5a))

## [0.24.1](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.24.0...aio-coding-hub-v0.24.1) (2026-02-11)


### Bug Fixes

* **gateway:** 调整上游连接超时与错误切换逻辑 ([a6870ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6870ae14b6b5bb8edce814ca37da2000e84f6b4))

## [0.24.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.23.0...aio-coding-hub-v0.24.0) (2026-02-10)


### Features

* **settings:** 新增缓存异常监测功能开关 ([ea681c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ea681c155346da3057f25e25867ab75a818f4157))
* **ui:** 统一 Loading/Empty/Error 状态反馈组件 ([b27f64a](https://github.com/dyndynjyxa/aio-coding-hub/commit/b27f64a79a1ad655a37992527794759f9f8c5370))


### Bug Fixes

* **ci:** 修复CI流程中正则表达式重复问题 ([609f2ea](https://github.com/dyndynjyxa/aio-coding-hub/commit/609f2ea657034a58b6f175b995e05431829f6dd9))
* **codex-tab:** sandbox_mode danger-full-access 选择不生效及高级配置不回显 ([3943401](https://github.com/dyndynjyxa/aio-coding-hub/commit/3943401524c8ef6a179e65e8ee62fd2ac4f784cb))
* **deps:** 升级 react-router-dom 7.11→7.13 修复 XSS 漏洞 ([c4db630](https://github.com/dyndynjyxa/aio-coding-hub/commit/c4db630b4478cab2f1180f03b324cfc0af9d42e3))
* **theme:** 修复 dark mode 切换闪烁回退问题 ([ac9500c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac9500c04a108c5bc15921353b691394a689ac54))

## [0.23.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.22.0...aio-coding-hub-v0.23.0) (2026-02-08)


### Features

* **app:** 优化页面路由懒加载和加载反馈体验 ([504256e](https://github.com/dyndynjyxa/aio-coding-hub/commit/504256e11ed517797405f8bb4ab3d353607cb1ae))
* mac最小化优化 ([1f11a5d](https://github.com/dyndynjyxa/aio-coding-hub/commit/1f11a5df9623f8cab90bb40f5c499c5d3569f01d))
* **route:** 支持展示同一 provider 的连续尝试次数及跳过状态 ([6659157](https://github.com/dyndynjyxa/aio-coding-hub/commit/6659157a12c3809b1e4d8de7bf95b95afe6ed383))


### Bug Fixes

* **gateway:** finalize failed requests and align error codes ([871de9c](https://github.com/dyndynjyxa/aio-coding-hub/commit/871de9c9c9e06ed6cb03026ceb21dfc4b0ba18d3))
* **skills:** 移除批量导入功能并改为刷新本机列表 ([45b9618](https://github.com/dyndynjyxa/aio-coding-hub/commit/45b961847b458f72f2bc062bdcdfef93c8722da8))

## [0.22.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.21.0...aio-coding-hub-v0.22.0) (2026-02-07)


### Features

* **cli-manager:** add experimental agent teams setting and update related tests ([6e1dd0c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6e1dd0c82a19f7fa40728fa7fa6d7f23605dfab0))
* **components:** enhance HomeRequestLogsPanel and RealtimeTraceCards with new formatting utilities ([e6a3550](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6a3550636e4278100a24d009e49ade1056733bc))
* **home:** enhance status handling with failover support ([5aab8a9](https://github.com/dyndynjyxa/aio-coding-hub/commit/5aab8a92c4440062d18a3ecf495375d11a461660))
* **theme:** add dark mode and native window theme sync ([588a373](https://github.com/dyndynjyxa/aio-coding-hub/commit/588a37311c456bd2182ac1676c8e9b309157353f))


### Bug Fixes

* **tests:** update RealtimeTraceCards test to reflect token display format change ([f43633e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f43633e5381f44447bbf8aa64c1065d222687c59))

## [0.21.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.20.0...aio-coding-hub-v0.21.0) (2026-02-06)


### Features

* add TextEvidenceSection component for improved output display in ClaudeModelValidationResultPanel ([47be119](https://github.com/dyndynjyxa/aio-coding-hub/commit/47be119a83c365b3e7b41f22308be7550ecaede5))
* **app:** add LogsPage route and enhance CLI manager settings ([6796715](https://github.com/dyndynjyxa/aio-coding-hub/commit/679671526be935129211978b3598a3962f7e2a8e))
* **cache-anomaly-monitor:** implement cache anomaly monitoring feature ([59d69d1](https://github.com/dyndynjyxa/aio-coding-hub/commit/59d69d1528d888a84d71038873321f3273d4ded1))
* **charts:** migrate from ECharts to Recharts for improved charting capabilities ([b747b61](https://github.com/dyndynjyxa/aio-coding-hub/commit/b747b61a8dd587043a7e8de01a5f9b3d64ead7ae))
* **claude-validation:** add signature and caching roundtrip probes ([15badee](https://github.com/dyndynjyxa/aio-coding-hub/commit/15badee08b0c14f71695e6e71f0b165e4844371c))
* **ClaudeModelValidation:** enhance cross-provider validation and UI feedback ([bf83c7e](https://github.com/dyndynjyxa/aio-coding-hub/commit/bf83c7e03c7edf78795cd51a943c01a88e0b17d7))
* **ClaudeModelValidation:** enhance output token validation and error handling ([d245288](https://github.com/dyndynjyxa/aio-coding-hub/commit/d245288d7a4937ca7b0213ebd79d9c0d5e3c76b4))
* **ClaudeModelValidation:** implement cross-provider signature validation and enhance request handling ([2e102d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/2e102d4f3fd2745e4480a5884272baeafe66b6d0))
* **cli-manager:** add Codex and Gemini tabs for CLI management ([6f1ee01](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f1ee011fdef3aa323e35dfe537e6ea3fc03e087))
* **cli-manager:** add collaboration features to CodexTab ([f6ab4e4](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6ab4e48ebdc5cdafa132e1513c25be01964b81d))
* **cli-manager:** add experimental MCP CLI features and max output tokens management ([84ac464](https://github.com/dyndynjyxa/aio-coding-hub/commit/84ac4645e7f4eb1e6235b85d7ca63f16b04c8ee6))
* **cli-manager:** add TOML configuration support for Codex ([826737a](https://github.com/dyndynjyxa/aio-coding-hub/commit/826737a89b74076d5bd90d960e876fb86a12e1cd))
* **cli-manager:** enhance Claude and WSL settings management ([3aacb6a](https://github.com/dyndynjyxa/aio-coding-hub/commit/3aacb6ad4a05b238a80064529445de398b1978b5))
* **cli-manager:** enhance Claude settings management and UI ([cbc1160](https://github.com/dyndynjyxa/aio-coding-hub/commit/cbc11605ee2cc64b9b08c51e4b33b70b72d168fa))
* **cli-manager:** enhance CodexTab with sandbox mode management ([759c19d](https://github.com/dyndynjyxa/aio-coding-hub/commit/759c19d02553f929f5a91c6051a0134e198e298e))
* **cli-manager:** enhance WSL settings and improve CLI manager performance ([be50456](https://github.com/dyndynjyxa/aio-coding-hub/commit/be50456432ef1f00cf1567c664a168779a23fc58))
* **cli-manager:** update GeneralTab and cache anomaly monitoring logic ([c84a1eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/c84a1eb6c312c938ba067551e7b3303bba761edf))
* **cli:** add CLI proxy startup recovery feature ([eb40a6f](https://github.com/dyndynjyxa/aio-coding-hub/commit/eb40a6f03d9a3b50b501795727a06b7ce9013fc1))
* **CliManager:** add response fixer configuration limits and UI inputs ([0023ad6](https://github.com/dyndynjyxa/aio-coding-hub/commit/0023ad69abf91f48a5144250e20b53ea0b2e24bf))
* **ConsolePage:** revamp console log display and functionality ([1d28397](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d28397e88c0b6d43a4d73b348c49c93cb18efde))
* **core:** improve workspace imports and startup resilience ([1d7dbf0](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d7dbf0b73cde2b78a70be945c6fd8e248b30c1d))
* **date-range:** refactor custom date range handling in HomeCostPanel and UsagePage ([95b1ad1](https://github.com/dyndynjyxa/aio-coding-hub/commit/95b1ad11b16a0f3e9b187b4883ef85a367d712e0))
* enhance provider model configuration with support for model whitelisting and mapping ([4f44510](https://github.com/dyndynjyxa/aio-coding-hub/commit/4f445106fefa10badae230de52c9fee09bd2486f))
* **failover-loop:** refactor context management for improved clarity and usability ([b96df01](https://github.com/dyndynjyxa/aio-coding-hub/commit/b96df013317c77ca610bb640a43b2d60058e6847))
* **failover-loop:** refactor error handling and logging in failover loop ([ef2662c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef2662c335ae4057c591b531b0784c86cd73951e))
* **home:** add provider limit usage overview tab ([c224748](https://github.com/dyndynjyxa/aio-coding-hub/commit/c224748c590d1b242df045a7e81667a623d0ec0b))
* **home:** add provider limit usage overview tab ([6473253](https://github.com/dyndynjyxa/aio-coding-hub/commit/64732536a25e140193242e9829aacb2cce15f05d))
* **home:** add window start timestamps for provider limit usage ([69a91a2](https://github.com/dyndynjyxa/aio-coding-hub/commit/69a91a21567b6a2c9af2ac879936dd75a6004e37))
* **home:** enhance HomeCostPanel with data-testid attributes and update tests ([b1d23d2](https://github.com/dyndynjyxa/aio-coding-hub/commit/b1d23d2575afb25d3314911b0c6488a872f56b46))
* **home:** enhance UI components and improve layout consistency ([70d9655](https://github.com/dyndynjyxa/aio-coding-hub/commit/70d9655cf1a740676e81bcb0a1e204b81abdd1c9))
* **home:** implement CLI proxy environment conflict checks ([aceff42](https://github.com/dyndynjyxa/aio-coding-hub/commit/aceff428537261c5132b5cb4ebdcb22d16efec82))
* **home:** implement CLI proxy environment conflict checks ([4c014ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/4c014ca5ab04dab7aa5ca38799d69d1a8eb3cc3e))
* **home:** implement window foreground detection for usage heatmap refresh ([4e66f35](https://github.com/dyndynjyxa/aio-coding-hub/commit/4e66f359f198ddddc52b6cd4c0ab8cdb59630a27))
* init ([7e30c40](https://github.com/dyndynjyxa/aio-coding-hub/commit/7e30c40727d50980bcd43c2f275419a74fa3b148))
* integrate PageHeader component across multiple pages for consistent UI ([330da27](https://github.com/dyndynjyxa/aio-coding-hub/commit/330da276f9ef8e91744a9534d59590a3a6fec5ff))
* **mcp:** implement batch import for local skills and add global error reporting ([51e053c](https://github.com/dyndynjyxa/aio-coding-hub/commit/51e053c81f1db00696f0cb968860033deee12cce))
* **model-prices:** add model price alias rules ([60cbcc1](https://github.com/dyndynjyxa/aio-coding-hub/commit/60cbcc1c65ff025e79313facaf27e625a3de9997))
* **provider-editor:** enhance ProviderEditorDialog with limit configuration cards ([cc14a00](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc14a009303129793ceec840933155dcfad775d8))
* **providers:** collapse model mapping editors ([4672961](https://github.com/dyndynjyxa/aio-coding-hub/commit/4672961c8facbd27d715a762864c2bf4f32ac932))
* **request-end:** enhance request logging and event emission ([809a684](https://github.com/dyndynjyxa/aio-coding-hub/commit/809a6843a2c2eb952bbb00f7c56694c908d99bd1))
* **request-end:** refactor request logging and event emission ([e56909e](https://github.com/dyndynjyxa/aio-coding-hub/commit/e56909e374c26608407a1d63d32c9d55f914b355))
* **responsive:** enhance layout and styling for improved mobile experience ([e22483a](https://github.com/dyndynjyxa/aio-coding-hub/commit/e22483a8da8c35822c6e5b0532aa1461cb68cf95))
* **settings:** update Claude settings and remove deprecated options ([49e055a](https://github.com/dyndynjyxa/aio-coding-hub/commit/49e055aeb176819cb56a925aecbd51f362008762))
* **SkillsMarketPage:** enhance UI with tab selection and external links ([2849017](https://github.com/dyndynjyxa/aio-coding-hub/commit/2849017554128279822fef9b667d8ec166a08432))
* **sort-modes:** add toggle functionality for enabling/disabling providers in sort modes and update related services ([e6cd992](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6cd99241f5c86da3f62afebefeffdf33e62b8f0))
* **tauri:** add WSL support and listen modes ([a357007](https://github.com/dyndynjyxa/aio-coding-hub/commit/a35700753e9633493f6e939d1700ce979d635c93))
* **tauri:** update error handling and add thiserror dependency ([10d918a](https://github.com/dyndynjyxa/aio-coding-hub/commit/10d918a0a0a746b4d25c1c236f88c0110ec664c3))
* **tests:** add comprehensive unit tests for various components ([98fb022](https://github.com/dyndynjyxa/aio-coding-hub/commit/98fb022d1487e593e1fe50a1e9d02592671a4944))
* **ui:** align CLI manager with network and WSL settings ([ae5b5fc](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae5b5fc99330b55872e1c30da6e653d7433b7d48))
* **updater:** display release notes in update dialog ([b891ed7](https://github.com/dyndynjyxa/aio-coding-hub/commit/b891ed75a1b67a9df281917ffa94e684fb372664))
* **usage:** add cache rate trend functionality to UsagePage ([5535e7d](https://github.com/dyndynjyxa/aio-coding-hub/commit/5535e7d3480237284037ef841d2878bfa6a180f5))
* **usage:** add cost tracking to usage statistics ([765ea8a](https://github.com/dyndynjyxa/aio-coding-hub/commit/765ea8ae06efe2cc39c6eac7e95dccfbba4de541))
* **usage:** add summary stat cards with filter layout optimization ([f6c9206](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6c9206ba6e0b424748ba05f0ac749884d5bf0d3))
* **workspaces:** add WorkspacesPage route and enhance CLI manager settings ([2784072](https://github.com/dyndynjyxa/aio-coding-hub/commit/2784072e26748aa21f679c245f74cf3063f4177c))
* **workspaces:** enhance MCP and Prompts management with workspace support ([791ee6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/791ee6c88b06c16c596b82dcf7956ea5b98dc18d))
* **workspaces:** improve workspace switching and management functionality ([496c9c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/496c9c35daa23be4d2c44f0653bb11bcb5206e1f))


### Bug Fixes

* **ci:** add explicit permissions to release-please job ([8c77c04](https://github.com/dyndynjyxa/aio-coding-hub/commit/8c77c041b0d6f5081abf6ea9a39e1031dc56ad75))
* **ci:** ensure consistent use of RELEASE_PLEASE_TOKEN in release workflow for GitHub actions ([f8a439d](https://github.com/dyndynjyxa/aio-coding-hub/commit/f8a439d213346a907dfc7355b6b6caf4b4194799))
* **ci:** quote FALLBACK_NOTES value in release workflow ([536896c](https://github.com/dyndynjyxa/aio-coding-hub/commit/536896ca4072f48002e8c83b91d78164ed40384f))
* **ci:** revert release-please job to original config ([0d1afad](https://github.com/dyndynjyxa/aio-coding-hub/commit/0d1afade58707c53554f7680dd7455d6ef547187))
* **ci:** update release workflow to consistently use RELEASE_PLEASE_TOKEN for GitHub actions ([2c45c46](https://github.com/dyndynjyxa/aio-coding-hub/commit/2c45c461ddf157a4b781c146a0ebed5cd9dc1a44))
* **ci:** update release workflow to use RELEASE_PLEASE_TOKEN for GitHub actions ([48ec3ce](https://github.com/dyndynjyxa/aio-coding-hub/commit/48ec3ce92e28702ee9920eaa42d2f7374b07be31))
* **ci:** use PAT for release-please to fix permission issue ([a2919f2](https://github.com/dyndynjyxa/aio-coding-hub/commit/a2919f2446d8fcb1c4c7ac8dc6a1b4f63f705157))
* **ClaudeModelValidationDialog:** adjust dialog max-width for responsive design ([11bdfd0](https://github.com/dyndynjyxa/aio-coding-hub/commit/11bdfd0df1b51923a561211f6cc6d5383eedf76c))
* **gateway:** reject forwarding when CLI proxy disabled ([c9edd10](https://github.com/dyndynjyxa/aio-coding-hub/commit/c9edd10cd2f41ef86c8c4c8a3ca2262c8bcb09ef))
* **home:** update HomeCostPanel with accessibility labels and improve test assertions ([0bda6c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/0bda6c1ce8aec6920b30810882f3a804c62ab732))
* **usage:** align cache creation ttl to 5m only ([8d28bcd](https://github.com/dyndynjyxa/aio-coding-hub/commit/8d28bcd2f5d7f8d6bac1a7f65f974c04c5fce337))

## [0.20.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.19.0...aio-coding-hub-v0.20.0) (2026-02-06)


### Features

* **core:** improve workspace imports and startup resilience ([1d7dbf0](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d7dbf0b73cde2b78a70be945c6fd8e248b30c1d))
* **mcp:** implement batch import for local skills and add global error reporting ([51e053c](https://github.com/dyndynjyxa/aio-coding-hub/commit/51e053c81f1db00696f0cb968860033deee12cce))


### Bug Fixes

* **ClaudeModelValidationDialog:** adjust dialog max-width for responsive design ([11bdfd0](https://github.com/dyndynjyxa/aio-coding-hub/commit/11bdfd0df1b51923a561211f6cc6d5383eedf76c))

## [0.19.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.18.0...aio-coding-hub-v0.19.0) (2026-02-04)


### Features

* **sort-modes:** add toggle functionality for enabling/disabling providers in sort modes and update related services ([e6cd992](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6cd99241f5c86da3f62afebefeffdf33e62b8f0))


### Bug Fixes

* **ci:** add explicit permissions to release-please job ([8c77c04](https://github.com/dyndynjyxa/aio-coding-hub/commit/8c77c041b0d6f5081abf6ea9a39e1031dc56ad75))
* **ci:** ensure consistent use of RELEASE_PLEASE_TOKEN in release workflow for GitHub actions ([f8a439d](https://github.com/dyndynjyxa/aio-coding-hub/commit/f8a439d213346a907dfc7355b6b6caf4b4194799))
* **ci:** quote FALLBACK_NOTES value in release workflow ([536896c](https://github.com/dyndynjyxa/aio-coding-hub/commit/536896ca4072f48002e8c83b91d78164ed40384f))
* **ci:** revert release-please job to original config ([0d1afad](https://github.com/dyndynjyxa/aio-coding-hub/commit/0d1afade58707c53554f7680dd7455d6ef547187))
* **ci:** update release workflow to consistently use RELEASE_PLEASE_TOKEN for GitHub actions ([2c45c46](https://github.com/dyndynjyxa/aio-coding-hub/commit/2c45c461ddf157a4b781c146a0ebed5cd9dc1a44))
* **ci:** update release workflow to use RELEASE_PLEASE_TOKEN for GitHub actions ([48ec3ce](https://github.com/dyndynjyxa/aio-coding-hub/commit/48ec3ce92e28702ee9920eaa42d2f7374b07be31))
* **ci:** use PAT for release-please to fix permission issue ([a2919f2](https://github.com/dyndynjyxa/aio-coding-hub/commit/a2919f2446d8fcb1c4c7ac8dc6a1b4f63f705157))

## [0.18.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.17.0...aio-coding-hub-v0.18.0) (2026-02-03)


### Features

* **charts:** migrate from ECharts to Recharts for improved charting capabilities ([b747b61](https://github.com/dyndynjyxa/aio-coding-hub/commit/b747b61a8dd587043a7e8de01a5f9b3d64ead7ae))
* **cli-manager:** add TOML configuration support for Codex ([826737a](https://github.com/dyndynjyxa/aio-coding-hub/commit/826737a89b74076d5bd90d960e876fb86a12e1cd))
* **cli:** add CLI proxy startup recovery feature ([eb40a6f](https://github.com/dyndynjyxa/aio-coding-hub/commit/eb40a6f03d9a3b50b501795727a06b7ce9013fc1))
* **home:** add provider limit usage overview tab ([c224748](https://github.com/dyndynjyxa/aio-coding-hub/commit/c224748c590d1b242df045a7e81667a623d0ec0b))
* **home:** add provider limit usage overview tab ([6473253](https://github.com/dyndynjyxa/aio-coding-hub/commit/64732536a25e140193242e9829aacb2cce15f05d))
* **home:** add window start timestamps for provider limit usage ([69a91a2](https://github.com/dyndynjyxa/aio-coding-hub/commit/69a91a21567b6a2c9af2ac879936dd75a6004e37))
* **home:** enhance HomeCostPanel with data-testid attributes and update tests ([b1d23d2](https://github.com/dyndynjyxa/aio-coding-hub/commit/b1d23d2575afb25d3314911b0c6488a872f56b46))
* **home:** enhance UI components and improve layout consistency ([70d9655](https://github.com/dyndynjyxa/aio-coding-hub/commit/70d9655cf1a740676e81bcb0a1e204b81abdd1c9))
* **home:** implement CLI proxy environment conflict checks ([aceff42](https://github.com/dyndynjyxa/aio-coding-hub/commit/aceff428537261c5132b5cb4ebdcb22d16efec82))
* **home:** implement CLI proxy environment conflict checks ([4c014ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/4c014ca5ab04dab7aa5ca38799d69d1a8eb3cc3e))
* **responsive:** enhance layout and styling for improved mobile experience ([e22483a](https://github.com/dyndynjyxa/aio-coding-hub/commit/e22483a8da8c35822c6e5b0532aa1461cb68cf95))
* **settings:** update Claude settings and remove deprecated options ([49e055a](https://github.com/dyndynjyxa/aio-coding-hub/commit/49e055aeb176819cb56a925aecbd51f362008762))
* **tauri:** update error handling and add thiserror dependency ([10d918a](https://github.com/dyndynjyxa/aio-coding-hub/commit/10d918a0a0a746b4d25c1c236f88c0110ec664c3))
* **usage:** add summary stat cards with filter layout optimization ([f6c9206](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6c9206ba6e0b424748ba05f0ac749884d5bf0d3))


### Bug Fixes

* **home:** update HomeCostPanel with accessibility labels and improve test assertions ([0bda6c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/0bda6c1ce8aec6920b30810882f3a804c62ab732))

## [0.17.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.16.0...aio-coding-hub-v0.17.0) (2026-02-02)


### Features

* **provider-editor:** enhance ProviderEditorDialog with limit configuration cards ([cc14a00](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc14a009303129793ceec840933155dcfad775d8))
* **usage:** add cache rate trend functionality to UsagePage ([5535e7d](https://github.com/dyndynjyxa/aio-coding-hub/commit/5535e7d3480237284037ef841d2878bfa6a180f5))
* **workspaces:** add WorkspacesPage route and enhance CLI manager settings ([2784072](https://github.com/dyndynjyxa/aio-coding-hub/commit/2784072e26748aa21f679c245f74cf3063f4177c))
* **workspaces:** enhance MCP and Prompts management with workspace support ([791ee6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/791ee6c88b06c16c596b82dcf7956ea5b98dc18d))
* **workspaces:** improve workspace switching and management functionality ([496c9c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/496c9c35daa23be4d2c44f0653bb11bcb5206e1f))

## [0.16.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.15.0...aio-coding-hub-v0.16.0) (2026-01-29)


### Features

* **app:** add LogsPage route and enhance CLI manager settings ([6796715](https://github.com/dyndynjyxa/aio-coding-hub/commit/679671526be935129211978b3598a3962f7e2a8e))
* **cache-anomaly-monitor:** implement cache anomaly monitoring feature ([59d69d1](https://github.com/dyndynjyxa/aio-coding-hub/commit/59d69d1528d888a84d71038873321f3273d4ded1))
* **cli-manager:** add collaboration features to CodexTab ([f6ab4e4](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6ab4e48ebdc5cdafa132e1513c25be01964b81d))
* **cli-manager:** add experimental MCP CLI features and max output tokens management ([84ac464](https://github.com/dyndynjyxa/aio-coding-hub/commit/84ac4645e7f4eb1e6235b85d7ca63f16b04c8ee6))
* **cli-manager:** enhance WSL settings and improve CLI manager performance ([be50456](https://github.com/dyndynjyxa/aio-coding-hub/commit/be50456432ef1f00cf1567c664a168779a23fc58))
* **cli-manager:** update GeneralTab and cache anomaly monitoring logic ([c84a1eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/c84a1eb6c312c938ba067551e7b3303bba761edf))
* **date-range:** refactor custom date range handling in HomeCostPanel and UsagePage ([95b1ad1](https://github.com/dyndynjyxa/aio-coding-hub/commit/95b1ad11b16a0f3e9b187b4883ef85a367d712e0))
* **failover-loop:** refactor context management for improved clarity and usability ([b96df01](https://github.com/dyndynjyxa/aio-coding-hub/commit/b96df013317c77ca610bb640a43b2d60058e6847))
* **failover-loop:** refactor error handling and logging in failover loop ([ef2662c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef2662c335ae4057c591b531b0784c86cd73951e))
* **request-end:** enhance request logging and event emission ([809a684](https://github.com/dyndynjyxa/aio-coding-hub/commit/809a6843a2c2eb952bbb00f7c56694c908d99bd1))
* **request-end:** refactor request logging and event emission ([e56909e](https://github.com/dyndynjyxa/aio-coding-hub/commit/e56909e374c26608407a1d63d32c9d55f914b355))

## [0.15.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.14.0...aio-coding-hub-v0.15.0) (2026-01-27)


### Features

* **cli-manager:** add Codex and Gemini tabs for CLI management ([6f1ee01](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f1ee011fdef3aa323e35dfe537e6ea3fc03e087))
* **cli-manager:** enhance Claude and WSL settings management ([3aacb6a](https://github.com/dyndynjyxa/aio-coding-hub/commit/3aacb6ad4a05b238a80064529445de398b1978b5))
* **cli-manager:** enhance Claude settings management and UI ([cbc1160](https://github.com/dyndynjyxa/aio-coding-hub/commit/cbc11605ee2cc64b9b08c51e4b33b70b72d168fa))
* **cli-manager:** enhance CodexTab with sandbox mode management ([759c19d](https://github.com/dyndynjyxa/aio-coding-hub/commit/759c19d02553f929f5a91c6051a0134e198e298e))
* **tests:** add comprehensive unit tests for various components ([98fb022](https://github.com/dyndynjyxa/aio-coding-hub/commit/98fb022d1487e593e1fe50a1e9d02592671a4944))
* **usage:** add cost tracking to usage statistics ([765ea8a](https://github.com/dyndynjyxa/aio-coding-hub/commit/765ea8ae06efe2cc39c6eac7e95dccfbba4de541))

## [0.14.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.13.0...aio-coding-hub-v0.14.0) (2026-01-25)


### Features

* **ClaudeModelValidation:** enhance cross-provider validation and UI feedback ([bf83c7e](https://github.com/dyndynjyxa/aio-coding-hub/commit/bf83c7e03c7edf78795cd51a943c01a88e0b17d7))
* **ClaudeModelValidation:** enhance output token validation and error handling ([d245288](https://github.com/dyndynjyxa/aio-coding-hub/commit/d245288d7a4937ca7b0213ebd79d9c0d5e3c76b4))
* **ClaudeModelValidation:** implement cross-provider signature validation and enhance request handling ([2e102d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/2e102d4f3fd2745e4480a5884272baeafe66b6d0))
* **CliManager:** add response fixer configuration limits and UI inputs ([0023ad6](https://github.com/dyndynjyxa/aio-coding-hub/commit/0023ad69abf91f48a5144250e20b53ea0b2e24bf))
* **ConsolePage:** revamp console log display and functionality ([1d28397](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d28397e88c0b6d43a4d73b348c49c93cb18efde))
* integrate PageHeader component across multiple pages for consistent UI ([330da27](https://github.com/dyndynjyxa/aio-coding-hub/commit/330da276f9ef8e91744a9534d59590a3a6fec5ff))
* **SkillsMarketPage:** enhance UI with tab selection and external links ([2849017](https://github.com/dyndynjyxa/aio-coding-hub/commit/2849017554128279822fef9b667d8ec166a08432))

## [0.13.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.12.0...aio-coding-hub-v0.13.0) (2026-01-20)


### Features

* add TextEvidenceSection component for improved output display in ClaudeModelValidationResultPanel ([47be119](https://github.com/dyndynjyxa/aio-coding-hub/commit/47be119a83c365b3e7b41f22308be7550ecaede5))
* **claude-validation:** add signature and caching roundtrip probes ([15badee](https://github.com/dyndynjyxa/aio-coding-hub/commit/15badee08b0c14f71695e6e71f0b165e4844371c))
* enhance provider model configuration with support for model whitelisting and mapping ([4f44510](https://github.com/dyndynjyxa/aio-coding-hub/commit/4f445106fefa10badae230de52c9fee09bd2486f))
* **home:** implement window foreground detection for usage heatmap refresh ([4e66f35](https://github.com/dyndynjyxa/aio-coding-hub/commit/4e66f359f198ddddc52b6cd4c0ab8cdb59630a27))
* init ([7e30c40](https://github.com/dyndynjyxa/aio-coding-hub/commit/7e30c40727d50980bcd43c2f275419a74fa3b148))
* **model-prices:** add model price alias rules ([60cbcc1](https://github.com/dyndynjyxa/aio-coding-hub/commit/60cbcc1c65ff025e79313facaf27e625a3de9997))
* **providers:** collapse model mapping editors ([4672961](https://github.com/dyndynjyxa/aio-coding-hub/commit/4672961c8facbd27d715a762864c2bf4f32ac932))
* **tauri:** add WSL support and listen modes ([a357007](https://github.com/dyndynjyxa/aio-coding-hub/commit/a35700753e9633493f6e939d1700ce979d635c93))
* **ui:** align CLI manager with network and WSL settings ([ae5b5fc](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae5b5fc99330b55872e1c30da6e653d7433b7d48))


### Bug Fixes

* **gateway:** reject forwarding when CLI proxy disabled ([c9edd10](https://github.com/dyndynjyxa/aio-coding-hub/commit/c9edd10cd2f41ef86c8c4c8a3ca2262c8bcb09ef))
* **usage:** align cache creation ttl to 5m only ([8d28bcd](https://github.com/dyndynjyxa/aio-coding-hub/commit/8d28bcd2f5d7f8d6bac1a7f65f974c04c5fce337))

## [0.12.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.11.0...aio-coding-hub-v0.12.0) (2026-01-20)


### Features

* add TextEvidenceSection component for improved output display in ClaudeModelValidationResultPanel ([47be119](https://github.com/dyndynjyxa/aio-coding-hub/commit/47be119a83c365b3e7b41f22308be7550ecaede5))
* **claude-validation:** add signature and caching roundtrip probes ([15badee](https://github.com/dyndynjyxa/aio-coding-hub/commit/15badee08b0c14f71695e6e71f0b165e4844371c))
* enhance provider model configuration with support for model whitelisting and mapping ([4f44510](https://github.com/dyndynjyxa/aio-coding-hub/commit/4f445106fefa10badae230de52c9fee09bd2486f))
* **home:** implement window foreground detection for usage heatmap refresh ([4e66f35](https://github.com/dyndynjyxa/aio-coding-hub/commit/4e66f359f198ddddc52b6cd4c0ab8cdb59630a27))
* **model-prices:** add model price alias rules ([60cbcc1](https://github.com/dyndynjyxa/aio-coding-hub/commit/60cbcc1c65ff025e79313facaf27e625a3de9997))
* **providers:** collapse model mapping editors ([4672961](https://github.com/dyndynjyxa/aio-coding-hub/commit/4672961c8facbd27d715a762864c2bf4f32ac932))
* **tauri:** add WSL support and listen modes ([a357007](https://github.com/dyndynjyxa/aio-coding-hub/commit/a35700753e9633493f6e939d1700ce979d635c93))
* **ui:** align CLI manager with network and WSL settings ([ae5b5fc](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae5b5fc99330b55872e1c30da6e653d7433b7d48))


### Bug Fixes

* **gateway:** reject forwarding when CLI proxy disabled ([c9edd10](https://github.com/dyndynjyxa/aio-coding-hub/commit/c9edd10cd2f41ef86c8c4c8a3ca2262c8bcb09ef))
* **usage:** align cache creation ttl to 5m only ([8d28bcd](https://github.com/dyndynjyxa/aio-coding-hub/commit/8d28bcd2f5d7f8d6bac1a7f65f974c04c5fce337))

## [0.11.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.10.0...aio-coding-hub-v0.11.0) (2026-01-18)


### Features

* init ([7e30c40](https://github.com/dyndynjyxa/aio-coding-hub/commit/7e30c40727d50980bcd43c2f275419a74fa3b148))

## [0.10.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.9.0...aio-coding-hub-v0.10.0) (2026-01-18)


### Features

* init ([7e30c40](https://github.com/dyndynjyxa/aio-coding-hub/commit/7e30c40727d50980bcd43c2f275419a74fa3b148))

## [0.9.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.8.0...aio-coding-hub-v0.9.0) (2026-01-18)


### Features

* init ([7e30c40](https://github.com/dyndynjyxa/aio-coding-hub/commit/7e30c40727d50980bcd43c2f275419a74fa3b148))

## [0.8.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.7.0...aio-coding-hub-v0.8.0) (2026-01-17)


### Features

* add lucide-react icons to CLI Manager and Prompts pages, enhance button styles for better UX ([a8c947a](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8c947a6286ccb5db76e0722433454cb093e2319))
* add scatter plot functionality for cost analysis by CLI, provider, and model; update HomeCostPanel to support new data structure and improve cost tracking visuals ([5861144](https://github.com/dyndynjyxa/aio-coding-hub/commit/5861144e77076154be88160be2f30bbc72ce397f))
* enhance Claude model validation with new checks for output configuration, tool support, and multi-turn capabilities; update home overview panel and request log detail dialog for improved cost tracking ([56c4d8b](https://github.com/dyndynjyxa/aio-coding-hub/commit/56c4d8b8f05e7d142954c1230e9bcfe9b1503a71))
* enhance git hook installation process and improve error handling in install-git-hooks script; update package.json to ensure hooks are installed post-installation ([5030838](https://github.com/dyndynjyxa/aio-coding-hub/commit/5030838ccab6999f2351aae7ffa54f7e480b23c2))
* init ([7cf47ed](https://github.com/dyndynjyxa/aio-coding-hub/commit/7cf47ed0f0ab3b3f702e127ce9368d57d52ac9b5))
* 验证改为两轮分别测试不同指标 ([566f7b8](https://github.com/dyndynjyxa/aio-coding-hub/commit/566f7b821a01e441d1044ce1ce3a26abfc0def22))


### Bug Fixes

* **tauri:** replace invalid saturating_shl retry backoff ([b789ace](https://github.com/dyndynjyxa/aio-coding-hub/commit/b789ace7c4ff4c882abd7e443b2657cbd8b82e2d))

## [0.7.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.6.0...aio-coding-hub-v0.7.0) (2026-01-17)


### Features

* add scatter plot functionality for cost analysis by CLI, provider, and model; update HomeCostPanel to support new data structure and improve cost tracking visuals ([5861144](https://github.com/dyndynjyxa/aio-coding-hub/commit/5861144e77076154be88160be2f30bbc72ce397f))
* enhance Claude model validation with new checks for output configuration, tool support, and multi-turn capabilities; update home overview panel and request log detail dialog for improved cost tracking ([56c4d8b](https://github.com/dyndynjyxa/aio-coding-hub/commit/56c4d8b8f05e7d142954c1230e9bcfe9b1503a71))
* enhance git hook installation process and improve error handling in install-git-hooks script; update package.json to ensure hooks are installed post-installation ([5030838](https://github.com/dyndynjyxa/aio-coding-hub/commit/5030838ccab6999f2351aae7ffa54f7e480b23c2))

## [0.6.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.5.0...aio-coding-hub-v0.6.0) (2026-01-17)


### Features

* add lucide-react icons to CLI Manager and Prompts pages, enhance button styles for better UX ([a8c947a](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8c947a6286ccb5db76e0722433454cb093e2319))
* init ([7cf47ed](https://github.com/dyndynjyxa/aio-coding-hub/commit/7cf47ed0f0ab3b3f702e127ce9368d57d52ac9b5))
* 验证改为两轮分别测试不同指标 ([566f7b8](https://github.com/dyndynjyxa/aio-coding-hub/commit/566f7b821a01e441d1044ce1ce3a26abfc0def22))


### Bug Fixes

* **tauri:** replace invalid saturating_shl retry backoff ([b789ace](https://github.com/dyndynjyxa/aio-coding-hub/commit/b789ace7c4ff4c882abd7e443b2657cbd8b82e2d))

## [0.5.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.4.0...aio-coding-hub-v0.5.0) (2026-01-17)


### Features

* init ([7cf47ed](https://github.com/dyndynjyxa/aio-coding-hub/commit/7cf47ed0f0ab3b3f702e127ce9368d57d52ac9b5))
* 验证改为两轮分别测试不同指标 ([566f7b8](https://github.com/dyndynjyxa/aio-coding-hub/commit/566f7b821a01e441d1044ce1ce3a26abfc0def22))

## [0.4.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.3.0...aio-coding-hub-v0.4.0) (2026-01-17)


### Features

* init ([7cf47ed](https://github.com/dyndynjyxa/aio-coding-hub/commit/7cf47ed0f0ab3b3f702e127ce9368d57d52ac9b5))
* 验证改为两轮分别测试不同指标 ([566f7b8](https://github.com/dyndynjyxa/aio-coding-hub/commit/566f7b821a01e441d1044ce1ce3a26abfc0def22))

## [0.3.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.2.0...aio-coding-hub-v0.3.0) (2026-01-17)


### Features

* 验证改为两轮分别测试不同指标 ([566f7b8](https://github.com/dyndynjyxa/aio-coding-hub/commit/566f7b821a01e441d1044ce1ce3a26abfc0def22))

## [0.2.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.1.0...aio-coding-hub-v0.2.0) (2026-01-16)


### Features

* init ([7cf47ed](https://github.com/dyndynjyxa/aio-coding-hub/commit/7cf47ed0f0ab3b3f702e127ce9368d57d52ac9b5))
