# Changelog

## [0.35.2](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.35.1...aio-coding-hub-v0.35.2) (2026-04-04)


### Features

* **providers:** 添加 OAuth 限制重置倒计时 & 修复托盘退出重启问题 ([#168](https://github.com/dyndynjyxa/aio-coding-hub/issues/168)) ([4ca1248](https://github.com/dyndynjyxa/aio-coding-hub/commit/4ca124860c48fd3764ecf1da861adef072267ba7))
* **ui:** 设置与首页展示优化，并修复 Node 24 下 pre-push 单测问题 ([#169](https://github.com/dyndynjyxa/aio-coding-hub/issues/169)) ([165db18](https://github.com/dyndynjyxa/aio-coding-hub/commit/165db18dce650cee175a8df07f9bc80168575aab))


### Bug Fixes

* **gateway:** 修复 HALF_OPEN 状态下熔断未及时解除的前端展示问题 ([#166](https://github.com/dyndynjyxa/aio-coding-hub/issues/166)) ([4c809f9](https://github.com/dyndynjyxa/aio-coding-hub/commit/4c809f9b0b6aee4e426d8f3a6d2aa53699fd7b35))

## [0.35.1](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.35.0...aio-coding-hub-v0.35.1) (2026-04-03)


### Features

* **circuit-breaker:** 支持运行时动态更新断路器配置 ([3cb8722](https://github.com/dyndynjyxa/aio-coding-hub/commit/3cb87220345020ea8916f192ac6c84281d2736f2))
* **gateway:** 增强上游错误处理与界面错误详情展示 ([85f67c4](https://github.com/dyndynjyxa/aio-coding-hub/commit/85f67c4c1ffe08d102883458a2d9d29e0b2b98cf))


### Bug Fixes

* **ci:** 修复Rust测试线程数为单线程运行 ([87ec1fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/87ec1fe962d7b2913841e739e3ee7026f7145c36))
* **cli_update:** 修复提取语义版本字符串的字符截取逻辑 ([782fb95](https://github.com/dyndynjyxa/aio-coding-hub/commit/782fb95d05236afbbfdf80b4bfb15921b9c34b88))
* **gateway:** claude api-key auth fallback ([ac68780](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac687808a15cc8d412714d623e4a32ef10ece310))
* **gateway:** 修复 CX2CC 无 SSE 响应头成功响应的分类时机 ([#160](https://github.com/dyndynjyxa/aio-coding-hub/issues/160)) ([9edcd5b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9edcd5bf8396a0a2a84e5b368d984717b4d7c81f))
* **usage:** 修复 Usage 统计中的 CX2CC 缓存命中率计算 ([#159](https://github.com/dyndynjyxa/aio-coding-hub/issues/159)) ([3472560](https://github.com/dyndynjyxa/aio-coding-hub/commit/3472560f62f88e4eae4b4d53e920c290869a2c7d))
* WebView2 不可恢复状态检测与分级恢复 ([#156](https://github.com/dyndynjyxa/aio-coding-hub/issues/156)) ([4869745](https://github.com/dyndynjyxa/aio-coding-hub/commit/4869745cb352279829317f18b5e1956c381b5b38))

## [0.35.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.34.0...aio-coding-hub-v0.35.0) (2026-04-02)


### ⚠ BREAKING CHANGES

* **infra:** request_attempt_logs 表不再创建或写入

### Features

* add Claude OAuth support with multi-account switching ([#149](https://github.com/dyndynjyxa/aio-coding-hub/issues/149)) ([ef787b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef787b1ddf309ffcf9ffcb923d92bf2af3f557b6))
* add CX2CC tab, config export/import, Gemini config enhance, CLI… ([#138](https://github.com/dyndynjyxa/aio-coding-hub/issues/138)) ([7aba381](https://github.com/dyndynjyxa/aio-coding-hub/commit/7aba381a462812e6c6ea505de3650397d23b48b4))
* add gemini oauth code assist proxy support ([4a892d0](https://github.com/dyndynjyxa/aio-coding-hub/commit/4a892d035fbcdd5273c909de063eede5d42598fa))
* add TextEvidenceSection component for improved output display in ClaudeModelValidationResultPanel ([47be119](https://github.com/dyndynjyxa/aio-coding-hub/commit/47be119a83c365b3e7b41f22308be7550ecaede5))
* **app:** add LogsPage route and enhance CLI manager settings ([6796715](https://github.com/dyndynjyxa/aio-coding-hub/commit/679671526be935129211978b3598a3962f7e2a8e))
* **app:** 优化页面路由懒加载和加载反馈体验 ([504256e](https://github.com/dyndynjyxa/aio-coding-hub/commit/504256e11ed517797405f8bb4ab3d353607cb1ae))
* **app:** 添加应用心跳机制与监听支持 ([df78171](https://github.com/dyndynjyxa/aio-coding-hub/commit/df781712d16a2f5ccc99e37a687d39cb4e4f56c5))
* **bindings:** 添加 HomeUsagePeriod 类型契约断言 ([a540d52](https://github.com/dyndynjyxa/aio-coding-hub/commit/a540d52012652075deee18c58d3e4b5611f4d547))
* **build:** 添加Wayland兼容的AppImage构建支持 ([95c7130](https://github.com/dyndynjyxa/aio-coding-hub/commit/95c71307038962fc0e37fbd7b6df844301d8decb))
* **build:** 添加Windows便携版ZIP构建和WebView2检查 ([fef41e1](https://github.com/dyndynjyxa/aio-coding-hub/commit/fef41e12f2a1301743b40de4c7402f4b02fa80f4))
* **cache-anomaly-monitor:** implement cache anomaly monitoring feature ([59d69d1](https://github.com/dyndynjyxa/aio-coding-hub/commit/59d69d1528d888a84d71038873321f3273d4ded1))
* **charts:** migrate from ECharts to Recharts for improved charting capabilities ([b747b61](https://github.com/dyndynjyxa/aio-coding-hub/commit/b747b61a8dd587043a7e8de01a5f9b3d64ead7ae))
* **claude-validation:** add signature and caching roundtrip probes ([15badee](https://github.com/dyndynjyxa/aio-coding-hub/commit/15badee08b0c14f71695e6e71f0b165e4844371c))
* **ClaudeModelValidationDialog:** 添加综合协议验证功能与UI展示 ([b6ede30](https://github.com/dyndynjyxa/aio-coding-hub/commit/b6ede30e61e19f9af69e8d7153cbc711b3e3a180))
* **ClaudeModelValidation:** enhance cross-provider validation and UI feedback ([bf83c7e](https://github.com/dyndynjyxa/aio-coding-hub/commit/bf83c7e03c7edf78795cd51a943c01a88e0b17d7))
* **ClaudeModelValidation:** enhance output token validation and error handling ([d245288](https://github.com/dyndynjyxa/aio-coding-hub/commit/d245288d7a4937ca7b0213ebd79d9c0d5e3c76b4))
* **ClaudeModelValidation:** implement cross-provider signature validation and enhance request handling ([2e102d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/2e102d4f3fd2745e4480a5884272baeafe66b6d0))
* **claudeModelValidation:** 优化模型验证对话框和多轮验证流程支持 ([3f6c031](https://github.com/dyndynjyxa/aio-coding-hub/commit/3f6c03102db7aaeaa2d8399ebdac1cc4787c2a84))
* **cli_proxy:** 添加 merge-restore 功能以保留用户更改 ([a7d05c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/a7d05c97fe1228d83ccf97be5ed9b6ec26c7bde0))
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
* **cli-manager:** 对齐 CCH 基础配置开关 ([55779ef](https://github.com/dyndynjyxa/aio-coding-hub/commit/55779efce6964a17b423869af77f3ebff567d62c))
* **cli-manager:** 支持自定义GPT-5.4模型上下文窗口和自动压缩限制 ([024dca7](https://github.com/dyndynjyxa/aio-coding-hub/commit/024dca78798022fef014a6a4c5b7a63eb6ecd663))
* **cli-sessions:** P0批次1 - 安全校验/React Query/Router state/虚拟化依赖 ([53c9d43](https://github.com/dyndynjyxa/aio-coding-hub/commit/53c9d4320566b03ff5e655ad2442d46a67f77572))
* **cli-sessions:** P0批次2 - 应用安全校验和状态管理重构 ([2dd098f](https://github.com/dyndynjyxa/aio-coding-hub/commit/2dd098fd39d0d45040b83a0cbd80cb13d4a29352))
* **cli-sessions:** P0批次3 - 三个列表虚拟化 ([d48fccd](https://github.com/dyndynjyxa/aio-coding-hub/commit/d48fccd775086bd11c6cf01147fedf0a3c44e86a))
* **cli-sessions:** P1批次 - 布局一致性和Shell转义安全 ([84723f3](https://github.com/dyndynjyxa/aio-coding-hub/commit/84723f377e5088d2f80574be84e6fa819038bb36))
* **cli:** add CLI proxy startup recovery feature ([eb40a6f](https://github.com/dyndynjyxa/aio-coding-hub/commit/eb40a6f03d9a3b50b501795727a06b7ce9013fc1))
* **CliManager:** add response fixer configuration limits and UI inputs ([0023ad6](https://github.com/dyndynjyxa/aio-coding-hub/commit/0023ad69abf91f48a5144250e20b53ea0b2e24bf))
* **cli:** 添加 Claude 终端启动命令生成及剪贴板管理插件支持 ([ba3d3c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba3d3c20d8d1d40accc0d97780c4dae54d48c71a))
* **codex:** add multi_agent feature toggle ([f56a201](https://github.com/dyndynjyxa/aio-coding-hub/commit/f56a2016bb535812e1a00edb0056e0faa6f44957))
* **codex:** support configurable Windows .codex locations ([#114](https://github.com/dyndynjyxa/aio-coding-hub/issues/114)) ([a8c77cd](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8c77cdebdca52f7bb5ca758837c2b04bac14e3f))
* **components:** enhance HomeRequestLogsPanel and RealtimeTraceCards with new formatting utilities ([e6a3550](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6a3550636e4278100a24d009e49ade1056733bc))
* **config:** 新增 personality 输出风格和 websocket 实验功能支持 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **config:** 新增计划模式推理强度配置项 ([6f5da87](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f5da87696af5c90054d6ca5be0f25e0a91eba40))
* **config:** 添加 GPT-5.4 关联配置及快速模式支持 ([38a11cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/38a11cb4c33300a2a7edb19848d97e25e966688b))
* **ConsolePage:** revamp console log display and functionality ([1d28397](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d28397e88c0b6d43a4d73b348c49c93cb18efde))
* **console:** 改进控制台界面并增加日志过滤和搜索功能 ([93badf8](https://github.com/dyndynjyxa/aio-coding-hub/commit/93badf8766c9a415c821c3f14084183d18d31687))
* **core:** improve workspace imports and startup resilience ([1d7dbf0](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d7dbf0b73cde2b78a70be945c6fd8e248b30c1d))
* **cx2cc:** add Codex-to-Claude-Code translation bridge ([#100](https://github.com/dyndynjyxa/aio-coding-hub/issues/100)) ([44d9c41](https://github.com/dyndynjyxa/aio-coding-hub/commit/44d9c416e434b72699e95984dbc156d413fc78dc))
* **date-range:** refactor custom date range handling in HomeCostPanel and UsagePage ([95b1ad1](https://github.com/dyndynjyxa/aio-coding-hub/commit/95b1ad11b16a0f3e9b187b4883ef85a367d712e0))
* **domain:** 支持 cx2cc 提供者及缓存令牌追踪 ([3662c26](https://github.com/dyndynjyxa/aio-coding-hub/commit/3662c26c2ea72c25ef7e330a52a414a708164cd1))
* enhance provider model configuration with support for model whitelisting and mapping ([4f44510](https://github.com/dyndynjyxa/aio-coding-hub/commit/4f445106fefa10badae230de52c9fee09bd2486f))
* **failover-loop:** refactor context management for improved clarity and usability ([b96df01](https://github.com/dyndynjyxa/aio-coding-hub/commit/b96df013317c77ca610bb640a43b2d60058e6847))
* **failover-loop:** refactor error handling and logging in failover loop ([ef2662c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef2662c335ae4057c591b531b0784c86cd73951e))
* **gateway,providers,oauth:** codex-oauth-with-proxy ([#76](https://github.com/dyndynjyxa/aio-coding-hub/issues/76)) ([f37e59a](https://github.com/dyndynjyxa/aio-coding-hub/commit/f37e59a3dbfdc8a95d18e592f1931c775f19c2ab))
* **gateway:** 优化协议兼容性处理与请求头管理 ([6be9161](https://github.com/dyndynjyxa/aio-coding-hub/commit/6be91613fbc0bfcd9b87fe419a8f9ad08178a1f7))
* **gateway:** 实现熔断器半开状态与相关逻辑 ([9bd8146](https://github.com/dyndynjyxa/aio-coding-hub/commit/9bd81467541ceb40009dce68e052aeee44e82e36))
* **gateway:** 对齐 CCH 基础配置整流 ([a1c1b5c](https://github.com/dyndynjyxa/aio-coding-hub/commit/a1c1b5c2b730b89184d59fac207d252e9d415c93))
* **gateway:** 支持会话绑定提供者的查询与恢复 ([cbfb17e](https://github.com/dyndynjyxa/aio-coding-hub/commit/cbfb17e385f6ee4689148c8630f56bed0c281c95))
* **gateway:** 支持强制指定请求的Provider并更新Claude终端启动逻辑 ([904a9aa](https://github.com/dyndynjyxa/aio-coding-hub/commit/904a9aa6805681fb5642e350105803ce01ae998b))
* **gateway:** 新增计费头修正功能及提供流空闲超时配置能力 ([bbf3d29](https://github.com/dyndynjyxa/aio-coding-hub/commit/bbf3d2910d6e5c7cda76b3a11032eb117b709376))
* **gateway:** 添加决策链以丰富失败重试事件细节 ([262e98c](https://github.com/dyndynjyxa/aio-coding-hub/commit/262e98cb0ac974c5452e037ac51d5730625d1ef2))
* **home:** add provider limit usage overview tab ([c224748](https://github.com/dyndynjyxa/aio-coding-hub/commit/c224748c590d1b242df045a7e81667a623d0ec0b))
* **home:** add provider limit usage overview tab ([6473253](https://github.com/dyndynjyxa/aio-coding-hub/commit/64732536a25e140193242e9829aacb2cce15f05d))
* **home:** add window start timestamps for provider limit usage ([69a91a2](https://github.com/dyndynjyxa/aio-coding-hub/commit/69a91a21567b6a2c9af2ac879936dd75a6004e37))
* **home:** enhance HomeCostPanel with data-testid attributes and update tests ([b1d23d2](https://github.com/dyndynjyxa/aio-coding-hub/commit/b1d23d2575afb25d3314911b0c6488a872f56b46))
* **home:** enhance status handling with failover support ([5aab8a9](https://github.com/dyndynjyxa/aio-coding-hub/commit/5aab8a92c4440062d18a3ecf495375d11a461660))
* **home:** enhance UI components and improve layout consistency ([70d9655](https://github.com/dyndynjyxa/aio-coding-hub/commit/70d9655cf1a740676e81bcb0a1e204b81abdd1c9))
* **home:** implement CLI proxy environment conflict checks ([aceff42](https://github.com/dyndynjyxa/aio-coding-hub/commit/aceff428537261c5132b5cb4ebdcb22d16efec82))
* **home:** implement CLI proxy environment conflict checks ([4c014ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/4c014ca5ab04dab7aa5ca38799d69d1a8eb3cc3e))
* **home:** implement window foreground detection for usage heatmap refresh ([4e66f35](https://github.com/dyndynjyxa/aio-coding-hub/commit/4e66f359f198ddddc52b6cd4c0ab8cdb59630a27))
* **home:** 添加供应商限额面板刷新按钮 ([18f4f91](https://github.com/dyndynjyxa/aio-coding-hub/commit/18f4f91d375f860312a126354a65efad6244e7ae))
* **home:** 添加工作区模式切换按钮并持久化请求日志简洁模式 ([d73bedd](https://github.com/dyndynjyxa/aio-coding-hub/commit/d73bedd3bc27d82375969d09fa94336a268bd075))
* init ([7e30c40](https://github.com/dyndynjyxa/aio-coding-hub/commit/7e30c40727d50980bcd43c2f275419a74fa3b148))
* integrate PageHeader component across multiple pages for consistent UI ([330da27](https://github.com/dyndynjyxa/aio-coding-hub/commit/330da276f9ef8e91744a9534d59590a3a6fec5ff))
* mac最小化优化 ([1f11a5d](https://github.com/dyndynjyxa/aio-coding-hub/commit/1f11a5df9623f8cab90bb40f5c499c5d3569f01d))
* **mcp:** implement batch import for local skills and add global error reporting ([51e053c](https://github.com/dyndynjyxa/aio-coding-hub/commit/51e053c81f1db00696f0cb968860033deee12cce))
* **mcp:** 优化服务器键生成与导入逻辑，支持保留大小写 ([f61a5c6](https://github.com/dyndynjyxa/aio-coding-hub/commit/f61a5c6be09795efaf9c4aafe7fee5f86329c0ea))
* **mcp:** 添加 env 和 header 键值对格式校验 ([7d698ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/7d698ca04f1cc9f9c0d420f6d7fec2a53f2d643a))
* **model-prices:** add model price alias rules ([60cbcc1](https://github.com/dyndynjyxa/aio-coding-hub/commit/60cbcc1c65ff025e79313facaf27e625a3de9997))
* **Oauth:** adding gemini Oauth support ([fd1eb79](https://github.com/dyndynjyxa/aio-coding-hub/commit/fd1eb7933410510e9ee46e469234d91658a49fcf))
* **provider-chain:** 优化供应商链路视图及日志详情显示 ([72d4bc4](https://github.com/dyndynjyxa/aio-coding-hub/commit/72d4bc49dfd1380ce503ec23a98eb81b55a1d488))
* **provider-editor:** enhance ProviderEditorDialog with limit configuration cards ([cc14a00](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc14a009303129793ceec840933155dcfad775d8))
* **providers:** collapse model mapping editors ([4672961](https://github.com/dyndynjyxa/aio-coding-hub/commit/4672961c8facbd27d715a762864c2bf4f32ac932))
* **providers:** 在供应商列表页增加名称搜索功能 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **providers:** 增加供应商备注字段并支持读取API Key ([fec7057](https://github.com/dyndynjyxa/aio-coding-hub/commit/fec7057d137a7c0b38b46b5591a9e19e624b679a))
* **providers:** 支持复制供应商配置并预填创建表单 ([c8fb707](https://github.com/dyndynjyxa/aio-coding-hub/commit/c8fb7078131ad67e4d2895f81467a9358b897d2c))
* **providers:** 添加供应商标签功能，支持分类筛选 ([7671732](https://github.com/dyndynjyxa/aio-coding-hub/commit/76717329a119f9a65dc2f3a9dffe0a887063b0bd))
* **proxy:** 增加请求中断日志中尝试信息的捕获与展示 ([0ad0955](https://github.com/dyndynjyxa/aio-coding-hub/commit/0ad0955730da9ee46e1904838412fe546794e0f5))
* **proxy:** 实现对 Claude 请求的观察与请求日志生命周期管理 ([107d892](https://github.com/dyndynjyxa/aio-coding-hub/commit/107d89202defcd4bbb8727b15ec88bcf6bfe49e0))
* **proxy:** 支持 Codex 会话 ID 补全功能 ([25f8321](https://github.com/dyndynjyxa/aio-coding-hub/commit/25f8321536ccdfe9a68de1866bb9762794ac97b7))
* **request-end:** enhance request logging and event emission ([809a684](https://github.com/dyndynjyxa/aio-coding-hub/commit/809a6843a2c2eb952bbb00f7c56694c908d99bd1))
* **request-end:** refactor request logging and event emission ([e56909e](https://github.com/dyndynjyxa/aio-coding-hub/commit/e56909e374c26608407a1d63d32c9d55f914b355))
* **responsive:** enhance layout and styling for improved mobile experience ([e22483a](https://github.com/dyndynjyxa/aio-coding-hub/commit/e22483a8da8c35822c6e5b0532aa1461cb68cf95))
* **route:** 支持展示同一 provider 的连续尝试次数及跳过状态 ([6659157](https://github.com/dyndynjyxa/aio-coding-hub/commit/6659157a12c3809b1e4d8de7bf95b95afe6ed383))
* **settings:** update Claude settings and remove deprecated options ([49e055a](https://github.com/dyndynjyxa/aio-coding-hub/commit/49e055aeb176819cb56a925aecbd51f362008762))
* **settings:** 新增任务结束提醒开关和相关功能 ([2fb4022](https://github.com/dyndynjyxa/aio-coding-hub/commit/2fb4022ff2f1078238ced25188960f19da188260))
* **settings:** 新增关闭 Claude Git 参与者功能 ([cf23f0d](https://github.com/dyndynjyxa/aio-coding-hub/commit/cf23f0dec000cff63f222b0198360485deb60804))
* **settings:** 新增缓存异常监测功能开关 ([ea681c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ea681c155346da3057f25e25867ab75a818f4157))
* **settings:** 新增静默启动配置支持 ([fcc436b](https://github.com/dyndynjyxa/aio-coding-hub/commit/fcc436b5dc5baf3b57e7160bd7e3562275535072))
* **settings:** 添加通知声音开关配置项 ([d13ee18](https://github.com/dyndynjyxa/aio-coding-hub/commit/d13ee18228cde971e7490468c6831aa679a636ab))
* **settings:** 添加通知音效开关及完善通用配置界面 ([e8d285a](https://github.com/dyndynjyxa/aio-coding-hub/commit/e8d285a704ba3f1a82fda9b16f9c2dbba393a57d))
* **SkillsMarketPage:** enhance UI with tab selection and external links ([2849017](https://github.com/dyndynjyxa/aio-coding-hub/commit/2849017554128279822fef9b667d8ec166a08432))
* **skills:** support repo market and local CLI workflows ([3a0a24f](https://github.com/dyndynjyxa/aio-coding-hub/commit/3a0a24f13405d493237402dace175807b42de462))
* **skills:** 支持从本地源恢复缺失的ssot技能目录 ([4414f7d](https://github.com/dyndynjyxa/aio-coding-hub/commit/4414f7d38a44e4aff7de0730fe591b93146d7468))
* **skills:** 添加 Skill 返回本机已安装功能 ([84cfd0b](https://github.com/dyndynjyxa/aio-coding-hub/commit/84cfd0be09c5c38dbd9613533062a3daef6c57df))
* **sort-modes:** add toggle functionality for enabling/disabling providers in sort modes and update related services ([e6cd992](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6cd99241f5c86da3f62afebefeffdf33e62b8f0))
* **taskCompleteNotifyEvents:** 改进任务完成通知的请求追踪与静默期控制 ([e3f03d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/e3f03d4bf2276db371e170ee4739cd379b6d3fe5))
* **tauri:** add WSL support and listen modes ([a357007](https://github.com/dyndynjyxa/aio-coding-hub/commit/a35700753e9633493f6e939d1700ce979d635c93))
* **tauri:** update error handling and add thiserror dependency ([10d918a](https://github.com/dyndynjyxa/aio-coding-hub/commit/10d918a0a0a746b4d25c1c236f88c0110ec664c3))
* **tests:** add comprehensive unit tests for various components ([98fb022](https://github.com/dyndynjyxa/aio-coding-hub/commit/98fb022d1487e593e1fe50a1e9d02592671a4944))
* **theme:** add dark mode and native window theme sync ([588a373](https://github.com/dyndynjyxa/aio-coding-hub/commit/588a37311c456bd2182ac1676c8e9b309157353f))
* **ui:** align CLI manager with network and WSL settings ([ae5b5fc](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae5b5fc99330b55872e1c30da6e653d7433b7d48))
* **ui:** 优化首页概览、CLI 代理状态与供应商页交互 ([#106](https://github.com/dyndynjyxa/aio-coding-hub/issues/106)) ([358e73f](https://github.com/dyndynjyxa/aio-coding-hub/commit/358e73f598fd48c7bbf131a3117e4801cec03ced))
* **ui:** 优化首页概览、供应商管理与设置页主题入口 ([#102](https://github.com/dyndynjyxa/aio-coding-hub/issues/102)) ([0adec0a](https://github.com/dyndynjyxa/aio-coding-hub/commit/0adec0af676842e9c5aee3e08e3a46d890d31c11))
* **ui:** 本次主要优化了首页概览、设置页和代理记录页，同时顺手处理了首页包体和 Tauri 构建告警。 ([#111](https://github.com/dyndynjyxa/aio-coding-hub/issues/111)) ([4e6dbad](https://github.com/dyndynjyxa/aio-coding-hub/commit/4e6dbad14cab8334d47a3a30ebfcf70485ad2e0d))
* **ui:** 统一 Loading/Empty/Error 状态反馈组件 ([b27f64a](https://github.com/dyndynjyxa/aio-coding-hub/commit/b27f64a79a1ad655a37992527794759f9f8c5370))
* **updater:** display release notes in update dialog ([b891ed7](https://github.com/dyndynjyxa/aio-coding-hub/commit/b891ed75a1b67a9df281917ffa94e684fb372664))
* **usage:** add cache rate trend functionality to UsagePage ([5535e7d](https://github.com/dyndynjyxa/aio-coding-hub/commit/5535e7d3480237284037ef841d2878bfa6a180f5))
* **usage:** add cost tracking to usage statistics ([765ea8a](https://github.com/dyndynjyxa/aio-coding-hub/commit/765ea8ae06efe2cc39c6eac7e95dccfbba4de541))
* **usage:** add summary stat cards with filter layout optimization ([f6c9206](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6c9206ba6e0b424748ba05f0ac749884d5bf0d3))
* **usage:** 添加供应商过滤支持以改进使用统计查询 ([e2178d9](https://github.com/dyndynjyxa/aio-coding-hub/commit/e2178d93ae413307cd4bb5f830c30550a6e0af1d))
* **workspaces:** add WorkspacesPage route and enhance CLI manager settings ([2784072](https://github.com/dyndynjyxa/aio-coding-hub/commit/2784072e26748aa21f679c245f74cf3063f4177c))
* **workspaces:** enhance MCP and Prompts management with workspace support ([791ee6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/791ee6c88b06c16c596b82dcf7956ea5b98dc18d))
* **workspaces:** improve workspace switching and management functionality ([496c9c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/496c9c35daa23be4d2c44f0653bb11bcb5206e1f))
* **wsl:** 完善 WSL 会话浏览、MCP/提示词同步及删除功能，补充前端测试覆盖率 ([b1308a1](https://github.com/dyndynjyxa/aio-coding-hub/commit/b1308a1e7f3eaf7e871862f3119d7678be72c6aa))
* **wsl:** 支持WSL宿主机地址模式自动检测与自定义 ([f809e86](https://github.com/dyndynjyxa/aio-coding-hub/commit/f809e86ab3690214ba2407c14e4dee1febd9ab14))
* **wsl:** 支持浏览 WSL 环境中的 CLI 会话记录，增加wsl提示词和mcp同步，并有可见提示 ([6147df2](https://github.com/dyndynjyxa/aio-coding-hub/commit/6147df28f904798b7d8e02c5e5946ec3453187c6))
* **wsl:** 支持浏览 WSL 环境中的 CLI 会话记录，增加wsl提示词和mcp同步，并有可见提示 ([41d7d26](https://github.com/dyndynjyxa/aio-coding-hub/commit/41d7d2661a03a3dea90bcb63a06f384fde1ac31a))
* **wsl:** 添加 WSL 配置生命周期管理 — 退出恢复与崩溃自愈 ([bcbf54d](https://github.com/dyndynjyxa/aio-coding-hub/commit/bcbf54df5bf1e6b851f223ffad2a0ee135647c13))
* **wsl:** 添加启动时自动检测和配置 WSL 环境功能 ([707bd55](https://github.com/dyndynjyxa/aio-coding-hub/commit/707bd5518fd9881715ec5f3df0ef9250970570bb))


### Bug Fixes

* **app:** 修复并增强数据库重置和WSL配置功能 ([caf135a](https://github.com/dyndynjyxa/aio-coding-hub/commit/caf135a98905bcc4594f9f6c3729297e3c6dcf1c))
* **ci:** add explicit permissions to release-please job ([8c77c04](https://github.com/dyndynjyxa/aio-coding-hub/commit/8c77c041b0d6f5081abf6ea9a39e1031dc56ad75))
* **ci:** ensure consistent use of RELEASE_PLEASE_TOKEN in release workflow for GitHub actions ([f8a439d](https://github.com/dyndynjyxa/aio-coding-hub/commit/f8a439d213346a907dfc7355b6b6caf4b4194799))
* **ci:** quote FALLBACK_NOTES value in release workflow ([536896c](https://github.com/dyndynjyxa/aio-coding-hub/commit/536896ca4072f48002e8c83b91d78164ed40384f))
* **ci:** revert release-please job to original config ([0d1afad](https://github.com/dyndynjyxa/aio-coding-hub/commit/0d1afade58707c53554f7680dd7455d6ef547187))
* **ci:** satisfy rust clippy checks ([38d0bc0](https://github.com/dyndynjyxa/aio-coding-hub/commit/38d0bc0081cc90b24bd83e4ced30566d861ea69b))
* **ci:** update release workflow to consistently use RELEASE_PLEASE_TOKEN for GitHub actions ([2c45c46](https://github.com/dyndynjyxa/aio-coding-hub/commit/2c45c461ddf157a4b781c146a0ebed5cd9dc1a44))
* **ci:** update release workflow to use RELEASE_PLEASE_TOKEN for GitHub actions ([48ec3ce](https://github.com/dyndynjyxa/aio-coding-hub/commit/48ec3ce92e28702ee9920eaa42d2f7374b07be31))
* **ci:** use PAT for release-please to fix permission issue ([a2919f2](https://github.com/dyndynjyxa/aio-coding-hub/commit/a2919f2446d8fcb1c4c7ac8dc6a1b4f63f705157))
* **ci:** 优化 pre-push 钩子和测试命令的标准输入配置 ([d14abe2](https://github.com/dyndynjyxa/aio-coding-hub/commit/d14abe2ce2f35e53ea0ab750e8598aafb65f33ed))
* **ci:** 修复CI流程中正则表达式重复问题 ([609f2ea](https://github.com/dyndynjyxa/aio-coding-hub/commit/609f2ea657034a58b6f175b995e05431829f6dd9))
* **ci:** 修复release.yml中AppImage路径解析问题 ([1876bd4](https://github.com/dyndynjyxa/aio-coding-hub/commit/1876bd428f888e9c2eb286a20b9aeeb00dda5c5a))
* **ci:** 修复release工作流中appimagetool下载地址 ([67d8071](https://github.com/dyndynjyxa/aio-coding-hub/commit/67d8071fecf6dfc227f8e41d03f067ec481bfd4f))
* **ci:** 修复正则表达式转义字符问题 ([ba805c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba805c3b2f147f153cbe39042b078547eb36da5a))
* **ci:** 修正发布流程中更新器 JSON 文件参数名称 ([9a4c0f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/9a4c0f063bd645e3286e67f1607334a84a5cb6fd))
* **ClaudeModelValidationDialog:** adjust dialog max-width for responsive design ([11bdfd0](https://github.com/dyndynjyxa/aio-coding-hub/commit/11bdfd0df1b51923a561211f6cc6d5383eedf76c))
* **cli_manager:** 修复 run_in_login_shell 的平台兼容性处理 ([9f81433](https://github.com/dyndynjyxa/aio-coding-hub/commit/9f814337d08f36c08e37486f95b9c9da62880fc1))
* **cli_proxy:** 修复数据库初始化错误处理逻辑 ([9230475](https://github.com/dyndynjyxa/aio-coding-hub/commit/9230475e4401e93e3dc34ad285ace4a03186cd16))
* **cli-manager:** 修正模型参数解析逻辑以避免默认值覆盖 ([ae2b7fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae2b7feecfd6e06462b8b0990204fc55d6ad3ccf))
* **cli-proxy:** 修复 Codex CLI 在 Windows 下的沙箱配置与认证模式 ([b79d072](https://github.com/dyndynjyxa/aio-coding-hub/commit/b79d072f39405c15125a8a6391bff122c9a5f315))
* **cli-sessions:** 修复SessionsPage Card容器flex布局 ([2d2a048](https://github.com/dyndynjyxa/aio-coding-hub/commit/2d2a0481f9811160223f9753d6edec5a28dcfc6c))
* **codex-tab:** sandbox_mode danger-full-access 选择不生效及高级配置不回显 ([3943401](https://github.com/dyndynjyxa/aio-coding-hub/commit/3943401524c8ef6a179e65e8ee62fd2ac4f784cb))
* **codex:** resolve oauth login failure and stale provider names ([#116](https://github.com/dyndynjyxa/aio-coding-hub/issues/116)) ([48d647e](https://github.com/dyndynjyxa/aio-coding-hub/commit/48d647ef429232562c9b56e88d8a37f57960604a))
* **codex:** 修复切换 Codex Home 后代理状态与实际配置不一致 ([#125](https://github.com/dyndynjyxa/aio-coding-hub/issues/125)) ([e8f80b2](https://github.com/dyndynjyxa/aio-coding-hub/commit/e8f80b2838509b748d38ed6411219218b4077bf5))
* **commands:** 修复构建Claude启动命令时路径参数传递 ([9e714f4](https://github.com/dyndynjyxa/aio-coding-hub/commit/9e714f48bdbf7dff2d729131774518845303290a))
* **components:** 修复ClaudeModelValidationDialog点击事件阻塞问题 ([9bd5f53](https://github.com/dyndynjyxa/aio-coding-hub/commit/9bd5f53f38d01fc312b979e3620b5e0badaba585))
* **cost:** 修正成本乘数比较逻辑为包含零值 ([db69776](https://github.com/dyndynjyxa/aio-coding-hub/commit/db6977635f96fc55fa1b4d84d1d71e67dd8dd0b1))
* **db:** 修复providers表note字段迁移及数据处理问题 ([fa8ecda](https://github.com/dyndynjyxa/aio-coding-hub/commit/fa8ecda9bad231ed424c812c1c7cd3d0850b8565))
* **deps:** 升级 react-router-dom 7.11→7.13 修复 XSS 漏洞 ([c4db630](https://github.com/dyndynjyxa/aio-coding-hub/commit/c4db630b4478cab2f1180f03b324cfc0af9d42e3))
* **domain:** 修复 SSE 流错误处理逻辑 ([4446939](https://github.com/dyndynjyxa/aio-coding-hub/commit/4446939c457735727f96dce0b09c91ae6b616c1d))
* **domain:** 修正 claude_terminal_launch_context 函数中 provider_id 验证和查询逻辑 ([316d375](https://github.com/dyndynjyxa/aio-coding-hub/commit/316d3754fc628d0a1e435aabf469ca992cb63efa))
* **gateway:** allow failover success handler arg count ([0c559cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/0c559cba801d59332a94eb650571487b6b18a53b))
* **gateway:** finalize failed requests and align error codes ([871de9c](https://github.com/dyndynjyxa/aio-coding-hub/commit/871de9c9c9e06ed6cb03026ceb21dfc4b0ba18d3))
* **gateway:** reject forwarding when CLI proxy disabled ([c9edd10](https://github.com/dyndynjyxa/aio-coding-hub/commit/c9edd10cd2f41ef86c8c4c8a3ca2262c8bcb09ef))
* **gateway:** 修正 Heartbeat 事件变量可见性及用法 ([a540d52](https://github.com/dyndynjyxa/aio-coding-hub/commit/a540d52012652075deee18c58d3e4b5611f4d547))
* **gateway:** 修正 provider 重新排序时清除会话绑定逻辑 ([f03c02e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f03c02e5332fe84496a029b2e5d130b558cfad31))
* **gateway:** 调整上游连接超时与错误切换逻辑 ([a6870ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6870ae14b6b5bb8edce814ca37da2000e84f6b4))
* **heartbeat_watchdog:** 优化窗口重载的错误处理逻辑 ([6188e6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6188e6c29135c0126490882ceb54302d112ec79c))
* **home:** update HomeCostPanel with accessibility labels and improve test assertions ([0bda6c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/0bda6c1ce8aec6920b30810882f3a804c62ab732))
* **macOS:** 修复macOS 在 CLI 管理页面读取不到工具 version 并提示 INTERNAL_ERROR ([#126](https://github.com/dyndynjyxa/aio-coding-hub/issues/126)) ([ce35a94](https://github.com/dyndynjyxa/aio-coding-hub/commit/ce35a945dd278b453a1ddebbe8e95fd5f4f12ee6))
* **mcp:** 修复导入服务器时基于 server_key 的重复处理 ([f6e6458](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6e64582e1e4c478ec014e61258611d971f8b3b4))
* **notification:** 优化系统通知及心跳监测逻辑 ([cfeb63f](https://github.com/dyndynjyxa/aio-coding-hub/commit/cfeb63f6b331df3be3feddbf0aac258018dba824))
* **prompts:** 修改新增和编辑时 Prompt 保存按钮禁用条件 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **providers:** 敏感配置信息变更时清理运行时 session 绑定 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **proxy:** 允许 resolve_session_bound_provider_id 函数有多个参数 ([705457e](https://github.com/dyndynjyxa/aio-coding-hub/commit/705457eac5092ccd98a0f3523ea1986c67dd9d9c))
* **release:** keep Cargo.lock synced for release PRs ([c5bd423](https://github.com/dyndynjyxa/aio-coding-hub/commit/c5bd42355641cb8dcef13a1fa04d41795aeb5c82))
* **router:** 修复侧边栏快速点击导航卡死问题 ([8b1f862](https://github.com/dyndynjyxa/aio-coding-hub/commit/8b1f862542b8bd94770ab5f937772a53a4bd1d22))
* **scripts:** 修复生成绑定文件格式化问题 ([9ff697b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9ff697b6b8c93238f48518f27f274df33ba55a9a))
* **sessions:** 剥离替换字符并调整按钮样式 ([3586a16](https://github.com/dyndynjyxa/aio-coding-hub/commit/3586a165ceb8ba3afc42a34fdf4228c71b18cce5))
* **settings:** 增强设置读取错误处理和写入保护 ([6759e60](https://github.com/dyndynjyxa/aio-coding-hub/commit/6759e60edf1cb93e63f8c05787ed4f0a320a175b))
* **skills:** unify card layout between general and local skill sections ([7f3b7bb](https://github.com/dyndynjyxa/aio-coding-hub/commit/7f3b7bba9efe0818b11ae6ddbfa0dc1c1576f0fa))
* **skills:** 优化目录复制以正确处理符号链接 ([8b21265](https://github.com/dyndynjyxa/aio-coding-hub/commit/8b21265d116a184f82008b13792d472d68f13356))
* **skills:** 优化符号链接目录的移除逻辑 ([2ef9928](https://github.com/dyndynjyxa/aio-coding-hub/commit/2ef9928eb3d1d4fa23cb22e2035384e7be2a01f6))
* **skills:** 移除批量导入功能并改为刷新本机列表 ([45b9618](https://github.com/dyndynjyxa/aio-coding-hub/commit/45b961847b458f72f2bc062bdcdfef93c8722da8))
* suppress dead_code warnings on non-Windows CI and fix clippy lint ([8371047](https://github.com/dyndynjyxa/aio-coding-hub/commit/8371047c8a09fbf448f9aa11babc0fafe8b15f20))
* **tauri:** 修复 Cargo.lock 同步及 CLI 代理切换后 MCP 同步问题 ([1854b00](https://github.com/dyndynjyxa/aio-coding-hub/commit/1854b00205b4b3a257b984b5a6c5271067b2545e))
* **tauri:** 修复 Windows 下 WSL 同步触发引用错误 ([696453d](https://github.com/dyndynjyxa/aio-coding-hub/commit/696453d0ab794902c20166d0ee2c5c5356911738))
* **tauri:** 修复可执行文件查找时的权限判断问题 ([e3815ce](https://github.com/dyndynjyxa/aio-coding-hub/commit/e3815cec86d2f3bd8635fd7f95de8e30b5f36fa9))
* **tauri:** 修复注册表键句柄初始化问题 ([6f468c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f468c9a692e1b7aa8bd343a8ce74ce07d7ad813))
* **tests:** update RealtimeTraceCards test to reflect token display format change ([f43633e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f43633e5381f44447bbf8aa64c1065d222687c59))
* **theme:** 修复 dark mode 切换闪烁回退问题 ([ac9500c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac9500c04a108c5bc15921353b691394a689ac54))
* **usage:** align cache creation ttl to 5m only ([8d28bcd](https://github.com/dyndynjyxa/aio-coding-hub/commit/8d28bcd2f5d7f8d6bac1a7f65f974c04c5fce337))
* **utils:** 修正computeOutputTokensPerSecond回退逻辑 ([7c59559](https://github.com/dyndynjyxa/aio-coding-hub/commit/7c59559ad0b525e478577d74518173753530a980))
* **wsl:** atomic write, remove dead fallback, add TOML comment ([7decfb5](https://github.com/dyndynjyxa/aio-coding-hub/commit/7decfb5803378ee3bfb5433bc88adb64562a6857))
* **wsl:** 修复 WSL 脚本错误消息编码和路径写入问题 ([bccd5c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/bccd5c93110641898e04cd1cde031df604af637b))
* **wsl:** 修复WSL适配器IPv4解析和状态脚本处理 ([06ae9b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/06ae9b1f462572550f327c3afe4034179b1da491))
* **wsl:** 修正同步边界与 sessions 状态 ([cc41840](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc41840fb88744c926544436cf885b440f240403))
* 修复clippy redundant_pattern_matching警告 ([b0274a6](https://github.com/dyndynjyxa/aio-coding-hub/commit/b0274a663396a2958e470e2167933470bf34a70a))


### Performance Improvements

* **home:** 优化实时追踪卡片动画与样式过渡效果 ([48c9966](https://github.com/dyndynjyxa/aio-coding-hub/commit/48c99669d5658884bc594dcebdb35219d0c25e23))


### Code Refactoring

* **infra:** 移除 request_attempt_logs 独立表，改用 request_logs.attempts_json 派生 ([3fb2627](https://github.com/dyndynjyxa/aio-coding-hub/commit/3fb2627407e1b9b57758a8a40d3a7d7393278e1b))

## [0.33.11](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.33.10...aio-coding-hub-v0.33.11) (2026-04-02)


### Features

* add Claude OAuth support with multi-account switching ([#149](https://github.com/dyndynjyxa/aio-coding-hub/issues/149)) ([ef787b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef787b1ddf309ffcf9ffcb923d92bf2af3f557b6))

## [0.33.10](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.33.9...aio-coding-hub-v0.33.10) (2026-03-31)


### Features

* **gateway:** 实现熔断器半开状态与相关逻辑 ([9bd8146](https://github.com/dyndynjyxa/aio-coding-hub/commit/9bd81467541ceb40009dce68e052aeee44e82e36))
* **gateway:** 新增计费头修正功能及提供流空闲超时配置能力 ([bbf3d29](https://github.com/dyndynjyxa/aio-coding-hub/commit/bbf3d2910d6e5c7cda76b3a11032eb117b709376))

## [0.33.9](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.33.8...aio-coding-hub-v0.33.9) (2026-03-29)


### Features

* **proxy:** 实现对 Claude 请求的观察与请求日志生命周期管理 ([107d892](https://github.com/dyndynjyxa/aio-coding-hub/commit/107d89202defcd4bbb8727b15ec88bcf6bfe49e0))

## [0.33.8](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.33.7...aio-coding-hub-v0.33.8) (2026-03-29)


### Bug Fixes

* **tauri:** 修复 Windows 下 WSL 同步触发引用错误 ([696453d](https://github.com/dyndynjyxa/aio-coding-hub/commit/696453d0ab794902c20166d0ee2c5c5356911738))

## [0.33.7](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.33.6...aio-coding-hub-v0.33.7) (2026-03-29)


### Features

* add CX2CC tab, config export/import, Gemini config enhance, CLI… ([#138](https://github.com/dyndynjyxa/aio-coding-hub/issues/138)) ([7aba381](https://github.com/dyndynjyxa/aio-coding-hub/commit/7aba381a462812e6c6ea505de3650397d23b48b4))


### Bug Fixes

* **cli_proxy:** 修复数据库初始化错误处理逻辑 ([9230475](https://github.com/dyndynjyxa/aio-coding-hub/commit/9230475e4401e93e3dc34ad285ace4a03186cd16))
* **settings:** 增强设置读取错误处理和写入保护 ([6759e60](https://github.com/dyndynjyxa/aio-coding-hub/commit/6759e60edf1cb93e63f8c05787ed4f0a320a175b))

## [0.33.6](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.33.5...aio-coding-hub-v0.33.6) (2026-03-27)


### Features

* **proxy:** 增加请求中断日志中尝试信息的捕获与展示 ([0ad0955](https://github.com/dyndynjyxa/aio-coding-hub/commit/0ad0955730da9ee46e1904838412fe546794e0f5))
* **settings:** 添加通知声音开关配置项 ([d13ee18](https://github.com/dyndynjyxa/aio-coding-hub/commit/d13ee18228cde971e7490468c6831aa679a636ab))
* **settings:** 添加通知音效开关及完善通用配置界面 ([e8d285a](https://github.com/dyndynjyxa/aio-coding-hub/commit/e8d285a704ba3f1a82fda9b16f9c2dbba393a57d))

## [0.33.5](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.33.4...aio-coding-hub-v0.33.5) (2026-03-26)


### Bug Fixes

* **codex:** 修复切换 Codex Home 后代理状态与实际配置不一致 ([#125](https://github.com/dyndynjyxa/aio-coding-hub/issues/125)) ([e8f80b2](https://github.com/dyndynjyxa/aio-coding-hub/commit/e8f80b2838509b748d38ed6411219218b4077bf5))
* **macOS:** 修复macOS 在 CLI 管理页面读取不到工具 version 并提示 INTERNAL_ERROR ([#126](https://github.com/dyndynjyxa/aio-coding-hub/issues/126)) ([ce35a94](https://github.com/dyndynjyxa/aio-coding-hub/commit/ce35a945dd278b453a1ddebbe8e95fd5f4f12ee6))
* **skills:** unify card layout between general and local skill sections ([7f3b7bb](https://github.com/dyndynjyxa/aio-coding-hub/commit/7f3b7bba9efe0818b11ae6ddbfa0dc1c1576f0fa))
* **skills:** 优化目录复制以正确处理符号链接 ([8b21265](https://github.com/dyndynjyxa/aio-coding-hub/commit/8b21265d116a184f82008b13792d472d68f13356))

## [0.33.4](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.33.3...aio-coding-hub-v0.33.4) (2026-03-25)


### Features

* **bindings:** 添加 HomeUsagePeriod 类型契约断言 ([a540d52](https://github.com/dyndynjyxa/aio-coding-hub/commit/a540d52012652075deee18c58d3e4b5611f4d547))


### Bug Fixes

* **commands:** 修复构建Claude启动命令时路径参数传递 ([9e714f4](https://github.com/dyndynjyxa/aio-coding-hub/commit/9e714f48bdbf7dff2d729131774518845303290a))
* **gateway:** 修正 Heartbeat 事件变量可见性及用法 ([a540d52](https://github.com/dyndynjyxa/aio-coding-hub/commit/a540d52012652075deee18c58d3e4b5611f4d547))

## [0.33.3](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.33.2...aio-coding-hub-v0.33.3) (2026-03-24)


### Features

* **gateway:** 优化协议兼容性处理与请求头管理 ([6be9161](https://github.com/dyndynjyxa/aio-coding-hub/commit/6be91613fbc0bfcd9b87fe419a8f9ad08178a1f7))


### Bug Fixes

* **app:** 修复并增强数据库重置和WSL配置功能 ([caf135a](https://github.com/dyndynjyxa/aio-coding-hub/commit/caf135a98905bcc4594f9f6c3729297e3c6dcf1c))
* **ci:** 优化 pre-push 钩子和测试命令的标准输入配置 ([d14abe2](https://github.com/dyndynjyxa/aio-coding-hub/commit/d14abe2ce2f35e53ea0ab750e8598aafb65f33ed))
* **tauri:** 修复可执行文件查找时的权限判断问题 ([e3815ce](https://github.com/dyndynjyxa/aio-coding-hub/commit/e3815cec86d2f3bd8635fd7f95de8e30b5f36fa9))

## [0.33.2](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.33.1...aio-coding-hub-v0.33.2) (2026-03-24)


### Bug Fixes

* **codex:** resolve oauth login failure and stale provider names ([#116](https://github.com/dyndynjyxa/aio-coding-hub/issues/116)) ([48d647e](https://github.com/dyndynjyxa/aio-coding-hub/commit/48d647ef429232562c9b56e88d8a37f57960604a))
* **release:** keep Cargo.lock synced for release PRs ([c5bd423](https://github.com/dyndynjyxa/aio-coding-hub/commit/c5bd42355641cb8dcef13a1fa04d41795aeb5c82))

## [0.33.1](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.33.0...aio-coding-hub-v0.33.1) (2026-03-24)


### Features

* **codex:** support configurable Windows .codex locations ([#114](https://github.com/dyndynjyxa/aio-coding-hub/issues/114)) ([a8c77cd](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8c77cdebdca52f7bb5ca758837c2b04bac14e3f))
* **skills:** support repo market and local CLI workflows ([3a0a24f](https://github.com/dyndynjyxa/aio-coding-hub/commit/3a0a24f13405d493237402dace175807b42de462))

## [0.33.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.32.8...aio-coding-hub-v0.33.0) (2026-03-22)


### ⚠ BREAKING CHANGES

* **infra:** request_attempt_logs 表不再创建或写入

### Features

* add gemini oauth code assist proxy support ([4a892d0](https://github.com/dyndynjyxa/aio-coding-hub/commit/4a892d035fbcdd5273c909de063eede5d42598fa))
* add TextEvidenceSection component for improved output display in ClaudeModelValidationResultPanel ([47be119](https://github.com/dyndynjyxa/aio-coding-hub/commit/47be119a83c365b3e7b41f22308be7550ecaede5))
* **app:** add LogsPage route and enhance CLI manager settings ([6796715](https://github.com/dyndynjyxa/aio-coding-hub/commit/679671526be935129211978b3598a3962f7e2a8e))
* **app:** 优化页面路由懒加载和加载反馈体验 ([504256e](https://github.com/dyndynjyxa/aio-coding-hub/commit/504256e11ed517797405f8bb4ab3d353607cb1ae))
* **app:** 添加应用心跳机制与监听支持 ([df78171](https://github.com/dyndynjyxa/aio-coding-hub/commit/df781712d16a2f5ccc99e37a687d39cb4e4f56c5))
* **build:** 添加Wayland兼容的AppImage构建支持 ([95c7130](https://github.com/dyndynjyxa/aio-coding-hub/commit/95c71307038962fc0e37fbd7b6df844301d8decb))
* **build:** 添加Windows便携版ZIP构建和WebView2检查 ([fef41e1](https://github.com/dyndynjyxa/aio-coding-hub/commit/fef41e12f2a1301743b40de4c7402f4b02fa80f4))
* **cache-anomaly-monitor:** implement cache anomaly monitoring feature ([59d69d1](https://github.com/dyndynjyxa/aio-coding-hub/commit/59d69d1528d888a84d71038873321f3273d4ded1))
* **charts:** migrate from ECharts to Recharts for improved charting capabilities ([b747b61](https://github.com/dyndynjyxa/aio-coding-hub/commit/b747b61a8dd587043a7e8de01a5f9b3d64ead7ae))
* **claude-validation:** add signature and caching roundtrip probes ([15badee](https://github.com/dyndynjyxa/aio-coding-hub/commit/15badee08b0c14f71695e6e71f0b165e4844371c))
* **ClaudeModelValidationDialog:** 添加综合协议验证功能与UI展示 ([b6ede30](https://github.com/dyndynjyxa/aio-coding-hub/commit/b6ede30e61e19f9af69e8d7153cbc711b3e3a180))
* **ClaudeModelValidation:** enhance cross-provider validation and UI feedback ([bf83c7e](https://github.com/dyndynjyxa/aio-coding-hub/commit/bf83c7e03c7edf78795cd51a943c01a88e0b17d7))
* **ClaudeModelValidation:** enhance output token validation and error handling ([d245288](https://github.com/dyndynjyxa/aio-coding-hub/commit/d245288d7a4937ca7b0213ebd79d9c0d5e3c76b4))
* **ClaudeModelValidation:** implement cross-provider signature validation and enhance request handling ([2e102d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/2e102d4f3fd2745e4480a5884272baeafe66b6d0))
* **claudeModelValidation:** 优化模型验证对话框和多轮验证流程支持 ([3f6c031](https://github.com/dyndynjyxa/aio-coding-hub/commit/3f6c03102db7aaeaa2d8399ebdac1cc4787c2a84))
* **cli_proxy:** 添加 merge-restore 功能以保留用户更改 ([a7d05c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/a7d05c97fe1228d83ccf97be5ed9b6ec26c7bde0))
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
* **cli-manager:** 对齐 CCH 基础配置开关 ([55779ef](https://github.com/dyndynjyxa/aio-coding-hub/commit/55779efce6964a17b423869af77f3ebff567d62c))
* **cli-manager:** 支持自定义GPT-5.4模型上下文窗口和自动压缩限制 ([024dca7](https://github.com/dyndynjyxa/aio-coding-hub/commit/024dca78798022fef014a6a4c5b7a63eb6ecd663))
* **cli-sessions:** P0批次1 - 安全校验/React Query/Router state/虚拟化依赖 ([53c9d43](https://github.com/dyndynjyxa/aio-coding-hub/commit/53c9d4320566b03ff5e655ad2442d46a67f77572))
* **cli-sessions:** P0批次2 - 应用安全校验和状态管理重构 ([2dd098f](https://github.com/dyndynjyxa/aio-coding-hub/commit/2dd098fd39d0d45040b83a0cbd80cb13d4a29352))
* **cli-sessions:** P0批次3 - 三个列表虚拟化 ([d48fccd](https://github.com/dyndynjyxa/aio-coding-hub/commit/d48fccd775086bd11c6cf01147fedf0a3c44e86a))
* **cli-sessions:** P1批次 - 布局一致性和Shell转义安全 ([84723f3](https://github.com/dyndynjyxa/aio-coding-hub/commit/84723f377e5088d2f80574be84e6fa819038bb36))
* **cli:** add CLI proxy startup recovery feature ([eb40a6f](https://github.com/dyndynjyxa/aio-coding-hub/commit/eb40a6f03d9a3b50b501795727a06b7ce9013fc1))
* **CliManager:** add response fixer configuration limits and UI inputs ([0023ad6](https://github.com/dyndynjyxa/aio-coding-hub/commit/0023ad69abf91f48a5144250e20b53ea0b2e24bf))
* **cli:** 添加 Claude 终端启动命令生成及剪贴板管理插件支持 ([ba3d3c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba3d3c20d8d1d40accc0d97780c4dae54d48c71a))
* **codex:** add multi_agent feature toggle ([f56a201](https://github.com/dyndynjyxa/aio-coding-hub/commit/f56a2016bb535812e1a00edb0056e0faa6f44957))
* **components:** enhance HomeRequestLogsPanel and RealtimeTraceCards with new formatting utilities ([e6a3550](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6a3550636e4278100a24d009e49ade1056733bc))
* **config:** 新增 personality 输出风格和 websocket 实验功能支持 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **config:** 新增计划模式推理强度配置项 ([6f5da87](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f5da87696af5c90054d6ca5be0f25e0a91eba40))
* **config:** 添加 GPT-5.4 关联配置及快速模式支持 ([38a11cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/38a11cb4c33300a2a7edb19848d97e25e966688b))
* **ConsolePage:** revamp console log display and functionality ([1d28397](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d28397e88c0b6d43a4d73b348c49c93cb18efde))
* **console:** 改进控制台界面并增加日志过滤和搜索功能 ([93badf8](https://github.com/dyndynjyxa/aio-coding-hub/commit/93badf8766c9a415c821c3f14084183d18d31687))
* **core:** improve workspace imports and startup resilience ([1d7dbf0](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d7dbf0b73cde2b78a70be945c6fd8e248b30c1d))
* **cx2cc:** add Codex-to-Claude-Code translation bridge ([#100](https://github.com/dyndynjyxa/aio-coding-hub/issues/100)) ([44d9c41](https://github.com/dyndynjyxa/aio-coding-hub/commit/44d9c416e434b72699e95984dbc156d413fc78dc))
* **date-range:** refactor custom date range handling in HomeCostPanel and UsagePage ([95b1ad1](https://github.com/dyndynjyxa/aio-coding-hub/commit/95b1ad11b16a0f3e9b187b4883ef85a367d712e0))
* **domain:** 支持 cx2cc 提供者及缓存令牌追踪 ([3662c26](https://github.com/dyndynjyxa/aio-coding-hub/commit/3662c26c2ea72c25ef7e330a52a414a708164cd1))
* enhance provider model configuration with support for model whitelisting and mapping ([4f44510](https://github.com/dyndynjyxa/aio-coding-hub/commit/4f445106fefa10badae230de52c9fee09bd2486f))
* **failover-loop:** refactor context management for improved clarity and usability ([b96df01](https://github.com/dyndynjyxa/aio-coding-hub/commit/b96df013317c77ca610bb640a43b2d60058e6847))
* **failover-loop:** refactor error handling and logging in failover loop ([ef2662c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef2662c335ae4057c591b531b0784c86cd73951e))
* **gateway,providers,oauth:** codex-oauth-with-proxy ([#76](https://github.com/dyndynjyxa/aio-coding-hub/issues/76)) ([f37e59a](https://github.com/dyndynjyxa/aio-coding-hub/commit/f37e59a3dbfdc8a95d18e592f1931c775f19c2ab))
* **gateway:** 对齐 CCH 基础配置整流 ([a1c1b5c](https://github.com/dyndynjyxa/aio-coding-hub/commit/a1c1b5c2b730b89184d59fac207d252e9d415c93))
* **gateway:** 支持会话绑定提供者的查询与恢复 ([cbfb17e](https://github.com/dyndynjyxa/aio-coding-hub/commit/cbfb17e385f6ee4689148c8630f56bed0c281c95))
* **gateway:** 支持强制指定请求的Provider并更新Claude终端启动逻辑 ([904a9aa](https://github.com/dyndynjyxa/aio-coding-hub/commit/904a9aa6805681fb5642e350105803ce01ae998b))
* **gateway:** 添加决策链以丰富失败重试事件细节 ([262e98c](https://github.com/dyndynjyxa/aio-coding-hub/commit/262e98cb0ac974c5452e037ac51d5730625d1ef2))
* **home:** add provider limit usage overview tab ([c224748](https://github.com/dyndynjyxa/aio-coding-hub/commit/c224748c590d1b242df045a7e81667a623d0ec0b))
* **home:** add provider limit usage overview tab ([6473253](https://github.com/dyndynjyxa/aio-coding-hub/commit/64732536a25e140193242e9829aacb2cce15f05d))
* **home:** add window start timestamps for provider limit usage ([69a91a2](https://github.com/dyndynjyxa/aio-coding-hub/commit/69a91a21567b6a2c9af2ac879936dd75a6004e37))
* **home:** enhance HomeCostPanel with data-testid attributes and update tests ([b1d23d2](https://github.com/dyndynjyxa/aio-coding-hub/commit/b1d23d2575afb25d3314911b0c6488a872f56b46))
* **home:** enhance status handling with failover support ([5aab8a9](https://github.com/dyndynjyxa/aio-coding-hub/commit/5aab8a92c4440062d18a3ecf495375d11a461660))
* **home:** enhance UI components and improve layout consistency ([70d9655](https://github.com/dyndynjyxa/aio-coding-hub/commit/70d9655cf1a740676e81bcb0a1e204b81abdd1c9))
* **home:** implement CLI proxy environment conflict checks ([aceff42](https://github.com/dyndynjyxa/aio-coding-hub/commit/aceff428537261c5132b5cb4ebdcb22d16efec82))
* **home:** implement CLI proxy environment conflict checks ([4c014ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/4c014ca5ab04dab7aa5ca38799d69d1a8eb3cc3e))
* **home:** implement window foreground detection for usage heatmap refresh ([4e66f35](https://github.com/dyndynjyxa/aio-coding-hub/commit/4e66f359f198ddddc52b6cd4c0ab8cdb59630a27))
* **home:** 添加供应商限额面板刷新按钮 ([18f4f91](https://github.com/dyndynjyxa/aio-coding-hub/commit/18f4f91d375f860312a126354a65efad6244e7ae))
* **home:** 添加工作区模式切换按钮并持久化请求日志简洁模式 ([d73bedd](https://github.com/dyndynjyxa/aio-coding-hub/commit/d73bedd3bc27d82375969d09fa94336a268bd075))
* init ([7e30c40](https://github.com/dyndynjyxa/aio-coding-hub/commit/7e30c40727d50980bcd43c2f275419a74fa3b148))
* integrate PageHeader component across multiple pages for consistent UI ([330da27](https://github.com/dyndynjyxa/aio-coding-hub/commit/330da276f9ef8e91744a9534d59590a3a6fec5ff))
* mac最小化优化 ([1f11a5d](https://github.com/dyndynjyxa/aio-coding-hub/commit/1f11a5df9623f8cab90bb40f5c499c5d3569f01d))
* **mcp:** implement batch import for local skills and add global error reporting ([51e053c](https://github.com/dyndynjyxa/aio-coding-hub/commit/51e053c81f1db00696f0cb968860033deee12cce))
* **mcp:** 优化服务器键生成与导入逻辑，支持保留大小写 ([f61a5c6](https://github.com/dyndynjyxa/aio-coding-hub/commit/f61a5c6be09795efaf9c4aafe7fee5f86329c0ea))
* **mcp:** 添加 env 和 header 键值对格式校验 ([7d698ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/7d698ca04f1cc9f9c0d420f6d7fec2a53f2d643a))
* **model-prices:** add model price alias rules ([60cbcc1](https://github.com/dyndynjyxa/aio-coding-hub/commit/60cbcc1c65ff025e79313facaf27e625a3de9997))
* **Oauth:** adding gemini Oauth support ([fd1eb79](https://github.com/dyndynjyxa/aio-coding-hub/commit/fd1eb7933410510e9ee46e469234d91658a49fcf))
* **provider-chain:** 优化供应商链路视图及日志详情显示 ([72d4bc4](https://github.com/dyndynjyxa/aio-coding-hub/commit/72d4bc49dfd1380ce503ec23a98eb81b55a1d488))
* **provider-editor:** enhance ProviderEditorDialog with limit configuration cards ([cc14a00](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc14a009303129793ceec840933155dcfad775d8))
* **providers:** collapse model mapping editors ([4672961](https://github.com/dyndynjyxa/aio-coding-hub/commit/4672961c8facbd27d715a762864c2bf4f32ac932))
* **providers:** 在供应商列表页增加名称搜索功能 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **providers:** 增加供应商备注字段并支持读取API Key ([fec7057](https://github.com/dyndynjyxa/aio-coding-hub/commit/fec7057d137a7c0b38b46b5591a9e19e624b679a))
* **providers:** 支持复制供应商配置并预填创建表单 ([c8fb707](https://github.com/dyndynjyxa/aio-coding-hub/commit/c8fb7078131ad67e4d2895f81467a9358b897d2c))
* **providers:** 添加供应商标签功能，支持分类筛选 ([7671732](https://github.com/dyndynjyxa/aio-coding-hub/commit/76717329a119f9a65dc2f3a9dffe0a887063b0bd))
* **proxy:** 支持 Codex 会话 ID 补全功能 ([25f8321](https://github.com/dyndynjyxa/aio-coding-hub/commit/25f8321536ccdfe9a68de1866bb9762794ac97b7))
* **request-end:** enhance request logging and event emission ([809a684](https://github.com/dyndynjyxa/aio-coding-hub/commit/809a6843a2c2eb952bbb00f7c56694c908d99bd1))
* **request-end:** refactor request logging and event emission ([e56909e](https://github.com/dyndynjyxa/aio-coding-hub/commit/e56909e374c26608407a1d63d32c9d55f914b355))
* **responsive:** enhance layout and styling for improved mobile experience ([e22483a](https://github.com/dyndynjyxa/aio-coding-hub/commit/e22483a8da8c35822c6e5b0532aa1461cb68cf95))
* **route:** 支持展示同一 provider 的连续尝试次数及跳过状态 ([6659157](https://github.com/dyndynjyxa/aio-coding-hub/commit/6659157a12c3809b1e4d8de7bf95b95afe6ed383))
* **settings:** update Claude settings and remove deprecated options ([49e055a](https://github.com/dyndynjyxa/aio-coding-hub/commit/49e055aeb176819cb56a925aecbd51f362008762))
* **settings:** 新增任务结束提醒开关和相关功能 ([2fb4022](https://github.com/dyndynjyxa/aio-coding-hub/commit/2fb4022ff2f1078238ced25188960f19da188260))
* **settings:** 新增关闭 Claude Git 参与者功能 ([cf23f0d](https://github.com/dyndynjyxa/aio-coding-hub/commit/cf23f0dec000cff63f222b0198360485deb60804))
* **settings:** 新增缓存异常监测功能开关 ([ea681c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ea681c155346da3057f25e25867ab75a818f4157))
* **settings:** 新增静默启动配置支持 ([fcc436b](https://github.com/dyndynjyxa/aio-coding-hub/commit/fcc436b5dc5baf3b57e7160bd7e3562275535072))
* **SkillsMarketPage:** enhance UI with tab selection and external links ([2849017](https://github.com/dyndynjyxa/aio-coding-hub/commit/2849017554128279822fef9b667d8ec166a08432))
* **skills:** 支持从本地源恢复缺失的ssot技能目录 ([4414f7d](https://github.com/dyndynjyxa/aio-coding-hub/commit/4414f7d38a44e4aff7de0730fe591b93146d7468))
* **skills:** 添加 Skill 返回本机已安装功能 ([84cfd0b](https://github.com/dyndynjyxa/aio-coding-hub/commit/84cfd0be09c5c38dbd9613533062a3daef6c57df))
* **sort-modes:** add toggle functionality for enabling/disabling providers in sort modes and update related services ([e6cd992](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6cd99241f5c86da3f62afebefeffdf33e62b8f0))
* **taskCompleteNotifyEvents:** 改进任务完成通知的请求追踪与静默期控制 ([e3f03d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/e3f03d4bf2276db371e170ee4739cd379b6d3fe5))
* **tauri:** add WSL support and listen modes ([a357007](https://github.com/dyndynjyxa/aio-coding-hub/commit/a35700753e9633493f6e939d1700ce979d635c93))
* **tauri:** update error handling and add thiserror dependency ([10d918a](https://github.com/dyndynjyxa/aio-coding-hub/commit/10d918a0a0a746b4d25c1c236f88c0110ec664c3))
* **tests:** add comprehensive unit tests for various components ([98fb022](https://github.com/dyndynjyxa/aio-coding-hub/commit/98fb022d1487e593e1fe50a1e9d02592671a4944))
* **theme:** add dark mode and native window theme sync ([588a373](https://github.com/dyndynjyxa/aio-coding-hub/commit/588a37311c456bd2182ac1676c8e9b309157353f))
* **ui:** align CLI manager with network and WSL settings ([ae5b5fc](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae5b5fc99330b55872e1c30da6e653d7433b7d48))
* **ui:** 优化首页概览、CLI 代理状态与供应商页交互 ([#106](https://github.com/dyndynjyxa/aio-coding-hub/issues/106)) ([358e73f](https://github.com/dyndynjyxa/aio-coding-hub/commit/358e73f598fd48c7bbf131a3117e4801cec03ced))
* **ui:** 优化首页概览、供应商管理与设置页主题入口 ([#102](https://github.com/dyndynjyxa/aio-coding-hub/issues/102)) ([0adec0a](https://github.com/dyndynjyxa/aio-coding-hub/commit/0adec0af676842e9c5aee3e08e3a46d890d31c11))
* **ui:** 本次主要优化了首页概览、设置页和代理记录页，同时顺手处理了首页包体和 Tauri 构建告警。 ([#111](https://github.com/dyndynjyxa/aio-coding-hub/issues/111)) ([4e6dbad](https://github.com/dyndynjyxa/aio-coding-hub/commit/4e6dbad14cab8334d47a3a30ebfcf70485ad2e0d))
* **ui:** 统一 Loading/Empty/Error 状态反馈组件 ([b27f64a](https://github.com/dyndynjyxa/aio-coding-hub/commit/b27f64a79a1ad655a37992527794759f9f8c5370))
* **updater:** display release notes in update dialog ([b891ed7](https://github.com/dyndynjyxa/aio-coding-hub/commit/b891ed75a1b67a9df281917ffa94e684fb372664))
* **usage:** add cache rate trend functionality to UsagePage ([5535e7d](https://github.com/dyndynjyxa/aio-coding-hub/commit/5535e7d3480237284037ef841d2878bfa6a180f5))
* **usage:** add cost tracking to usage statistics ([765ea8a](https://github.com/dyndynjyxa/aio-coding-hub/commit/765ea8ae06efe2cc39c6eac7e95dccfbba4de541))
* **usage:** add summary stat cards with filter layout optimization ([f6c9206](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6c9206ba6e0b424748ba05f0ac749884d5bf0d3))
* **usage:** 添加供应商过滤支持以改进使用统计查询 ([e2178d9](https://github.com/dyndynjyxa/aio-coding-hub/commit/e2178d93ae413307cd4bb5f830c30550a6e0af1d))
* **workspaces:** add WorkspacesPage route and enhance CLI manager settings ([2784072](https://github.com/dyndynjyxa/aio-coding-hub/commit/2784072e26748aa21f679c245f74cf3063f4177c))
* **workspaces:** enhance MCP and Prompts management with workspace support ([791ee6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/791ee6c88b06c16c596b82dcf7956ea5b98dc18d))
* **workspaces:** improve workspace switching and management functionality ([496c9c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/496c9c35daa23be4d2c44f0653bb11bcb5206e1f))
* **wsl:** 完善 WSL 会话浏览、MCP/提示词同步及删除功能，补充前端测试覆盖率 ([b1308a1](https://github.com/dyndynjyxa/aio-coding-hub/commit/b1308a1e7f3eaf7e871862f3119d7678be72c6aa))
* **wsl:** 支持WSL宿主机地址模式自动检测与自定义 ([f809e86](https://github.com/dyndynjyxa/aio-coding-hub/commit/f809e86ab3690214ba2407c14e4dee1febd9ab14))
* **wsl:** 支持浏览 WSL 环境中的 CLI 会话记录，增加wsl提示词和mcp同步，并有可见提示 ([6147df2](https://github.com/dyndynjyxa/aio-coding-hub/commit/6147df28f904798b7d8e02c5e5946ec3453187c6))
* **wsl:** 支持浏览 WSL 环境中的 CLI 会话记录，增加wsl提示词和mcp同步，并有可见提示 ([41d7d26](https://github.com/dyndynjyxa/aio-coding-hub/commit/41d7d2661a03a3dea90bcb63a06f384fde1ac31a))
* **wsl:** 添加 WSL 配置生命周期管理 — 退出恢复与崩溃自愈 ([bcbf54d](https://github.com/dyndynjyxa/aio-coding-hub/commit/bcbf54df5bf1e6b851f223ffad2a0ee135647c13))
* **wsl:** 添加启动时自动检测和配置 WSL 环境功能 ([707bd55](https://github.com/dyndynjyxa/aio-coding-hub/commit/707bd5518fd9881715ec5f3df0ef9250970570bb))


### Bug Fixes

* **ci:** add explicit permissions to release-please job ([8c77c04](https://github.com/dyndynjyxa/aio-coding-hub/commit/8c77c041b0d6f5081abf6ea9a39e1031dc56ad75))
* **ci:** ensure consistent use of RELEASE_PLEASE_TOKEN in release workflow for GitHub actions ([f8a439d](https://github.com/dyndynjyxa/aio-coding-hub/commit/f8a439d213346a907dfc7355b6b6caf4b4194799))
* **ci:** quote FALLBACK_NOTES value in release workflow ([536896c](https://github.com/dyndynjyxa/aio-coding-hub/commit/536896ca4072f48002e8c83b91d78164ed40384f))
* **ci:** revert release-please job to original config ([0d1afad](https://github.com/dyndynjyxa/aio-coding-hub/commit/0d1afade58707c53554f7680dd7455d6ef547187))
* **ci:** satisfy rust clippy checks ([38d0bc0](https://github.com/dyndynjyxa/aio-coding-hub/commit/38d0bc0081cc90b24bd83e4ced30566d861ea69b))
* **ci:** update release workflow to consistently use RELEASE_PLEASE_TOKEN for GitHub actions ([2c45c46](https://github.com/dyndynjyxa/aio-coding-hub/commit/2c45c461ddf157a4b781c146a0ebed5cd9dc1a44))
* **ci:** update release workflow to use RELEASE_PLEASE_TOKEN for GitHub actions ([48ec3ce](https://github.com/dyndynjyxa/aio-coding-hub/commit/48ec3ce92e28702ee9920eaa42d2f7374b07be31))
* **ci:** use PAT for release-please to fix permission issue ([a2919f2](https://github.com/dyndynjyxa/aio-coding-hub/commit/a2919f2446d8fcb1c4c7ac8dc6a1b4f63f705157))
* **ci:** 修复CI流程中正则表达式重复问题 ([609f2ea](https://github.com/dyndynjyxa/aio-coding-hub/commit/609f2ea657034a58b6f175b995e05431829f6dd9))
* **ci:** 修复release.yml中AppImage路径解析问题 ([1876bd4](https://github.com/dyndynjyxa/aio-coding-hub/commit/1876bd428f888e9c2eb286a20b9aeeb00dda5c5a))
* **ci:** 修复release工作流中appimagetool下载地址 ([67d8071](https://github.com/dyndynjyxa/aio-coding-hub/commit/67d8071fecf6dfc227f8e41d03f067ec481bfd4f))
* **ci:** 修复正则表达式转义字符问题 ([ba805c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba805c3b2f147f153cbe39042b078547eb36da5a))
* **ci:** 修正发布流程中更新器 JSON 文件参数名称 ([9a4c0f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/9a4c0f063bd645e3286e67f1607334a84a5cb6fd))
* **ClaudeModelValidationDialog:** adjust dialog max-width for responsive design ([11bdfd0](https://github.com/dyndynjyxa/aio-coding-hub/commit/11bdfd0df1b51923a561211f6cc6d5383eedf76c))
* **cli_manager:** 修复 run_in_login_shell 的平台兼容性处理 ([9f81433](https://github.com/dyndynjyxa/aio-coding-hub/commit/9f814337d08f36c08e37486f95b9c9da62880fc1))
* **cli-manager:** 修正模型参数解析逻辑以避免默认值覆盖 ([ae2b7fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae2b7feecfd6e06462b8b0990204fc55d6ad3ccf))
* **cli-proxy:** 修复 Codex CLI 在 Windows 下的沙箱配置与认证模式 ([b79d072](https://github.com/dyndynjyxa/aio-coding-hub/commit/b79d072f39405c15125a8a6391bff122c9a5f315))
* **cli-sessions:** 修复SessionsPage Card容器flex布局 ([2d2a048](https://github.com/dyndynjyxa/aio-coding-hub/commit/2d2a0481f9811160223f9753d6edec5a28dcfc6c))
* **codex-tab:** sandbox_mode danger-full-access 选择不生效及高级配置不回显 ([3943401](https://github.com/dyndynjyxa/aio-coding-hub/commit/3943401524c8ef6a179e65e8ee62fd2ac4f784cb))
* **components:** 修复ClaudeModelValidationDialog点击事件阻塞问题 ([9bd5f53](https://github.com/dyndynjyxa/aio-coding-hub/commit/9bd5f53f38d01fc312b979e3620b5e0badaba585))
* **cost:** 修正成本乘数比较逻辑为包含零值 ([db69776](https://github.com/dyndynjyxa/aio-coding-hub/commit/db6977635f96fc55fa1b4d84d1d71e67dd8dd0b1))
* **db:** 修复providers表note字段迁移及数据处理问题 ([fa8ecda](https://github.com/dyndynjyxa/aio-coding-hub/commit/fa8ecda9bad231ed424c812c1c7cd3d0850b8565))
* **deps:** 升级 react-router-dom 7.11→7.13 修复 XSS 漏洞 ([c4db630](https://github.com/dyndynjyxa/aio-coding-hub/commit/c4db630b4478cab2f1180f03b324cfc0af9d42e3))
* **domain:** 修复 SSE 流错误处理逻辑 ([4446939](https://github.com/dyndynjyxa/aio-coding-hub/commit/4446939c457735727f96dce0b09c91ae6b616c1d))
* **domain:** 修正 claude_terminal_launch_context 函数中 provider_id 验证和查询逻辑 ([316d375](https://github.com/dyndynjyxa/aio-coding-hub/commit/316d3754fc628d0a1e435aabf469ca992cb63efa))
* **gateway:** allow failover success handler arg count ([0c559cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/0c559cba801d59332a94eb650571487b6b18a53b))
* **gateway:** finalize failed requests and align error codes ([871de9c](https://github.com/dyndynjyxa/aio-coding-hub/commit/871de9c9c9e06ed6cb03026ceb21dfc4b0ba18d3))
* **gateway:** reject forwarding when CLI proxy disabled ([c9edd10](https://github.com/dyndynjyxa/aio-coding-hub/commit/c9edd10cd2f41ef86c8c4c8a3ca2262c8bcb09ef))
* **gateway:** 修正 provider 重新排序时清除会话绑定逻辑 ([f03c02e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f03c02e5332fe84496a029b2e5d130b558cfad31))
* **gateway:** 调整上游连接超时与错误切换逻辑 ([a6870ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6870ae14b6b5bb8edce814ca37da2000e84f6b4))
* **heartbeat_watchdog:** 优化窗口重载的错误处理逻辑 ([6188e6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6188e6c29135c0126490882ceb54302d112ec79c))
* **home:** update HomeCostPanel with accessibility labels and improve test assertions ([0bda6c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/0bda6c1ce8aec6920b30810882f3a804c62ab732))
* **mcp:** 修复导入服务器时基于 server_key 的重复处理 ([f6e6458](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6e64582e1e4c478ec014e61258611d971f8b3b4))
* **prompts:** 修改新增和编辑时 Prompt 保存按钮禁用条件 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **providers:** 敏感配置信息变更时清理运行时 session 绑定 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **proxy:** 允许 resolve_session_bound_provider_id 函数有多个参数 ([705457e](https://github.com/dyndynjyxa/aio-coding-hub/commit/705457eac5092ccd98a0f3523ea1986c67dd9d9c))
* **router:** 修复侧边栏快速点击导航卡死问题 ([8b1f862](https://github.com/dyndynjyxa/aio-coding-hub/commit/8b1f862542b8bd94770ab5f937772a53a4bd1d22))
* **scripts:** 修复生成绑定文件格式化问题 ([9ff697b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9ff697b6b8c93238f48518f27f274df33ba55a9a))
* **sessions:** 剥离替换字符并调整按钮样式 ([3586a16](https://github.com/dyndynjyxa/aio-coding-hub/commit/3586a165ceb8ba3afc42a34fdf4228c71b18cce5))
* **skills:** 优化符号链接目录的移除逻辑 ([2ef9928](https://github.com/dyndynjyxa/aio-coding-hub/commit/2ef9928eb3d1d4fa23cb22e2035384e7be2a01f6))
* **skills:** 移除批量导入功能并改为刷新本机列表 ([45b9618](https://github.com/dyndynjyxa/aio-coding-hub/commit/45b961847b458f72f2bc062bdcdfef93c8722da8))
* suppress dead_code warnings on non-Windows CI and fix clippy lint ([8371047](https://github.com/dyndynjyxa/aio-coding-hub/commit/8371047c8a09fbf448f9aa11babc0fafe8b15f20))
* **tauri:** 修复 Cargo.lock 同步及 CLI 代理切换后 MCP 同步问题 ([1854b00](https://github.com/dyndynjyxa/aio-coding-hub/commit/1854b00205b4b3a257b984b5a6c5271067b2545e))
* **tauri:** 修复注册表键句柄初始化问题 ([6f468c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f468c9a692e1b7aa8bd343a8ce74ce07d7ad813))
* **tests:** update RealtimeTraceCards test to reflect token display format change ([f43633e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f43633e5381f44447bbf8aa64c1065d222687c59))
* **theme:** 修复 dark mode 切换闪烁回退问题 ([ac9500c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac9500c04a108c5bc15921353b691394a689ac54))
* **usage:** align cache creation ttl to 5m only ([8d28bcd](https://github.com/dyndynjyxa/aio-coding-hub/commit/8d28bcd2f5d7f8d6bac1a7f65f974c04c5fce337))
* **utils:** 修正computeOutputTokensPerSecond回退逻辑 ([7c59559](https://github.com/dyndynjyxa/aio-coding-hub/commit/7c59559ad0b525e478577d74518173753530a980))
* **wsl:** atomic write, remove dead fallback, add TOML comment ([7decfb5](https://github.com/dyndynjyxa/aio-coding-hub/commit/7decfb5803378ee3bfb5433bc88adb64562a6857))
* **wsl:** 修复 WSL 脚本错误消息编码和路径写入问题 ([bccd5c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/bccd5c93110641898e04cd1cde031df604af637b))
* **wsl:** 修复WSL适配器IPv4解析和状态脚本处理 ([06ae9b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/06ae9b1f462572550f327c3afe4034179b1da491))
* **wsl:** 修正同步边界与 sessions 状态 ([cc41840](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc41840fb88744c926544436cf885b440f240403))
* 修复clippy redundant_pattern_matching警告 ([b0274a6](https://github.com/dyndynjyxa/aio-coding-hub/commit/b0274a663396a2958e470e2167933470bf34a70a))


### Performance Improvements

* **home:** 优化实时追踪卡片动画与样式过渡效果 ([48c9966](https://github.com/dyndynjyxa/aio-coding-hub/commit/48c99669d5658884bc594dcebdb35219d0c25e23))


### Code Refactoring

* **infra:** 移除 request_attempt_logs 独立表，改用 request_logs.attempts_json 派生 ([3fb2627](https://github.com/dyndynjyxa/aio-coding-hub/commit/3fb2627407e1b9b57758a8a40d3a7d7393278e1b))

## [0.32.8](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.32.7...aio-coding-hub-v0.32.8) (2026-03-22)


### Features

* **ui:** 本次主要优化了首页概览、设置页和代理记录页，同时顺手处理了首页包体和 Tauri 构建告警。 ([#111](https://github.com/dyndynjyxa/aio-coding-hub/issues/111)) ([4e6dbad](https://github.com/dyndynjyxa/aio-coding-hub/commit/4e6dbad14cab8334d47a3a30ebfcf70485ad2e0d))

## [0.32.7](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.32.6...aio-coding-hub-v0.32.7) (2026-03-21)


### Features

* **home:** 添加供应商限额面板刷新按钮 ([18f4f91](https://github.com/dyndynjyxa/aio-coding-hub/commit/18f4f91d375f860312a126354a65efad6244e7ae))
* **proxy:** 支持 Codex 会话 ID 补全功能 ([25f8321](https://github.com/dyndynjyxa/aio-coding-hub/commit/25f8321536ccdfe9a68de1866bb9762794ac97b7))
* **ui:** 优化首页概览、CLI 代理状态与供应商页交互 ([#106](https://github.com/dyndynjyxa/aio-coding-hub/issues/106)) ([358e73f](https://github.com/dyndynjyxa/aio-coding-hub/commit/358e73f598fd48c7bbf131a3117e4801cec03ced))

## [0.32.6](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.32.5...aio-coding-hub-v0.32.6) (2026-03-18)


### Features

* **provider-chain:** 优化供应商链路视图及日志详情显示 ([72d4bc4](https://github.com/dyndynjyxa/aio-coding-hub/commit/72d4bc49dfd1380ce503ec23a98eb81b55a1d488))

## [0.32.5](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.32.4...aio-coding-hub-v0.32.5) (2026-03-18)


### Features

* **cx2cc:** add Codex-to-Claude-Code translation bridge ([#100](https://github.com/dyndynjyxa/aio-coding-hub/issues/100)) ([44d9c41](https://github.com/dyndynjyxa/aio-coding-hub/commit/44d9c416e434b72699e95984dbc156d413fc78dc))
* **domain:** 支持 cx2cc 提供者及缓存令牌追踪 ([3662c26](https://github.com/dyndynjyxa/aio-coding-hub/commit/3662c26c2ea72c25ef7e330a52a414a708164cd1))
* **home:** 添加工作区模式切换按钮并持久化请求日志简洁模式 ([d73bedd](https://github.com/dyndynjyxa/aio-coding-hub/commit/d73bedd3bc27d82375969d09fa94336a268bd075))
* **ui:** 优化首页概览、供应商管理与设置页主题入口 ([#102](https://github.com/dyndynjyxa/aio-coding-hub/issues/102)) ([0adec0a](https://github.com/dyndynjyxa/aio-coding-hub/commit/0adec0af676842e9c5aee3e08e3a46d890d31c11))


### Bug Fixes

* **domain:** 修正 claude_terminal_launch_context 函数中 provider_id 验证和查询逻辑 ([316d375](https://github.com/dyndynjyxa/aio-coding-hub/commit/316d3754fc628d0a1e435aabf469ca992cb63efa))

## [0.32.4](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.32.3...aio-coding-hub-v0.32.4) (2026-03-16)


### Bug Fixes

* **ci:** 修复release.yml中AppImage路径解析问题 ([1876bd4](https://github.com/dyndynjyxa/aio-coding-hub/commit/1876bd428f888e9c2eb286a20b9aeeb00dda5c5a))

## [0.32.3](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.32.2...aio-coding-hub-v0.32.3) (2026-03-16)


### Bug Fixes

* **ci:** 修复release工作流中appimagetool下载地址 ([67d8071](https://github.com/dyndynjyxa/aio-coding-hub/commit/67d8071fecf6dfc227f8e41d03f067ec481bfd4f))

## [0.32.2](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.32.1...aio-coding-hub-v0.32.2) (2026-03-16)


### Features

* **build:** 添加Wayland兼容的AppImage构建支持 ([95c7130](https://github.com/dyndynjyxa/aio-coding-hub/commit/95c71307038962fc0e37fbd7b6df844301d8decb))
* **cli_proxy:** 添加 merge-restore 功能以保留用户更改 ([a7d05c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/a7d05c97fe1228d83ccf97be5ed9b6ec26c7bde0))
* **settings:** 新增关闭 Claude Git 参与者功能 ([cf23f0d](https://github.com/dyndynjyxa/aio-coding-hub/commit/cf23f0dec000cff63f222b0198360485deb60804))
* **wsl:** 添加 WSL 配置生命周期管理 — 退出恢复与崩溃自愈 ([bcbf54d](https://github.com/dyndynjyxa/aio-coding-hub/commit/bcbf54df5bf1e6b851f223ffad2a0ee135647c13))


### Bug Fixes

* suppress dead_code warnings on non-Windows CI and fix clippy lint ([8371047](https://github.com/dyndynjyxa/aio-coding-hub/commit/8371047c8a09fbf448f9aa11babc0fafe8b15f20))
* **wsl:** atomic write, remove dead fallback, add TOML comment ([7decfb5](https://github.com/dyndynjyxa/aio-coding-hub/commit/7decfb5803378ee3bfb5433bc88adb64562a6857))

## [0.32.1](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.32.0...aio-coding-hub-v0.32.1) (2026-03-15)


### Features

* **config:** 新增 personality 输出风格和 websocket 实验功能支持 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **providers:** 在供应商列表页增加名称搜索功能 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))


### Bug Fixes

* **prompts:** 修改新增和编辑时 Prompt 保存按钮禁用条件 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **providers:** 敏感配置信息变更时清理运行时 session 绑定 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))

## [0.32.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.31.2...aio-coding-hub-v0.32.0) (2026-03-14)


### ⚠ BREAKING CHANGES

* **infra:** request_attempt_logs 表不再创建或写入

### Features

* add gemini oauth code assist proxy support ([4a892d0](https://github.com/dyndynjyxa/aio-coding-hub/commit/4a892d035fbcdd5273c909de063eede5d42598fa))
* add TextEvidenceSection component for improved output display in ClaudeModelValidationResultPanel ([47be119](https://github.com/dyndynjyxa/aio-coding-hub/commit/47be119a83c365b3e7b41f22308be7550ecaede5))
* **app:** add LogsPage route and enhance CLI manager settings ([6796715](https://github.com/dyndynjyxa/aio-coding-hub/commit/679671526be935129211978b3598a3962f7e2a8e))
* **app:** 优化页面路由懒加载和加载反馈体验 ([504256e](https://github.com/dyndynjyxa/aio-coding-hub/commit/504256e11ed517797405f8bb4ab3d353607cb1ae))
* **app:** 添加应用心跳机制与监听支持 ([df78171](https://github.com/dyndynjyxa/aio-coding-hub/commit/df781712d16a2f5ccc99e37a687d39cb4e4f56c5))
* **build:** 添加Windows便携版ZIP构建和WebView2检查 ([fef41e1](https://github.com/dyndynjyxa/aio-coding-hub/commit/fef41e12f2a1301743b40de4c7402f4b02fa80f4))
* **cache-anomaly-monitor:** implement cache anomaly monitoring feature ([59d69d1](https://github.com/dyndynjyxa/aio-coding-hub/commit/59d69d1528d888a84d71038873321f3273d4ded1))
* **charts:** migrate from ECharts to Recharts for improved charting capabilities ([b747b61](https://github.com/dyndynjyxa/aio-coding-hub/commit/b747b61a8dd587043a7e8de01a5f9b3d64ead7ae))
* **claude-validation:** add signature and caching roundtrip probes ([15badee](https://github.com/dyndynjyxa/aio-coding-hub/commit/15badee08b0c14f71695e6e71f0b165e4844371c))
* **ClaudeModelValidationDialog:** 添加综合协议验证功能与UI展示 ([b6ede30](https://github.com/dyndynjyxa/aio-coding-hub/commit/b6ede30e61e19f9af69e8d7153cbc711b3e3a180))
* **ClaudeModelValidation:** enhance cross-provider validation and UI feedback ([bf83c7e](https://github.com/dyndynjyxa/aio-coding-hub/commit/bf83c7e03c7edf78795cd51a943c01a88e0b17d7))
* **ClaudeModelValidation:** enhance output token validation and error handling ([d245288](https://github.com/dyndynjyxa/aio-coding-hub/commit/d245288d7a4937ca7b0213ebd79d9c0d5e3c76b4))
* **ClaudeModelValidation:** implement cross-provider signature validation and enhance request handling ([2e102d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/2e102d4f3fd2745e4480a5884272baeafe66b6d0))
* **claudeModelValidation:** 优化模型验证对话框和多轮验证流程支持 ([3f6c031](https://github.com/dyndynjyxa/aio-coding-hub/commit/3f6c03102db7aaeaa2d8399ebdac1cc4787c2a84))
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
* **cli-manager:** 对齐 CCH 基础配置开关 ([55779ef](https://github.com/dyndynjyxa/aio-coding-hub/commit/55779efce6964a17b423869af77f3ebff567d62c))
* **cli-manager:** 支持自定义GPT-5.4模型上下文窗口和自动压缩限制 ([024dca7](https://github.com/dyndynjyxa/aio-coding-hub/commit/024dca78798022fef014a6a4c5b7a63eb6ecd663))
* **cli-sessions:** P0批次1 - 安全校验/React Query/Router state/虚拟化依赖 ([53c9d43](https://github.com/dyndynjyxa/aio-coding-hub/commit/53c9d4320566b03ff5e655ad2442d46a67f77572))
* **cli-sessions:** P0批次2 - 应用安全校验和状态管理重构 ([2dd098f](https://github.com/dyndynjyxa/aio-coding-hub/commit/2dd098fd39d0d45040b83a0cbd80cb13d4a29352))
* **cli-sessions:** P0批次3 - 三个列表虚拟化 ([d48fccd](https://github.com/dyndynjyxa/aio-coding-hub/commit/d48fccd775086bd11c6cf01147fedf0a3c44e86a))
* **cli-sessions:** P1批次 - 布局一致性和Shell转义安全 ([84723f3](https://github.com/dyndynjyxa/aio-coding-hub/commit/84723f377e5088d2f80574be84e6fa819038bb36))
* **cli:** add CLI proxy startup recovery feature ([eb40a6f](https://github.com/dyndynjyxa/aio-coding-hub/commit/eb40a6f03d9a3b50b501795727a06b7ce9013fc1))
* **CliManager:** add response fixer configuration limits and UI inputs ([0023ad6](https://github.com/dyndynjyxa/aio-coding-hub/commit/0023ad69abf91f48a5144250e20b53ea0b2e24bf))
* **cli:** 添加 Claude 终端启动命令生成及剪贴板管理插件支持 ([ba3d3c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba3d3c20d8d1d40accc0d97780c4dae54d48c71a))
* **codex:** add multi_agent feature toggle ([f56a201](https://github.com/dyndynjyxa/aio-coding-hub/commit/f56a2016bb535812e1a00edb0056e0faa6f44957))
* **components:** enhance HomeRequestLogsPanel and RealtimeTraceCards with new formatting utilities ([e6a3550](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6a3550636e4278100a24d009e49ade1056733bc))
* **config:** 新增计划模式推理强度配置项 ([6f5da87](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f5da87696af5c90054d6ca5be0f25e0a91eba40))
* **config:** 添加 GPT-5.4 关联配置及快速模式支持 ([38a11cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/38a11cb4c33300a2a7edb19848d97e25e966688b))
* **ConsolePage:** revamp console log display and functionality ([1d28397](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d28397e88c0b6d43a4d73b348c49c93cb18efde))
* **console:** 改进控制台界面并增加日志过滤和搜索功能 ([93badf8](https://github.com/dyndynjyxa/aio-coding-hub/commit/93badf8766c9a415c821c3f14084183d18d31687))
* **core:** improve workspace imports and startup resilience ([1d7dbf0](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d7dbf0b73cde2b78a70be945c6fd8e248b30c1d))
* **date-range:** refactor custom date range handling in HomeCostPanel and UsagePage ([95b1ad1](https://github.com/dyndynjyxa/aio-coding-hub/commit/95b1ad11b16a0f3e9b187b4883ef85a367d712e0))
* enhance provider model configuration with support for model whitelisting and mapping ([4f44510](https://github.com/dyndynjyxa/aio-coding-hub/commit/4f445106fefa10badae230de52c9fee09bd2486f))
* **failover-loop:** refactor context management for improved clarity and usability ([b96df01](https://github.com/dyndynjyxa/aio-coding-hub/commit/b96df013317c77ca610bb640a43b2d60058e6847))
* **failover-loop:** refactor error handling and logging in failover loop ([ef2662c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef2662c335ae4057c591b531b0784c86cd73951e))
* **gateway,providers,oauth:** codex-oauth-with-proxy ([#76](https://github.com/dyndynjyxa/aio-coding-hub/issues/76)) ([f37e59a](https://github.com/dyndynjyxa/aio-coding-hub/commit/f37e59a3dbfdc8a95d18e592f1931c775f19c2ab))
* **gateway:** 对齐 CCH 基础配置整流 ([a1c1b5c](https://github.com/dyndynjyxa/aio-coding-hub/commit/a1c1b5c2b730b89184d59fac207d252e9d415c93))
* **gateway:** 支持会话绑定提供者的查询与恢复 ([cbfb17e](https://github.com/dyndynjyxa/aio-coding-hub/commit/cbfb17e385f6ee4689148c8630f56bed0c281c95))
* **gateway:** 支持强制指定请求的Provider并更新Claude终端启动逻辑 ([904a9aa](https://github.com/dyndynjyxa/aio-coding-hub/commit/904a9aa6805681fb5642e350105803ce01ae998b))
* **gateway:** 添加决策链以丰富失败重试事件细节 ([262e98c](https://github.com/dyndynjyxa/aio-coding-hub/commit/262e98cb0ac974c5452e037ac51d5730625d1ef2))
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
* **mcp:** 优化服务器键生成与导入逻辑，支持保留大小写 ([f61a5c6](https://github.com/dyndynjyxa/aio-coding-hub/commit/f61a5c6be09795efaf9c4aafe7fee5f86329c0ea))
* **mcp:** 添加 env 和 header 键值对格式校验 ([7d698ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/7d698ca04f1cc9f9c0d420f6d7fec2a53f2d643a))
* **model-prices:** add model price alias rules ([60cbcc1](https://github.com/dyndynjyxa/aio-coding-hub/commit/60cbcc1c65ff025e79313facaf27e625a3de9997))
* **Oauth:** adding gemini Oauth support ([fd1eb79](https://github.com/dyndynjyxa/aio-coding-hub/commit/fd1eb7933410510e9ee46e469234d91658a49fcf))
* **provider-editor:** enhance ProviderEditorDialog with limit configuration cards ([cc14a00](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc14a009303129793ceec840933155dcfad775d8))
* **providers:** collapse model mapping editors ([4672961](https://github.com/dyndynjyxa/aio-coding-hub/commit/4672961c8facbd27d715a762864c2bf4f32ac932))
* **providers:** 增加供应商备注字段并支持读取API Key ([fec7057](https://github.com/dyndynjyxa/aio-coding-hub/commit/fec7057d137a7c0b38b46b5591a9e19e624b679a))
* **providers:** 支持复制供应商配置并预填创建表单 ([c8fb707](https://github.com/dyndynjyxa/aio-coding-hub/commit/c8fb7078131ad67e4d2895f81467a9358b897d2c))
* **providers:** 添加供应商标签功能，支持分类筛选 ([7671732](https://github.com/dyndynjyxa/aio-coding-hub/commit/76717329a119f9a65dc2f3a9dffe0a887063b0bd))
* **request-end:** enhance request logging and event emission ([809a684](https://github.com/dyndynjyxa/aio-coding-hub/commit/809a6843a2c2eb952bbb00f7c56694c908d99bd1))
* **request-end:** refactor request logging and event emission ([e56909e](https://github.com/dyndynjyxa/aio-coding-hub/commit/e56909e374c26608407a1d63d32c9d55f914b355))
* **responsive:** enhance layout and styling for improved mobile experience ([e22483a](https://github.com/dyndynjyxa/aio-coding-hub/commit/e22483a8da8c35822c6e5b0532aa1461cb68cf95))
* **route:** 支持展示同一 provider 的连续尝试次数及跳过状态 ([6659157](https://github.com/dyndynjyxa/aio-coding-hub/commit/6659157a12c3809b1e4d8de7bf95b95afe6ed383))
* **settings:** update Claude settings and remove deprecated options ([49e055a](https://github.com/dyndynjyxa/aio-coding-hub/commit/49e055aeb176819cb56a925aecbd51f362008762))
* **settings:** 新增任务结束提醒开关和相关功能 ([2fb4022](https://github.com/dyndynjyxa/aio-coding-hub/commit/2fb4022ff2f1078238ced25188960f19da188260))
* **settings:** 新增缓存异常监测功能开关 ([ea681c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ea681c155346da3057f25e25867ab75a818f4157))
* **settings:** 新增静默启动配置支持 ([fcc436b](https://github.com/dyndynjyxa/aio-coding-hub/commit/fcc436b5dc5baf3b57e7160bd7e3562275535072))
* **SkillsMarketPage:** enhance UI with tab selection and external links ([2849017](https://github.com/dyndynjyxa/aio-coding-hub/commit/2849017554128279822fef9b667d8ec166a08432))
* **skills:** 支持从本地源恢复缺失的ssot技能目录 ([4414f7d](https://github.com/dyndynjyxa/aio-coding-hub/commit/4414f7d38a44e4aff7de0730fe591b93146d7468))
* **skills:** 添加 Skill 返回本机已安装功能 ([84cfd0b](https://github.com/dyndynjyxa/aio-coding-hub/commit/84cfd0be09c5c38dbd9613533062a3daef6c57df))
* **sort-modes:** add toggle functionality for enabling/disabling providers in sort modes and update related services ([e6cd992](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6cd99241f5c86da3f62afebefeffdf33e62b8f0))
* **taskCompleteNotifyEvents:** 改进任务完成通知的请求追踪与静默期控制 ([e3f03d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/e3f03d4bf2276db371e170ee4739cd379b6d3fe5))
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
* **usage:** 添加供应商过滤支持以改进使用统计查询 ([e2178d9](https://github.com/dyndynjyxa/aio-coding-hub/commit/e2178d93ae413307cd4bb5f830c30550a6e0af1d))
* **workspaces:** add WorkspacesPage route and enhance CLI manager settings ([2784072](https://github.com/dyndynjyxa/aio-coding-hub/commit/2784072e26748aa21f679c245f74cf3063f4177c))
* **workspaces:** enhance MCP and Prompts management with workspace support ([791ee6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/791ee6c88b06c16c596b82dcf7956ea5b98dc18d))
* **workspaces:** improve workspace switching and management functionality ([496c9c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/496c9c35daa23be4d2c44f0653bb11bcb5206e1f))
* **wsl:** 完善 WSL 会话浏览、MCP/提示词同步及删除功能，补充前端测试覆盖率 ([b1308a1](https://github.com/dyndynjyxa/aio-coding-hub/commit/b1308a1e7f3eaf7e871862f3119d7678be72c6aa))
* **wsl:** 支持WSL宿主机地址模式自动检测与自定义 ([f809e86](https://github.com/dyndynjyxa/aio-coding-hub/commit/f809e86ab3690214ba2407c14e4dee1febd9ab14))
* **wsl:** 支持浏览 WSL 环境中的 CLI 会话记录，增加wsl提示词和mcp同步，并有可见提示 ([6147df2](https://github.com/dyndynjyxa/aio-coding-hub/commit/6147df28f904798b7d8e02c5e5946ec3453187c6))
* **wsl:** 支持浏览 WSL 环境中的 CLI 会话记录，增加wsl提示词和mcp同步，并有可见提示 ([41d7d26](https://github.com/dyndynjyxa/aio-coding-hub/commit/41d7d2661a03a3dea90bcb63a06f384fde1ac31a))
* **wsl:** 添加启动时自动检测和配置 WSL 环境功能 ([707bd55](https://github.com/dyndynjyxa/aio-coding-hub/commit/707bd5518fd9881715ec5f3df0ef9250970570bb))


### Bug Fixes

* **ci:** add explicit permissions to release-please job ([8c77c04](https://github.com/dyndynjyxa/aio-coding-hub/commit/8c77c041b0d6f5081abf6ea9a39e1031dc56ad75))
* **ci:** ensure consistent use of RELEASE_PLEASE_TOKEN in release workflow for GitHub actions ([f8a439d](https://github.com/dyndynjyxa/aio-coding-hub/commit/f8a439d213346a907dfc7355b6b6caf4b4194799))
* **ci:** quote FALLBACK_NOTES value in release workflow ([536896c](https://github.com/dyndynjyxa/aio-coding-hub/commit/536896ca4072f48002e8c83b91d78164ed40384f))
* **ci:** revert release-please job to original config ([0d1afad](https://github.com/dyndynjyxa/aio-coding-hub/commit/0d1afade58707c53554f7680dd7455d6ef547187))
* **ci:** satisfy rust clippy checks ([38d0bc0](https://github.com/dyndynjyxa/aio-coding-hub/commit/38d0bc0081cc90b24bd83e4ced30566d861ea69b))
* **ci:** update release workflow to consistently use RELEASE_PLEASE_TOKEN for GitHub actions ([2c45c46](https://github.com/dyndynjyxa/aio-coding-hub/commit/2c45c461ddf157a4b781c146a0ebed5cd9dc1a44))
* **ci:** update release workflow to use RELEASE_PLEASE_TOKEN for GitHub actions ([48ec3ce](https://github.com/dyndynjyxa/aio-coding-hub/commit/48ec3ce92e28702ee9920eaa42d2f7374b07be31))
* **ci:** use PAT for release-please to fix permission issue ([a2919f2](https://github.com/dyndynjyxa/aio-coding-hub/commit/a2919f2446d8fcb1c4c7ac8dc6a1b4f63f705157))
* **ci:** 修复CI流程中正则表达式重复问题 ([609f2ea](https://github.com/dyndynjyxa/aio-coding-hub/commit/609f2ea657034a58b6f175b995e05431829f6dd9))
* **ci:** 修复正则表达式转义字符问题 ([ba805c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba805c3b2f147f153cbe39042b078547eb36da5a))
* **ci:** 修正发布流程中更新器 JSON 文件参数名称 ([9a4c0f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/9a4c0f063bd645e3286e67f1607334a84a5cb6fd))
* **ClaudeModelValidationDialog:** adjust dialog max-width for responsive design ([11bdfd0](https://github.com/dyndynjyxa/aio-coding-hub/commit/11bdfd0df1b51923a561211f6cc6d5383eedf76c))
* **cli_manager:** 修复 run_in_login_shell 的平台兼容性处理 ([9f81433](https://github.com/dyndynjyxa/aio-coding-hub/commit/9f814337d08f36c08e37486f95b9c9da62880fc1))
* **cli-manager:** 修正模型参数解析逻辑以避免默认值覆盖 ([ae2b7fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae2b7feecfd6e06462b8b0990204fc55d6ad3ccf))
* **cli-proxy:** 修复 Codex CLI 在 Windows 下的沙箱配置与认证模式 ([b79d072](https://github.com/dyndynjyxa/aio-coding-hub/commit/b79d072f39405c15125a8a6391bff122c9a5f315))
* **cli-sessions:** 修复SessionsPage Card容器flex布局 ([2d2a048](https://github.com/dyndynjyxa/aio-coding-hub/commit/2d2a0481f9811160223f9753d6edec5a28dcfc6c))
* **codex-tab:** sandbox_mode danger-full-access 选择不生效及高级配置不回显 ([3943401](https://github.com/dyndynjyxa/aio-coding-hub/commit/3943401524c8ef6a179e65e8ee62fd2ac4f784cb))
* **components:** 修复ClaudeModelValidationDialog点击事件阻塞问题 ([9bd5f53](https://github.com/dyndynjyxa/aio-coding-hub/commit/9bd5f53f38d01fc312b979e3620b5e0badaba585))
* **cost:** 修正成本乘数比较逻辑为包含零值 ([db69776](https://github.com/dyndynjyxa/aio-coding-hub/commit/db6977635f96fc55fa1b4d84d1d71e67dd8dd0b1))
* **db:** 修复providers表note字段迁移及数据处理问题 ([fa8ecda](https://github.com/dyndynjyxa/aio-coding-hub/commit/fa8ecda9bad231ed424c812c1c7cd3d0850b8565))
* **deps:** 升级 react-router-dom 7.11→7.13 修复 XSS 漏洞 ([c4db630](https://github.com/dyndynjyxa/aio-coding-hub/commit/c4db630b4478cab2f1180f03b324cfc0af9d42e3))
* **domain:** 修复 SSE 流错误处理逻辑 ([4446939](https://github.com/dyndynjyxa/aio-coding-hub/commit/4446939c457735727f96dce0b09c91ae6b616c1d))
* **gateway:** allow failover success handler arg count ([0c559cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/0c559cba801d59332a94eb650571487b6b18a53b))
* **gateway:** finalize failed requests and align error codes ([871de9c](https://github.com/dyndynjyxa/aio-coding-hub/commit/871de9c9c9e06ed6cb03026ceb21dfc4b0ba18d3))
* **gateway:** reject forwarding when CLI proxy disabled ([c9edd10](https://github.com/dyndynjyxa/aio-coding-hub/commit/c9edd10cd2f41ef86c8c4c8a3ca2262c8bcb09ef))
* **gateway:** 修正 provider 重新排序时清除会话绑定逻辑 ([f03c02e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f03c02e5332fe84496a029b2e5d130b558cfad31))
* **gateway:** 调整上游连接超时与错误切换逻辑 ([a6870ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6870ae14b6b5bb8edce814ca37da2000e84f6b4))
* **heartbeat_watchdog:** 优化窗口重载的错误处理逻辑 ([6188e6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6188e6c29135c0126490882ceb54302d112ec79c))
* **home:** update HomeCostPanel with accessibility labels and improve test assertions ([0bda6c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/0bda6c1ce8aec6920b30810882f3a804c62ab732))
* **mcp:** 修复导入服务器时基于 server_key 的重复处理 ([f6e6458](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6e64582e1e4c478ec014e61258611d971f8b3b4))
* **proxy:** 允许 resolve_session_bound_provider_id 函数有多个参数 ([705457e](https://github.com/dyndynjyxa/aio-coding-hub/commit/705457eac5092ccd98a0f3523ea1986c67dd9d9c))
* **router:** 修复侧边栏快速点击导航卡死问题 ([8b1f862](https://github.com/dyndynjyxa/aio-coding-hub/commit/8b1f862542b8bd94770ab5f937772a53a4bd1d22))
* **scripts:** 修复生成绑定文件格式化问题 ([9ff697b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9ff697b6b8c93238f48518f27f274df33ba55a9a))
* **sessions:** 剥离替换字符并调整按钮样式 ([3586a16](https://github.com/dyndynjyxa/aio-coding-hub/commit/3586a165ceb8ba3afc42a34fdf4228c71b18cce5))
* **skills:** 优化符号链接目录的移除逻辑 ([2ef9928](https://github.com/dyndynjyxa/aio-coding-hub/commit/2ef9928eb3d1d4fa23cb22e2035384e7be2a01f6))
* **skills:** 移除批量导入功能并改为刷新本机列表 ([45b9618](https://github.com/dyndynjyxa/aio-coding-hub/commit/45b961847b458f72f2bc062bdcdfef93c8722da8))
* **tauri:** 修复 Cargo.lock 同步及 CLI 代理切换后 MCP 同步问题 ([1854b00](https://github.com/dyndynjyxa/aio-coding-hub/commit/1854b00205b4b3a257b984b5a6c5271067b2545e))
* **tauri:** 修复注册表键句柄初始化问题 ([6f468c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f468c9a692e1b7aa8bd343a8ce74ce07d7ad813))
* **tests:** update RealtimeTraceCards test to reflect token display format change ([f43633e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f43633e5381f44447bbf8aa64c1065d222687c59))
* **theme:** 修复 dark mode 切换闪烁回退问题 ([ac9500c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac9500c04a108c5bc15921353b691394a689ac54))
* **usage:** align cache creation ttl to 5m only ([8d28bcd](https://github.com/dyndynjyxa/aio-coding-hub/commit/8d28bcd2f5d7f8d6bac1a7f65f974c04c5fce337))
* **utils:** 修正computeOutputTokensPerSecond回退逻辑 ([7c59559](https://github.com/dyndynjyxa/aio-coding-hub/commit/7c59559ad0b525e478577d74518173753530a980))
* **wsl:** 修复 WSL 脚本错误消息编码和路径写入问题 ([bccd5c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/bccd5c93110641898e04cd1cde031df604af637b))
* **wsl:** 修复WSL适配器IPv4解析和状态脚本处理 ([06ae9b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/06ae9b1f462572550f327c3afe4034179b1da491))
* **wsl:** 修正同步边界与 sessions 状态 ([cc41840](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc41840fb88744c926544436cf885b440f240403))
* 修复clippy redundant_pattern_matching警告 ([b0274a6](https://github.com/dyndynjyxa/aio-coding-hub/commit/b0274a663396a2958e470e2167933470bf34a70a))


### Performance Improvements

* **home:** 优化实时追踪卡片动画与样式过渡效果 ([48c9966](https://github.com/dyndynjyxa/aio-coding-hub/commit/48c99669d5658884bc594dcebdb35219d0c25e23))


### Code Refactoring

* **infra:** 移除 request_attempt_logs 独立表，改用 request_logs.attempts_json 派生 ([3fb2627](https://github.com/dyndynjyxa/aio-coding-hub/commit/3fb2627407e1b9b57758a8a40d3a7d7393278e1b))

## [0.31.2](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.31.1...aio-coding-hub-v0.31.2) (2026-03-14)


### Bug Fixes

* **tauri:** 修复注册表键句柄初始化问题 ([6f468c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f468c9a692e1b7aa8bd343a8ce74ce07d7ad813))

## [0.31.1](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.31.0...aio-coding-hub-v0.31.1) (2026-03-14)


### Features

* **build:** 添加Windows便携版ZIP构建和WebView2检查 ([fef41e1](https://github.com/dyndynjyxa/aio-coding-hub/commit/fef41e12f2a1301743b40de4c7402f4b02fa80f4))

## [0.31.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.30.15...aio-coding-hub-v0.31.0) (2026-03-12)


### ⚠ BREAKING CHANGES

* **infra:** request_attempt_logs 表不再创建或写入

### Features

* add gemini oauth code assist proxy support ([4a892d0](https://github.com/dyndynjyxa/aio-coding-hub/commit/4a892d035fbcdd5273c909de063eede5d42598fa))
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
* **claudeModelValidation:** 优化模型验证对话框和多轮验证流程支持 ([3f6c031](https://github.com/dyndynjyxa/aio-coding-hub/commit/3f6c03102db7aaeaa2d8399ebdac1cc4787c2a84))
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
* **cli-manager:** 对齐 CCH 基础配置开关 ([55779ef](https://github.com/dyndynjyxa/aio-coding-hub/commit/55779efce6964a17b423869af77f3ebff567d62c))
* **cli-manager:** 支持自定义GPT-5.4模型上下文窗口和自动压缩限制 ([024dca7](https://github.com/dyndynjyxa/aio-coding-hub/commit/024dca78798022fef014a6a4c5b7a63eb6ecd663))
* **cli-sessions:** P0批次1 - 安全校验/React Query/Router state/虚拟化依赖 ([53c9d43](https://github.com/dyndynjyxa/aio-coding-hub/commit/53c9d4320566b03ff5e655ad2442d46a67f77572))
* **cli-sessions:** P0批次2 - 应用安全校验和状态管理重构 ([2dd098f](https://github.com/dyndynjyxa/aio-coding-hub/commit/2dd098fd39d0d45040b83a0cbd80cb13d4a29352))
* **cli-sessions:** P0批次3 - 三个列表虚拟化 ([d48fccd](https://github.com/dyndynjyxa/aio-coding-hub/commit/d48fccd775086bd11c6cf01147fedf0a3c44e86a))
* **cli-sessions:** P1批次 - 布局一致性和Shell转义安全 ([84723f3](https://github.com/dyndynjyxa/aio-coding-hub/commit/84723f377e5088d2f80574be84e6fa819038bb36))
* **cli:** add CLI proxy startup recovery feature ([eb40a6f](https://github.com/dyndynjyxa/aio-coding-hub/commit/eb40a6f03d9a3b50b501795727a06b7ce9013fc1))
* **CliManager:** add response fixer configuration limits and UI inputs ([0023ad6](https://github.com/dyndynjyxa/aio-coding-hub/commit/0023ad69abf91f48a5144250e20b53ea0b2e24bf))
* **cli:** 添加 Claude 终端启动命令生成及剪贴板管理插件支持 ([ba3d3c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba3d3c20d8d1d40accc0d97780c4dae54d48c71a))
* **codex:** add multi_agent feature toggle ([f56a201](https://github.com/dyndynjyxa/aio-coding-hub/commit/f56a2016bb535812e1a00edb0056e0faa6f44957))
* **components:** enhance HomeRequestLogsPanel and RealtimeTraceCards with new formatting utilities ([e6a3550](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6a3550636e4278100a24d009e49ade1056733bc))
* **config:** 新增计划模式推理强度配置项 ([6f5da87](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f5da87696af5c90054d6ca5be0f25e0a91eba40))
* **config:** 添加 GPT-5.4 关联配置及快速模式支持 ([38a11cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/38a11cb4c33300a2a7edb19848d97e25e966688b))
* **ConsolePage:** revamp console log display and functionality ([1d28397](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d28397e88c0b6d43a4d73b348c49c93cb18efde))
* **console:** 改进控制台界面并增加日志过滤和搜索功能 ([93badf8](https://github.com/dyndynjyxa/aio-coding-hub/commit/93badf8766c9a415c821c3f14084183d18d31687))
* **core:** improve workspace imports and startup resilience ([1d7dbf0](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d7dbf0b73cde2b78a70be945c6fd8e248b30c1d))
* **date-range:** refactor custom date range handling in HomeCostPanel and UsagePage ([95b1ad1](https://github.com/dyndynjyxa/aio-coding-hub/commit/95b1ad11b16a0f3e9b187b4883ef85a367d712e0))
* enhance provider model configuration with support for model whitelisting and mapping ([4f44510](https://github.com/dyndynjyxa/aio-coding-hub/commit/4f445106fefa10badae230de52c9fee09bd2486f))
* **failover-loop:** refactor context management for improved clarity and usability ([b96df01](https://github.com/dyndynjyxa/aio-coding-hub/commit/b96df013317c77ca610bb640a43b2d60058e6847))
* **failover-loop:** refactor error handling and logging in failover loop ([ef2662c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef2662c335ae4057c591b531b0784c86cd73951e))
* **gateway,providers,oauth:** codex-oauth-with-proxy ([#76](https://github.com/dyndynjyxa/aio-coding-hub/issues/76)) ([f37e59a](https://github.com/dyndynjyxa/aio-coding-hub/commit/f37e59a3dbfdc8a95d18e592f1931c775f19c2ab))
* **gateway:** 对齐 CCH 基础配置整流 ([a1c1b5c](https://github.com/dyndynjyxa/aio-coding-hub/commit/a1c1b5c2b730b89184d59fac207d252e9d415c93))
* **gateway:** 支持会话绑定提供者的查询与恢复 ([cbfb17e](https://github.com/dyndynjyxa/aio-coding-hub/commit/cbfb17e385f6ee4689148c8630f56bed0c281c95))
* **gateway:** 支持强制指定请求的Provider并更新Claude终端启动逻辑 ([904a9aa](https://github.com/dyndynjyxa/aio-coding-hub/commit/904a9aa6805681fb5642e350105803ce01ae998b))
* **gateway:** 添加决策链以丰富失败重试事件细节 ([262e98c](https://github.com/dyndynjyxa/aio-coding-hub/commit/262e98cb0ac974c5452e037ac51d5730625d1ef2))
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
* **mcp:** 优化服务器键生成与导入逻辑，支持保留大小写 ([f61a5c6](https://github.com/dyndynjyxa/aio-coding-hub/commit/f61a5c6be09795efaf9c4aafe7fee5f86329c0ea))
* **mcp:** 添加 env 和 header 键值对格式校验 ([7d698ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/7d698ca04f1cc9f9c0d420f6d7fec2a53f2d643a))
* **model-prices:** add model price alias rules ([60cbcc1](https://github.com/dyndynjyxa/aio-coding-hub/commit/60cbcc1c65ff025e79313facaf27e625a3de9997))
* **Oauth:** adding gemini Oauth support ([fd1eb79](https://github.com/dyndynjyxa/aio-coding-hub/commit/fd1eb7933410510e9ee46e469234d91658a49fcf))
* **provider-editor:** enhance ProviderEditorDialog with limit configuration cards ([cc14a00](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc14a009303129793ceec840933155dcfad775d8))
* **providers:** collapse model mapping editors ([4672961](https://github.com/dyndynjyxa/aio-coding-hub/commit/4672961c8facbd27d715a762864c2bf4f32ac932))
* **providers:** 增加供应商备注字段并支持读取API Key ([fec7057](https://github.com/dyndynjyxa/aio-coding-hub/commit/fec7057d137a7c0b38b46b5591a9e19e624b679a))
* **providers:** 支持复制供应商配置并预填创建表单 ([c8fb707](https://github.com/dyndynjyxa/aio-coding-hub/commit/c8fb7078131ad67e4d2895f81467a9358b897d2c))
* **providers:** 添加供应商标签功能，支持分类筛选 ([7671732](https://github.com/dyndynjyxa/aio-coding-hub/commit/76717329a119f9a65dc2f3a9dffe0a887063b0bd))
* **request-end:** enhance request logging and event emission ([809a684](https://github.com/dyndynjyxa/aio-coding-hub/commit/809a6843a2c2eb952bbb00f7c56694c908d99bd1))
* **request-end:** refactor request logging and event emission ([e56909e](https://github.com/dyndynjyxa/aio-coding-hub/commit/e56909e374c26608407a1d63d32c9d55f914b355))
* **responsive:** enhance layout and styling for improved mobile experience ([e22483a](https://github.com/dyndynjyxa/aio-coding-hub/commit/e22483a8da8c35822c6e5b0532aa1461cb68cf95))
* **route:** 支持展示同一 provider 的连续尝试次数及跳过状态 ([6659157](https://github.com/dyndynjyxa/aio-coding-hub/commit/6659157a12c3809b1e4d8de7bf95b95afe6ed383))
* **settings:** update Claude settings and remove deprecated options ([49e055a](https://github.com/dyndynjyxa/aio-coding-hub/commit/49e055aeb176819cb56a925aecbd51f362008762))
* **settings:** 新增任务结束提醒开关和相关功能 ([2fb4022](https://github.com/dyndynjyxa/aio-coding-hub/commit/2fb4022ff2f1078238ced25188960f19da188260))
* **settings:** 新增缓存异常监测功能开关 ([ea681c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ea681c155346da3057f25e25867ab75a818f4157))
* **settings:** 新增静默启动配置支持 ([fcc436b](https://github.com/dyndynjyxa/aio-coding-hub/commit/fcc436b5dc5baf3b57e7160bd7e3562275535072))
* **SkillsMarketPage:** enhance UI with tab selection and external links ([2849017](https://github.com/dyndynjyxa/aio-coding-hub/commit/2849017554128279822fef9b667d8ec166a08432))
* **skills:** 支持从本地源恢复缺失的ssot技能目录 ([4414f7d](https://github.com/dyndynjyxa/aio-coding-hub/commit/4414f7d38a44e4aff7de0730fe591b93146d7468))
* **skills:** 添加 Skill 返回本机已安装功能 ([84cfd0b](https://github.com/dyndynjyxa/aio-coding-hub/commit/84cfd0be09c5c38dbd9613533062a3daef6c57df))
* **sort-modes:** add toggle functionality for enabling/disabling providers in sort modes and update related services ([e6cd992](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6cd99241f5c86da3f62afebefeffdf33e62b8f0))
* **taskCompleteNotifyEvents:** 改进任务完成通知的请求追踪与静默期控制 ([e3f03d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/e3f03d4bf2276db371e170ee4739cd379b6d3fe5))
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
* **usage:** 添加供应商过滤支持以改进使用统计查询 ([e2178d9](https://github.com/dyndynjyxa/aio-coding-hub/commit/e2178d93ae413307cd4bb5f830c30550a6e0af1d))
* **workspaces:** add WorkspacesPage route and enhance CLI manager settings ([2784072](https://github.com/dyndynjyxa/aio-coding-hub/commit/2784072e26748aa21f679c245f74cf3063f4177c))
* **workspaces:** enhance MCP and Prompts management with workspace support ([791ee6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/791ee6c88b06c16c596b82dcf7956ea5b98dc18d))
* **workspaces:** improve workspace switching and management functionality ([496c9c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/496c9c35daa23be4d2c44f0653bb11bcb5206e1f))
* **wsl:** 完善 WSL 会话浏览、MCP/提示词同步及删除功能，补充前端测试覆盖率 ([b1308a1](https://github.com/dyndynjyxa/aio-coding-hub/commit/b1308a1e7f3eaf7e871862f3119d7678be72c6aa))
* **wsl:** 支持WSL宿主机地址模式自动检测与自定义 ([f809e86](https://github.com/dyndynjyxa/aio-coding-hub/commit/f809e86ab3690214ba2407c14e4dee1febd9ab14))
* **wsl:** 支持浏览 WSL 环境中的 CLI 会话记录，增加wsl提示词和mcp同步，并有可见提示 ([6147df2](https://github.com/dyndynjyxa/aio-coding-hub/commit/6147df28f904798b7d8e02c5e5946ec3453187c6))
* **wsl:** 支持浏览 WSL 环境中的 CLI 会话记录，增加wsl提示词和mcp同步，并有可见提示 ([41d7d26](https://github.com/dyndynjyxa/aio-coding-hub/commit/41d7d2661a03a3dea90bcb63a06f384fde1ac31a))
* **wsl:** 添加启动时自动检测和配置 WSL 环境功能 ([707bd55](https://github.com/dyndynjyxa/aio-coding-hub/commit/707bd5518fd9881715ec5f3df0ef9250970570bb))


### Bug Fixes

* **ci:** add explicit permissions to release-please job ([8c77c04](https://github.com/dyndynjyxa/aio-coding-hub/commit/8c77c041b0d6f5081abf6ea9a39e1031dc56ad75))
* **ci:** ensure consistent use of RELEASE_PLEASE_TOKEN in release workflow for GitHub actions ([f8a439d](https://github.com/dyndynjyxa/aio-coding-hub/commit/f8a439d213346a907dfc7355b6b6caf4b4194799))
* **ci:** quote FALLBACK_NOTES value in release workflow ([536896c](https://github.com/dyndynjyxa/aio-coding-hub/commit/536896ca4072f48002e8c83b91d78164ed40384f))
* **ci:** revert release-please job to original config ([0d1afad](https://github.com/dyndynjyxa/aio-coding-hub/commit/0d1afade58707c53554f7680dd7455d6ef547187))
* **ci:** satisfy rust clippy checks ([38d0bc0](https://github.com/dyndynjyxa/aio-coding-hub/commit/38d0bc0081cc90b24bd83e4ced30566d861ea69b))
* **ci:** update release workflow to consistently use RELEASE_PLEASE_TOKEN for GitHub actions ([2c45c46](https://github.com/dyndynjyxa/aio-coding-hub/commit/2c45c461ddf157a4b781c146a0ebed5cd9dc1a44))
* **ci:** update release workflow to use RELEASE_PLEASE_TOKEN for GitHub actions ([48ec3ce](https://github.com/dyndynjyxa/aio-coding-hub/commit/48ec3ce92e28702ee9920eaa42d2f7374b07be31))
* **ci:** use PAT for release-please to fix permission issue ([a2919f2](https://github.com/dyndynjyxa/aio-coding-hub/commit/a2919f2446d8fcb1c4c7ac8dc6a1b4f63f705157))
* **ci:** 修复CI流程中正则表达式重复问题 ([609f2ea](https://github.com/dyndynjyxa/aio-coding-hub/commit/609f2ea657034a58b6f175b995e05431829f6dd9))
* **ci:** 修复正则表达式转义字符问题 ([ba805c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba805c3b2f147f153cbe39042b078547eb36da5a))
* **ci:** 修正发布流程中更新器 JSON 文件参数名称 ([9a4c0f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/9a4c0f063bd645e3286e67f1607334a84a5cb6fd))
* **ClaudeModelValidationDialog:** adjust dialog max-width for responsive design ([11bdfd0](https://github.com/dyndynjyxa/aio-coding-hub/commit/11bdfd0df1b51923a561211f6cc6d5383eedf76c))
* **cli_manager:** 修复 run_in_login_shell 的平台兼容性处理 ([9f81433](https://github.com/dyndynjyxa/aio-coding-hub/commit/9f814337d08f36c08e37486f95b9c9da62880fc1))
* **cli-manager:** 修正模型参数解析逻辑以避免默认值覆盖 ([ae2b7fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae2b7feecfd6e06462b8b0990204fc55d6ad3ccf))
* **cli-proxy:** 修复 Codex CLI 在 Windows 下的沙箱配置与认证模式 ([b79d072](https://github.com/dyndynjyxa/aio-coding-hub/commit/b79d072f39405c15125a8a6391bff122c9a5f315))
* **cli-sessions:** 修复SessionsPage Card容器flex布局 ([2d2a048](https://github.com/dyndynjyxa/aio-coding-hub/commit/2d2a0481f9811160223f9753d6edec5a28dcfc6c))
* **codex-tab:** sandbox_mode danger-full-access 选择不生效及高级配置不回显 ([3943401](https://github.com/dyndynjyxa/aio-coding-hub/commit/3943401524c8ef6a179e65e8ee62fd2ac4f784cb))
* **components:** 修复ClaudeModelValidationDialog点击事件阻塞问题 ([9bd5f53](https://github.com/dyndynjyxa/aio-coding-hub/commit/9bd5f53f38d01fc312b979e3620b5e0badaba585))
* **cost:** 修正成本乘数比较逻辑为包含零值 ([db69776](https://github.com/dyndynjyxa/aio-coding-hub/commit/db6977635f96fc55fa1b4d84d1d71e67dd8dd0b1))
* **db:** 修复providers表note字段迁移及数据处理问题 ([fa8ecda](https://github.com/dyndynjyxa/aio-coding-hub/commit/fa8ecda9bad231ed424c812c1c7cd3d0850b8565))
* **deps:** 升级 react-router-dom 7.11→7.13 修复 XSS 漏洞 ([c4db630](https://github.com/dyndynjyxa/aio-coding-hub/commit/c4db630b4478cab2f1180f03b324cfc0af9d42e3))
* **domain:** 修复 SSE 流错误处理逻辑 ([4446939](https://github.com/dyndynjyxa/aio-coding-hub/commit/4446939c457735727f96dce0b09c91ae6b616c1d))
* **gateway:** allow failover success handler arg count ([0c559cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/0c559cba801d59332a94eb650571487b6b18a53b))
* **gateway:** finalize failed requests and align error codes ([871de9c](https://github.com/dyndynjyxa/aio-coding-hub/commit/871de9c9c9e06ed6cb03026ceb21dfc4b0ba18d3))
* **gateway:** reject forwarding when CLI proxy disabled ([c9edd10](https://github.com/dyndynjyxa/aio-coding-hub/commit/c9edd10cd2f41ef86c8c4c8a3ca2262c8bcb09ef))
* **gateway:** 修正 provider 重新排序时清除会话绑定逻辑 ([f03c02e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f03c02e5332fe84496a029b2e5d130b558cfad31))
* **gateway:** 调整上游连接超时与错误切换逻辑 ([a6870ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6870ae14b6b5bb8edce814ca37da2000e84f6b4))
* **heartbeat_watchdog:** 优化窗口重载的错误处理逻辑 ([6188e6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6188e6c29135c0126490882ceb54302d112ec79c))
* **home:** update HomeCostPanel with accessibility labels and improve test assertions ([0bda6c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/0bda6c1ce8aec6920b30810882f3a804c62ab732))
* **mcp:** 修复导入服务器时基于 server_key 的重复处理 ([f6e6458](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6e64582e1e4c478ec014e61258611d971f8b3b4))
* **proxy:** 允许 resolve_session_bound_provider_id 函数有多个参数 ([705457e](https://github.com/dyndynjyxa/aio-coding-hub/commit/705457eac5092ccd98a0f3523ea1986c67dd9d9c))
* **router:** 修复侧边栏快速点击导航卡死问题 ([8b1f862](https://github.com/dyndynjyxa/aio-coding-hub/commit/8b1f862542b8bd94770ab5f937772a53a4bd1d22))
* **scripts:** 修复生成绑定文件格式化问题 ([9ff697b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9ff697b6b8c93238f48518f27f274df33ba55a9a))
* **sessions:** 剥离替换字符并调整按钮样式 ([3586a16](https://github.com/dyndynjyxa/aio-coding-hub/commit/3586a165ceb8ba3afc42a34fdf4228c71b18cce5))
* **skills:** 优化符号链接目录的移除逻辑 ([2ef9928](https://github.com/dyndynjyxa/aio-coding-hub/commit/2ef9928eb3d1d4fa23cb22e2035384e7be2a01f6))
* **skills:** 移除批量导入功能并改为刷新本机列表 ([45b9618](https://github.com/dyndynjyxa/aio-coding-hub/commit/45b961847b458f72f2bc062bdcdfef93c8722da8))
* **tauri:** 修复 Cargo.lock 同步及 CLI 代理切换后 MCP 同步问题 ([1854b00](https://github.com/dyndynjyxa/aio-coding-hub/commit/1854b00205b4b3a257b984b5a6c5271067b2545e))
* **tests:** update RealtimeTraceCards test to reflect token display format change ([f43633e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f43633e5381f44447bbf8aa64c1065d222687c59))
* **theme:** 修复 dark mode 切换闪烁回退问题 ([ac9500c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac9500c04a108c5bc15921353b691394a689ac54))
* **usage:** align cache creation ttl to 5m only ([8d28bcd](https://github.com/dyndynjyxa/aio-coding-hub/commit/8d28bcd2f5d7f8d6bac1a7f65f974c04c5fce337))
* **utils:** 修正computeOutputTokensPerSecond回退逻辑 ([7c59559](https://github.com/dyndynjyxa/aio-coding-hub/commit/7c59559ad0b525e478577d74518173753530a980))
* **wsl:** 修复 WSL 脚本错误消息编码和路径写入问题 ([bccd5c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/bccd5c93110641898e04cd1cde031df604af637b))
* **wsl:** 修复WSL适配器IPv4解析和状态脚本处理 ([06ae9b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/06ae9b1f462572550f327c3afe4034179b1da491))
* **wsl:** 修正同步边界与 sessions 状态 ([cc41840](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc41840fb88744c926544436cf885b440f240403))
* 修复clippy redundant_pattern_matching警告 ([b0274a6](https://github.com/dyndynjyxa/aio-coding-hub/commit/b0274a663396a2958e470e2167933470bf34a70a))


### Performance Improvements

* **home:** 优化实时追踪卡片动画与样式过渡效果 ([48c9966](https://github.com/dyndynjyxa/aio-coding-hub/commit/48c99669d5658884bc594dcebdb35219d0c25e23))


### Code Refactoring

* **infra:** 移除 request_attempt_logs 独立表，改用 request_logs.attempts_json 派生 ([3fb2627](https://github.com/dyndynjyxa/aio-coding-hub/commit/3fb2627407e1b9b57758a8a40d3a7d7393278e1b))

## [0.30.15](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.30.14...aio-coding-hub-v0.30.15) (2026-03-12)


### Features

* **wsl:** 完善 WSL 会话浏览、MCP/提示词同步及删除功能，补充前端测试覆盖率 ([b1308a1](https://github.com/dyndynjyxa/aio-coding-hub/commit/b1308a1e7f3eaf7e871862f3119d7678be72c6aa))
* **wsl:** 支持浏览 WSL 环境中的 CLI 会话记录，增加wsl提示词和mcp同步，并有可见提示 ([6147df2](https://github.com/dyndynjyxa/aio-coding-hub/commit/6147df28f904798b7d8e02c5e5946ec3453187c6))
* **wsl:** 支持浏览 WSL 环境中的 CLI 会话记录，增加wsl提示词和mcp同步，并有可见提示 ([41d7d26](https://github.com/dyndynjyxa/aio-coding-hub/commit/41d7d2661a03a3dea90bcb63a06f384fde1ac31a))


### Bug Fixes

* **wsl:** 修正同步边界与 sessions 状态 ([cc41840](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc41840fb88744c926544436cf885b440f240403))

## [0.30.14](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.30.13...aio-coding-hub-v0.30.14) (2026-03-11)


### Features

* **usage:** 添加供应商过滤支持以改进使用统计查询 ([e2178d9](https://github.com/dyndynjyxa/aio-coding-hub/commit/e2178d93ae413307cd4bb5f830c30550a6e0af1d))


### Performance Improvements

* **home:** 优化实时追踪卡片动画与样式过渡效果 ([48c9966](https://github.com/dyndynjyxa/aio-coding-hub/commit/48c99669d5658884bc594dcebdb35219d0c25e23))

## [0.30.13](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.30.12...aio-coding-hub-v0.30.13) (2026-03-10)


### Features

* add gemini oauth code assist proxy support ([4a892d0](https://github.com/dyndynjyxa/aio-coding-hub/commit/4a892d035fbcdd5273c909de063eede5d42598fa))
* **Oauth:** adding gemini Oauth support ([fd1eb79](https://github.com/dyndynjyxa/aio-coding-hub/commit/fd1eb7933410510e9ee46e469234d91658a49fcf))
* **providers:** 支持复制供应商配置并预填创建表单 ([c8fb707](https://github.com/dyndynjyxa/aio-coding-hub/commit/c8fb7078131ad67e4d2895f81467a9358b897d2c))


### Bug Fixes

* **ci:** satisfy rust clippy checks ([38d0bc0](https://github.com/dyndynjyxa/aio-coding-hub/commit/38d0bc0081cc90b24bd83e4ced30566d861ea69b))
* **cli-proxy:** 修复 Codex CLI 在 Windows 下的沙箱配置与认证模式 ([b79d072](https://github.com/dyndynjyxa/aio-coding-hub/commit/b79d072f39405c15125a8a6391bff122c9a5f315))
* **gateway:** allow failover success handler arg count ([0c559cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/0c559cba801d59332a94eb650571487b6b18a53b))
* **scripts:** 修复生成绑定文件格式化问题 ([9ff697b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9ff697b6b8c93238f48518f27f274df33ba55a9a))

## [0.30.12](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.30.11...aio-coding-hub-v0.30.12) (2026-03-09)


### Features

* **cli-manager:** 支持自定义GPT-5.4模型上下文窗口和自动压缩限制 ([024dca7](https://github.com/dyndynjyxa/aio-coding-hub/commit/024dca78798022fef014a6a4c5b7a63eb6ecd663))
* **config:** 添加 GPT-5.4 关联配置及快速模式支持 ([38a11cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/38a11cb4c33300a2a7edb19848d97e25e966688b))
* **gateway,providers,oauth:** codex-oauth-with-proxy ([#76](https://github.com/dyndynjyxa/aio-coding-hub/issues/76)) ([f37e59a](https://github.com/dyndynjyxa/aio-coding-hub/commit/f37e59a3dbfdc8a95d18e592f1931c775f19c2ab))
* **settings:** 新增静默启动配置支持 ([fcc436b](https://github.com/dyndynjyxa/aio-coding-hub/commit/fcc436b5dc5baf3b57e7160bd7e3562275535072))


### Bug Fixes

* **cli-manager:** 修正模型参数解析逻辑以避免默认值覆盖 ([ae2b7fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae2b7feecfd6e06462b8b0990204fc55d6ad3ccf))
* **utils:** 修正computeOutputTokensPerSecond回退逻辑 ([7c59559](https://github.com/dyndynjyxa/aio-coding-hub/commit/7c59559ad0b525e478577d74518173753530a980))

## [0.30.11](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.30.10...aio-coding-hub-v0.30.11) (2026-03-03)


### Bug Fixes

* **cost:** 修正成本乘数比较逻辑为包含零值 ([db69776](https://github.com/dyndynjyxa/aio-coding-hub/commit/db6977635f96fc55fa1b4d84d1d71e67dd8dd0b1))

## [0.30.10](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.30.9...aio-coding-hub-v0.30.10) (2026-03-03)


### Features

* **providers:** 增加供应商备注字段并支持读取API Key ([fec7057](https://github.com/dyndynjyxa/aio-coding-hub/commit/fec7057d137a7c0b38b46b5591a9e19e624b679a))


### Bug Fixes

* **db:** 修复providers表note字段迁移及数据处理问题 ([fa8ecda](https://github.com/dyndynjyxa/aio-coding-hub/commit/fa8ecda9bad231ed424c812c1c7cd3d0850b8565))

## [0.30.9](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.30.8...aio-coding-hub-v0.30.9) (2026-03-02)


### Features

* **mcp:** 优化服务器键生成与导入逻辑，支持保留大小写 ([f61a5c6](https://github.com/dyndynjyxa/aio-coding-hub/commit/f61a5c6be09795efaf9c4aafe7fee5f86329c0ea))
* **skills:** 支持从本地源恢复缺失的ssot技能目录 ([4414f7d](https://github.com/dyndynjyxa/aio-coding-hub/commit/4414f7d38a44e4aff7de0730fe591b93146d7468))


### Bug Fixes

* **mcp:** 修复导入服务器时基于 server_key 的重复处理 ([f6e6458](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6e64582e1e4c478ec014e61258611d971f8b3b4))

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
