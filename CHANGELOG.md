# Changelog

## [0.60.10](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.60.9...aio-coding-hub-v0.60.10) (2026-07-09)


### Bug Fixes

* **gateway:** arm request abort guard before active-request registration ([49d590c](https://github.com/dyndynjyxa/aio-coding-hub/commit/49d590c4df63a4cd9a1428cb5cbacb58b1d0f080))
* **home:** keep freshness watchdog alive while a recent log row lacks terminal state ([46ce59e](https://github.com/dyndynjyxa/aio-coding-hub/commit/46ce59eebce5ad962592db6121c9e77ae8cc0044))
* **home:** keep signal-driven request log refresh alive while backgrounded ([4ed3b4b](https://github.com/dyndynjyxa/aio-coding-hub/commit/4ed3b4b4d89ef840813e3f32e2ccc0a6ccd1e432))

## [0.60.9](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.60.8...aio-coding-hub-v0.60.9) (2026-07-07)


### Bug Fixes

* **home:** refresh stale request log activity ([dc30559](https://github.com/dyndynjyxa/aio-coding-hub/commit/dc30559e63ed37d3946ca2f6b8061bc5d3cc6c33))

## [0.60.8](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.60.7...aio-coding-hub-v0.60.8) (2026-07-05)


### Features

* **gateway:** 熔断跳过归因落库并在日志详情展示原因与冷却时间 ([530e6d3](https://github.com/dyndynjyxa/aio-coding-hub/commit/530e6d3b16815b67bf5fe036fc356cfab267b2c8))
* **settings:** 请求日志留存策略可见并提供数据库压缩入口 ([c23e4bd](https://github.com/dyndynjyxa/aio-coding-hub/commit/c23e4bd7f19355dba75afa63bec19666ae613c7f))


### Bug Fixes

* **gateway:** 流式空闲超时在流式转发路径真正生效 ([6197e81](https://github.com/dyndynjyxa/aio-coding-hub/commit/6197e81046867e229f0e1dfe0a98f1bf311a484a))
* **ui:** failover 徽章推导与落库语义收敛并清理状态死代码 ([5b5eae8](https://github.com/dyndynjyxa/aio-coding-hub/commit/5b5eae8ee0088b0be407e64acd2a36426768976b))


### Code Refactoring

* **gateway:** 熔断通知文案迁移前端渲染并删除后端文本构造 ([5cd4915](https://github.com/dyndynjyxa/aio-coding-hub/commit/5cd49151d1ea93034d0e371a3d68aa09800f10f7))
* **ui:** 拆分 HomeLogShared 杂物间为三个单一职责模块 ([8590f3f](https://github.com/dyndynjyxa/aio-coding-hub/commit/8590f3f04fdbfc2529d754ac90e93c189af81540))
* **ui:** 清理 review 发现的 P2 项并移除 traceRoute skipped 死分支 ([ddf131f](https://github.com/dyndynjyxa/aio-coding-hub/commit/ddf131fe735a4459d4ed0e177dd1b4c82063bb93))
* **ui:** 网关事件 payload 类型改由 Specta 生成派生 ([85b4007](https://github.com/dyndynjyxa/aio-coding-hub/commit/85b40072327181f00e2efe7020353dbcf4fbe9ab))

## [0.60.7](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.60.6...aio-coding-hub-v0.60.7) (2026-07-05)


### Features

* **ui:** 请求日志错误卡片展示失败尝试归因摘要 ([bdca955](https://github.com/dyndynjyxa/aio-coding-hub/commit/bdca955f9ce48c2abfd1a1ae33a29aeace322f2b))


### Bug Fixes

* **gateway:** 识别 compact 请求并放宽首字节超时至 300 秒 ([bbbdeaf](https://github.com/dyndynjyxa/aio-coding-hub/commit/bbbdeaf019f93b3069d8a554647c24ef5813fd8c))

## [0.60.6](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.60.5...aio-coding-hub-v0.60.6) (2026-07-03)


### Bug Fixes

* **release:** 以 0.60.6 重新发布 0.60.5 的全部内容 ([b4378f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/b4378f135ae72afc87bd56aa72723df0c9e1c327))

## [0.60.5](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.60.4...aio-coding-hub-v0.60.5) (2026-07-03)


### Bug Fixes

* **home:** unify in-progress request rendering as realtime cards ([faaa440](https://github.com/dyndynjyxa/aio-coding-hub/commit/faaa4406d6892c6f16c2bf7239b2be7ef3c2bede))
* **watchdog:** 后台修复死亡 WebView,重构白屏恢复状态机 ([edeb28e](https://github.com/dyndynjyxa/aio-coding-hub/commit/edeb28e994b1ea6d8b7cccdd21027fd4176ac1e5))


### Code Refactoring

* single-source check stages, add Homebrew cask release pipeline ([e873103](https://github.com/dyndynjyxa/aio-coding-hub/commit/e873103a331c2a153acc59223038bfd52ca192d7))

## [0.60.4](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.60.3...aio-coding-hub-v0.60.4) (2026-07-03)


### Bug Fixes

* **gateway:** derive in-progress request logs from active request registry ([be0d772](https://github.com/dyndynjyxa/aio-coding-hub/commit/be0d7725e91134a90574e61c943e6de98547b63e)), closes [#323](https://github.com/dyndynjyxa/aio-coding-hub/issues/323)


### Code Refactoring

* single-source cross-layer contracts, hot-path DB offload, request-log retention ([0f1d8d8](https://github.com/dyndynjyxa/aio-coding-hub/commit/0f1d8d8984651e14d76837892b431c4a3bf546bc))

## [0.60.3](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.60.2...aio-coding-hub-v0.60.3) (2026-07-01)


### Features

* add plugin file preview ipc ([5f82075](https://github.com/dyndynjyxa/aio-coding-hub/commit/5f820756a7e8d3da3d0ecb557a83331ca4e61515))
* add plugin lifecycle previews ([5b5200b](https://github.com/dyndynjyxa/aio-coding-hub/commit/5b5200b5e994c6583f7e6161134c3d8f82c052bd))
* **create-aio-plugin:** add replay and publish parity ([0928613](https://github.com/dyndynjyxa/aio-coding-hub/commit/09286132ed0dc515400da49055da98e727d21a73))
* monitor streaming request lifecycle ([35beb37](https://github.com/dyndynjyxa/aio-coding-hub/commit/35beb37e55e44ac5e2ff9a4bd836b5d9a6e7ccb5))
* **plugin-devtools:** add plugin doctor diagnostics ([9354298](https://github.com/dyndynjyxa/aio-coding-hub/commit/9354298d9b578a90b9b491a5a1cec6bcad6d89be))
* **plugin-devtools:** add strict plugin validation ([963e191](https://github.com/dyndynjyxa/aio-coding-hub/commit/963e1914767ebd7288f2ddc0be21a7d391e341d8))
* **plugin-devtools:** explain declarative rule replay ([e768f79](https://github.com/dyndynjyxa/aio-coding-hub/commit/e768f790e7c9259aaf46407d9db99c13c87233c2))
* **plugins:** add example developer loop templates ([14118c0](https://github.com/dyndynjyxa/aio-coding-hub/commit/14118c03a6e08cbf1def1b2f0b6dc2a1154c62fc))
* **plugins:** add extension host instance registry ([4415158](https://github.com/dyndynjyxa/aio-coding-hub/commit/44151588b61f40aab4e37e7c8abe5ce2dff68aae))
* **plugins:** add hook resource budgets ([096ef7c](https://github.com/dyndynjyxa/aio-coding-hub/commit/096ef7c4a5efac16a1b320d312e5903c1143a7db))
* **plugins:** add lifecycle preview and update diff ([d2117a5](https://github.com/dyndynjyxa/aio-coding-hub/commit/d2117a55edeadecbcb33c7d3e64a55b4b52162de))
* **plugins:** add managed extension host worker ([54bc811](https://github.com/dyndynjyxa/aio-coding-hub/commit/54bc811f13dfc5b0a0e6ee1246e4587ad7c3f943))
* **plugins:** add market card view model ([3e5755f](https://github.com/dyndynjyxa/aio-coding-hub/commit/3e5755ffb58f2153b5a169b3d870035d81fde435))
* **plugins:** add runtime lifecycle registry ([818a43d](https://github.com/dyndynjyxa/aio-coding-hub/commit/818a43d04febb2cc3579a959d4042f530d57c093))
* **plugins:** add structured runtime reports ([3a718fb](https://github.com/dyndynjyxa/aio-coding-hub/commit/3a718fbe0676b19674986307003c55a499012f77))
* **plugins:** define extension host manifest contract ([f63219e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f63219e4f3d711de8274a53a22db87263fac1628))
* **plugins:** define extension protocol bridge skeleton ([ef294b7](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef294b72cb225404e2ec9d533b7d2d6143a8c265))
* **plugins:** enforce extension host capabilities ([08b5a9a](https://github.com/dyndynjyxa/aio-coding-hub/commit/08b5a9a9087da16c822482509cf4aa2cc6001cf0))
* **plugins:** enforce extension host manifest contract ([8ae8dbc](https://github.com/dyndynjyxa/aio-coding-hub/commit/8ae8dbc312dd298e77380b16e93ac7dd8728b65b))
* **plugins:** execute commands through extension host registry ([689f5bc](https://github.com/dyndynjyxa/aio-coding-hub/commit/689f5bceea7e1f21ce027a4bdd4532d0a840ccd4))
* **plugins:** execute extension commands with reports ([3a7674d](https://github.com/dyndynjyxa/aio-coding-hub/commit/3a7674dff2e31c88b6948d5dd46744848ce2a6be))
* **plugins:** export trace replay fixtures ([4038339](https://github.com/dyndynjyxa/aio-coding-hub/commit/403833904a4b4b4022d763e37a44b3b16bcc2f43))
* **plugins:** expose active contribution registry ([52c38fb](https://github.com/dyndynjyxa/aio-coding-hub/commit/52c38fb742e094065cad449a873bfe74d48bea9a))
* **plugins:** make sdk contract extension host only ([5936902](https://github.com/dyndynjyxa/aio-coding-hub/commit/59369023dcdba32769613283eb61667fcf99ed39))
* **plugins:** productize marketplace panel ([f27a9a9](https://github.com/dyndynjyxa/aio-coding-hub/commit/f27a9a9df7c9c06c1a84b17acdb7488201db54da))
* **plugins:** productize observability and market UI ([0552e4e](https://github.com/dyndynjyxa/aio-coding-hub/commit/0552e4e006ba29660f83be0426b0ea9f5705da5c))
* **plugins:** reject unsupported legacy plugin packages ([57c6363](https://github.com/dyndynjyxa/aio-coding-hub/commit/57c63639159b9ed2782f7feb90dada0006e2b2d5))
* **plugins:** render host-owned UI contributions ([c80ee91](https://github.com/dyndynjyxa/aio-coding-hub/commit/c80ee914cc214c10667863458392a36ee349063f))
* **plugins:** scaffold extension host plugins ([be77b5f](https://github.com/dyndynjyxa/aio-coding-hub/commit/be77b5f3b338f8158ea75967bd25d27f3b7f1008))
* **plugins:** show extension contribution impact ([a762e28](https://github.com/dyndynjyxa/aio-coding-hub/commit/a762e28cb77ef91a0087148d706e8923008cc9e1))
* **plugins:** show runtime observability in plugin details ([7fbb4e7](https://github.com/dyndynjyxa/aio-coding-hub/commit/7fbb4e701f01f4ba0b8bbb11e081368b49c10688))
* **plugins:** wire extension host gateway hooks ([34c0314](https://github.com/dyndynjyxa/aio-coding-hub/commit/34c03142e56da6e2f3c8fbff62ac1ee5f79acc8d))
* **providers:** add plugin extension value storage ([cd35a84](https://github.com/dyndynjyxa/aio-coding-hub/commit/cd35a84fa3f7ab2a4d4f6acf8f024ab055549e2e))
* **providers:** 整合供应商路由排序 ([#307](https://github.com/dyndynjyxa/aio-coding-hub/issues/307)) ([d2e4465](https://github.com/dyndynjyxa/aio-coding-hub/commit/d2e4465c6de9850771253a12519335462fa10892))


### Bug Fixes

* address plugin platform review findings ([cdb9d87](https://github.com/dyndynjyxa/aio-coding-hub/commit/cdb9d87e77fd4e4a1d80e79003eeef7cbb22dfa8))
* **cli-manager:** include exe parent dir in PATH for version probe ([#306](https://github.com/dyndynjyxa/aio-coding-hub/issues/306)) ([78d27e2](https://github.com/dyndynjyxa/aio-coding-hub/commit/78d27e2c85c4bf07ae7c6a3947d246ba5bba2051))
* isolate provider route action at card edge ([83e8884](https://github.com/dyndynjyxa/aio-coding-hub/commit/83e888463fe2c0db01c838d116b0757f711c0c4e))
* keep provider route action width stable ([4c74357](https://github.com/dyndynjyxa/aio-coding-hub/commit/4c743574ba24bbb9e29a13b3d4a982f5b3392719))
* move provider route action to card edge ([9bc4145](https://github.com/dyndynjyxa/aio-coding-hub/commit/9bc4145229628e4d4d59d8e828669fa83cefb15e))
* **plugin-devtools:** align strict rule permissions with hooks ([d6e8f5d](https://github.com/dyndynjyxa/aio-coding-hub/commit/d6e8f5ded0a067b511758e12a25b032ca1b84f96))
* **plugin-devtools:** align strict rules with host runtime ([8381e66](https://github.com/dyndynjyxa/aio-coding-hub/commit/8381e66cbe21b1397c5086b443861254a38eb6a4))
* **plugin-devtools:** avoid js regex strict false positives ([6f5dc4e](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f5dc4ec059ceb575087e6be31a983b57d248dd5))
* **plugin-devtools:** catch malformed strict regex syntax ([7cf4e5b](https://github.com/dyndynjyxa/aio-coding-hub/commit/7cf4e5b0af9c42660df3fc40c1f20bd6d30a4561))
* **plugin-devtools:** check runtime file presence by path ([6478a86](https://github.com/dyndynjyxa/aio-coding-hub/commit/6478a8643647b421190f7b16a5d17c8d25df71ed))
* **plugin-devtools:** classify empty strict rule documents ([e7df3f4](https://github.com/dyndynjyxa/aio-coding-hub/commit/e7df3f45aa0bee216e0f8f268f1cc843566d3fd1))
* **plugin-devtools:** classify malformed doctor manifests ([ffab898](https://github.com/dyndynjyxa/aio-coding-hub/commit/ffab898636468b35a12f7801ff6fc785dfb00d9b))
* **plugin-devtools:** diagnose malformed runtime shapes ([c082124](https://github.com/dyndynjyxa/aio-coding-hub/commit/c0821242a4c227b336190ec19d3d7d32daf53981))
* **plugin-devtools:** explain extended regex flags ([5ee20eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/5ee20eb3a19dbca26157ae95358abf1c917f335f))
* **plugin-devtools:** explain rust inline regex flags ([213296e](https://github.com/dyndynjyxa/aio-coding-hub/commit/213296e6c86e1d63fb5cdc269b51d13533d534a7))
* **plugin-devtools:** guard doctor manifest field shapes ([41dd845](https://github.com/dyndynjyxa/aio-coding-hub/commit/41dd845801365c30e9f0b1d30ab05e683f931f3d))
* **plugin-devtools:** harden doctor file and metadata checks ([94cf4c0](https://github.com/dyndynjyxa/aio-coding-hub/commit/94cf4c02721f5bc2f6bb05d6cf6d666d96d394e4))
* **plugin-devtools:** normalize malformed wasm entry checks ([1c47926](https://github.com/dyndynjyxa/aio-coding-hub/commit/1c47926dfad63f87cf2ede7b3333d09451535d07))
* **plugin-devtools:** preserve legacy replay regex behavior ([1e1c7d8](https://github.com/dyndynjyxa/aio-coding-hub/commit/1e1c7d8f032372577051e9867228a5d118195e11))
* **plugin-devtools:** refine strict action diagnostics ([6ea9ed7](https://github.com/dyndynjyxa/aio-coding-hub/commit/6ea9ed77ac9a6659d62eaa55b164d5c6b717b7f5))
* **plugin-devtools:** reject alternation-leading repeaters ([0ddc5f5](https://github.com/dyndynjyxa/aio-coding-hub/commit/0ddc5f567b288b8305b284d8acfcfd6cd67138cc))
* **plugin-devtools:** reject group-leading repeaters ([43c0eb7](https://github.com/dyndynjyxa/aio-coding-hub/commit/43c0eb776161ae786947af3af116d9f686d207de))
* **plugin-devtools:** replay unicode regex classes ([43b8231](https://github.com/dyndynjyxa/aio-coding-hub/commit/43b82312e2d81de08db077c3004175f996e0ed2d))
* **plugin-devtools:** return doctor diagnostics for invalid manifests ([52bbb50](https://github.com/dyndynjyxa/aio-coding-hub/commit/52bbb50a4848b5e11325b6e1f02582b4d07cbcff))
* **plugin-devtools:** validate declarative rule path entries ([d55164f](https://github.com/dyndynjyxa/aio-coding-hub/commit/d55164f114440ead46b2136d9301457417206f3e))
* **plugin-devtools:** validate merged strict rule shapes ([fb7494c](https://github.com/dyndynjyxa/aio-coding-hub/commit/fb7494ccffa0f47c44909e112af90de67de1604f))
* **plugin-devtools:** validate strict rule runtime limits ([39f58c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/39f58c99abac2d3e43aaab51c25e140d92832438))
* **plugin-devtools:** validate strict rule structure ([8c59920](https://github.com/dyndynjyxa/aio-coding-hub/commit/8c59920976071133ec01a162d0b5f95acbcbb8cc))
* **plugin-devtools:** warn on unsupported replay regex flags ([7283908](https://github.com/dyndynjyxa/aio-coding-hub/commit/7283908e5882f6f3f0c2a45b90239a106c6d128b))
* **plugin-sdk:** align permission dependencies with hook contract ([5a105c0](https://github.com/dyndynjyxa/aio-coding-hub/commit/5a105c0a090410e3bd8b7b62acc5f804d5ebef5b))
* **plugins:** align extension contract consumers ([517eb96](https://github.com/dyndynjyxa/aio-coding-hub/commit/517eb96df8a9fe8b4f9a8a21a66e9559ca3db82a))
* **plugins:** align extension host docs with runtime contract ([812910d](https://github.com/dyndynjyxa/aio-coding-hub/commit/812910d8d5a1c680e672931fb924ec8b60c045ab))
* **plugins:** align extension host scaffold checks ([89c2fbf](https://github.com/dyndynjyxa/aio-coding-hub/commit/89c2fbf2b6360b9241870125e418e98b5bbb5fb4))
* **plugins:** align lifecycle rollback and trust state ([5891e62](https://github.com/dyndynjyxa/aio-coding-hub/commit/5891e6200fcc657074e946d7e51f26cf7cf781f3))
* **plugins:** align manifest api version contract ([0e99322](https://github.com/dyndynjyxa/aio-coding-hub/commit/0e993229d049a06498e50304e405cfa5e2e232ae))
* **plugins:** align service permissions with extension host runtime ([eb4fceb](https://github.com/dyndynjyxa/aio-coding-hub/commit/eb4fceb87c40c7ab8ee44d295b75df1193408f09))
* **plugins:** allow extension manifests in devtools validation ([f961f66](https://github.com/dyndynjyxa/aio-coding-hub/commit/f961f66effd97638b66de9933982b21dcbffb319))
* **plugins:** avoid global registry lock during host execution ([6265064](https://github.com/dyndynjyxa/aio-coding-hub/commit/6265064b877b78ce55ecc2bede6d65c09921b167))
* **plugins:** block reserved official market listings ([bdbd629](https://github.com/dyndynjyxa/aio-coding-hub/commit/bdbd629ed0372e7dde9966351dffcbc149104d58))
* **plugins:** block unroutable market cards ([35a89bd](https://github.com/dyndynjyxa/aio-coding-hub/commit/35a89bdd092b1283ec9e74c51b9cd24b9b0a0553))
* **plugins:** bound runtime artifact reads ([13141ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/13141ca8b479175af7c3854ed2829181642a6f59))
* **plugins:** clarify lifecycle preview UI ([96e9241](https://github.com/dyndynjyxa/aio-coding-hub/commit/96e9241449443729a9a177f7d079c936d0ddb790))
* **plugins:** clarify protocol bridge boundary ([574b093](https://github.com/dyndynjyxa/aio-coding-hub/commit/574b09395d79fbb4d348ea01ba3d38b097a8afdd))
* **plugins:** clean runtime report pruning ([4dfd5ab](https://github.com/dyndynjyxa/aio-coding-hub/commit/4dfd5abfb386a97aad8cc425d40b674d58a69d02))
* **plugins:** clean up extension host gateway lifecycles ([2eab4ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/2eab4ca9b80566a3785ca66bd91008d42698e2af))
* **plugins:** derive installed market state from summaries ([d68c07e](https://github.com/dyndynjyxa/aio-coding-hub/commit/d68c07e41d482fd916f0fc6fb78ec434770c5010))
* **plugins:** detect null gateway rules marker ([50626d9](https://github.com/dyndynjyxa/aio-coding-hub/commit/50626d96e4a48ebe17baabcfd72979bfecd123fb))
* **plugins:** dispose extension hosts on plugin lifecycle changes ([a716901](https://github.com/dyndynjyxa/aio-coding-hub/commit/a716901580d60eb86a271f8016c53e6a101b52ef))
* **plugins:** drop failed warm extension hosts ([d9bcf1d](https://github.com/dyndynjyxa/aio-coding-hub/commit/d9bcf1d1c90053a6715c2ffb21fcddf2669e85c8))
* **plugins:** enforce command capability at worker dispatch ([0036ba6](https://github.com/dyndynjyxa/aio-coding-hub/commit/0036ba6685c4f1b985b78936145a36e60bda9c38))
* **plugins:** enforce full devtools contract drift checks ([ee4cc1d](https://github.com/dyndynjyxa/aio-coding-hub/commit/ee4cc1dcdc6e14d58737936d028dbb5a9600bb69))
* **plugins:** guard pending permission selection ([10eadd9](https://github.com/dyndynjyxa/aio-coding-hub/commit/10eadd9ce8ef7e69954a0530546319d9f5fdf297))
* **plugins:** guard registry dispose all against in flight starts ([8b2a37d](https://github.com/dyndynjyxa/aio-coding-hub/commit/8b2a37dedb90fba68126730c4a50d54e6d2f0ca7))
* **plugins:** guard stale declarative rules plans ([24799a8](https://github.com/dyndynjyxa/aio-coding-hub/commit/24799a8d9bd266f40d318f873215f220c868e658))
* **plugins:** harden runtime mutation boundaries ([1c060ce](https://github.com/dyndynjyxa/aio-coding-hub/commit/1c060ce059b7c6c57c5a5948f17850e7b7cad6ba))
* **plugins:** honor gateway hook timeout and fallback executor ([1df1091](https://github.com/dyndynjyxa/aio-coding-hub/commit/1df1091b1f13ef37fc1fa8df1037de5842631a3f))
* **plugins:** isolate provider UI contribution state ([6e753e6](https://github.com/dyndynjyxa/aio-coding-hub/commit/6e753e62fd4fd5e83c8cf7d6e1ea6be4674577c5))
* **plugins:** make create plugin docs command executable ([b5c2fc4](https://github.com/dyndynjyxa/aio-coding-hub/commit/b5c2fc4c1935e51d97f41ab2c0b1c66a5ecc2e0e))
* **plugins:** make lifecycle state writes transactional ([65060d7](https://github.com/dyndynjyxa/aio-coding-hub/commit/65060d71400ac7ebe0ae6b820008dff77eea07e7))
* **plugins:** make native runtime preview source aware ([c0e42f6](https://github.com/dyndynjyxa/aio-coding-hub/commit/c0e42f633a3a57bb4147c542d2d16e0f9e6b39a9))
* **plugins:** mark stale replay plans superseded ([51263e9](https://github.com/dyndynjyxa/aio-coding-hub/commit/51263e94c27d14c61ca072372979cb707890b19b))
* **plugins:** normalize third party native privacy filter rows ([81a0bec](https://github.com/dyndynjyxa/aio-coding-hub/commit/81a0bec9a198c4cb298628bcf0dcb94d44cc2880))
* **plugins:** prefer exact market source trust ([4fbbba3](https://github.com/dyndynjyxa/aio-coding-hub/commit/4fbbba31334b814b9752e6e24a2cec235618d21a))
* **plugins:** preserve market source trust context ([eb1260e](https://github.com/dyndynjyxa/aio-coding-hub/commit/eb1260e51b421823b5248483fcc1272483a3ea0f))
* **plugins:** prevent stale replay developer paths ([cc149d9](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc149d92e8d5180d11a89d6db9255e6c21148af1))
* **plugins:** prune runtime reports ([b88feb9](https://github.com/dyndynjyxa/aio-coding-hub/commit/b88feb927c056bd6066d4e167b873cb4bac14ea1))
* **plugins:** reconcile rollback state ([69a39ce](https://github.com/dyndynjyxa/aio-coding-hub/commit/69a39ce03c4bec7e5e19f4ce735859644cb0165a))
* **plugins:** reconcile selection after uninstall ([306b5b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/306b5b124d2099c0b6621c8d07b702c8758d2ad4))
* **plugins:** refine contribution impact diff ([330418c](https://github.com/dyndynjyxa/aio-coding-hub/commit/330418c7c4f4e27c2a3bf41e4e223444549106ca))
* **plugins:** reject command execution before activation without capability ([2f2ba02](https://github.com/dyndynjyxa/aio-coding-hub/commit/2f2ba02b9075de7fdd8db98f0f776b51504e46ea))
* **plugins:** reject gateway rules by presence ([b940985](https://github.com/dyndynjyxa/aio-coding-hub/commit/b940985020e3abbced44bf81c5519316d35cb8f8))
* **plugins:** route advanced marketplace installs remotely ([56d39d7](https://github.com/dyndynjyxa/aio-coding-hub/commit/56d39d7b34c27a43c1f8870947710e69e0c88d4d))
* **plugins:** satisfy privacy filter clippy check ([d6d93ba](https://github.com/dyndynjyxa/aio-coding-hub/commit/d6d93bac2c86f989077ae978bbd899de7c423525))
* **plugins:** serialize registry starts per plugin ([c5184ff](https://github.com/dyndynjyxa/aio-coding-hub/commit/c5184fff836b96499571f79a09a53ed01dee88a0))
* **plugins:** simplify plugins page user layout ([3b70933](https://github.com/dyndynjyxa/aio-coding-hub/commit/3b7093309a0c20a42d9681d65189ae5639647346))
* **plugins:** stabilize unsupported gateway rules validation ([70b089b](https://github.com/dyndynjyxa/aio-coding-hub/commit/70b089b49d66decd7ba7203c13cdf59e22e19322))
* **plugins:** sync extension manifest contract ([8dec73f](https://github.com/dyndynjyxa/aio-coding-hub/commit/8dec73f9639e6f3fee6c46de03df69d998e16a17))
* **plugins:** trim advanced market source inputs ([5da7508](https://github.com/dyndynjyxa/aio-coding-hub/commit/5da750892e12ac8f0f7d22b60a917f5cceeaca5f))
* **plugins:** validate extension contributions ([718f93c](https://github.com/dyndynjyxa/aio-coding-hub/commit/718f93c9fffdea23ab1763452380118f546e7d80))
* **plugins:** validate extension host package shape ([8463f4f](https://github.com/dyndynjyxa/aio-coding-hub/commit/8463f4f35bcebdcdd574426fc059e0bfacddb882))
* **plugins:** validate lifecycle rollback target ([41a1e45](https://github.com/dyndynjyxa/aio-coding-hub/commit/41a1e4507b7d3c3c4005c6f0e62f684570398a4f))
* **plugins:** validate protocol bridge ids in host ([239d98e](https://github.com/dyndynjyxa/aio-coding-hub/commit/239d98eceb17d7ed7260f4af1bbe3e7b09d9549d))
* **providers:** tighten extension value storage semantics ([aa08ed9](https://github.com/dyndynjyxa/aio-coding-hub/commit/aa08ed9f9435fd394f592b3f4ad0a51f67332459))
* recognize claude message_stop stream marker ([b8d5df0](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8d5df0a41b2be7dae54158952a8282f4f4742ee))
* restore recharts component identity ([b325aa4](https://github.com/dyndynjyxa/aio-coding-hub/commit/b325aa432077543bd93f0be6004eba9cd063ca0a))
* **updater:** open changelog links externally ([7f4e332](https://github.com/dyndynjyxa/aio-coding-hub/commit/7f4e332bee95fc486c391633241c1d5264a48f23))


### Code Refactoring

* **gateway:** add provider adapter capability facade ([773992b](https://github.com/dyndynjyxa/aio-coding-hub/commit/773992b2b0d5fa70cdd2e46fc36ecbafd4909519))
* **gateway:** route cx2cc count tokens through provider adapter ([d7d54ed](https://github.com/dyndynjyxa/aio-coding-hub/commit/d7d54ed1128000f1fc628c0b1a81fa614c113759))
* **plugins:** add internal hook registry ([108e637](https://github.com/dyndynjyxa/aio-coding-hub/commit/108e637d13ecb7a8b177f561ce6888c3da0bdc3e))
* **plugins:** centralize rust plugin contract metadata ([395f69b](https://github.com/dyndynjyxa/aio-coding-hub/commit/395f69b3991f889896bf1a6fc6f644a33fd98e3c))
* **plugins:** enforce mutations through hook descriptors ([ff52064](https://github.com/dyndynjyxa/aio-coding-hub/commit/ff5206440360dad9d6c2525c9c723a75bab04919))
* **plugins:** extract runtime reports panel ([4c43df7](https://github.com/dyndynjyxa/aio-coding-hub/commit/4c43df7f8f0b91b4797245f048ff2bcc50dea824))
* **plugins:** introduce runtime manager policy facade ([61e0886](https://github.com/dyndynjyxa/aio-coding-hub/commit/61e0886ccc44b0f792f1e60fbe356742b2cb3d64))
* **plugins:** migrate privacy filter to extension host ([6cbd01b](https://github.com/dyndynjyxa/aio-coding-hub/commit/6cbd01b7651caf435c858ed7966f9589939e383e))
* **plugins:** remove declarative rule runtime dispatch ([e5a1c5d](https://github.com/dyndynjyxa/aio-coding-hub/commit/e5a1c5d124b56350da1f7a2231d7fc691b708f1f))
* **plugins:** remove declarative rules runtime ([8062b5e](https://github.com/dyndynjyxa/aio-coding-hub/commit/8062b5ea9c673b7e1412da96143ab1dfe77319ec))
* **plugins:** share runtime cache key helpers ([05e685a](https://github.com/dyndynjyxa/aio-coding-hub/commit/05e685a94bc3c7b7e77777161386945c07c4543d))
* **plugins:** split official privacy filter runtime ([3b8ed70](https://github.com/dyndynjyxa/aio-coding-hub/commit/3b8ed702a93d6bd7895dd60fa40a60fffeb33380))
* **plugins:** 提取并复用更新插件详情和概要的逻辑函数 ([31bafc1](https://github.com/dyndynjyxa/aio-coding-hub/commit/31bafc12e5a3e81b15ebcd1eb940681269a8f2d3))
* **providers:** 移除排序模板相关代码并简化启用开关逻辑 ([27258af](https://github.com/dyndynjyxa/aio-coding-hub/commit/27258af0c725f731a568e6b179c6b0faf9a5345b))
* resolve react doctor diagnostics ([a28dc41](https://github.com/dyndynjyxa/aio-coding-hub/commit/a28dc41582f6654852df71b6623c7715f49a93c7))

## [0.60.2](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.60.1...aio-coding-hub-v0.60.2) (2026-06-21)


### Features

* add Codex OAuth reset quota controls ([3bc184b](https://github.com/dyndynjyxa/aio-coding-hub/commit/3bc184b0da93f6edf212d0cfc4a1665c7ca000f8))


### Bug Fixes

* keep long-running request logs visible ([6c4e3aa](https://github.com/dyndynjyxa/aio-coding-hub/commit/6c4e3aa7a56b4c26a7c9bb78cbe590cc3439ce34))
* **providers:** delete request logs with usage cleanup ([a6b1705](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6b170509ca9e4010410c346b2bff076d2749044))


### Code Refactoring

* **claude-validation:** 移除Claude模型验证相关组件和实现 ([dfde520](https://github.com/dyndynjyxa/aio-coding-hub/commit/dfde520166d1d79f8961de8af730316a2c00fa73))
* **providers:** 删除 Provider 时支持清理用量统计 ([0393a75](https://github.com/dyndynjyxa/aio-coding-hub/commit/0393a758065f0107646205b869ce4638bed595e0))

## [0.60.1](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.60.0...aio-coding-hub-v0.60.1) (2026-06-15)


### Features

* **app:** 优化升级检测以支持GitHub发布内容回退 ([8fa895f](https://github.com/dyndynjyxa/aio-coding-hub/commit/8fa895fe8a9cb0cb6b40622c3fd770faeef9bc9d))


### Bug Fixes

* **providers:** enable provider card drag handle ([560db46](https://github.com/dyndynjyxa/aio-coding-hub/commit/560db46793c409418eb5d18b2f40ea3666febdb8))
* **release:** keep pre-1.0 features as patch releases ([1e9d19e](https://github.com/dyndynjyxa/aio-coding-hub/commit/1e9d19e0b72f0906a922a7d6c16437b66bac62dd))
* **release:** patch-bump pre-1.0 features ([519b8f5](https://github.com/dyndynjyxa/aio-coding-hub/commit/519b8f5b8fca0ad94c570743e22361823fe79ab6))
* **release:** prepare 0.60.1 patch ([97d59e6](https://github.com/dyndynjyxa/aio-coding-hub/commit/97d59e65ae63bca7c9946a5e3333f2ee7a631f32))

## [0.60.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.56.0...aio-coding-hub-v0.60.0) (2026-06-14)


### Features

* complete Claude/Codex unified plugin system ([#296](https://github.com/dyndynjyxa/aio-coding-hub/issues/296)) ([ae4734b](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae4734b6548f5e6808e587b4f6e36d6a9fba503b))
* **providers:** 支持 Codex 设备码 OAuth 登录 ([#279](https://github.com/dyndynjyxa/aio-coding-hub/issues/279)) ([16ac1b6](https://github.com/dyndynjyxa/aio-coding-hub/commit/16ac1b621d39e76b1b48d3044b89afcd22f36b8d))


### Bug Fixes

* **cli:** normalize CLI version comparison ([4590af0](https://github.com/dyndynjyxa/aio-coding-hub/commit/4590af00dfc39939e4ae162f7bdd6c84c935aff6)), closes [#289](https://github.com/dyndynjyxa/aio-coding-hub/issues/289)
* **skills:** 修复外来 managed skill 刷新识别 ([1ab7378](https://github.com/dyndynjyxa/aio-coding-hub/commit/1ab7378c8d66db0b3319f3a080cc14504ab99a16))
* 保留供应商编辑后的列表滚动位置 ([#291](https://github.com/dyndynjyxa/aio-coding-hub/issues/291)) ([42271ec](https://github.com/dyndynjyxa/aio-coding-hub/commit/42271ecef5be84db88545eb93ed5c683b14ebdcc))


### Styles

* **ui:** 统一对话框最大宽度及交互样式调整 ([94ea461](https://github.com/dyndynjyxa/aio-coding-hub/commit/94ea461bf032d448ca2616ff37cf988efb620e70))


### Miscellaneous

* **release:** set next release to 0.60.0 ([9346fe3](https://github.com/dyndynjyxa/aio-coding-hub/commit/9346fe3ea6c3d7cc3a8a9fa4522e2c9e0e32c7b8))

## [0.56.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.55.0...aio-coding-hub-v0.56.0) (2026-06-04)


### Features

* **providers:** 复制供应商时副本插入到源供应商下方 ([#249](https://github.com/dyndynjyxa/aio-coding-hub/issues/249)) ([5888a60](https://github.com/dyndynjyxa/aio-coding-hub/commit/5888a6009dab29dddec40dae6b44ecb62ae08083))


### Bug Fixes

* **skills:** list Windows linked local skills ([36296bd](https://github.com/dyndynjyxa/aio-coding-hub/commit/36296bd2b2a5990a5dae06e299e518bfb5531895))

## [0.55.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.54.0...aio-coding-hub-v0.55.0) (2026-06-03)


### Features

* **skills:** 支持按仓库单独刷新技能发现 ([83d112b](https://github.com/dyndynjyxa/aio-coding-hub/commit/83d112bd4b33ccbf23ff00bd99a5021651460561))


### Code Refactoring

* **ui:** 统一原子组件浮层圆角/阴影/焦点环/状态色 ([18ad29d](https://github.com/dyndynjyxa/aio-coding-hub/commit/18ad29d5c32c80ae67c9f4752d675b0ca9f5653f))
* **ui:** 补齐 Select/Textarea/TabList 焦点环一致性 ([be9b9bd](https://github.com/dyndynjyxa/aio-coding-hub/commit/be9b9bd6c391460c5b06af298bed650c50a6f481))


### Styles

* **layout:** 统一外壳节奏与 Sidebar 精修 ([bcfb24e](https://github.com/dyndynjyxa/aio-coding-hub/commit/bcfb24e3d38311f0eb039e6dca4858fc24749f53))
* **pages:** 统一页面垂直节奏与表单控件 token ([5ed53bf](https://github.com/dyndynjyxa/aio-coding-hub/commit/5ed53bfc4346e335d47608d3056ae0daaaf490d1))
* **skills:** 重构技能卡片布局并清理硬编码 ([e165343](https://github.com/dyndynjyxa/aio-coding-hub/commit/e1653438e1307917c5ee27d045e4e160fde24676))
* **tokens:** 新增阴影语义 token 并注册 display 字体 ([c9ee72a](https://github.com/dyndynjyxa/aio-coding-hub/commit/c9ee72a9d54eedc92cc01ca0887d5f6935e0b19a))

## [0.54.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.53.0...aio-coding-hub-v0.54.0) (2026-06-03)


### Features

* **config:** 优化release-please配置的changelog分区 ([69315b7](https://github.com/dyndynjyxa/aio-coding-hub/commit/69315b7543bacf590ed25392ed0fa6940307b681))


### Code Refactoring

* **proxy:** 移除内部请求标记相关代码并简化测试逻辑 ([5f16257](https://github.com/dyndynjyxa/aio-coding-hub/commit/5f1625707dd1df85de460463e2901864080eb7c6))

## [0.53.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.52.1...aio-coding-hub-v0.53.0) (2026-06-02)


### Features

* **proxy:** 本地拦截处理CX2CC Claude计数Token请求 ([9fa1201](https://github.com/dyndynjyxa/aio-coding-hub/commit/9fa12019f9e0c1a641f31419fa730cca6f813e47))


### Bug Fixes

* **gateway:** prioritize Claude model slots before thinking fallback ([95d5ece](https://github.com/dyndynjyxa/aio-coding-hub/commit/95d5ece7fe80e03b834b56e3e4cb394c7c952a8c))
* **scripts:** 修复pnpm audit命令参数以只检查生产依赖 ([d0bc737](https://github.com/dyndynjyxa/aio-coding-hub/commit/d0bc737cbb054aedac97c4a75c596de438364e8f))

## [0.52.1](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.52.0...aio-coding-hub-v0.52.1) (2026-05-28)


### Bug Fixes

* **windows:** hide native titlebar icon ([06f7d70](https://github.com/dyndynjyxa/aio-coding-hub/commit/06f7d70d951ec1333dbecd2d2091b362af15ac95))

## [0.52.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.51.1...aio-coding-hub-v0.52.0) (2026-05-28)


### Features

* **sidebar:** 在Windows运行环境中隐藏侧边栏Logo ([15b9bcf](https://github.com/dyndynjyxa/aio-coding-hub/commit/15b9bcfc18178871cc1c95369dec227e82a1ca93))

## [0.51.1](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.51.0...aio-coding-hub-v0.51.1) (2026-05-28)


### Bug Fixes

* **sidebar:** restore repair button accessibility ([ce38ce8](https://github.com/dyndynjyxa/aio-coding-hub/commit/ce38ce82cb189eefa66480657e2fea7bcf37b7e4))

## [0.51.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.50.0...aio-coding-hub-v0.51.0) (2026-05-27)


### Features

* **sidebar:** 优化侧边栏主题按钮样式和全局样式变量 ([fda23a8](https://github.com/dyndynjyxa/aio-coding-hub/commit/fda23a8f11c669c134f17931073bad2f501e9fe0))

## [0.50.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.49.0...aio-coding-hub-v0.50.0) (2026-05-27)


### ⚠ BREAKING CHANGES

* **infra:** request_attempt_logs 表不再创建或写入

### Features

* add Claude OAuth support with multi-account switching ([#149](https://github.com/dyndynjyxa/aio-coding-hub/issues/149)) ([ef787b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef787b1ddf309ffcf9ffcb923d92bf2af3f557b6))
* add CX2CC tab, config export/import, Gemini config enhance, CLI… ([#138](https://github.com/dyndynjyxa/aio-coding-hub/issues/138)) ([7aba381](https://github.com/dyndynjyxa/aio-coding-hub/commit/7aba381a462812e6c6ea505de3650397d23b48b4))
* add gemini oauth code assist proxy support ([4a892d0](https://github.com/dyndynjyxa/aio-coding-hub/commit/4a892d035fbcdd5273c909de063eede5d42598fa))
* **app:** 优化页面路由懒加载和加载反馈体验 ([504256e](https://github.com/dyndynjyxa/aio-coding-hub/commit/504256e11ed517797405f8bb4ab3d353607cb1ae))
* **app:** 添加应用心跳机制与监听支持 ([df78171](https://github.com/dyndynjyxa/aio-coding-hub/commit/df781712d16a2f5ccc99e37a687d39cb4e4f56c5))
* **app:** 集成应用启动状态管理和展示组件 ([2c7eb80](https://github.com/dyndynjyxa/aio-coding-hub/commit/2c7eb802e5317e12105eceed9533d3865b69b201))
* **bindings:** 添加 HomeUsagePeriod 类型契约断言 ([a540d52](https://github.com/dyndynjyxa/aio-coding-hub/commit/a540d52012652075deee18c58d3e4b5611f4d547))
* **build:** 添加Wayland兼容的AppImage构建支持 ([95c7130](https://github.com/dyndynjyxa/aio-coding-hub/commit/95c71307038962fc0e37fbd7b6df844301d8decb))
* **build:** 添加Windows便携版ZIP构建和WebView2检查 ([fef41e1](https://github.com/dyndynjyxa/aio-coding-hub/commit/fef41e12f2a1301743b40de4c7402f4b02fa80f4))
* **circuit-breaker:** 支持运行时动态更新断路器配置 ([3cb8722](https://github.com/dyndynjyxa/aio-coding-hub/commit/3cb87220345020ea8916f192ac6c84281d2736f2))
* **ClaudeModelValidationDialog:** 添加综合协议验证功能与UI展示 ([b6ede30](https://github.com/dyndynjyxa/aio-coding-hub/commit/b6ede30e61e19f9af69e8d7153cbc711b3e3a180))
* **claudeModelValidation:** 优化模型验证对话框和多轮验证流程支持 ([3f6c031](https://github.com/dyndynjyxa/aio-coding-hub/commit/3f6c03102db7aaeaa2d8399ebdac1cc4787c2a84))
* **cli_proxy:** 添加 merge-restore 功能以保留用户更改 ([a7d05c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/a7d05c97fe1228d83ccf97be5ed9b6ec26c7bde0))
* **cli_proxy:** 添加自动同步以修复代理配置漂移问题 ([0a7b856](https://github.com/dyndynjyxa/aio-coding-hub/commit/0a7b856e87d62fd3be84f6314a89f3f1a6706e92))
* **cli_sessions:** 优化 Claude-CLI 项目路径解析和工作目录提取 ([a0024ac](https://github.com/dyndynjyxa/aio-coding-hub/commit/a0024acf6c2e3bc1615d304d6ae681fbb4550371))
* **cli-manager:** add experimental agent teams setting and update related tests ([6e1dd0c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6e1dd0c82a19f7fa40728fa7fa6d7f23605dfab0))
* **cli-manager:** 在通用配置页新增上游代理相关设置支持 ([0b2489f](https://github.com/dyndynjyxa/aio-coding-hub/commit/0b2489f4c201af5daba697b4f272ca6de5767e54))
* **cli-manager:** 对齐 CCH 基础配置开关 ([55779ef](https://github.com/dyndynjyxa/aio-coding-hub/commit/55779efce6964a17b423869af77f3ebff567d62c))
* **cli-manager:** 支持 Claude Code 环境变量配置并修复网关重置缓存 ([#192](https://github.com/dyndynjyxa/aio-coding-hub/issues/192)) ([b41a2cf](https://github.com/dyndynjyxa/aio-coding-hub/commit/b41a2cfad6a964fe3f7dd9bd97a3db523bcc59b4))
* **cli-manager:** 支持自定义GPT-5.4模型上下文窗口和自动压缩限制 ([024dca7](https://github.com/dyndynjyxa/aio-coding-hub/commit/024dca78798022fef014a6a4c5b7a63eb6ecd663))
* **cli-sessions:** P0批次1 - 安全校验/React Query/Router state/虚拟化依赖 ([53c9d43](https://github.com/dyndynjyxa/aio-coding-hub/commit/53c9d4320566b03ff5e655ad2442d46a67f77572))
* **cli-sessions:** P0批次2 - 应用安全校验和状态管理重构 ([2dd098f](https://github.com/dyndynjyxa/aio-coding-hub/commit/2dd098fd39d0d45040b83a0cbd80cb13d4a29352))
* **cli-sessions:** P0批次3 - 三个列表虚拟化 ([d48fccd](https://github.com/dyndynjyxa/aio-coding-hub/commit/d48fccd775086bd11c6cf01147fedf0a3c44e86a))
* **cli-sessions:** P1批次 - 布局一致性和Shell转义安全 ([84723f3](https://github.com/dyndynjyxa/aio-coding-hub/commit/84723f377e5088d2f80574be84e6fa819038bb36))
* **cli:** 添加 Claude 终端启动命令生成及剪贴板管理插件支持 ([ba3d3c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba3d3c20d8d1d40accc0d97780c4dae54d48c71a))
* **codex:** add multi_agent feature toggle ([f56a201](https://github.com/dyndynjyxa/aio-coding-hub/commit/f56a2016bb535812e1a00edb0056e0faa6f44957))
* **codex:** support configurable Windows .codex locations ([#114](https://github.com/dyndynjyxa/aio-coding-hub/issues/114)) ([a8c77cd](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8c77cdebdca52f7bb5ca758837c2b04bac14e3f))
* **components:** enhance HomeRequestLogsPanel and RealtimeTraceCards with new formatting utilities ([e6a3550](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6a3550636e4278100a24d009e49ade1056733bc))
* **config:** 新增 personality 输出风格和 websocket 实验功能支持 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **config:** 新增计划模式推理强度配置项 ([6f5da87](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f5da87696af5c90054d6ca5be0f25e0a91eba40))
* **config:** 添加 GPT-5.4 关联配置及快速模式支持 ([38a11cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/38a11cb4c33300a2a7edb19848d97e25e966688b))
* **console:** 改进控制台界面并增加日志过滤和搜索功能 ([93badf8](https://github.com/dyndynjyxa/aio-coding-hub/commit/93badf8766c9a415c821c3f14084183d18d31687))
* **cx2cc:** add Codex-to-Claude-Code translation bridge ([#100](https://github.com/dyndynjyxa/aio-coding-hub/issues/100)) ([44d9c41](https://github.com/dyndynjyxa/aio-coding-hub/commit/44d9c416e434b72699e95984dbc156d413fc78dc))
* **deps:** 新增 react-markdown 和 tailwindcss 排版插件依赖 ([a58f747](https://github.com/dyndynjyxa/aio-coding-hub/commit/a58f7477ba4c35f4f0c5b4812d985c8294cf8cf3))
* **deps:** 添加 @mdxeditor/editor 依赖 ([8da488a](https://github.com/dyndynjyxa/aio-coding-hub/commit/8da488a5c0296b793eb184d0a081f09eda14af2e))
* **domain:** 优化按小时统计总 token 计算方式 ([8f3df70](https://github.com/dyndynjyxa/aio-coding-hub/commit/8f3df7039eb5b66d1780359e0ecbface88d3bec9))
* **domain:** 支持 cx2cc 提供者及缓存令牌追踪 ([3662c26](https://github.com/dyndynjyxa/aio-coding-hub/commit/3662c26c2ea72c25ef7e330a52a414a708164cd1))
* **gateway,providers,oauth:** codex-oauth-with-proxy ([#76](https://github.com/dyndynjyxa/aio-coding-hub/issues/76)) ([f37e59a](https://github.com/dyndynjyxa/aio-coding-hub/commit/f37e59a3dbfdc8a95d18e592f1931c775f19c2ab))
* **gateway:** Circuit Breaker 引入 5 分钟滑动窗口衰减与 HalfOpen 渐进恢复 ([f90e59e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f90e59e3e72b5ff889ce37c1e8e9a225f15e7de3))
* **gateway:** Session TTL 改为滑动窗口，每次使用自动续期 ([3a6cb5c](https://github.com/dyndynjyxa/aio-coding-hub/commit/3a6cb5c631c733133c96aa172be7e445d85444dd))
* **gateway:** 优化协议兼容性处理与请求头管理 ([6be9161](https://github.com/dyndynjyxa/aio-coding-hub/commit/6be91613fbc0bfcd9b87fe419a8f9ad08178a1f7))
* **gateway:** 增强上游错误处理与界面错误详情展示 ([85f67c4](https://github.com/dyndynjyxa/aio-coding-hub/commit/85f67c4c1ffe08d102883458a2d9d29e0b2b98cf))
* **gateway:** 实现熔断器半开状态与相关逻辑 ([9bd8146](https://github.com/dyndynjyxa/aio-coding-hub/commit/9bd81467541ceb40009dce68e052aeee44e82e36))
* **gateway:** 对齐 CCH 基础配置整流 ([a1c1b5c](https://github.com/dyndynjyxa/aio-coding-hub/commit/a1c1b5c2b730b89184d59fac207d252e9d415c93))
* **gateway:** 支持会话绑定提供者的查询与恢复 ([cbfb17e](https://github.com/dyndynjyxa/aio-coding-hub/commit/cbfb17e385f6ee4689148c8630f56bed0c281c95))
* **gateway:** 支持强制指定请求的Provider并更新Claude终端启动逻辑 ([904a9aa](https://github.com/dyndynjyxa/aio-coding-hub/commit/904a9aa6805681fb5642e350105803ce01ae998b))
* **gateway:** 新增计费头修正功能及提供流空闲超时配置能力 ([bbf3d29](https://github.com/dyndynjyxa/aio-coding-hub/commit/bbf3d2910d6e5c7cda76b3a11032eb117b709376))
* **gateway:** 添加决策链以丰富失败重试事件细节 ([262e98c](https://github.com/dyndynjyxa/aio-coding-hub/commit/262e98cb0ac974c5452e037ac51d5730625d1ef2))
* **home:** enhance status handling with failover support ([5aab8a9](https://github.com/dyndynjyxa/aio-coding-hub/commit/5aab8a92c4440062d18a3ecf495375d11a461660))
* **home:** 在主面板中添加 OAuth 配额标签页及相关刷新功能 ([fd8e57b](https://github.com/dyndynjyxa/aio-coding-hub/commit/fd8e57b820ad780140c4284a71ab31d9d25eac6e))
* **home:** 支持Codex优先服务层并展示fast徽章 ([bbdb79d](https://github.com/dyndynjyxa/aio-coding-hub/commit/bbdb79d8940473fb81847b60c238f78415a7f504))
* **home:** 添加供应商限额面板刷新按钮 ([18f4f91](https://github.com/dyndynjyxa/aio-coding-hub/commit/18f4f91d375f860312a126354a65efad6244e7ae))
* **home:** 添加工作区模式切换按钮并持久化请求日志简洁模式 ([d73bedd](https://github.com/dyndynjyxa/aio-coding-hub/commit/d73bedd3bc27d82375969d09fa94336a268bd075))
* implement batch 1-2 issues ([#19](https://github.com/dyndynjyxa/aio-coding-hub/issues/19), [#225](https://github.com/dyndynjyxa/aio-coding-hub/issues/225), [#226](https://github.com/dyndynjyxa/aio-coding-hub/issues/226), [#227](https://github.com/dyndynjyxa/aio-coding-hub/issues/227), [#228](https://github.com/dyndynjyxa/aio-coding-hub/issues/228), [#231](https://github.com/dyndynjyxa/aio-coding-hub/issues/231)) ([#232](https://github.com/dyndynjyxa/aio-coding-hub/issues/232)) ([83b9f6e](https://github.com/dyndynjyxa/aio-coding-hub/commit/83b9f6e788523bfd9411a21ad5beea10e478209f))
* mac最小化优化 ([1f11a5d](https://github.com/dyndynjyxa/aio-coding-hub/commit/1f11a5df9623f8cab90bb40f5c499c5d3569f01d))
* **mcp:** 优化服务器键生成与导入逻辑，支持保留大小写 ([f61a5c6](https://github.com/dyndynjyxa/aio-coding-hub/commit/f61a5c6be09795efaf9c4aafe7fee5f86329c0ea))
* **mcp:** 添加 env 和 header 键值对格式校验 ([7d698ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/7d698ca04f1cc9f9c0d420f6d7fec2a53f2d643a))
* **Oauth:** adding gemini Oauth support ([fd1eb79](https://github.com/dyndynjyxa/aio-coding-hub/commit/fd1eb7933410510e9ee46e469234d91658a49fcf))
* **provider-chain:** 优化供应商链路视图及日志详情显示 ([72d4bc4](https://github.com/dyndynjyxa/aio-coding-hub/commit/72d4bc49dfd1380ce503ec23a98eb81b55a1d488))
* **provider-chain:** 优化展示故障切换尝试详情和错误结构化信息 ([e2d530f](https://github.com/dyndynjyxa/aio-coding-hub/commit/e2d530fceb3fcf52e9a7745847b13963e854994c))
* **providers:** 在供应商列表页增加名称搜索功能 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **providers:** 增加供应商备注字段并支持读取API Key ([fec7057](https://github.com/dyndynjyxa/aio-coding-hub/commit/fec7057d137a7c0b38b46b5591a9e19e624b679a))
* **providers:** 实现供应商编辑器和多功能配置组件 ([e9f2a45](https://github.com/dyndynjyxa/aio-coding-hub/commit/e9f2a45f64b30c12b66ddad13cf378cccfd4d3a7))
* **providers:** 支持复制供应商配置并预填创建表单 ([c8fb707](https://github.com/dyndynjyxa/aio-coding-hub/commit/c8fb7078131ad67e4d2895f81467a9358b897d2c))
* **providers:** 添加 OAuth 限制重置倒计时 & 修复托盘退出重启问题 ([#168](https://github.com/dyndynjyxa/aio-coding-hub/issues/168)) ([4ca1248](https://github.com/dyndynjyxa/aio-coding-hub/commit/4ca124860c48fd3764ecf1da861adef072267ba7))
* **providers:** 添加供应商标签功能，支持分类筛选 ([7671732](https://github.com/dyndynjyxa/aio-coding-hub/commit/76717329a119f9a65dc2f3a9dffe0a887063b0bd))
* **proxy:** 丰富错误详情上下文并优化请求记录错误展示 ([1ac8eef](https://github.com/dyndynjyxa/aio-coding-hub/commit/1ac8eefbf6107de383ee349abb5ee0e89c774a6f))
* **proxy:** 优化 Codex 上游请求的 continuation ID 重试机制 ([7aad32c](https://github.com/dyndynjyxa/aio-coding-hub/commit/7aad32c63b19c13534ccb9deeac7ed28dc0031cb))
* **proxy:** 增加活动排序模式ID支持并改进失败重试逻辑 ([3383982](https://github.com/dyndynjyxa/aio-coding-hub/commit/33839827e36cf7af495174cd809d185c4144d0f1))
* **proxy:** 增加请求中断日志中尝试信息的捕获与展示 ([0ad0955](https://github.com/dyndynjyxa/aio-coding-hub/commit/0ad0955730da9ee46e1904838412fe546794e0f5))
* **proxy:** 实现对 Claude 请求的观察与请求日志生命周期管理 ([107d892](https://github.com/dyndynjyxa/aio-coding-hub/commit/107d89202defcd4bbb8727b15ec88bcf6bfe49e0))
* **proxy:** 支持 Codex 会话 ID 补全功能 ([25f8321](https://github.com/dyndynjyxa/aio-coding-hub/commit/25f8321536ccdfe9a68de1866bb9762794ac97b7))
* **route:** 支持展示同一 provider 的连续尝试次数及跳过状态 ([6659157](https://github.com/dyndynjyxa/aio-coding-hub/commit/6659157a12c3809b1e4d8de7bf95b95afe6ed383))
* **service:** 增加辅助函数 mapGeneratedCommandResponse ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **settings:** 新增cx2cc配置项并添加序列化支持 ([3afaa64](https://github.com/dyndynjyxa/aio-coding-hub/commit/3afaa64ca0ca7c505a83a9906218ba3167769cd0))
* **settings:** 新增任务结束提醒开关和相关功能 ([2fb4022](https://github.com/dyndynjyxa/aio-coding-hub/commit/2fb4022ff2f1078238ced25188960f19da188260))
* **settings:** 新增关闭 Claude Git 参与者功能 ([cf23f0d](https://github.com/dyndynjyxa/aio-coding-hub/commit/cf23f0dec000cff63f222b0198360485deb60804))
* **settings:** 新增缓存异常监测功能开关 ([ea681c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ea681c155346da3057f25e25867ab75a818f4157))
* **settings:** 新增静默启动配置支持 ([fcc436b](https://github.com/dyndynjyxa/aio-coding-hub/commit/fcc436b5dc5baf3b57e7160bd7e3562275535072))
* **settings:** 添加通知声音开关配置项 ([d13ee18](https://github.com/dyndynjyxa/aio-coding-hub/commit/d13ee18228cde971e7490468c6831aa679a636ab))
* **settings:** 添加通知音效开关及完善通用配置界面 ([e8d285a](https://github.com/dyndynjyxa/aio-coding-hub/commit/e8d285a704ba3f1a82fda9b16f9c2dbba393a57d))
* **skills:** support repo market and local CLI workflows ([3a0a24f](https://github.com/dyndynjyxa/aio-coding-hub/commit/3a0a24f13405d493237402dace175807b42de462))
* **skills:** 支持从本地源恢复缺失的ssot技能目录 ([4414f7d](https://github.com/dyndynjyxa/aio-coding-hub/commit/4414f7d38a44e4aff7de0730fe591b93146d7468))
* **skills:** 支持推理强度新增等级xhigh ([36dbb77](https://github.com/dyndynjyxa/aio-coding-hub/commit/36dbb773c94f7acaa9d1a9e6ed68d8df02d6c950))
* **skills:** 添加 Skill 返回本机已安装功能 ([84cfd0b](https://github.com/dyndynjyxa/aio-coding-hub/commit/84cfd0be09c5c38dbd9613533062a3daef6c57df))
* **taskCompleteNotifyEvents:** 改进任务完成通知的请求追踪与静默期控制 ([e3f03d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/e3f03d4bf2276db371e170ee4739cd379b6d3fe5))
* **tauri-domain:** 增强Provider可用性状态检测逻辑 ([478a3ec](https://github.com/dyndynjyxa/aio-coding-hub/commit/478a3ec910ed7a6e3c84fa4fa6f8a017912867a2))
* **tauri-infra-claude_hooks:** 支持合并已有Hook Group中的未知字段和Hook条目 ([478a3ec](https://github.com/dyndynjyxa/aio-coding-hub/commit/478a3ec910ed7a6e3c84fa4fa6f8a017912867a2))
* **theme:** add dark mode and native window theme sync ([588a373](https://github.com/dyndynjyxa/aio-coding-hub/commit/588a37311c456bd2182ac1676c8e9b309157353f))
* **theme:** 支持 Windows 系统主题跟随 ([#218](https://github.com/dyndynjyxa/aio-coding-hub/issues/218)) ([71b7b5d](https://github.com/dyndynjyxa/aio-coding-hub/commit/71b7b5df0310657d7919b8ba1b3c29ebcc37c70d))
* **ui:** 优化首页最近代理记录卡片展示与预览数据 ([#181](https://github.com/dyndynjyxa/aio-coding-hub/issues/181)) ([c5a0068](https://github.com/dyndynjyxa/aio-coding-hub/commit/c5a0068ac81e6d97fd9a021c346ee7f0024da42d))
* **ui:** 优化首页概览、CLI 代理状态与供应商页交互 ([#106](https://github.com/dyndynjyxa/aio-coding-hub/issues/106)) ([358e73f](https://github.com/dyndynjyxa/aio-coding-hub/commit/358e73f598fd48c7bbf131a3117e4801cec03ced))
* **ui:** 优化首页概览、供应商管理与设置页主题入口 ([#102](https://github.com/dyndynjyxa/aio-coding-hub/issues/102)) ([0adec0a](https://github.com/dyndynjyxa/aio-coding-hub/commit/0adec0af676842e9c5aee3e08e3a46d890d31c11))
* **ui:** 建立 shadcn 标准设计系统并完成 UI 层 token 迁移 ([194048f](https://github.com/dyndynjyxa/aio-coding-hub/commit/194048f98ecdfa982d284c303ad14ef2a0bdb0d2))
* **ui:** 本次主要优化了首页概览、设置页和代理记录页，同时顺手处理了首页包体和 Tauri 构建告警。 ([#111](https://github.com/dyndynjyxa/aio-coding-hub/issues/111)) ([4e6dbad](https://github.com/dyndynjyxa/aio-coding-hub/commit/4e6dbad14cab8334d47a3a30ebfcf70485ad2e0d))
* **ui:** 统一 Loading/Empty/Error 状态反馈组件 ([b27f64a](https://github.com/dyndynjyxa/aio-coding-hub/commit/b27f64a79a1ad655a37992527794759f9f8c5370))
* **ui:** 设置与首页展示优化，并修复 Node 24 下 pre-push 单测问题 ([#169](https://github.com/dyndynjyxa/aio-coding-hub/issues/169)) ([165db18](https://github.com/dyndynjyxa/aio-coding-hub/commit/165db18dce650cee175a8df07f9bc80168575aab))
* **ui:** 首页个性化布局新增今日供应商用量总览 ([#222](https://github.com/dyndynjyxa/aio-coding-hub/issues/222)) ([07e3f92](https://github.com/dyndynjyxa/aio-coding-hub/commit/07e3f929f1102c54162b1a70cc842e5f1acbb151))
* **ui:** 首页最近代理记录与配置信息展示优化 ([#185](https://github.com/dyndynjyxa/aio-coding-hub/issues/185)) ([84d98ef](https://github.com/dyndynjyxa/aio-coding-hub/commit/84d98ef1b0489bb64e83f577c355a4780fe07227))
* **update:** 更新对话框展示更新日志 ([#171](https://github.com/dyndynjyxa/aio-coding-hub/issues/171)) ([c55ccb2](https://github.com/dyndynjyxa/aio-coding-hub/commit/c55ccb2cb1fc18b1e109f41192a4362f3a4ff8af))
* **usage:** 新增供应商可用率时间线功能模块 ([8ab09b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/8ab09b1356d069dcbb7002c53a01038aa5a5a216))
* **usage:** 添加供应商过滤支持以改进使用统计查询 ([e2178d9](https://github.com/dyndynjyxa/aio-coding-hub/commit/e2178d93ae413307cd4bb5f830c30550a6e0af1d))
* **wsl:** 完善 WSL 会话浏览、MCP/提示词同步及删除功能，补充前端测试覆盖率 ([b1308a1](https://github.com/dyndynjyxa/aio-coding-hub/commit/b1308a1e7f3eaf7e871862f3119d7678be72c6aa))
* **wsl:** 支持WSL宿主机地址模式自动检测与自定义 ([f809e86](https://github.com/dyndynjyxa/aio-coding-hub/commit/f809e86ab3690214ba2407c14e4dee1febd9ab14))
* **wsl:** 支持浏览 WSL 环境中的 CLI 会话记录，增加wsl提示词和mcp同步，并有可见提示 ([6147df2](https://github.com/dyndynjyxa/aio-coding-hub/commit/6147df28f904798b7d8e02c5e5946ec3453187c6))
* **wsl:** 支持浏览 WSL 环境中的 CLI 会话记录，增加wsl提示词和mcp同步，并有可见提示 ([41d7d26](https://github.com/dyndynjyxa/aio-coding-hub/commit/41d7d2661a03a3dea90bcb63a06f384fde1ac31a))
* **wsl:** 添加 WSL 配置生命周期管理 — 退出恢复与崩溃自愈 ([bcbf54d](https://github.com/dyndynjyxa/aio-coding-hub/commit/bcbf54df5bf1e6b851f223ffad2a0ee135647c13))
* **wsl:** 添加启动时自动检测和配置 WSL 环境功能 ([707bd55](https://github.com/dyndynjyxa/aio-coding-hub/commit/707bd5518fd9881715ec5f3df0ef9250970570bb))
* 优化首页用量面板的日期详情、文件夹筛选和列表展示 ([#242](https://github.com/dyndynjyxa/aio-coding-hub/issues/242)) ([5218ec5](https://github.com/dyndynjyxa/aio-coding-hub/commit/5218ec5cb48210a40ac1ba36d405fd096f33f6b9))
* 展示 Claude 模型映射 ([#236](https://github.com/dyndynjyxa/aio-coding-hub/issues/236)) ([5554b46](https://github.com/dyndynjyxa/aio-coding-hub/commit/5554b468583f5fd287d9b4c31ea40cdfeb5ec36c))
* 支持 CX2CC 使用当前 AIO 服务 Codex 网关作为来源 ([#194](https://github.com/dyndynjyxa/aio-coding-hub/issues/194)) ([2bf7117](https://github.com/dyndynjyxa/aio-coding-hub/commit/2bf7117585f20e03971831a04bd721fb6f620d67))
* 首页新增 Token 用量面板 ([#204](https://github.com/dyndynjyxa/aio-coding-hub/issues/204)) ([4513106](https://github.com/dyndynjyxa/aio-coding-hub/commit/45131069f2d67a0c909c1558102305f1cfe41028))


### Bug Fixes

* **app:** 修复启动状态空值判断，改进Cx2cc选项卡状态管理 ([654eb78](https://github.com/dyndynjyxa/aio-coding-hub/commit/654eb7843fe48c3393d4119f89510de7462a0442))
* **app:** 修复并增强数据库重置和WSL配置功能 ([caf135a](https://github.com/dyndynjyxa/aio-coding-hub/commit/caf135a98905bcc4594f9f6c3729297e3c6dcf1c))
* **ci:** satisfy rust clippy checks ([38d0bc0](https://github.com/dyndynjyxa/aio-coding-hub/commit/38d0bc0081cc90b24bd83e4ced30566d861ea69b))
* **ci:** 优化 pre-push 钩子和测试命令的标准输入配置 ([d14abe2](https://github.com/dyndynjyxa/aio-coding-hub/commit/d14abe2ce2f35e53ea0ab750e8598aafb65f33ed))
* **ci:** 修复CI流程中正则表达式重复问题 ([609f2ea](https://github.com/dyndynjyxa/aio-coding-hub/commit/609f2ea657034a58b6f175b995e05431829f6dd9))
* **ci:** 修复release.yml中AppImage路径解析问题 ([1876bd4](https://github.com/dyndynjyxa/aio-coding-hub/commit/1876bd428f888e9c2eb286a20b9aeeb00dda5c5a))
* **ci:** 修复release工作流中appimagetool下载地址 ([67d8071](https://github.com/dyndynjyxa/aio-coding-hub/commit/67d8071fecf6dfc227f8e41d03f067ec481bfd4f))
* **ci:** 修复Rust测试线程数为单线程运行 ([87ec1fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/87ec1fe962d7b2913841e739e3ee7026f7145c36))
* **ci:** 修复正则表达式转义字符问题 ([ba805c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba805c3b2f147f153cbe39042b078547eb36da5a))
* **ci:** 修正发布流程中更新器 JSON 文件参数名称 ([9a4c0f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/9a4c0f063bd645e3286e67f1607334a84a5cb6fd))
* **claude-model-validation:** 优化界面中文提示文本 ([57651b8](https://github.com/dyndynjyxa/aio-coding-hub/commit/57651b8bafc32c782733c938b44c95a423216b79))
* **cli_manager:** 修复 run_in_login_shell 的平台兼容性处理 ([9f81433](https://github.com/dyndynjyxa/aio-coding-hub/commit/9f814337d08f36c08e37486f95b9c9da62880fc1))
* **cli_proxy:** 修复数据库初始化错误处理逻辑 ([9230475](https://github.com/dyndynjyxa/aio-coding-hub/commit/9230475e4401e93e3dc34ad285ace4a03186cd16))
* **cli_update:** 修复提取语义版本字符串的字符截取逻辑 ([782fb95](https://github.com/dyndynjyxa/aio-coding-hub/commit/782fb95d05236afbbfdf80b4bfb15921b9c34b88))
* **cli-manager:** 优化Hooks配置保存及编辑器行为 ([478a3ec](https://github.com/dyndynjyxa/aio-coding-hub/commit/478a3ec910ed7a6e3c84fa4fa6f8a017912867a2))
* **cli-manager:** 修正模型参数解析逻辑以避免默认值覆盖 ([ae2b7fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae2b7feecfd6e06462b8b0990204fc55d6ad3ccf))
* **cli-proxy:** 修复 Codex CLI 在 Windows 下的沙箱配置与认证模式 ([b79d072](https://github.com/dyndynjyxa/aio-coding-hub/commit/b79d072f39405c15125a8a6391bff122c9a5f315))
* **cli-sessions:** 修复SessionsPage Card容器flex布局 ([2d2a048](https://github.com/dyndynjyxa/aio-coding-hub/commit/2d2a0481f9811160223f9753d6edec5a28dcfc6c))
* **codex-tab:** sandbox_mode danger-full-access 选择不生效及高级配置不回显 ([3943401](https://github.com/dyndynjyxa/aio-coding-hub/commit/3943401524c8ef6a179e65e8ee62fd2ac4f784cb))
* **codex:** resolve oauth login failure and stale provider names ([#116](https://github.com/dyndynjyxa/aio-coding-hub/issues/116)) ([48d647e](https://github.com/dyndynjyxa/aio-coding-hub/commit/48d647ef429232562c9b56e88d8a37f57960604a))
* **codex:** 修复 Pi 本地 endpoint 与 Responses 兼容约束 ([#180](https://github.com/dyndynjyxa/aio-coding-hub/issues/180)) ([8e7085e](https://github.com/dyndynjyxa/aio-coding-hub/commit/8e7085e69d1aabca28d398190239e99bcab4fe03))
* **codex:** 修复切换 Codex Home 后代理状态与实际配置不一致 ([#125](https://github.com/dyndynjyxa/aio-coding-hub/issues/125)) ([e8f80b2](https://github.com/dyndynjyxa/aio-coding-hub/commit/e8f80b2838509b748d38ed6411219218b4077bf5))
* **commands:** 修复构建Claude启动命令时路径参数传递 ([9e714f4](https://github.com/dyndynjyxa/aio-coding-hub/commit/9e714f48bdbf7dff2d729131774518845303290a))
* **components:** 修复ClaudeModelValidationDialog点击事件阻塞问题 ([9bd5f53](https://github.com/dyndynjyxa/aio-coding-hub/commit/9bd5f53f38d01fc312b979e3620b5e0badaba585))
* **cost:** price CX2CC requests with translated model basis ([#175](https://github.com/dyndynjyxa/aio-coding-hub/issues/175)) ([342f585](https://github.com/dyndynjyxa/aio-coding-hub/commit/342f585c9b713077431e83ada0d008550fa5f708))
* **cost:** 修正成本乘数比较逻辑为包含零值 ([db69776](https://github.com/dyndynjyxa/aio-coding-hub/commit/db6977635f96fc55fa1b4d84d1d71e67dd8dd0b1))
* **db:** 修复providers表note字段迁移及数据处理问题 ([fa8ecda](https://github.com/dyndynjyxa/aio-coding-hub/commit/fa8ecda9bad231ed424c812c1c7cd3d0850b8565))
* **deps:** 升级 react-router-dom 7.11→7.13 修复 XSS 漏洞 ([c4db630](https://github.com/dyndynjyxa/aio-coding-hub/commit/c4db630b4478cab2f1180f03b324cfc0af9d42e3))
* **domain:** 修复 SSE 流错误处理逻辑 ([4446939](https://github.com/dyndynjyxa/aio-coding-hub/commit/4446939c457735727f96dce0b09c91ae6b616c1d))
* **domain:** 修正 claude_terminal_launch_context 函数中 provider_id 验证和查询逻辑 ([316d375](https://github.com/dyndynjyxa/aio-coding-hub/commit/316d3754fc628d0a1e435aabf469ca992cb63efa))
* **failover:** 区分网关过滤与上游请求失败 ([441b6f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/441b6f0d84565bdd21b8d52334aaedd60425ebef))
* gate OAuth quota snapshots ([0a6e53c](https://github.com/dyndynjyxa/aio-coding-hub/commit/0a6e53c67869b3b6713813807e533313534d830e))
* **gateway:** allow failover success handler arg count ([0c559cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/0c559cba801d59332a94eb650571487b6b18a53b))
* **gateway:** claude api-key auth fallback ([ac68780](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac687808a15cc8d412714d623e4a32ef10ece310))
* **gateway:** finalize failed requests and align error codes ([871de9c](https://github.com/dyndynjyxa/aio-coding-hub/commit/871de9c9c9e06ed6cb03026ceb21dfc4b0ba18d3))
* **gateway:** preserve provider order and fail over on quota exhaustion ([#255](https://github.com/dyndynjyxa/aio-coding-hub/issues/255)) ([13bba9b](https://github.com/dyndynjyxa/aio-coding-hub/commit/13bba9b047b1e7f2867cc0cbbbb8ba5e58a37ea6))
* **gateway:** 修复 CX2CC 无 SSE 响应头成功响应的分类时机 ([#160](https://github.com/dyndynjyxa/aio-coding-hub/issues/160)) ([9edcd5b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9edcd5bf8396a0a2a84e5b368d984717b4d7c81f))
* **gateway:** 修复 HALF_OPEN 状态下熔断未及时解除的前端展示问题 ([#166](https://github.com/dyndynjyxa/aio-coding-hub/issues/166)) ([4c809f9](https://github.com/dyndynjyxa/aio-coding-hub/commit/4c809f9b0b6aee4e426d8f3a6d2aa53699fd7b35))
* **gateway:** 修复代理初始化时未启用代理配置的问题 ([934aee9](https://github.com/dyndynjyxa/aio-coding-hub/commit/934aee94964226860ff40e885763685227af2535))
* **gateway:** 修正 Heartbeat 事件变量可见性及用法 ([a540d52](https://github.com/dyndynjyxa/aio-coding-hub/commit/a540d52012652075deee18c58d3e4b5611f4d547))
* **gateway:** 修正 provider 重新排序时清除会话绑定逻辑 ([f03c02e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f03c02e5332fe84496a029b2e5d130b558cfad31))
* **gateway:** 调整上游连接超时与错误切换逻辑 ([a6870ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6870ae14b6b5bb8edce814ca37da2000e84f6b4))
* **heartbeat_watchdog:** reload 返回 Ok 但实际异步失败时升级到窗口重建 ([#172](https://github.com/dyndynjyxa/aio-coding-hub/issues/172)) ([e4f77b0](https://github.com/dyndynjyxa/aio-coding-hub/commit/e4f77b01c7204767b61d8bb787a374b270383736))
* **heartbeat_watchdog:** 优化窗口重载的错误处理逻辑 ([6188e6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6188e6c29135c0126490882ceb54302d112ec79c))
* **home:** 修正 HomeRequestLogsPanel 多处测试数据的时间戳字段 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **hooks/useGatewayQuerySync:** 在invalidateUsageDerived中添加providerLimitUsageKeys缓存失效 ([4fee7e9](https://github.com/dyndynjyxa/aio-coding-hub/commit/4fee7e90b2492ed0bbb0150eaf7ab897c2fbd36b))
* **macOS:** 修复macOS 在 CLI 管理页面读取不到工具 version 并提示 INTERNAL_ERROR ([#126](https://github.com/dyndynjyxa/aio-coding-hub/issues/126)) ([ce35a94](https://github.com/dyndynjyxa/aio-coding-hub/commit/ce35a945dd278b453a1ddebbe8e95fd5f4f12ee6))
* **mcp:** 修复 MCP Server Dialog 支持 SSE 类型和保存逻辑 ([a219783](https://github.com/dyndynjyxa/aio-coding-hub/commit/a2197836c2aa9a431be0e2c8bc8b243dad2072fd))
* **mcp:** 修复导入服务器时基于 server_key 的重复处理 ([f6e6458](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6e64582e1e4c478ec014e61258611d971f8b3b4))
* **mcp:** 修正 McpServerDialog 组件中 patch 对象命名规范 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **mcp:** 修正测试用例中 McpServerDialog 的 preserveKeys 命名 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **notification:** 优化系统通知及心跳监测逻辑 ([cfeb63f](https://github.com/dyndynjyxa/aio-coding-hub/commit/cfeb63f6b331df3be3feddbf0aac258018dba824))
* **notification:** 修复 macOS 媒体键被通知音效抢占的问题 ([#251](https://github.com/dyndynjyxa/aio-coding-hub/issues/251)) ([831bcf8](https://github.com/dyndynjyxa/aio-coding-hub/commit/831bcf8146e7dd8f5c5ce221507d70c0875edbee))
* **oauth:** 调整 OAuth 刷新周期并移除 CLI 管理页 Claude OAuth 卡片 ([#184](https://github.com/dyndynjyxa/aio-coding-hub/issues/184)) ([3640ec7](https://github.com/dyndynjyxa/aio-coding-hub/commit/3640ec7853d81ba9dce8d8e4a049f0319f4827af))
* open sidebar repo link in system browser ([d3bd836](https://github.com/dyndynjyxa/aio-coding-hub/commit/d3bd83615b99c287b7f606eb75373fdc62d8c971))
* **pages:** 统一 SessionsMessagesPage 参数命名风格 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **pages:** 统一 SessionsProjectPage 参数命名风格 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **prompts:** 修改新增和编辑时 Prompt 保存按钮禁用条件 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **providers:** 敏感配置信息变更时清理运行时 session 绑定 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **provider:** 修正anthropic模型名称版本号 ([c668b7a](https://github.com/dyndynjyxa/aio-coding-hub/commit/c668b7a29c37081c117a06d51d38b3f0bcef68e1))
* **proxy:** 优化未匹配客户端错误的中止逻辑 ([05eb435](https://github.com/dyndynjyxa/aio-coding-hub/commit/05eb435e499786d8308f548d5115adf378adc7f6))
* **proxy:** 修改 failover_loop 逻辑中的匹配分支 ([2f8eff5](https://github.com/dyndynjyxa/aio-coding-hub/commit/2f8eff5fa4a4fc85f7bcd4f18d80a40d40f23ee8))
* **proxy:** 修正非匹配客户端错误的中止判定逻辑 ([56f37e7](https://github.com/dyndynjyxa/aio-coding-hub/commit/56f37e75fbf0e9266f031e9daabc5d215eea1593))
* **proxy:** 允许 resolve_session_bound_provider_id 函数有多个参数 ([705457e](https://github.com/dyndynjyxa/aio-coding-hub/commit/705457eac5092ccd98a0f3523ea1986c67dd9d9c))
* **query:** 规范 query 模块参数命名和传递 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* reduce macOS webview memory pressure ([273fe70](https://github.com/dyndynjyxa/aio-coding-hub/commit/273fe70bc3faab6590fcec0890ebf81361bbe64e))
* **release:** keep bindings exporter out of app bundle ([a6dc64f](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6dc64f363db4d2e9106a636b896f1c450394de6))
* **release:** keep Cargo.lock synced for release PRs ([c5bd423](https://github.com/dyndynjyxa/aio-coding-hub/commit/c5bd42355641cb8dcef13a1fa04d41795aeb5c82))
* repair CI lockfile and clippy ([d403671](https://github.com/dyndynjyxa/aio-coding-hub/commit/d4036715b46ef9a6b6237f8cce8d0839bc94873e))
* **router:** 修复侧边栏快速点击导航卡死问题 ([8b1f862](https://github.com/dyndynjyxa/aio-coding-hub/commit/8b1f862542b8bd94770ab5f937772a53a4bd1d22))
* **rust:** resolve clippy warnings for needless lifetimes and useless conversion ([b5f204f](https://github.com/dyndynjyxa/aio-coding-hub/commit/b5f204f1d309cc3e21f56427760f5969d1972e95))
* **scripts:** 修复生成绑定文件格式化问题 ([9ff697b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9ff697b6b8c93238f48518f27f274df33ba55a9a))
* **sessions:** 剥离替换字符并调整按钮样式 ([3586a16](https://github.com/dyndynjyxa/aio-coding-hub/commit/3586a165ceb8ba3afc42a34fdf4228c71b18cce5))
* **settings:** 保持完整快照防止默认值变更导致设置漂移 ([6a8626c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6a8626cdc93b06134279fe3d2020ecf1ce0d48f8))
* **settings:** 增强设置读取错误处理和写入保护 ([6759e60](https://github.com/dyndynjyxa/aio-coding-hub/commit/6759e60edf1cb93e63f8c05787ed4f0a320a175b))
* **settings:** 设置默认关闭Billing Header整流器 ([9331253](https://github.com/dyndynjyxa/aio-coding-hub/commit/933125358a9574225cd1f317fcdc9d6b1679c6c4))
* **settings:** 限制上游流式空闲超时最小值为60秒 ([e7284c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/e7284c251950bcfa27db72385603756472329646))
* **skills:** unify card layout between general and local skill sections ([7f3b7bb](https://github.com/dyndynjyxa/aio-coding-hub/commit/7f3b7bba9efe0818b11ae6ddbfa0dc1c1576f0fa))
* **skills:** 优化目录复制以正确处理符号链接 ([8b21265](https://github.com/dyndynjyxa/aio-coding-hub/commit/8b21265d116a184f82008b13792d472d68f13356))
* **skills:** 优化符号链接目录的移除逻辑 ([2ef9928](https://github.com/dyndynjyxa/aio-coding-hub/commit/2ef9928eb3d1d4fa23cb22e2035384e7be2a01f6))
* **skills:** 移除批量导入功能并改为刷新本机列表 ([45b9618](https://github.com/dyndynjyxa/aio-coding-hub/commit/45b961847b458f72f2bc062bdcdfef93c8722da8))
* suppress dead_code warnings on non-Windows CI and fix clippy lint ([8371047](https://github.com/dyndynjyxa/aio-coding-hub/commit/8371047c8a09fbf448f9aa11babc0fafe8b15f20))
* **tauri-infra-cli_proxy:** 防止Codex代理启用时部分写入配置 ([478a3ec](https://github.com/dyndynjyxa/aio-coding-hub/commit/478a3ec910ed7a6e3c84fa4fa6f8a017912867a2))
* **tauri:** 修复 Cargo.lock 同步及 CLI 代理切换后 MCP 同步问题 ([1854b00](https://github.com/dyndynjyxa/aio-coding-hub/commit/1854b00205b4b3a257b984b5a6c5271067b2545e))
* **tauri:** 修复 Windows 下 WSL 同步触发引用错误 ([696453d](https://github.com/dyndynjyxa/aio-coding-hub/commit/696453d0ab794902c20166d0ee2c5c5356911738))
* **tauri:** 修复可执行文件查找时的权限判断问题 ([e3815ce](https://github.com/dyndynjyxa/aio-coding-hub/commit/e3815cec86d2f3bd8635fd7f95de8e30b5f36fa9))
* **tauri:** 修复注册表键句柄初始化问题 ([6f468c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f468c9a692e1b7aa8bd343a8ce74ce07d7ad813))
* **tests:** update RealtimeTraceCards test to reflect token display format change ([f43633e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f43633e5381f44447bbf8aa64c1065d222687c59))
* **test:** 调整请求日志测试中 created_at_ms 赋值逻辑 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **theme:** 修复 dark mode 切换闪烁回退问题 ([ac9500c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac9500c04a108c5bc15921353b691394a689ac54))
* **ui:** 修复 Sidebar 组件状态文字显示异常问题 ([ff0b1ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/ff0b1ae74e8c08fa5c73044fa924e7c78e24284d))
* **update:** restore changelog and local preview flow ([#178](https://github.com/dyndynjyxa/aio-coding-hub/issues/178)) ([36a564d](https://github.com/dyndynjyxa/aio-coding-hub/commit/36a564dcaa33a6bd9c76a9bda30599a1d3f92c52))
* **usage:** 修复 Usage 统计中的 CX2CC 缓存命中率计算 ([#159](https://github.com/dyndynjyxa/aio-coding-hub/issues/159)) ([3472560](https://github.com/dyndynjyxa/aio-coding-hub/commit/3472560f62f88e4eae4b4d53e920c290869a2c7d))
* **utils:** 修正computeOutputTokensPerSecond回退逻辑 ([7c59559](https://github.com/dyndynjyxa/aio-coding-hub/commit/7c59559ad0b525e478577d74518173753530a980))
* **util:** 增加请求体大小限制至100MB并更新体积错误消息 ([b12c1ba](https://github.com/dyndynjyxa/aio-coding-hub/commit/b12c1baeccbfd29b8bcc4a055e1b6d7764d6d3dd))
* WebView2 不可恢复状态检测与分级恢复 ([#156](https://github.com/dyndynjyxa/aio-coding-hub/issues/156)) ([4869745](https://github.com/dyndynjyxa/aio-coding-hub/commit/4869745cb352279829317f18b5e1956c381b5b38))
* **workflows:** 修正release.yml中skip-github-pull-request的语法问题 ([ba32d2d](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba32d2d43660e94b1ac58ab1db664ac57b0c5e18))
* **wsl:** atomic write, remove dead fallback, add TOML comment ([7decfb5](https://github.com/dyndynjyxa/aio-coding-hub/commit/7decfb5803378ee3bfb5433bc88adb64562a6857))
* **wsl:** 修复 WSL 脚本错误消息编码和路径写入问题 ([bccd5c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/bccd5c93110641898e04cd1cde031df604af637b))
* **wsl:** 修复WSL适配器IPv4解析和状态脚本处理 ([06ae9b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/06ae9b1f462572550f327c3afe4034179b1da491))
* **wsl:** 修正wsl_auto_sync_core函数调用路径 ([cdb62a4](https://github.com/dyndynjyxa/aio-coding-hub/commit/cdb62a43d97d461696ebbc936c56f6982b3e0dae))
* **wsl:** 修正同步边界与 sessions 状态 ([cc41840](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc41840fb88744c926544436cf885b440f240403))
* 修复clippy redundant_pattern_matching警告 ([b0274a6](https://github.com/dyndynjyxa/aio-coding-hub/commit/b0274a663396a2958e470e2167933470bf34a70a))


### Performance Improvements

* **home:** 优化实时追踪卡片动画与样式过渡效果 ([48c9966](https://github.com/dyndynjyxa/aio-coding-hub/commit/48c99669d5658884bc594dcebdb35219d0c25e23))


### Code Refactoring

* **infra:** 移除 request_attempt_logs 独立表，改用 request_logs.attempts_json 派生 ([3fb2627](https://github.com/dyndynjyxa/aio-coding-hub/commit/3fb2627407e1b9b57758a8a40d3a7d7393278e1b))

## [0.41.2](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.41.1...aio-coding-hub-v0.41.2) (2026-05-22)


### Features

* **home:** 在主面板中添加 OAuth 配额标签页及相关刷新功能 ([fd8e57b](https://github.com/dyndynjyxa/aio-coding-hub/commit/fd8e57b820ad780140c4284a71ab31d9d25eac6e))


### Bug Fixes

* gate OAuth quota snapshots ([0a6e53c](https://github.com/dyndynjyxa/aio-coding-hub/commit/0a6e53c67869b3b6713813807e533313534d830e))

## [0.41.1](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.41.0...aio-coding-hub-v0.41.1) (2026-05-21)


### Bug Fixes

* open sidebar repo link in system browser ([d3bd836](https://github.com/dyndynjyxa/aio-coding-hub/commit/d3bd83615b99c287b7f606eb75373fdc62d8c971))

## [0.41.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.40.10...aio-coding-hub-v0.41.0) (2026-05-21)


### ⚠ BREAKING CHANGES

* **infra:** request_attempt_logs 表不再创建或写入

### Features

* add Claude OAuth support with multi-account switching ([#149](https://github.com/dyndynjyxa/aio-coding-hub/issues/149)) ([ef787b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef787b1ddf309ffcf9ffcb923d92bf2af3f557b6))
* add CX2CC tab, config export/import, Gemini config enhance, CLI… ([#138](https://github.com/dyndynjyxa/aio-coding-hub/issues/138)) ([7aba381](https://github.com/dyndynjyxa/aio-coding-hub/commit/7aba381a462812e6c6ea505de3650397d23b48b4))
* add gemini oauth code assist proxy support ([4a892d0](https://github.com/dyndynjyxa/aio-coding-hub/commit/4a892d035fbcdd5273c909de063eede5d42598fa))
* **app:** 优化页面路由懒加载和加载反馈体验 ([504256e](https://github.com/dyndynjyxa/aio-coding-hub/commit/504256e11ed517797405f8bb4ab3d353607cb1ae))
* **app:** 添加应用心跳机制与监听支持 ([df78171](https://github.com/dyndynjyxa/aio-coding-hub/commit/df781712d16a2f5ccc99e37a687d39cb4e4f56c5))
* **app:** 集成应用启动状态管理和展示组件 ([2c7eb80](https://github.com/dyndynjyxa/aio-coding-hub/commit/2c7eb802e5317e12105eceed9533d3865b69b201))
* **bindings:** 添加 HomeUsagePeriod 类型契约断言 ([a540d52](https://github.com/dyndynjyxa/aio-coding-hub/commit/a540d52012652075deee18c58d3e4b5611f4d547))
* **build:** 添加Wayland兼容的AppImage构建支持 ([95c7130](https://github.com/dyndynjyxa/aio-coding-hub/commit/95c71307038962fc0e37fbd7b6df844301d8decb))
* **build:** 添加Windows便携版ZIP构建和WebView2检查 ([fef41e1](https://github.com/dyndynjyxa/aio-coding-hub/commit/fef41e12f2a1301743b40de4c7402f4b02fa80f4))
* **circuit-breaker:** 支持运行时动态更新断路器配置 ([3cb8722](https://github.com/dyndynjyxa/aio-coding-hub/commit/3cb87220345020ea8916f192ac6c84281d2736f2))
* **ClaudeModelValidationDialog:** 添加综合协议验证功能与UI展示 ([b6ede30](https://github.com/dyndynjyxa/aio-coding-hub/commit/b6ede30e61e19f9af69e8d7153cbc711b3e3a180))
* **claudeModelValidation:** 优化模型验证对话框和多轮验证流程支持 ([3f6c031](https://github.com/dyndynjyxa/aio-coding-hub/commit/3f6c03102db7aaeaa2d8399ebdac1cc4787c2a84))
* **cli_proxy:** 添加 merge-restore 功能以保留用户更改 ([a7d05c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/a7d05c97fe1228d83ccf97be5ed9b6ec26c7bde0))
* **cli_proxy:** 添加自动同步以修复代理配置漂移问题 ([0a7b856](https://github.com/dyndynjyxa/aio-coding-hub/commit/0a7b856e87d62fd3be84f6314a89f3f1a6706e92))
* **cli_sessions:** 优化 Claude-CLI 项目路径解析和工作目录提取 ([a0024ac](https://github.com/dyndynjyxa/aio-coding-hub/commit/a0024acf6c2e3bc1615d304d6ae681fbb4550371))
* **cli-manager:** add experimental agent teams setting and update related tests ([6e1dd0c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6e1dd0c82a19f7fa40728fa7fa6d7f23605dfab0))
* **cli-manager:** 在通用配置页新增上游代理相关设置支持 ([0b2489f](https://github.com/dyndynjyxa/aio-coding-hub/commit/0b2489f4c201af5daba697b4f272ca6de5767e54))
* **cli-manager:** 对齐 CCH 基础配置开关 ([55779ef](https://github.com/dyndynjyxa/aio-coding-hub/commit/55779efce6964a17b423869af77f3ebff567d62c))
* **cli-manager:** 支持 Claude Code 环境变量配置并修复网关重置缓存 ([#192](https://github.com/dyndynjyxa/aio-coding-hub/issues/192)) ([b41a2cf](https://github.com/dyndynjyxa/aio-coding-hub/commit/b41a2cfad6a964fe3f7dd9bd97a3db523bcc59b4))
* **cli-manager:** 支持自定义GPT-5.4模型上下文窗口和自动压缩限制 ([024dca7](https://github.com/dyndynjyxa/aio-coding-hub/commit/024dca78798022fef014a6a4c5b7a63eb6ecd663))
* **cli-sessions:** P0批次1 - 安全校验/React Query/Router state/虚拟化依赖 ([53c9d43](https://github.com/dyndynjyxa/aio-coding-hub/commit/53c9d4320566b03ff5e655ad2442d46a67f77572))
* **cli-sessions:** P0批次2 - 应用安全校验和状态管理重构 ([2dd098f](https://github.com/dyndynjyxa/aio-coding-hub/commit/2dd098fd39d0d45040b83a0cbd80cb13d4a29352))
* **cli-sessions:** P0批次3 - 三个列表虚拟化 ([d48fccd](https://github.com/dyndynjyxa/aio-coding-hub/commit/d48fccd775086bd11c6cf01147fedf0a3c44e86a))
* **cli-sessions:** P1批次 - 布局一致性和Shell转义安全 ([84723f3](https://github.com/dyndynjyxa/aio-coding-hub/commit/84723f377e5088d2f80574be84e6fa819038bb36))
* **cli:** 添加 Claude 终端启动命令生成及剪贴板管理插件支持 ([ba3d3c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba3d3c20d8d1d40accc0d97780c4dae54d48c71a))
* **codex:** add multi_agent feature toggle ([f56a201](https://github.com/dyndynjyxa/aio-coding-hub/commit/f56a2016bb535812e1a00edb0056e0faa6f44957))
* **codex:** support configurable Windows .codex locations ([#114](https://github.com/dyndynjyxa/aio-coding-hub/issues/114)) ([a8c77cd](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8c77cdebdca52f7bb5ca758837c2b04bac14e3f))
* **components:** enhance HomeRequestLogsPanel and RealtimeTraceCards with new formatting utilities ([e6a3550](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6a3550636e4278100a24d009e49ade1056733bc))
* **config:** 新增 personality 输出风格和 websocket 实验功能支持 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **config:** 新增计划模式推理强度配置项 ([6f5da87](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f5da87696af5c90054d6ca5be0f25e0a91eba40))
* **config:** 添加 GPT-5.4 关联配置及快速模式支持 ([38a11cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/38a11cb4c33300a2a7edb19848d97e25e966688b))
* **console:** 改进控制台界面并增加日志过滤和搜索功能 ([93badf8](https://github.com/dyndynjyxa/aio-coding-hub/commit/93badf8766c9a415c821c3f14084183d18d31687))
* **core:** improve workspace imports and startup resilience ([1d7dbf0](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d7dbf0b73cde2b78a70be945c6fd8e248b30c1d))
* **cx2cc:** add Codex-to-Claude-Code translation bridge ([#100](https://github.com/dyndynjyxa/aio-coding-hub/issues/100)) ([44d9c41](https://github.com/dyndynjyxa/aio-coding-hub/commit/44d9c416e434b72699e95984dbc156d413fc78dc))
* **deps:** 新增 react-markdown 和 tailwindcss 排版插件依赖 ([a58f747](https://github.com/dyndynjyxa/aio-coding-hub/commit/a58f7477ba4c35f4f0c5b4812d985c8294cf8cf3))
* **deps:** 添加 @mdxeditor/editor 依赖 ([8da488a](https://github.com/dyndynjyxa/aio-coding-hub/commit/8da488a5c0296b793eb184d0a081f09eda14af2e))
* **domain:** 优化按小时统计总 token 计算方式 ([8f3df70](https://github.com/dyndynjyxa/aio-coding-hub/commit/8f3df7039eb5b66d1780359e0ecbface88d3bec9))
* **domain:** 支持 cx2cc 提供者及缓存令牌追踪 ([3662c26](https://github.com/dyndynjyxa/aio-coding-hub/commit/3662c26c2ea72c25ef7e330a52a414a708164cd1))
* **gateway,providers,oauth:** codex-oauth-with-proxy ([#76](https://github.com/dyndynjyxa/aio-coding-hub/issues/76)) ([f37e59a](https://github.com/dyndynjyxa/aio-coding-hub/commit/f37e59a3dbfdc8a95d18e592f1931c775f19c2ab))
* **gateway:** Circuit Breaker 引入 5 分钟滑动窗口衰减与 HalfOpen 渐进恢复 ([f90e59e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f90e59e3e72b5ff889ce37c1e8e9a225f15e7de3))
* **gateway:** Session TTL 改为滑动窗口，每次使用自动续期 ([3a6cb5c](https://github.com/dyndynjyxa/aio-coding-hub/commit/3a6cb5c631c733133c96aa172be7e445d85444dd))
* **gateway:** 优化协议兼容性处理与请求头管理 ([6be9161](https://github.com/dyndynjyxa/aio-coding-hub/commit/6be91613fbc0bfcd9b87fe419a8f9ad08178a1f7))
* **gateway:** 增强上游错误处理与界面错误详情展示 ([85f67c4](https://github.com/dyndynjyxa/aio-coding-hub/commit/85f67c4c1ffe08d102883458a2d9d29e0b2b98cf))
* **gateway:** 实现熔断器半开状态与相关逻辑 ([9bd8146](https://github.com/dyndynjyxa/aio-coding-hub/commit/9bd81467541ceb40009dce68e052aeee44e82e36))
* **gateway:** 对齐 CCH 基础配置整流 ([a1c1b5c](https://github.com/dyndynjyxa/aio-coding-hub/commit/a1c1b5c2b730b89184d59fac207d252e9d415c93))
* **gateway:** 支持会话绑定提供者的查询与恢复 ([cbfb17e](https://github.com/dyndynjyxa/aio-coding-hub/commit/cbfb17e385f6ee4689148c8630f56bed0c281c95))
* **gateway:** 支持强制指定请求的Provider并更新Claude终端启动逻辑 ([904a9aa](https://github.com/dyndynjyxa/aio-coding-hub/commit/904a9aa6805681fb5642e350105803ce01ae998b))
* **gateway:** 新增计费头修正功能及提供流空闲超时配置能力 ([bbf3d29](https://github.com/dyndynjyxa/aio-coding-hub/commit/bbf3d2910d6e5c7cda76b3a11032eb117b709376))
* **gateway:** 添加决策链以丰富失败重试事件细节 ([262e98c](https://github.com/dyndynjyxa/aio-coding-hub/commit/262e98cb0ac974c5452e037ac51d5730625d1ef2))
* **home:** enhance status handling with failover support ([5aab8a9](https://github.com/dyndynjyxa/aio-coding-hub/commit/5aab8a92c4440062d18a3ecf495375d11a461660))
* **home:** 支持Codex优先服务层并展示fast徽章 ([bbdb79d](https://github.com/dyndynjyxa/aio-coding-hub/commit/bbdb79d8940473fb81847b60c238f78415a7f504))
* **home:** 添加供应商限额面板刷新按钮 ([18f4f91](https://github.com/dyndynjyxa/aio-coding-hub/commit/18f4f91d375f860312a126354a65efad6244e7ae))
* **home:** 添加工作区模式切换按钮并持久化请求日志简洁模式 ([d73bedd](https://github.com/dyndynjyxa/aio-coding-hub/commit/d73bedd3bc27d82375969d09fa94336a268bd075))
* implement batch 1-2 issues ([#19](https://github.com/dyndynjyxa/aio-coding-hub/issues/19), [#225](https://github.com/dyndynjyxa/aio-coding-hub/issues/225), [#226](https://github.com/dyndynjyxa/aio-coding-hub/issues/226), [#227](https://github.com/dyndynjyxa/aio-coding-hub/issues/227), [#228](https://github.com/dyndynjyxa/aio-coding-hub/issues/228), [#231](https://github.com/dyndynjyxa/aio-coding-hub/issues/231)) ([#232](https://github.com/dyndynjyxa/aio-coding-hub/issues/232)) ([83b9f6e](https://github.com/dyndynjyxa/aio-coding-hub/commit/83b9f6e788523bfd9411a21ad5beea10e478209f))
* mac最小化优化 ([1f11a5d](https://github.com/dyndynjyxa/aio-coding-hub/commit/1f11a5df9623f8cab90bb40f5c499c5d3569f01d))
* **mcp:** implement batch import for local skills and add global error reporting ([51e053c](https://github.com/dyndynjyxa/aio-coding-hub/commit/51e053c81f1db00696f0cb968860033deee12cce))
* **mcp:** 优化服务器键生成与导入逻辑，支持保留大小写 ([f61a5c6](https://github.com/dyndynjyxa/aio-coding-hub/commit/f61a5c6be09795efaf9c4aafe7fee5f86329c0ea))
* **mcp:** 添加 env 和 header 键值对格式校验 ([7d698ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/7d698ca04f1cc9f9c0d420f6d7fec2a53f2d643a))
* **Oauth:** adding gemini Oauth support ([fd1eb79](https://github.com/dyndynjyxa/aio-coding-hub/commit/fd1eb7933410510e9ee46e469234d91658a49fcf))
* **provider-chain:** 优化供应商链路视图及日志详情显示 ([72d4bc4](https://github.com/dyndynjyxa/aio-coding-hub/commit/72d4bc49dfd1380ce503ec23a98eb81b55a1d488))
* **provider-chain:** 优化展示故障切换尝试详情和错误结构化信息 ([e2d530f](https://github.com/dyndynjyxa/aio-coding-hub/commit/e2d530fceb3fcf52e9a7745847b13963e854994c))
* **providers:** 在供应商列表页增加名称搜索功能 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **providers:** 增加供应商备注字段并支持读取API Key ([fec7057](https://github.com/dyndynjyxa/aio-coding-hub/commit/fec7057d137a7c0b38b46b5591a9e19e624b679a))
* **providers:** 实现供应商编辑器和多功能配置组件 ([e9f2a45](https://github.com/dyndynjyxa/aio-coding-hub/commit/e9f2a45f64b30c12b66ddad13cf378cccfd4d3a7))
* **providers:** 支持复制供应商配置并预填创建表单 ([c8fb707](https://github.com/dyndynjyxa/aio-coding-hub/commit/c8fb7078131ad67e4d2895f81467a9358b897d2c))
* **providers:** 添加 OAuth 限制重置倒计时 & 修复托盘退出重启问题 ([#168](https://github.com/dyndynjyxa/aio-coding-hub/issues/168)) ([4ca1248](https://github.com/dyndynjyxa/aio-coding-hub/commit/4ca124860c48fd3764ecf1da861adef072267ba7))
* **providers:** 添加供应商标签功能，支持分类筛选 ([7671732](https://github.com/dyndynjyxa/aio-coding-hub/commit/76717329a119f9a65dc2f3a9dffe0a887063b0bd))
* **proxy:** 丰富错误详情上下文并优化请求记录错误展示 ([1ac8eef](https://github.com/dyndynjyxa/aio-coding-hub/commit/1ac8eefbf6107de383ee349abb5ee0e89c774a6f))
* **proxy:** 优化 Codex 上游请求的 continuation ID 重试机制 ([7aad32c](https://github.com/dyndynjyxa/aio-coding-hub/commit/7aad32c63b19c13534ccb9deeac7ed28dc0031cb))
* **proxy:** 增加活动排序模式ID支持并改进失败重试逻辑 ([3383982](https://github.com/dyndynjyxa/aio-coding-hub/commit/33839827e36cf7af495174cd809d185c4144d0f1))
* **proxy:** 增加请求中断日志中尝试信息的捕获与展示 ([0ad0955](https://github.com/dyndynjyxa/aio-coding-hub/commit/0ad0955730da9ee46e1904838412fe546794e0f5))
* **proxy:** 实现对 Claude 请求的观察与请求日志生命周期管理 ([107d892](https://github.com/dyndynjyxa/aio-coding-hub/commit/107d89202defcd4bbb8727b15ec88bcf6bfe49e0))
* **proxy:** 支持 Codex 会话 ID 补全功能 ([25f8321](https://github.com/dyndynjyxa/aio-coding-hub/commit/25f8321536ccdfe9a68de1866bb9762794ac97b7))
* **route:** 支持展示同一 provider 的连续尝试次数及跳过状态 ([6659157](https://github.com/dyndynjyxa/aio-coding-hub/commit/6659157a12c3809b1e4d8de7bf95b95afe6ed383))
* **service:** 增加辅助函数 mapGeneratedCommandResponse ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **settings:** 新增cx2cc配置项并添加序列化支持 ([3afaa64](https://github.com/dyndynjyxa/aio-coding-hub/commit/3afaa64ca0ca7c505a83a9906218ba3167769cd0))
* **settings:** 新增任务结束提醒开关和相关功能 ([2fb4022](https://github.com/dyndynjyxa/aio-coding-hub/commit/2fb4022ff2f1078238ced25188960f19da188260))
* **settings:** 新增关闭 Claude Git 参与者功能 ([cf23f0d](https://github.com/dyndynjyxa/aio-coding-hub/commit/cf23f0dec000cff63f222b0198360485deb60804))
* **settings:** 新增缓存异常监测功能开关 ([ea681c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ea681c155346da3057f25e25867ab75a818f4157))
* **settings:** 新增静默启动配置支持 ([fcc436b](https://github.com/dyndynjyxa/aio-coding-hub/commit/fcc436b5dc5baf3b57e7160bd7e3562275535072))
* **settings:** 添加通知声音开关配置项 ([d13ee18](https://github.com/dyndynjyxa/aio-coding-hub/commit/d13ee18228cde971e7490468c6831aa679a636ab))
* **settings:** 添加通知音效开关及完善通用配置界面 ([e8d285a](https://github.com/dyndynjyxa/aio-coding-hub/commit/e8d285a704ba3f1a82fda9b16f9c2dbba393a57d))
* **skills:** support repo market and local CLI workflows ([3a0a24f](https://github.com/dyndynjyxa/aio-coding-hub/commit/3a0a24f13405d493237402dace175807b42de462))
* **skills:** 支持从本地源恢复缺失的ssot技能目录 ([4414f7d](https://github.com/dyndynjyxa/aio-coding-hub/commit/4414f7d38a44e4aff7de0730fe591b93146d7468))
* **skills:** 支持推理强度新增等级xhigh ([36dbb77](https://github.com/dyndynjyxa/aio-coding-hub/commit/36dbb773c94f7acaa9d1a9e6ed68d8df02d6c950))
* **skills:** 添加 Skill 返回本机已安装功能 ([84cfd0b](https://github.com/dyndynjyxa/aio-coding-hub/commit/84cfd0be09c5c38dbd9613533062a3daef6c57df))
* **taskCompleteNotifyEvents:** 改进任务完成通知的请求追踪与静默期控制 ([e3f03d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/e3f03d4bf2276db371e170ee4739cd379b6d3fe5))
* **tauri-domain:** 增强Provider可用性状态检测逻辑 ([478a3ec](https://github.com/dyndynjyxa/aio-coding-hub/commit/478a3ec910ed7a6e3c84fa4fa6f8a017912867a2))
* **tauri-infra-claude_hooks:** 支持合并已有Hook Group中的未知字段和Hook条目 ([478a3ec](https://github.com/dyndynjyxa/aio-coding-hub/commit/478a3ec910ed7a6e3c84fa4fa6f8a017912867a2))
* **theme:** add dark mode and native window theme sync ([588a373](https://github.com/dyndynjyxa/aio-coding-hub/commit/588a37311c456bd2182ac1676c8e9b309157353f))
* **theme:** 支持 Windows 系统主题跟随 ([#218](https://github.com/dyndynjyxa/aio-coding-hub/issues/218)) ([71b7b5d](https://github.com/dyndynjyxa/aio-coding-hub/commit/71b7b5df0310657d7919b8ba1b3c29ebcc37c70d))
* **ui:** 优化首页最近代理记录卡片展示与预览数据 ([#181](https://github.com/dyndynjyxa/aio-coding-hub/issues/181)) ([c5a0068](https://github.com/dyndynjyxa/aio-coding-hub/commit/c5a0068ac81e6d97fd9a021c346ee7f0024da42d))
* **ui:** 优化首页概览、CLI 代理状态与供应商页交互 ([#106](https://github.com/dyndynjyxa/aio-coding-hub/issues/106)) ([358e73f](https://github.com/dyndynjyxa/aio-coding-hub/commit/358e73f598fd48c7bbf131a3117e4801cec03ced))
* **ui:** 优化首页概览、供应商管理与设置页主题入口 ([#102](https://github.com/dyndynjyxa/aio-coding-hub/issues/102)) ([0adec0a](https://github.com/dyndynjyxa/aio-coding-hub/commit/0adec0af676842e9c5aee3e08e3a46d890d31c11))
* **ui:** 建立 shadcn 标准设计系统并完成 UI 层 token 迁移 ([194048f](https://github.com/dyndynjyxa/aio-coding-hub/commit/194048f98ecdfa982d284c303ad14ef2a0bdb0d2))
* **ui:** 本次主要优化了首页概览、设置页和代理记录页，同时顺手处理了首页包体和 Tauri 构建告警。 ([#111](https://github.com/dyndynjyxa/aio-coding-hub/issues/111)) ([4e6dbad](https://github.com/dyndynjyxa/aio-coding-hub/commit/4e6dbad14cab8334d47a3a30ebfcf70485ad2e0d))
* **ui:** 统一 Loading/Empty/Error 状态反馈组件 ([b27f64a](https://github.com/dyndynjyxa/aio-coding-hub/commit/b27f64a79a1ad655a37992527794759f9f8c5370))
* **ui:** 设置与首页展示优化，并修复 Node 24 下 pre-push 单测问题 ([#169](https://github.com/dyndynjyxa/aio-coding-hub/issues/169)) ([165db18](https://github.com/dyndynjyxa/aio-coding-hub/commit/165db18dce650cee175a8df07f9bc80168575aab))
* **ui:** 首页个性化布局新增今日供应商用量总览 ([#222](https://github.com/dyndynjyxa/aio-coding-hub/issues/222)) ([07e3f92](https://github.com/dyndynjyxa/aio-coding-hub/commit/07e3f929f1102c54162b1a70cc842e5f1acbb151))
* **ui:** 首页最近代理记录与配置信息展示优化 ([#185](https://github.com/dyndynjyxa/aio-coding-hub/issues/185)) ([84d98ef](https://github.com/dyndynjyxa/aio-coding-hub/commit/84d98ef1b0489bb64e83f577c355a4780fe07227))
* **update:** 更新对话框展示更新日志 ([#171](https://github.com/dyndynjyxa/aio-coding-hub/issues/171)) ([c55ccb2](https://github.com/dyndynjyxa/aio-coding-hub/commit/c55ccb2cb1fc18b1e109f41192a4362f3a4ff8af))
* **usage:** 新增供应商可用率时间线功能模块 ([8ab09b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/8ab09b1356d069dcbb7002c53a01038aa5a5a216))
* **usage:** 添加供应商过滤支持以改进使用统计查询 ([e2178d9](https://github.com/dyndynjyxa/aio-coding-hub/commit/e2178d93ae413307cd4bb5f830c30550a6e0af1d))
* **wsl:** 完善 WSL 会话浏览、MCP/提示词同步及删除功能，补充前端测试覆盖率 ([b1308a1](https://github.com/dyndynjyxa/aio-coding-hub/commit/b1308a1e7f3eaf7e871862f3119d7678be72c6aa))
* **wsl:** 支持WSL宿主机地址模式自动检测与自定义 ([f809e86](https://github.com/dyndynjyxa/aio-coding-hub/commit/f809e86ab3690214ba2407c14e4dee1febd9ab14))
* **wsl:** 支持浏览 WSL 环境中的 CLI 会话记录，增加wsl提示词和mcp同步，并有可见提示 ([6147df2](https://github.com/dyndynjyxa/aio-coding-hub/commit/6147df28f904798b7d8e02c5e5946ec3453187c6))
* **wsl:** 支持浏览 WSL 环境中的 CLI 会话记录，增加wsl提示词和mcp同步，并有可见提示 ([41d7d26](https://github.com/dyndynjyxa/aio-coding-hub/commit/41d7d2661a03a3dea90bcb63a06f384fde1ac31a))
* **wsl:** 添加 WSL 配置生命周期管理 — 退出恢复与崩溃自愈 ([bcbf54d](https://github.com/dyndynjyxa/aio-coding-hub/commit/bcbf54df5bf1e6b851f223ffad2a0ee135647c13))
* **wsl:** 添加启动时自动检测和配置 WSL 环境功能 ([707bd55](https://github.com/dyndynjyxa/aio-coding-hub/commit/707bd5518fd9881715ec5f3df0ef9250970570bb))
* 优化首页用量面板的日期详情、文件夹筛选和列表展示 ([#242](https://github.com/dyndynjyxa/aio-coding-hub/issues/242)) ([5218ec5](https://github.com/dyndynjyxa/aio-coding-hub/commit/5218ec5cb48210a40ac1ba36d405fd096f33f6b9))
* 展示 Claude 模型映射 ([#236](https://github.com/dyndynjyxa/aio-coding-hub/issues/236)) ([5554b46](https://github.com/dyndynjyxa/aio-coding-hub/commit/5554b468583f5fd287d9b4c31ea40cdfeb5ec36c))
* 支持 CX2CC 使用当前 AIO 服务 Codex 网关作为来源 ([#194](https://github.com/dyndynjyxa/aio-coding-hub/issues/194)) ([2bf7117](https://github.com/dyndynjyxa/aio-coding-hub/commit/2bf7117585f20e03971831a04bd721fb6f620d67))
* 首页新增 Token 用量面板 ([#204](https://github.com/dyndynjyxa/aio-coding-hub/issues/204)) ([4513106](https://github.com/dyndynjyxa/aio-coding-hub/commit/45131069f2d67a0c909c1558102305f1cfe41028))


### Bug Fixes

* **app:** 修复启动状态空值判断，改进Cx2cc选项卡状态管理 ([654eb78](https://github.com/dyndynjyxa/aio-coding-hub/commit/654eb7843fe48c3393d4119f89510de7462a0442))
* **app:** 修复并增强数据库重置和WSL配置功能 ([caf135a](https://github.com/dyndynjyxa/aio-coding-hub/commit/caf135a98905bcc4594f9f6c3729297e3c6dcf1c))
* **ci:** satisfy rust clippy checks ([38d0bc0](https://github.com/dyndynjyxa/aio-coding-hub/commit/38d0bc0081cc90b24bd83e4ced30566d861ea69b))
* **ci:** 优化 pre-push 钩子和测试命令的标准输入配置 ([d14abe2](https://github.com/dyndynjyxa/aio-coding-hub/commit/d14abe2ce2f35e53ea0ab750e8598aafb65f33ed))
* **ci:** 修复CI流程中正则表达式重复问题 ([609f2ea](https://github.com/dyndynjyxa/aio-coding-hub/commit/609f2ea657034a58b6f175b995e05431829f6dd9))
* **ci:** 修复release.yml中AppImage路径解析问题 ([1876bd4](https://github.com/dyndynjyxa/aio-coding-hub/commit/1876bd428f888e9c2eb286a20b9aeeb00dda5c5a))
* **ci:** 修复release工作流中appimagetool下载地址 ([67d8071](https://github.com/dyndynjyxa/aio-coding-hub/commit/67d8071fecf6dfc227f8e41d03f067ec481bfd4f))
* **ci:** 修复Rust测试线程数为单线程运行 ([87ec1fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/87ec1fe962d7b2913841e739e3ee7026f7145c36))
* **ci:** 修复正则表达式转义字符问题 ([ba805c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba805c3b2f147f153cbe39042b078547eb36da5a))
* **ci:** 修正发布流程中更新器 JSON 文件参数名称 ([9a4c0f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/9a4c0f063bd645e3286e67f1607334a84a5cb6fd))
* **claude-model-validation:** 优化界面中文提示文本 ([57651b8](https://github.com/dyndynjyxa/aio-coding-hub/commit/57651b8bafc32c782733c938b44c95a423216b79))
* **cli_manager:** 修复 run_in_login_shell 的平台兼容性处理 ([9f81433](https://github.com/dyndynjyxa/aio-coding-hub/commit/9f814337d08f36c08e37486f95b9c9da62880fc1))
* **cli_proxy:** 修复数据库初始化错误处理逻辑 ([9230475](https://github.com/dyndynjyxa/aio-coding-hub/commit/9230475e4401e93e3dc34ad285ace4a03186cd16))
* **cli_update:** 修复提取语义版本字符串的字符截取逻辑 ([782fb95](https://github.com/dyndynjyxa/aio-coding-hub/commit/782fb95d05236afbbfdf80b4bfb15921b9c34b88))
* **cli-manager:** 优化Hooks配置保存及编辑器行为 ([478a3ec](https://github.com/dyndynjyxa/aio-coding-hub/commit/478a3ec910ed7a6e3c84fa4fa6f8a017912867a2))
* **cli-manager:** 修正模型参数解析逻辑以避免默认值覆盖 ([ae2b7fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae2b7feecfd6e06462b8b0990204fc55d6ad3ccf))
* **cli-proxy:** 修复 Codex CLI 在 Windows 下的沙箱配置与认证模式 ([b79d072](https://github.com/dyndynjyxa/aio-coding-hub/commit/b79d072f39405c15125a8a6391bff122c9a5f315))
* **cli-sessions:** 修复SessionsPage Card容器flex布局 ([2d2a048](https://github.com/dyndynjyxa/aio-coding-hub/commit/2d2a0481f9811160223f9753d6edec5a28dcfc6c))
* **codex-tab:** sandbox_mode danger-full-access 选择不生效及高级配置不回显 ([3943401](https://github.com/dyndynjyxa/aio-coding-hub/commit/3943401524c8ef6a179e65e8ee62fd2ac4f784cb))
* **codex:** resolve oauth login failure and stale provider names ([#116](https://github.com/dyndynjyxa/aio-coding-hub/issues/116)) ([48d647e](https://github.com/dyndynjyxa/aio-coding-hub/commit/48d647ef429232562c9b56e88d8a37f57960604a))
* **codex:** 修复 Pi 本地 endpoint 与 Responses 兼容约束 ([#180](https://github.com/dyndynjyxa/aio-coding-hub/issues/180)) ([8e7085e](https://github.com/dyndynjyxa/aio-coding-hub/commit/8e7085e69d1aabca28d398190239e99bcab4fe03))
* **codex:** 修复切换 Codex Home 后代理状态与实际配置不一致 ([#125](https://github.com/dyndynjyxa/aio-coding-hub/issues/125)) ([e8f80b2](https://github.com/dyndynjyxa/aio-coding-hub/commit/e8f80b2838509b748d38ed6411219218b4077bf5))
* **commands:** 修复构建Claude启动命令时路径参数传递 ([9e714f4](https://github.com/dyndynjyxa/aio-coding-hub/commit/9e714f48bdbf7dff2d729131774518845303290a))
* **components:** 修复ClaudeModelValidationDialog点击事件阻塞问题 ([9bd5f53](https://github.com/dyndynjyxa/aio-coding-hub/commit/9bd5f53f38d01fc312b979e3620b5e0badaba585))
* **cost:** price CX2CC requests with translated model basis ([#175](https://github.com/dyndynjyxa/aio-coding-hub/issues/175)) ([342f585](https://github.com/dyndynjyxa/aio-coding-hub/commit/342f585c9b713077431e83ada0d008550fa5f708))
* **cost:** 修正成本乘数比较逻辑为包含零值 ([db69776](https://github.com/dyndynjyxa/aio-coding-hub/commit/db6977635f96fc55fa1b4d84d1d71e67dd8dd0b1))
* **db:** 修复providers表note字段迁移及数据处理问题 ([fa8ecda](https://github.com/dyndynjyxa/aio-coding-hub/commit/fa8ecda9bad231ed424c812c1c7cd3d0850b8565))
* **deps:** 升级 react-router-dom 7.11→7.13 修复 XSS 漏洞 ([c4db630](https://github.com/dyndynjyxa/aio-coding-hub/commit/c4db630b4478cab2f1180f03b324cfc0af9d42e3))
* **domain:** 修复 SSE 流错误处理逻辑 ([4446939](https://github.com/dyndynjyxa/aio-coding-hub/commit/4446939c457735727f96dce0b09c91ae6b616c1d))
* **domain:** 修正 claude_terminal_launch_context 函数中 provider_id 验证和查询逻辑 ([316d375](https://github.com/dyndynjyxa/aio-coding-hub/commit/316d3754fc628d0a1e435aabf469ca992cb63efa))
* **failover:** 区分网关过滤与上游请求失败 ([441b6f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/441b6f0d84565bdd21b8d52334aaedd60425ebef))
* **gateway:** allow failover success handler arg count ([0c559cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/0c559cba801d59332a94eb650571487b6b18a53b))
* **gateway:** claude api-key auth fallback ([ac68780](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac687808a15cc8d412714d623e4a32ef10ece310))
* **gateway:** finalize failed requests and align error codes ([871de9c](https://github.com/dyndynjyxa/aio-coding-hub/commit/871de9c9c9e06ed6cb03026ceb21dfc4b0ba18d3))
* **gateway:** preserve provider order and fail over on quota exhaustion ([#255](https://github.com/dyndynjyxa/aio-coding-hub/issues/255)) ([13bba9b](https://github.com/dyndynjyxa/aio-coding-hub/commit/13bba9b047b1e7f2867cc0cbbbb8ba5e58a37ea6))
* **gateway:** 修复 CX2CC 无 SSE 响应头成功响应的分类时机 ([#160](https://github.com/dyndynjyxa/aio-coding-hub/issues/160)) ([9edcd5b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9edcd5bf8396a0a2a84e5b368d984717b4d7c81f))
* **gateway:** 修复 HALF_OPEN 状态下熔断未及时解除的前端展示问题 ([#166](https://github.com/dyndynjyxa/aio-coding-hub/issues/166)) ([4c809f9](https://github.com/dyndynjyxa/aio-coding-hub/commit/4c809f9b0b6aee4e426d8f3a6d2aa53699fd7b35))
* **gateway:** 修复代理初始化时未启用代理配置的问题 ([934aee9](https://github.com/dyndynjyxa/aio-coding-hub/commit/934aee94964226860ff40e885763685227af2535))
* **gateway:** 修正 Heartbeat 事件变量可见性及用法 ([a540d52](https://github.com/dyndynjyxa/aio-coding-hub/commit/a540d52012652075deee18c58d3e4b5611f4d547))
* **gateway:** 修正 provider 重新排序时清除会话绑定逻辑 ([f03c02e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f03c02e5332fe84496a029b2e5d130b558cfad31))
* **gateway:** 调整上游连接超时与错误切换逻辑 ([a6870ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6870ae14b6b5bb8edce814ca37da2000e84f6b4))
* **heartbeat_watchdog:** reload 返回 Ok 但实际异步失败时升级到窗口重建 ([#172](https://github.com/dyndynjyxa/aio-coding-hub/issues/172)) ([e4f77b0](https://github.com/dyndynjyxa/aio-coding-hub/commit/e4f77b01c7204767b61d8bb787a374b270383736))
* **heartbeat_watchdog:** 优化窗口重载的错误处理逻辑 ([6188e6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6188e6c29135c0126490882ceb54302d112ec79c))
* **home:** 修正 HomeRequestLogsPanel 多处测试数据的时间戳字段 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **hooks/useGatewayQuerySync:** 在invalidateUsageDerived中添加providerLimitUsageKeys缓存失效 ([4fee7e9](https://github.com/dyndynjyxa/aio-coding-hub/commit/4fee7e90b2492ed0bbb0150eaf7ab897c2fbd36b))
* **macOS:** 修复macOS 在 CLI 管理页面读取不到工具 version 并提示 INTERNAL_ERROR ([#126](https://github.com/dyndynjyxa/aio-coding-hub/issues/126)) ([ce35a94](https://github.com/dyndynjyxa/aio-coding-hub/commit/ce35a945dd278b453a1ddebbe8e95fd5f4f12ee6))
* **mcp:** 修复 MCP Server Dialog 支持 SSE 类型和保存逻辑 ([a219783](https://github.com/dyndynjyxa/aio-coding-hub/commit/a2197836c2aa9a431be0e2c8bc8b243dad2072fd))
* **mcp:** 修复导入服务器时基于 server_key 的重复处理 ([f6e6458](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6e64582e1e4c478ec014e61258611d971f8b3b4))
* **mcp:** 修正 McpServerDialog 组件中 patch 对象命名规范 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **mcp:** 修正测试用例中 McpServerDialog 的 preserveKeys 命名 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **notification:** 优化系统通知及心跳监测逻辑 ([cfeb63f](https://github.com/dyndynjyxa/aio-coding-hub/commit/cfeb63f6b331df3be3feddbf0aac258018dba824))
* **notification:** 修复 macOS 媒体键被通知音效抢占的问题 ([#251](https://github.com/dyndynjyxa/aio-coding-hub/issues/251)) ([831bcf8](https://github.com/dyndynjyxa/aio-coding-hub/commit/831bcf8146e7dd8f5c5ce221507d70c0875edbee))
* **oauth:** 调整 OAuth 刷新周期并移除 CLI 管理页 Claude OAuth 卡片 ([#184](https://github.com/dyndynjyxa/aio-coding-hub/issues/184)) ([3640ec7](https://github.com/dyndynjyxa/aio-coding-hub/commit/3640ec7853d81ba9dce8d8e4a049f0319f4827af))
* **pages:** 统一 SessionsMessagesPage 参数命名风格 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **pages:** 统一 SessionsProjectPage 参数命名风格 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **prompts:** 修改新增和编辑时 Prompt 保存按钮禁用条件 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **providers:** 敏感配置信息变更时清理运行时 session 绑定 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **provider:** 修正anthropic模型名称版本号 ([c668b7a](https://github.com/dyndynjyxa/aio-coding-hub/commit/c668b7a29c37081c117a06d51d38b3f0bcef68e1))
* **proxy:** 优化未匹配客户端错误的中止逻辑 ([05eb435](https://github.com/dyndynjyxa/aio-coding-hub/commit/05eb435e499786d8308f548d5115adf378adc7f6))
* **proxy:** 修改 failover_loop 逻辑中的匹配分支 ([2f8eff5](https://github.com/dyndynjyxa/aio-coding-hub/commit/2f8eff5fa4a4fc85f7bcd4f18d80a40d40f23ee8))
* **proxy:** 修正非匹配客户端错误的中止判定逻辑 ([56f37e7](https://github.com/dyndynjyxa/aio-coding-hub/commit/56f37e75fbf0e9266f031e9daabc5d215eea1593))
* **proxy:** 允许 resolve_session_bound_provider_id 函数有多个参数 ([705457e](https://github.com/dyndynjyxa/aio-coding-hub/commit/705457eac5092ccd98a0f3523ea1986c67dd9d9c))
* **query:** 规范 query 模块参数命名和传递 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* reduce macOS webview memory pressure ([273fe70](https://github.com/dyndynjyxa/aio-coding-hub/commit/273fe70bc3faab6590fcec0890ebf81361bbe64e))
* **release:** keep bindings exporter out of app bundle ([a6dc64f](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6dc64f363db4d2e9106a636b896f1c450394de6))
* **release:** keep Cargo.lock synced for release PRs ([c5bd423](https://github.com/dyndynjyxa/aio-coding-hub/commit/c5bd42355641cb8dcef13a1fa04d41795aeb5c82))
* repair CI lockfile and clippy ([d403671](https://github.com/dyndynjyxa/aio-coding-hub/commit/d4036715b46ef9a6b6237f8cce8d0839bc94873e))
* **router:** 修复侧边栏快速点击导航卡死问题 ([8b1f862](https://github.com/dyndynjyxa/aio-coding-hub/commit/8b1f862542b8bd94770ab5f937772a53a4bd1d22))
* **rust:** resolve clippy warnings for needless lifetimes and useless conversion ([b5f204f](https://github.com/dyndynjyxa/aio-coding-hub/commit/b5f204f1d309cc3e21f56427760f5969d1972e95))
* **scripts:** 修复生成绑定文件格式化问题 ([9ff697b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9ff697b6b8c93238f48518f27f274df33ba55a9a))
* **sessions:** 剥离替换字符并调整按钮样式 ([3586a16](https://github.com/dyndynjyxa/aio-coding-hub/commit/3586a165ceb8ba3afc42a34fdf4228c71b18cce5))
* **settings:** 保持完整快照防止默认值变更导致设置漂移 ([6a8626c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6a8626cdc93b06134279fe3d2020ecf1ce0d48f8))
* **settings:** 增强设置读取错误处理和写入保护 ([6759e60](https://github.com/dyndynjyxa/aio-coding-hub/commit/6759e60edf1cb93e63f8c05787ed4f0a320a175b))
* **settings:** 设置默认关闭Billing Header整流器 ([9331253](https://github.com/dyndynjyxa/aio-coding-hub/commit/933125358a9574225cd1f317fcdc9d6b1679c6c4))
* **settings:** 限制上游流式空闲超时最小值为60秒 ([e7284c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/e7284c251950bcfa27db72385603756472329646))
* **skills:** unify card layout between general and local skill sections ([7f3b7bb](https://github.com/dyndynjyxa/aio-coding-hub/commit/7f3b7bba9efe0818b11ae6ddbfa0dc1c1576f0fa))
* **skills:** 优化目录复制以正确处理符号链接 ([8b21265](https://github.com/dyndynjyxa/aio-coding-hub/commit/8b21265d116a184f82008b13792d472d68f13356))
* **skills:** 优化符号链接目录的移除逻辑 ([2ef9928](https://github.com/dyndynjyxa/aio-coding-hub/commit/2ef9928eb3d1d4fa23cb22e2035384e7be2a01f6))
* **skills:** 移除批量导入功能并改为刷新本机列表 ([45b9618](https://github.com/dyndynjyxa/aio-coding-hub/commit/45b961847b458f72f2bc062bdcdfef93c8722da8))
* suppress dead_code warnings on non-Windows CI and fix clippy lint ([8371047](https://github.com/dyndynjyxa/aio-coding-hub/commit/8371047c8a09fbf448f9aa11babc0fafe8b15f20))
* **tauri-infra-cli_proxy:** 防止Codex代理启用时部分写入配置 ([478a3ec](https://github.com/dyndynjyxa/aio-coding-hub/commit/478a3ec910ed7a6e3c84fa4fa6f8a017912867a2))
* **tauri:** 修复 Cargo.lock 同步及 CLI 代理切换后 MCP 同步问题 ([1854b00](https://github.com/dyndynjyxa/aio-coding-hub/commit/1854b00205b4b3a257b984b5a6c5271067b2545e))
* **tauri:** 修复 Windows 下 WSL 同步触发引用错误 ([696453d](https://github.com/dyndynjyxa/aio-coding-hub/commit/696453d0ab794902c20166d0ee2c5c5356911738))
* **tauri:** 修复可执行文件查找时的权限判断问题 ([e3815ce](https://github.com/dyndynjyxa/aio-coding-hub/commit/e3815cec86d2f3bd8635fd7f95de8e30b5f36fa9))
* **tauri:** 修复注册表键句柄初始化问题 ([6f468c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f468c9a692e1b7aa8bd343a8ce74ce07d7ad813))
* **tests:** update RealtimeTraceCards test to reflect token display format change ([f43633e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f43633e5381f44447bbf8aa64c1065d222687c59))
* **test:** 调整请求日志测试中 created_at_ms 赋值逻辑 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **theme:** 修复 dark mode 切换闪烁回退问题 ([ac9500c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac9500c04a108c5bc15921353b691394a689ac54))
* **ui:** 修复 Sidebar 组件状态文字显示异常问题 ([ff0b1ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/ff0b1ae74e8c08fa5c73044fa924e7c78e24284d))
* **update:** restore changelog and local preview flow ([#178](https://github.com/dyndynjyxa/aio-coding-hub/issues/178)) ([36a564d](https://github.com/dyndynjyxa/aio-coding-hub/commit/36a564dcaa33a6bd9c76a9bda30599a1d3f92c52))
* **usage:** 修复 Usage 统计中的 CX2CC 缓存命中率计算 ([#159](https://github.com/dyndynjyxa/aio-coding-hub/issues/159)) ([3472560](https://github.com/dyndynjyxa/aio-coding-hub/commit/3472560f62f88e4eae4b4d53e920c290869a2c7d))
* **utils:** 修正computeOutputTokensPerSecond回退逻辑 ([7c59559](https://github.com/dyndynjyxa/aio-coding-hub/commit/7c59559ad0b525e478577d74518173753530a980))
* **util:** 增加请求体大小限制至100MB并更新体积错误消息 ([b12c1ba](https://github.com/dyndynjyxa/aio-coding-hub/commit/b12c1baeccbfd29b8bcc4a055e1b6d7764d6d3dd))
* WebView2 不可恢复状态检测与分级恢复 ([#156](https://github.com/dyndynjyxa/aio-coding-hub/issues/156)) ([4869745](https://github.com/dyndynjyxa/aio-coding-hub/commit/4869745cb352279829317f18b5e1956c381b5b38))
* **workflows:** 修正release.yml中skip-github-pull-request的语法问题 ([ba32d2d](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba32d2d43660e94b1ac58ab1db664ac57b0c5e18))
* **wsl:** atomic write, remove dead fallback, add TOML comment ([7decfb5](https://github.com/dyndynjyxa/aio-coding-hub/commit/7decfb5803378ee3bfb5433bc88adb64562a6857))
* **wsl:** 修复 WSL 脚本错误消息编码和路径写入问题 ([bccd5c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/bccd5c93110641898e04cd1cde031df604af637b))
* **wsl:** 修复WSL适配器IPv4解析和状态脚本处理 ([06ae9b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/06ae9b1f462572550f327c3afe4034179b1da491))
* **wsl:** 修正wsl_auto_sync_core函数调用路径 ([cdb62a4](https://github.com/dyndynjyxa/aio-coding-hub/commit/cdb62a43d97d461696ebbc936c56f6982b3e0dae))
* **wsl:** 修正同步边界与 sessions 状态 ([cc41840](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc41840fb88744c926544436cf885b440f240403))
* 修复clippy redundant_pattern_matching警告 ([b0274a6](https://github.com/dyndynjyxa/aio-coding-hub/commit/b0274a663396a2958e470e2167933470bf34a70a))


### Performance Improvements

* **home:** 优化实时追踪卡片动画与样式过渡效果 ([48c9966](https://github.com/dyndynjyxa/aio-coding-hub/commit/48c99669d5658884bc594dcebdb35219d0c25e23))


### Code Refactoring

* **infra:** 移除 request_attempt_logs 独立表，改用 request_logs.attempts_json 派生 ([3fb2627](https://github.com/dyndynjyxa/aio-coding-hub/commit/3fb2627407e1b9b57758a8a40d3a7d7393278e1b))

## [0.40.10](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.40.9...aio-coding-hub-v0.40.10) (2026-05-21)


### Bug Fixes

* **gateway:** preserve provider order and fail over on quota exhaustion ([#255](https://github.com/dyndynjyxa/aio-coding-hub/issues/255)) ([13bba9b](https://github.com/dyndynjyxa/aio-coding-hub/commit/13bba9b047b1e7f2867cc0cbbbb8ba5e58a37ea6))
* **notification:** 修复 macOS 媒体键被通知音效抢占的问题 ([#251](https://github.com/dyndynjyxa/aio-coding-hub/issues/251)) ([831bcf8](https://github.com/dyndynjyxa/aio-coding-hub/commit/831bcf8146e7dd8f5c5ce221507d70c0875edbee))
* reduce macOS webview memory pressure ([273fe70](https://github.com/dyndynjyxa/aio-coding-hub/commit/273fe70bc3faab6590fcec0890ebf81361bbe64e))

## [0.40.9](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.40.8...aio-coding-hub-v0.40.9) (2026-05-18)


### Features

* 优化首页用量面板的日期详情、文件夹筛选和列表展示 ([#242](https://github.com/dyndynjyxa/aio-coding-hub/issues/242)) ([5218ec5](https://github.com/dyndynjyxa/aio-coding-hub/commit/5218ec5cb48210a40ac1ba36d405fd096f33f6b9))


### Bug Fixes

* **proxy:** 修正非匹配客户端错误的中止判定逻辑 ([56f37e7](https://github.com/dyndynjyxa/aio-coding-hub/commit/56f37e75fbf0e9266f031e9daabc5d215eea1593))
* repair CI lockfile and clippy ([d403671](https://github.com/dyndynjyxa/aio-coding-hub/commit/d4036715b46ef9a6b6237f8cce8d0839bc94873e))

## [0.40.8](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.40.7...aio-coding-hub-v0.40.8) (2026-05-16)


### Features

* **proxy:** 增加活动排序模式ID支持并改进失败重试逻辑 ([3383982](https://github.com/dyndynjyxa/aio-coding-hub/commit/33839827e36cf7af495174cd809d185c4144d0f1))

## [0.40.7](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.40.6...aio-coding-hub-v0.40.7) (2026-05-09)


### Features

* 展示 Claude 模型映射 ([#236](https://github.com/dyndynjyxa/aio-coding-hub/issues/236)) ([5554b46](https://github.com/dyndynjyxa/aio-coding-hub/commit/5554b468583f5fd287d9b4c31ea40cdfeb5ec36c))

## [0.40.6](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.40.5...aio-coding-hub-v0.40.6) (2026-05-02)


### Features

* **usage:** 新增供应商可用率时间线功能模块 ([8ab09b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/8ab09b1356d069dcbb7002c53a01038aa5a5a216))

## [0.40.5](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.40.4...aio-coding-hub-v0.40.5) (2026-05-01)


### Features

* implement batch 1-2 issues ([#19](https://github.com/dyndynjyxa/aio-coding-hub/issues/19), [#225](https://github.com/dyndynjyxa/aio-coding-hub/issues/225), [#226](https://github.com/dyndynjyxa/aio-coding-hub/issues/226), [#227](https://github.com/dyndynjyxa/aio-coding-hub/issues/227), [#228](https://github.com/dyndynjyxa/aio-coding-hub/issues/228), [#231](https://github.com/dyndynjyxa/aio-coding-hub/issues/231)) ([#232](https://github.com/dyndynjyxa/aio-coding-hub/issues/232)) ([83b9f6e](https://github.com/dyndynjyxa/aio-coding-hub/commit/83b9f6e788523bfd9411a21ad5beea10e478209f))
* **tauri-domain:** 增强Provider可用性状态检测逻辑 ([478a3ec](https://github.com/dyndynjyxa/aio-coding-hub/commit/478a3ec910ed7a6e3c84fa4fa6f8a017912867a2))
* **tauri-infra-claude_hooks:** 支持合并已有Hook Group中的未知字段和Hook条目 ([478a3ec](https://github.com/dyndynjyxa/aio-coding-hub/commit/478a3ec910ed7a6e3c84fa4fa6f8a017912867a2))
* **ui:** 建立 shadcn 标准设计系统并完成 UI 层 token 迁移 ([194048f](https://github.com/dyndynjyxa/aio-coding-hub/commit/194048f98ecdfa982d284c303ad14ef2a0bdb0d2))


### Bug Fixes

* **cli-manager:** 优化Hooks配置保存及编辑器行为 ([478a3ec](https://github.com/dyndynjyxa/aio-coding-hub/commit/478a3ec910ed7a6e3c84fa4fa6f8a017912867a2))
* **provider:** 修正anthropic模型名称版本号 ([c668b7a](https://github.com/dyndynjyxa/aio-coding-hub/commit/c668b7a29c37081c117a06d51d38b3f0bcef68e1))
* **tauri-infra-cli_proxy:** 防止Codex代理启用时部分写入配置 ([478a3ec](https://github.com/dyndynjyxa/aio-coding-hub/commit/478a3ec910ed7a6e3c84fa4fa6f8a017912867a2))

## [0.40.4](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.40.3...aio-coding-hub-v0.40.4) (2026-04-28)


### Features

* **proxy:** 优化 Codex 上游请求的 continuation ID 重试机制 ([7aad32c](https://github.com/dyndynjyxa/aio-coding-hub/commit/7aad32c63b19c13534ccb9deeac7ed28dc0031cb))

## [0.40.3](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.40.2...aio-coding-hub-v0.40.3) (2026-04-27)


### Features

* **ui:** 首页个性化布局新增今日供应商用量总览 ([#222](https://github.com/dyndynjyxa/aio-coding-hub/issues/222)) ([07e3f92](https://github.com/dyndynjyxa/aio-coding-hub/commit/07e3f929f1102c54162b1a70cc842e5f1acbb151))

## [0.40.2](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.40.1...aio-coding-hub-v0.40.2) (2026-04-24)


### Features

* **theme:** 支持 Windows 系统主题跟随 ([#218](https://github.com/dyndynjyxa/aio-coding-hub/issues/218)) ([71b7b5d](https://github.com/dyndynjyxa/aio-coding-hub/commit/71b7b5df0310657d7919b8ba1b3c29ebcc37c70d))


### Bug Fixes

* **workflows:** 修正release.yml中skip-github-pull-request的语法问题 ([ba32d2d](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba32d2d43660e94b1ac58ab1db664ac57b0c5e18))

## [0.40.1](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.40.0...aio-coding-hub-v0.40.1) (2026-04-24)


### Features

* **home:** 支持Codex优先服务层并展示fast徽章 ([bbdb79d](https://github.com/dyndynjyxa/aio-coding-hub/commit/bbdb79d8940473fb81847b60c238f78415a7f504))
* **service:** 增加辅助函数 mapGeneratedCommandResponse ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))


### Bug Fixes

* **home:** 修正 HomeRequestLogsPanel 多处测试数据的时间戳字段 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **hooks/useGatewayQuerySync:** 在invalidateUsageDerived中添加providerLimitUsageKeys缓存失效 ([4fee7e9](https://github.com/dyndynjyxa/aio-coding-hub/commit/4fee7e90b2492ed0bbb0150eaf7ab897c2fbd36b))
* **mcp:** 修正 McpServerDialog 组件中 patch 对象命名规范 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **mcp:** 修正测试用例中 McpServerDialog 的 preserveKeys 命名 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **pages:** 统一 SessionsMessagesPage 参数命名风格 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **pages:** 统一 SessionsProjectPage 参数命名风格 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **proxy:** 修改 failover_loop 逻辑中的匹配分支 ([2f8eff5](https://github.com/dyndynjyxa/aio-coding-hub/commit/2f8eff5fa4a4fc85f7bcd4f18d80a40d40f23ee8))
* **query:** 规范 query 模块参数命名和传递 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))
* **test:** 调整请求日志测试中 created_at_ms 赋值逻辑 ([b8e14eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/b8e14ebc0ae8449e1fcebe939d1c95b67c033d4f))

## [0.40.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.39.3...aio-coding-hub-v0.40.0) (2026-04-21)


### ⚠ BREAKING CHANGES

* **infra:** request_attempt_logs 表不再创建或写入

### Features

* add Claude OAuth support with multi-account switching ([#149](https://github.com/dyndynjyxa/aio-coding-hub/issues/149)) ([ef787b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef787b1ddf309ffcf9ffcb923d92bf2af3f557b6))
* add CX2CC tab, config export/import, Gemini config enhance, CLI… ([#138](https://github.com/dyndynjyxa/aio-coding-hub/issues/138)) ([7aba381](https://github.com/dyndynjyxa/aio-coding-hub/commit/7aba381a462812e6c6ea505de3650397d23b48b4))
* add gemini oauth code assist proxy support ([4a892d0](https://github.com/dyndynjyxa/aio-coding-hub/commit/4a892d035fbcdd5273c909de063eede5d42598fa))
* **app:** add LogsPage route and enhance CLI manager settings ([6796715](https://github.com/dyndynjyxa/aio-coding-hub/commit/679671526be935129211978b3598a3962f7e2a8e))
* **app:** 优化页面路由懒加载和加载反馈体验 ([504256e](https://github.com/dyndynjyxa/aio-coding-hub/commit/504256e11ed517797405f8bb4ab3d353607cb1ae))
* **app:** 添加应用心跳机制与监听支持 ([df78171](https://github.com/dyndynjyxa/aio-coding-hub/commit/df781712d16a2f5ccc99e37a687d39cb4e4f56c5))
* **app:** 集成应用启动状态管理和展示组件 ([2c7eb80](https://github.com/dyndynjyxa/aio-coding-hub/commit/2c7eb802e5317e12105eceed9533d3865b69b201))
* **bindings:** 添加 HomeUsagePeriod 类型契约断言 ([a540d52](https://github.com/dyndynjyxa/aio-coding-hub/commit/a540d52012652075deee18c58d3e4b5611f4d547))
* **build:** 添加Wayland兼容的AppImage构建支持 ([95c7130](https://github.com/dyndynjyxa/aio-coding-hub/commit/95c71307038962fc0e37fbd7b6df844301d8decb))
* **build:** 添加Windows便携版ZIP构建和WebView2检查 ([fef41e1](https://github.com/dyndynjyxa/aio-coding-hub/commit/fef41e12f2a1301743b40de4c7402f4b02fa80f4))
* **cache-anomaly-monitor:** implement cache anomaly monitoring feature ([59d69d1](https://github.com/dyndynjyxa/aio-coding-hub/commit/59d69d1528d888a84d71038873321f3273d4ded1))
* **charts:** migrate from ECharts to Recharts for improved charting capabilities ([b747b61](https://github.com/dyndynjyxa/aio-coding-hub/commit/b747b61a8dd587043a7e8de01a5f9b3d64ead7ae))
* **circuit-breaker:** 支持运行时动态更新断路器配置 ([3cb8722](https://github.com/dyndynjyxa/aio-coding-hub/commit/3cb87220345020ea8916f192ac6c84281d2736f2))
* **ClaudeModelValidationDialog:** 添加综合协议验证功能与UI展示 ([b6ede30](https://github.com/dyndynjyxa/aio-coding-hub/commit/b6ede30e61e19f9af69e8d7153cbc711b3e3a180))
* **claudeModelValidation:** 优化模型验证对话框和多轮验证流程支持 ([3f6c031](https://github.com/dyndynjyxa/aio-coding-hub/commit/3f6c03102db7aaeaa2d8399ebdac1cc4787c2a84))
* **cli_proxy:** 添加 merge-restore 功能以保留用户更改 ([a7d05c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/a7d05c97fe1228d83ccf97be5ed9b6ec26c7bde0))
* **cli_proxy:** 添加自动同步以修复代理配置漂移问题 ([0a7b856](https://github.com/dyndynjyxa/aio-coding-hub/commit/0a7b856e87d62fd3be84f6314a89f3f1a6706e92))
* **cli_sessions:** 优化 Claude-CLI 项目路径解析和工作目录提取 ([a0024ac](https://github.com/dyndynjyxa/aio-coding-hub/commit/a0024acf6c2e3bc1615d304d6ae681fbb4550371))
* **cli-manager:** add collaboration features to CodexTab ([f6ab4e4](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6ab4e48ebdc5cdafa132e1513c25be01964b81d))
* **cli-manager:** add experimental agent teams setting and update related tests ([6e1dd0c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6e1dd0c82a19f7fa40728fa7fa6d7f23605dfab0))
* **cli-manager:** add experimental MCP CLI features and max output tokens management ([84ac464](https://github.com/dyndynjyxa/aio-coding-hub/commit/84ac4645e7f4eb1e6235b85d7ca63f16b04c8ee6))
* **cli-manager:** add TOML configuration support for Codex ([826737a](https://github.com/dyndynjyxa/aio-coding-hub/commit/826737a89b74076d5bd90d960e876fb86a12e1cd))
* **cli-manager:** enhance Claude and WSL settings management ([3aacb6a](https://github.com/dyndynjyxa/aio-coding-hub/commit/3aacb6ad4a05b238a80064529445de398b1978b5))
* **cli-manager:** enhance CodexTab with sandbox mode management ([759c19d](https://github.com/dyndynjyxa/aio-coding-hub/commit/759c19d02553f929f5a91c6051a0134e198e298e))
* **cli-manager:** enhance WSL settings and improve CLI manager performance ([be50456](https://github.com/dyndynjyxa/aio-coding-hub/commit/be50456432ef1f00cf1567c664a168779a23fc58))
* **cli-manager:** update GeneralTab and cache anomaly monitoring logic ([c84a1eb](https://github.com/dyndynjyxa/aio-coding-hub/commit/c84a1eb6c312c938ba067551e7b3303bba761edf))
* **cli-manager:** 在通用配置页新增上游代理相关设置支持 ([0b2489f](https://github.com/dyndynjyxa/aio-coding-hub/commit/0b2489f4c201af5daba697b4f272ca6de5767e54))
* **cli-manager:** 对齐 CCH 基础配置开关 ([55779ef](https://github.com/dyndynjyxa/aio-coding-hub/commit/55779efce6964a17b423869af77f3ebff567d62c))
* **cli-manager:** 支持 Claude Code 环境变量配置并修复网关重置缓存 ([#192](https://github.com/dyndynjyxa/aio-coding-hub/issues/192)) ([b41a2cf](https://github.com/dyndynjyxa/aio-coding-hub/commit/b41a2cfad6a964fe3f7dd9bd97a3db523bcc59b4))
* **cli-manager:** 支持自定义GPT-5.4模型上下文窗口和自动压缩限制 ([024dca7](https://github.com/dyndynjyxa/aio-coding-hub/commit/024dca78798022fef014a6a4c5b7a63eb6ecd663))
* **cli-sessions:** P0批次1 - 安全校验/React Query/Router state/虚拟化依赖 ([53c9d43](https://github.com/dyndynjyxa/aio-coding-hub/commit/53c9d4320566b03ff5e655ad2442d46a67f77572))
* **cli-sessions:** P0批次2 - 应用安全校验和状态管理重构 ([2dd098f](https://github.com/dyndynjyxa/aio-coding-hub/commit/2dd098fd39d0d45040b83a0cbd80cb13d4a29352))
* **cli-sessions:** P0批次3 - 三个列表虚拟化 ([d48fccd](https://github.com/dyndynjyxa/aio-coding-hub/commit/d48fccd775086bd11c6cf01147fedf0a3c44e86a))
* **cli-sessions:** P1批次 - 布局一致性和Shell转义安全 ([84723f3](https://github.com/dyndynjyxa/aio-coding-hub/commit/84723f377e5088d2f80574be84e6fa819038bb36))
* **cli:** add CLI proxy startup recovery feature ([eb40a6f](https://github.com/dyndynjyxa/aio-coding-hub/commit/eb40a6f03d9a3b50b501795727a06b7ce9013fc1))
* **cli:** 添加 Claude 终端启动命令生成及剪贴板管理插件支持 ([ba3d3c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba3d3c20d8d1d40accc0d97780c4dae54d48c71a))
* **codex:** add multi_agent feature toggle ([f56a201](https://github.com/dyndynjyxa/aio-coding-hub/commit/f56a2016bb535812e1a00edb0056e0faa6f44957))
* **codex:** support configurable Windows .codex locations ([#114](https://github.com/dyndynjyxa/aio-coding-hub/issues/114)) ([a8c77cd](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8c77cdebdca52f7bb5ca758837c2b04bac14e3f))
* **components:** enhance HomeRequestLogsPanel and RealtimeTraceCards with new formatting utilities ([e6a3550](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6a3550636e4278100a24d009e49ade1056733bc))
* **config:** 新增 personality 输出风格和 websocket 实验功能支持 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **config:** 新增计划模式推理强度配置项 ([6f5da87](https://github.com/dyndynjyxa/aio-coding-hub/commit/6f5da87696af5c90054d6ca5be0f25e0a91eba40))
* **config:** 添加 GPT-5.4 关联配置及快速模式支持 ([38a11cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/38a11cb4c33300a2a7edb19848d97e25e966688b))
* **console:** 改进控制台界面并增加日志过滤和搜索功能 ([93badf8](https://github.com/dyndynjyxa/aio-coding-hub/commit/93badf8766c9a415c821c3f14084183d18d31687))
* **core:** improve workspace imports and startup resilience ([1d7dbf0](https://github.com/dyndynjyxa/aio-coding-hub/commit/1d7dbf0b73cde2b78a70be945c6fd8e248b30c1d))
* **cx2cc:** add Codex-to-Claude-Code translation bridge ([#100](https://github.com/dyndynjyxa/aio-coding-hub/issues/100)) ([44d9c41](https://github.com/dyndynjyxa/aio-coding-hub/commit/44d9c416e434b72699e95984dbc156d413fc78dc))
* **date-range:** refactor custom date range handling in HomeCostPanel and UsagePage ([95b1ad1](https://github.com/dyndynjyxa/aio-coding-hub/commit/95b1ad11b16a0f3e9b187b4883ef85a367d712e0))
* **deps:** 新增 react-markdown 和 tailwindcss 排版插件依赖 ([a58f747](https://github.com/dyndynjyxa/aio-coding-hub/commit/a58f7477ba4c35f4f0c5b4812d985c8294cf8cf3))
* **deps:** 添加 @mdxeditor/editor 依赖 ([8da488a](https://github.com/dyndynjyxa/aio-coding-hub/commit/8da488a5c0296b793eb184d0a081f09eda14af2e))
* **domain:** 优化按小时统计总 token 计算方式 ([8f3df70](https://github.com/dyndynjyxa/aio-coding-hub/commit/8f3df7039eb5b66d1780359e0ecbface88d3bec9))
* **domain:** 支持 cx2cc 提供者及缓存令牌追踪 ([3662c26](https://github.com/dyndynjyxa/aio-coding-hub/commit/3662c26c2ea72c25ef7e330a52a414a708164cd1))
* **failover-loop:** refactor context management for improved clarity and usability ([b96df01](https://github.com/dyndynjyxa/aio-coding-hub/commit/b96df013317c77ca610bb640a43b2d60058e6847))
* **failover-loop:** refactor error handling and logging in failover loop ([ef2662c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef2662c335ae4057c591b531b0784c86cd73951e))
* **gateway,providers,oauth:** codex-oauth-with-proxy ([#76](https://github.com/dyndynjyxa/aio-coding-hub/issues/76)) ([f37e59a](https://github.com/dyndynjyxa/aio-coding-hub/commit/f37e59a3dbfdc8a95d18e592f1931c775f19c2ab))
* **gateway:** Circuit Breaker 引入 5 分钟滑动窗口衰减与 HalfOpen 渐进恢复 ([f90e59e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f90e59e3e72b5ff889ce37c1e8e9a225f15e7de3))
* **gateway:** Session TTL 改为滑动窗口，每次使用自动续期 ([3a6cb5c](https://github.com/dyndynjyxa/aio-coding-hub/commit/3a6cb5c631c733133c96aa172be7e445d85444dd))
* **gateway:** 优化协议兼容性处理与请求头管理 ([6be9161](https://github.com/dyndynjyxa/aio-coding-hub/commit/6be91613fbc0bfcd9b87fe419a8f9ad08178a1f7))
* **gateway:** 增强上游错误处理与界面错误详情展示 ([85f67c4](https://github.com/dyndynjyxa/aio-coding-hub/commit/85f67c4c1ffe08d102883458a2d9d29e0b2b98cf))
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
* **home:** 添加供应商限额面板刷新按钮 ([18f4f91](https://github.com/dyndynjyxa/aio-coding-hub/commit/18f4f91d375f860312a126354a65efad6244e7ae))
* **home:** 添加工作区模式切换按钮并持久化请求日志简洁模式 ([d73bedd](https://github.com/dyndynjyxa/aio-coding-hub/commit/d73bedd3bc27d82375969d09fa94336a268bd075))
* mac最小化优化 ([1f11a5d](https://github.com/dyndynjyxa/aio-coding-hub/commit/1f11a5df9623f8cab90bb40f5c499c5d3569f01d))
* **mcp:** implement batch import for local skills and add global error reporting ([51e053c](https://github.com/dyndynjyxa/aio-coding-hub/commit/51e053c81f1db00696f0cb968860033deee12cce))
* **mcp:** 优化服务器键生成与导入逻辑，支持保留大小写 ([f61a5c6](https://github.com/dyndynjyxa/aio-coding-hub/commit/f61a5c6be09795efaf9c4aafe7fee5f86329c0ea))
* **mcp:** 添加 env 和 header 键值对格式校验 ([7d698ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/7d698ca04f1cc9f9c0d420f6d7fec2a53f2d643a))
* **Oauth:** adding gemini Oauth support ([fd1eb79](https://github.com/dyndynjyxa/aio-coding-hub/commit/fd1eb7933410510e9ee46e469234d91658a49fcf))
* **provider-chain:** 优化供应商链路视图及日志详情显示 ([72d4bc4](https://github.com/dyndynjyxa/aio-coding-hub/commit/72d4bc49dfd1380ce503ec23a98eb81b55a1d488))
* **provider-chain:** 优化展示故障切换尝试详情和错误结构化信息 ([e2d530f](https://github.com/dyndynjyxa/aio-coding-hub/commit/e2d530fceb3fcf52e9a7745847b13963e854994c))
* **provider-editor:** enhance ProviderEditorDialog with limit configuration cards ([cc14a00](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc14a009303129793ceec840933155dcfad775d8))
* **providers:** 在供应商列表页增加名称搜索功能 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **providers:** 增加供应商备注字段并支持读取API Key ([fec7057](https://github.com/dyndynjyxa/aio-coding-hub/commit/fec7057d137a7c0b38b46b5591a9e19e624b679a))
* **providers:** 实现供应商编辑器和多功能配置组件 ([e9f2a45](https://github.com/dyndynjyxa/aio-coding-hub/commit/e9f2a45f64b30c12b66ddad13cf378cccfd4d3a7))
* **providers:** 支持复制供应商配置并预填创建表单 ([c8fb707](https://github.com/dyndynjyxa/aio-coding-hub/commit/c8fb7078131ad67e4d2895f81467a9358b897d2c))
* **providers:** 添加 OAuth 限制重置倒计时 & 修复托盘退出重启问题 ([#168](https://github.com/dyndynjyxa/aio-coding-hub/issues/168)) ([4ca1248](https://github.com/dyndynjyxa/aio-coding-hub/commit/4ca124860c48fd3764ecf1da861adef072267ba7))
* **providers:** 添加供应商标签功能，支持分类筛选 ([7671732](https://github.com/dyndynjyxa/aio-coding-hub/commit/76717329a119f9a65dc2f3a9dffe0a887063b0bd))
* **proxy:** 丰富错误详情上下文并优化请求记录错误展示 ([1ac8eef](https://github.com/dyndynjyxa/aio-coding-hub/commit/1ac8eefbf6107de383ee349abb5ee0e89c774a6f))
* **proxy:** 增加请求中断日志中尝试信息的捕获与展示 ([0ad0955](https://github.com/dyndynjyxa/aio-coding-hub/commit/0ad0955730da9ee46e1904838412fe546794e0f5))
* **proxy:** 实现对 Claude 请求的观察与请求日志生命周期管理 ([107d892](https://github.com/dyndynjyxa/aio-coding-hub/commit/107d89202defcd4bbb8727b15ec88bcf6bfe49e0))
* **proxy:** 支持 Codex 会话 ID 补全功能 ([25f8321](https://github.com/dyndynjyxa/aio-coding-hub/commit/25f8321536ccdfe9a68de1866bb9762794ac97b7))
* **request-end:** enhance request logging and event emission ([809a684](https://github.com/dyndynjyxa/aio-coding-hub/commit/809a6843a2c2eb952bbb00f7c56694c908d99bd1))
* **request-end:** refactor request logging and event emission ([e56909e](https://github.com/dyndynjyxa/aio-coding-hub/commit/e56909e374c26608407a1d63d32c9d55f914b355))
* **responsive:** enhance layout and styling for improved mobile experience ([e22483a](https://github.com/dyndynjyxa/aio-coding-hub/commit/e22483a8da8c35822c6e5b0532aa1461cb68cf95))
* **route:** 支持展示同一 provider 的连续尝试次数及跳过状态 ([6659157](https://github.com/dyndynjyxa/aio-coding-hub/commit/6659157a12c3809b1e4d8de7bf95b95afe6ed383))
* **settings:** update Claude settings and remove deprecated options ([49e055a](https://github.com/dyndynjyxa/aio-coding-hub/commit/49e055aeb176819cb56a925aecbd51f362008762))
* **settings:** 新增cx2cc配置项并添加序列化支持 ([3afaa64](https://github.com/dyndynjyxa/aio-coding-hub/commit/3afaa64ca0ca7c505a83a9906218ba3167769cd0))
* **settings:** 新增任务结束提醒开关和相关功能 ([2fb4022](https://github.com/dyndynjyxa/aio-coding-hub/commit/2fb4022ff2f1078238ced25188960f19da188260))
* **settings:** 新增关闭 Claude Git 参与者功能 ([cf23f0d](https://github.com/dyndynjyxa/aio-coding-hub/commit/cf23f0dec000cff63f222b0198360485deb60804))
* **settings:** 新增缓存异常监测功能开关 ([ea681c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ea681c155346da3057f25e25867ab75a818f4157))
* **settings:** 新增静默启动配置支持 ([fcc436b](https://github.com/dyndynjyxa/aio-coding-hub/commit/fcc436b5dc5baf3b57e7160bd7e3562275535072))
* **settings:** 添加通知声音开关配置项 ([d13ee18](https://github.com/dyndynjyxa/aio-coding-hub/commit/d13ee18228cde971e7490468c6831aa679a636ab))
* **settings:** 添加通知音效开关及完善通用配置界面 ([e8d285a](https://github.com/dyndynjyxa/aio-coding-hub/commit/e8d285a704ba3f1a82fda9b16f9c2dbba393a57d))
* **skills:** support repo market and local CLI workflows ([3a0a24f](https://github.com/dyndynjyxa/aio-coding-hub/commit/3a0a24f13405d493237402dace175807b42de462))
* **skills:** 支持从本地源恢复缺失的ssot技能目录 ([4414f7d](https://github.com/dyndynjyxa/aio-coding-hub/commit/4414f7d38a44e4aff7de0730fe591b93146d7468))
* **skills:** 支持推理强度新增等级xhigh ([36dbb77](https://github.com/dyndynjyxa/aio-coding-hub/commit/36dbb773c94f7acaa9d1a9e6ed68d8df02d6c950))
* **skills:** 添加 Skill 返回本机已安装功能 ([84cfd0b](https://github.com/dyndynjyxa/aio-coding-hub/commit/84cfd0be09c5c38dbd9613533062a3daef6c57df))
* **sort-modes:** add toggle functionality for enabling/disabling providers in sort modes and update related services ([e6cd992](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6cd99241f5c86da3f62afebefeffdf33e62b8f0))
* **taskCompleteNotifyEvents:** 改进任务完成通知的请求追踪与静默期控制 ([e3f03d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/e3f03d4bf2276db371e170ee4739cd379b6d3fe5))
* **tauri:** update error handling and add thiserror dependency ([10d918a](https://github.com/dyndynjyxa/aio-coding-hub/commit/10d918a0a0a746b4d25c1c236f88c0110ec664c3))
* **theme:** add dark mode and native window theme sync ([588a373](https://github.com/dyndynjyxa/aio-coding-hub/commit/588a37311c456bd2182ac1676c8e9b309157353f))
* **ui:** 优化首页最近代理记录卡片展示与预览数据 ([#181](https://github.com/dyndynjyxa/aio-coding-hub/issues/181)) ([c5a0068](https://github.com/dyndynjyxa/aio-coding-hub/commit/c5a0068ac81e6d97fd9a021c346ee7f0024da42d))
* **ui:** 优化首页概览、CLI 代理状态与供应商页交互 ([#106](https://github.com/dyndynjyxa/aio-coding-hub/issues/106)) ([358e73f](https://github.com/dyndynjyxa/aio-coding-hub/commit/358e73f598fd48c7bbf131a3117e4801cec03ced))
* **ui:** 优化首页概览、供应商管理与设置页主题入口 ([#102](https://github.com/dyndynjyxa/aio-coding-hub/issues/102)) ([0adec0a](https://github.com/dyndynjyxa/aio-coding-hub/commit/0adec0af676842e9c5aee3e08e3a46d890d31c11))
* **ui:** 本次主要优化了首页概览、设置页和代理记录页，同时顺手处理了首页包体和 Tauri 构建告警。 ([#111](https://github.com/dyndynjyxa/aio-coding-hub/issues/111)) ([4e6dbad](https://github.com/dyndynjyxa/aio-coding-hub/commit/4e6dbad14cab8334d47a3a30ebfcf70485ad2e0d))
* **ui:** 统一 Loading/Empty/Error 状态反馈组件 ([b27f64a](https://github.com/dyndynjyxa/aio-coding-hub/commit/b27f64a79a1ad655a37992527794759f9f8c5370))
* **ui:** 设置与首页展示优化，并修复 Node 24 下 pre-push 单测问题 ([#169](https://github.com/dyndynjyxa/aio-coding-hub/issues/169)) ([165db18](https://github.com/dyndynjyxa/aio-coding-hub/commit/165db18dce650cee175a8df07f9bc80168575aab))
* **ui:** 首页最近代理记录与配置信息展示优化 ([#185](https://github.com/dyndynjyxa/aio-coding-hub/issues/185)) ([84d98ef](https://github.com/dyndynjyxa/aio-coding-hub/commit/84d98ef1b0489bb64e83f577c355a4780fe07227))
* **updater:** display release notes in update dialog ([b891ed7](https://github.com/dyndynjyxa/aio-coding-hub/commit/b891ed75a1b67a9df281917ffa94e684fb372664))
* **update:** 更新对话框展示更新日志 ([#171](https://github.com/dyndynjyxa/aio-coding-hub/issues/171)) ([c55ccb2](https://github.com/dyndynjyxa/aio-coding-hub/commit/c55ccb2cb1fc18b1e109f41192a4362f3a4ff8af))
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
* 支持 CX2CC 使用当前 AIO 服务 Codex 网关作为来源 ([#194](https://github.com/dyndynjyxa/aio-coding-hub/issues/194)) ([2bf7117](https://github.com/dyndynjyxa/aio-coding-hub/commit/2bf7117585f20e03971831a04bd721fb6f620d67))
* 首页新增 Token 用量面板 ([#204](https://github.com/dyndynjyxa/aio-coding-hub/issues/204)) ([4513106](https://github.com/dyndynjyxa/aio-coding-hub/commit/45131069f2d67a0c909c1558102305f1cfe41028))


### Bug Fixes

* **app:** 修复启动状态空值判断，改进Cx2cc选项卡状态管理 ([654eb78](https://github.com/dyndynjyxa/aio-coding-hub/commit/654eb7843fe48c3393d4119f89510de7462a0442))
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
* **ci:** 修复Rust测试线程数为单线程运行 ([87ec1fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/87ec1fe962d7b2913841e739e3ee7026f7145c36))
* **ci:** 修复正则表达式转义字符问题 ([ba805c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba805c3b2f147f153cbe39042b078547eb36da5a))
* **ci:** 修正发布流程中更新器 JSON 文件参数名称 ([9a4c0f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/9a4c0f063bd645e3286e67f1607334a84a5cb6fd))
* **claude-model-validation:** 优化界面中文提示文本 ([57651b8](https://github.com/dyndynjyxa/aio-coding-hub/commit/57651b8bafc32c782733c938b44c95a423216b79))
* **ClaudeModelValidationDialog:** adjust dialog max-width for responsive design ([11bdfd0](https://github.com/dyndynjyxa/aio-coding-hub/commit/11bdfd0df1b51923a561211f6cc6d5383eedf76c))
* **cli_manager:** 修复 run_in_login_shell 的平台兼容性处理 ([9f81433](https://github.com/dyndynjyxa/aio-coding-hub/commit/9f814337d08f36c08e37486f95b9c9da62880fc1))
* **cli_proxy:** 修复数据库初始化错误处理逻辑 ([9230475](https://github.com/dyndynjyxa/aio-coding-hub/commit/9230475e4401e93e3dc34ad285ace4a03186cd16))
* **cli_update:** 修复提取语义版本字符串的字符截取逻辑 ([782fb95](https://github.com/dyndynjyxa/aio-coding-hub/commit/782fb95d05236afbbfdf80b4bfb15921b9c34b88))
* **cli-manager:** 修正模型参数解析逻辑以避免默认值覆盖 ([ae2b7fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae2b7feecfd6e06462b8b0990204fc55d6ad3ccf))
* **cli-proxy:** 修复 Codex CLI 在 Windows 下的沙箱配置与认证模式 ([b79d072](https://github.com/dyndynjyxa/aio-coding-hub/commit/b79d072f39405c15125a8a6391bff122c9a5f315))
* **cli-sessions:** 修复SessionsPage Card容器flex布局 ([2d2a048](https://github.com/dyndynjyxa/aio-coding-hub/commit/2d2a0481f9811160223f9753d6edec5a28dcfc6c))
* **codex-tab:** sandbox_mode danger-full-access 选择不生效及高级配置不回显 ([3943401](https://github.com/dyndynjyxa/aio-coding-hub/commit/3943401524c8ef6a179e65e8ee62fd2ac4f784cb))
* **codex:** resolve oauth login failure and stale provider names ([#116](https://github.com/dyndynjyxa/aio-coding-hub/issues/116)) ([48d647e](https://github.com/dyndynjyxa/aio-coding-hub/commit/48d647ef429232562c9b56e88d8a37f57960604a))
* **codex:** 修复 Pi 本地 endpoint 与 Responses 兼容约束 ([#180](https://github.com/dyndynjyxa/aio-coding-hub/issues/180)) ([8e7085e](https://github.com/dyndynjyxa/aio-coding-hub/commit/8e7085e69d1aabca28d398190239e99bcab4fe03))
* **codex:** 修复切换 Codex Home 后代理状态与实际配置不一致 ([#125](https://github.com/dyndynjyxa/aio-coding-hub/issues/125)) ([e8f80b2](https://github.com/dyndynjyxa/aio-coding-hub/commit/e8f80b2838509b748d38ed6411219218b4077bf5))
* **commands:** 修复构建Claude启动命令时路径参数传递 ([9e714f4](https://github.com/dyndynjyxa/aio-coding-hub/commit/9e714f48bdbf7dff2d729131774518845303290a))
* **components:** 修复ClaudeModelValidationDialog点击事件阻塞问题 ([9bd5f53](https://github.com/dyndynjyxa/aio-coding-hub/commit/9bd5f53f38d01fc312b979e3620b5e0badaba585))
* **cost:** price CX2CC requests with translated model basis ([#175](https://github.com/dyndynjyxa/aio-coding-hub/issues/175)) ([342f585](https://github.com/dyndynjyxa/aio-coding-hub/commit/342f585c9b713077431e83ada0d008550fa5f708))
* **cost:** 修正成本乘数比较逻辑为包含零值 ([db69776](https://github.com/dyndynjyxa/aio-coding-hub/commit/db6977635f96fc55fa1b4d84d1d71e67dd8dd0b1))
* **db:** 修复providers表note字段迁移及数据处理问题 ([fa8ecda](https://github.com/dyndynjyxa/aio-coding-hub/commit/fa8ecda9bad231ed424c812c1c7cd3d0850b8565))
* **deps:** 升级 react-router-dom 7.11→7.13 修复 XSS 漏洞 ([c4db630](https://github.com/dyndynjyxa/aio-coding-hub/commit/c4db630b4478cab2f1180f03b324cfc0af9d42e3))
* **domain:** 修复 SSE 流错误处理逻辑 ([4446939](https://github.com/dyndynjyxa/aio-coding-hub/commit/4446939c457735727f96dce0b09c91ae6b616c1d))
* **domain:** 修正 claude_terminal_launch_context 函数中 provider_id 验证和查询逻辑 ([316d375](https://github.com/dyndynjyxa/aio-coding-hub/commit/316d3754fc628d0a1e435aabf469ca992cb63efa))
* **failover:** 区分网关过滤与上游请求失败 ([441b6f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/441b6f0d84565bdd21b8d52334aaedd60425ebef))
* **gateway:** allow failover success handler arg count ([0c559cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/0c559cba801d59332a94eb650571487b6b18a53b))
* **gateway:** claude api-key auth fallback ([ac68780](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac687808a15cc8d412714d623e4a32ef10ece310))
* **gateway:** finalize failed requests and align error codes ([871de9c](https://github.com/dyndynjyxa/aio-coding-hub/commit/871de9c9c9e06ed6cb03026ceb21dfc4b0ba18d3))
* **gateway:** 修复 CX2CC 无 SSE 响应头成功响应的分类时机 ([#160](https://github.com/dyndynjyxa/aio-coding-hub/issues/160)) ([9edcd5b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9edcd5bf8396a0a2a84e5b368d984717b4d7c81f))
* **gateway:** 修复 HALF_OPEN 状态下熔断未及时解除的前端展示问题 ([#166](https://github.com/dyndynjyxa/aio-coding-hub/issues/166)) ([4c809f9](https://github.com/dyndynjyxa/aio-coding-hub/commit/4c809f9b0b6aee4e426d8f3a6d2aa53699fd7b35))
* **gateway:** 修复代理初始化时未启用代理配置的问题 ([934aee9](https://github.com/dyndynjyxa/aio-coding-hub/commit/934aee94964226860ff40e885763685227af2535))
* **gateway:** 修正 Heartbeat 事件变量可见性及用法 ([a540d52](https://github.com/dyndynjyxa/aio-coding-hub/commit/a540d52012652075deee18c58d3e4b5611f4d547))
* **gateway:** 修正 provider 重新排序时清除会话绑定逻辑 ([f03c02e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f03c02e5332fe84496a029b2e5d130b558cfad31))
* **gateway:** 调整上游连接超时与错误切换逻辑 ([a6870ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6870ae14b6b5bb8edce814ca37da2000e84f6b4))
* **heartbeat_watchdog:** reload 返回 Ok 但实际异步失败时升级到窗口重建 ([#172](https://github.com/dyndynjyxa/aio-coding-hub/issues/172)) ([e4f77b0](https://github.com/dyndynjyxa/aio-coding-hub/commit/e4f77b01c7204767b61d8bb787a374b270383736))
* **heartbeat_watchdog:** 优化窗口重载的错误处理逻辑 ([6188e6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6188e6c29135c0126490882ceb54302d112ec79c))
* **home:** update HomeCostPanel with accessibility labels and improve test assertions ([0bda6c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/0bda6c1ce8aec6920b30810882f3a804c62ab732))
* **macOS:** 修复macOS 在 CLI 管理页面读取不到工具 version 并提示 INTERNAL_ERROR ([#126](https://github.com/dyndynjyxa/aio-coding-hub/issues/126)) ([ce35a94](https://github.com/dyndynjyxa/aio-coding-hub/commit/ce35a945dd278b453a1ddebbe8e95fd5f4f12ee6))
* **mcp:** 修复 MCP Server Dialog 支持 SSE 类型和保存逻辑 ([a219783](https://github.com/dyndynjyxa/aio-coding-hub/commit/a2197836c2aa9a431be0e2c8bc8b243dad2072fd))
* **mcp:** 修复导入服务器时基于 server_key 的重复处理 ([f6e6458](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6e64582e1e4c478ec014e61258611d971f8b3b4))
* **notification:** 优化系统通知及心跳监测逻辑 ([cfeb63f](https://github.com/dyndynjyxa/aio-coding-hub/commit/cfeb63f6b331df3be3feddbf0aac258018dba824))
* **oauth:** 调整 OAuth 刷新周期并移除 CLI 管理页 Claude OAuth 卡片 ([#184](https://github.com/dyndynjyxa/aio-coding-hub/issues/184)) ([3640ec7](https://github.com/dyndynjyxa/aio-coding-hub/commit/3640ec7853d81ba9dce8d8e4a049f0319f4827af))
* **prompts:** 修改新增和编辑时 Prompt 保存按钮禁用条件 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **providers:** 敏感配置信息变更时清理运行时 session 绑定 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **proxy:** 优化未匹配客户端错误的中止逻辑 ([05eb435](https://github.com/dyndynjyxa/aio-coding-hub/commit/05eb435e499786d8308f548d5115adf378adc7f6))
* **proxy:** 允许 resolve_session_bound_provider_id 函数有多个参数 ([705457e](https://github.com/dyndynjyxa/aio-coding-hub/commit/705457eac5092ccd98a0f3523ea1986c67dd9d9c))
* **release:** keep Cargo.lock synced for release PRs ([c5bd423](https://github.com/dyndynjyxa/aio-coding-hub/commit/c5bd42355641cb8dcef13a1fa04d41795aeb5c82))
* **router:** 修复侧边栏快速点击导航卡死问题 ([8b1f862](https://github.com/dyndynjyxa/aio-coding-hub/commit/8b1f862542b8bd94770ab5f937772a53a4bd1d22))
* **rust:** resolve clippy warnings for needless lifetimes and useless conversion ([b5f204f](https://github.com/dyndynjyxa/aio-coding-hub/commit/b5f204f1d309cc3e21f56427760f5969d1972e95))
* **scripts:** 修复生成绑定文件格式化问题 ([9ff697b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9ff697b6b8c93238f48518f27f274df33ba55a9a))
* **sessions:** 剥离替换字符并调整按钮样式 ([3586a16](https://github.com/dyndynjyxa/aio-coding-hub/commit/3586a165ceb8ba3afc42a34fdf4228c71b18cce5))
* **settings:** 保持完整快照防止默认值变更导致设置漂移 ([6a8626c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6a8626cdc93b06134279fe3d2020ecf1ce0d48f8))
* **settings:** 增强设置读取错误处理和写入保护 ([6759e60](https://github.com/dyndynjyxa/aio-coding-hub/commit/6759e60edf1cb93e63f8c05787ed4f0a320a175b))
* **settings:** 设置默认关闭Billing Header整流器 ([9331253](https://github.com/dyndynjyxa/aio-coding-hub/commit/933125358a9574225cd1f317fcdc9d6b1679c6c4))
* **settings:** 限制上游流式空闲超时最小值为60秒 ([e7284c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/e7284c251950bcfa27db72385603756472329646))
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
* **ui:** 修复 Sidebar 组件状态文字显示异常问题 ([ff0b1ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/ff0b1ae74e8c08fa5c73044fa924e7c78e24284d))
* **update:** restore changelog and local preview flow ([#178](https://github.com/dyndynjyxa/aio-coding-hub/issues/178)) ([36a564d](https://github.com/dyndynjyxa/aio-coding-hub/commit/36a564dcaa33a6bd9c76a9bda30599a1d3f92c52))
* **usage:** 修复 Usage 统计中的 CX2CC 缓存命中率计算 ([#159](https://github.com/dyndynjyxa/aio-coding-hub/issues/159)) ([3472560](https://github.com/dyndynjyxa/aio-coding-hub/commit/3472560f62f88e4eae4b4d53e920c290869a2c7d))
* **utils:** 修正computeOutputTokensPerSecond回退逻辑 ([7c59559](https://github.com/dyndynjyxa/aio-coding-hub/commit/7c59559ad0b525e478577d74518173753530a980))
* **util:** 增加请求体大小限制至100MB并更新体积错误消息 ([b12c1ba](https://github.com/dyndynjyxa/aio-coding-hub/commit/b12c1baeccbfd29b8bcc4a055e1b6d7764d6d3dd))
* WebView2 不可恢复状态检测与分级恢复 ([#156](https://github.com/dyndynjyxa/aio-coding-hub/issues/156)) ([4869745](https://github.com/dyndynjyxa/aio-coding-hub/commit/4869745cb352279829317f18b5e1956c381b5b38))
* **wsl:** atomic write, remove dead fallback, add TOML comment ([7decfb5](https://github.com/dyndynjyxa/aio-coding-hub/commit/7decfb5803378ee3bfb5433bc88adb64562a6857))
* **wsl:** 修复 WSL 脚本错误消息编码和路径写入问题 ([bccd5c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/bccd5c93110641898e04cd1cde031df604af637b))
* **wsl:** 修复WSL适配器IPv4解析和状态脚本处理 ([06ae9b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/06ae9b1f462572550f327c3afe4034179b1da491))
* **wsl:** 修正wsl_auto_sync_core函数调用路径 ([cdb62a4](https://github.com/dyndynjyxa/aio-coding-hub/commit/cdb62a43d97d461696ebbc936c56f6982b3e0dae))
* **wsl:** 修正同步边界与 sessions 状态 ([cc41840](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc41840fb88744c926544436cf885b440f240403))
* 修复clippy redundant_pattern_matching警告 ([b0274a6](https://github.com/dyndynjyxa/aio-coding-hub/commit/b0274a663396a2958e470e2167933470bf34a70a))


### Performance Improvements

* **home:** 优化实时追踪卡片动画与样式过渡效果 ([48c9966](https://github.com/dyndynjyxa/aio-coding-hub/commit/48c99669d5658884bc594dcebdb35219d0c25e23))


### Code Refactoring

* **infra:** 移除 request_attempt_logs 独立表，改用 request_logs.attempts_json 派生 ([3fb2627](https://github.com/dyndynjyxa/aio-coding-hub/commit/3fb2627407e1b9b57758a8a40d3a7d7393278e1b))

## [0.39.3](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.39.2...aio-coding-hub-v0.39.3) (2026-04-21)


### Features

* **app:** 集成应用启动状态管理和展示组件 ([2c7eb80](https://github.com/dyndynjyxa/aio-coding-hub/commit/2c7eb802e5317e12105eceed9533d3865b69b201))
* **settings:** 新增cx2cc配置项并添加序列化支持 ([3afaa64](https://github.com/dyndynjyxa/aio-coding-hub/commit/3afaa64ca0ca7c505a83a9906218ba3167769cd0))


### Bug Fixes

* **app:** 修复启动状态空值判断，改进Cx2cc选项卡状态管理 ([654eb78](https://github.com/dyndynjyxa/aio-coding-hub/commit/654eb7843fe48c3393d4119f89510de7462a0442))
* **settings:** 保持完整快照防止默认值变更导致设置漂移 ([6a8626c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6a8626cdc93b06134279fe3d2020ecf1ce0d48f8))

## [0.39.2](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.39.1...aio-coding-hub-v0.39.2) (2026-04-19)


### Bug Fixes

* **rust:** resolve clippy warnings for needless lifetimes and useless conversion ([b5f204f](https://github.com/dyndynjyxa/aio-coding-hub/commit/b5f204f1d309cc3e21f56427760f5969d1972e95))

## [0.39.1](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.39.0...aio-coding-hub-v0.39.1) (2026-04-18)


### Bug Fixes

* **gateway:** 修复代理初始化时未启用代理配置的问题 ([934aee9](https://github.com/dyndynjyxa/aio-coding-hub/commit/934aee94964226860ff40e885763685227af2535))

## [0.39.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.38.4...aio-coding-hub-v0.39.0) (2026-04-17)


### ⚠ BREAKING CHANGES

* **infra:** request_attempt_logs 表不再创建或写入

### Features

* add Claude OAuth support with multi-account switching ([#149](https://github.com/dyndynjyxa/aio-coding-hub/issues/149)) ([ef787b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef787b1ddf309ffcf9ffcb923d92bf2af3f557b6))
* add CX2CC tab, config export/import, Gemini config enhance, CLI… ([#138](https://github.com/dyndynjyxa/aio-coding-hub/issues/138)) ([7aba381](https://github.com/dyndynjyxa/aio-coding-hub/commit/7aba381a462812e6c6ea505de3650397d23b48b4))
* add gemini oauth code assist proxy support ([4a892d0](https://github.com/dyndynjyxa/aio-coding-hub/commit/4a892d035fbcdd5273c909de063eede5d42598fa))
* **app:** add LogsPage route and enhance CLI manager settings ([6796715](https://github.com/dyndynjyxa/aio-coding-hub/commit/679671526be935129211978b3598a3962f7e2a8e))
* **app:** 优化页面路由懒加载和加载反馈体验 ([504256e](https://github.com/dyndynjyxa/aio-coding-hub/commit/504256e11ed517797405f8bb4ab3d353607cb1ae))
* **app:** 添加应用心跳机制与监听支持 ([df78171](https://github.com/dyndynjyxa/aio-coding-hub/commit/df781712d16a2f5ccc99e37a687d39cb4e4f56c5))
* **bindings:** 添加 HomeUsagePeriod 类型契约断言 ([a540d52](https://github.com/dyndynjyxa/aio-coding-hub/commit/a540d52012652075deee18c58d3e4b5611f4d547))
* **build:** 添加Wayland兼容的AppImage构建支持 ([95c7130](https://github.com/dyndynjyxa/aio-coding-hub/commit/95c71307038962fc0e37fbd7b6df844301d8decb))
* **build:** 添加Windows便携版ZIP构建和WebView2检查 ([fef41e1](https://github.com/dyndynjyxa/aio-coding-hub/commit/fef41e12f2a1301743b40de4c7402f4b02fa80f4))
* **cache-anomaly-monitor:** implement cache anomaly monitoring feature ([59d69d1](https://github.com/dyndynjyxa/aio-coding-hub/commit/59d69d1528d888a84d71038873321f3273d4ded1))
* **charts:** migrate from ECharts to Recharts for improved charting capabilities ([b747b61](https://github.com/dyndynjyxa/aio-coding-hub/commit/b747b61a8dd587043a7e8de01a5f9b3d64ead7ae))
* **circuit-breaker:** 支持运行时动态更新断路器配置 ([3cb8722](https://github.com/dyndynjyxa/aio-coding-hub/commit/3cb87220345020ea8916f192ac6c84281d2736f2))
* **ClaudeModelValidationDialog:** 添加综合协议验证功能与UI展示 ([b6ede30](https://github.com/dyndynjyxa/aio-coding-hub/commit/b6ede30e61e19f9af69e8d7153cbc711b3e3a180))
* **ClaudeModelValidation:** enhance cross-provider validation and UI feedback ([bf83c7e](https://github.com/dyndynjyxa/aio-coding-hub/commit/bf83c7e03c7edf78795cd51a943c01a88e0b17d7))
* **ClaudeModelValidation:** enhance output token validation and error handling ([d245288](https://github.com/dyndynjyxa/aio-coding-hub/commit/d245288d7a4937ca7b0213ebd79d9c0d5e3c76b4))
* **ClaudeModelValidation:** implement cross-provider signature validation and enhance request handling ([2e102d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/2e102d4f3fd2745e4480a5884272baeafe66b6d0))
* **claudeModelValidation:** 优化模型验证对话框和多轮验证流程支持 ([3f6c031](https://github.com/dyndynjyxa/aio-coding-hub/commit/3f6c03102db7aaeaa2d8399ebdac1cc4787c2a84))
* **cli_proxy:** 添加 merge-restore 功能以保留用户更改 ([a7d05c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/a7d05c97fe1228d83ccf97be5ed9b6ec26c7bde0))
* **cli_proxy:** 添加自动同步以修复代理配置漂移问题 ([0a7b856](https://github.com/dyndynjyxa/aio-coding-hub/commit/0a7b856e87d62fd3be84f6314a89f3f1a6706e92))
* **cli_sessions:** 优化 Claude-CLI 项目路径解析和工作目录提取 ([a0024ac](https://github.com/dyndynjyxa/aio-coding-hub/commit/a0024acf6c2e3bc1615d304d6ae681fbb4550371))
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
* **cli-manager:** 在通用配置页新增上游代理相关设置支持 ([0b2489f](https://github.com/dyndynjyxa/aio-coding-hub/commit/0b2489f4c201af5daba697b4f272ca6de5767e54))
* **cli-manager:** 对齐 CCH 基础配置开关 ([55779ef](https://github.com/dyndynjyxa/aio-coding-hub/commit/55779efce6964a17b423869af77f3ebff567d62c))
* **cli-manager:** 支持 Claude Code 环境变量配置并修复网关重置缓存 ([#192](https://github.com/dyndynjyxa/aio-coding-hub/issues/192)) ([b41a2cf](https://github.com/dyndynjyxa/aio-coding-hub/commit/b41a2cfad6a964fe3f7dd9bd97a3db523bcc59b4))
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
* **deps:** 新增 react-markdown 和 tailwindcss 排版插件依赖 ([a58f747](https://github.com/dyndynjyxa/aio-coding-hub/commit/a58f7477ba4c35f4f0c5b4812d985c8294cf8cf3))
* **deps:** 添加 @mdxeditor/editor 依赖 ([8da488a](https://github.com/dyndynjyxa/aio-coding-hub/commit/8da488a5c0296b793eb184d0a081f09eda14af2e))
* **domain:** 优化按小时统计总 token 计算方式 ([8f3df70](https://github.com/dyndynjyxa/aio-coding-hub/commit/8f3df7039eb5b66d1780359e0ecbface88d3bec9))
* **domain:** 支持 cx2cc 提供者及缓存令牌追踪 ([3662c26](https://github.com/dyndynjyxa/aio-coding-hub/commit/3662c26c2ea72c25ef7e330a52a414a708164cd1))
* **failover-loop:** refactor context management for improved clarity and usability ([b96df01](https://github.com/dyndynjyxa/aio-coding-hub/commit/b96df013317c77ca610bb640a43b2d60058e6847))
* **failover-loop:** refactor error handling and logging in failover loop ([ef2662c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef2662c335ae4057c591b531b0784c86cd73951e))
* **gateway,providers,oauth:** codex-oauth-with-proxy ([#76](https://github.com/dyndynjyxa/aio-coding-hub/issues/76)) ([f37e59a](https://github.com/dyndynjyxa/aio-coding-hub/commit/f37e59a3dbfdc8a95d18e592f1931c775f19c2ab))
* **gateway:** Circuit Breaker 引入 5 分钟滑动窗口衰减与 HalfOpen 渐进恢复 ([f90e59e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f90e59e3e72b5ff889ce37c1e8e9a225f15e7de3))
* **gateway:** Session TTL 改为滑动窗口，每次使用自动续期 ([3a6cb5c](https://github.com/dyndynjyxa/aio-coding-hub/commit/3a6cb5c631c733133c96aa172be7e445d85444dd))
* **gateway:** 优化协议兼容性处理与请求头管理 ([6be9161](https://github.com/dyndynjyxa/aio-coding-hub/commit/6be91613fbc0bfcd9b87fe419a8f9ad08178a1f7))
* **gateway:** 增强上游错误处理与界面错误详情展示 ([85f67c4](https://github.com/dyndynjyxa/aio-coding-hub/commit/85f67c4c1ffe08d102883458a2d9d29e0b2b98cf))
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
* **home:** 添加供应商限额面板刷新按钮 ([18f4f91](https://github.com/dyndynjyxa/aio-coding-hub/commit/18f4f91d375f860312a126354a65efad6244e7ae))
* **home:** 添加工作区模式切换按钮并持久化请求日志简洁模式 ([d73bedd](https://github.com/dyndynjyxa/aio-coding-hub/commit/d73bedd3bc27d82375969d09fa94336a268bd075))
* integrate PageHeader component across multiple pages for consistent UI ([330da27](https://github.com/dyndynjyxa/aio-coding-hub/commit/330da276f9ef8e91744a9534d59590a3a6fec5ff))
* mac最小化优化 ([1f11a5d](https://github.com/dyndynjyxa/aio-coding-hub/commit/1f11a5df9623f8cab90bb40f5c499c5d3569f01d))
* **mcp:** implement batch import for local skills and add global error reporting ([51e053c](https://github.com/dyndynjyxa/aio-coding-hub/commit/51e053c81f1db00696f0cb968860033deee12cce))
* **mcp:** 优化服务器键生成与导入逻辑，支持保留大小写 ([f61a5c6](https://github.com/dyndynjyxa/aio-coding-hub/commit/f61a5c6be09795efaf9c4aafe7fee5f86329c0ea))
* **mcp:** 添加 env 和 header 键值对格式校验 ([7d698ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/7d698ca04f1cc9f9c0d420f6d7fec2a53f2d643a))
* **Oauth:** adding gemini Oauth support ([fd1eb79](https://github.com/dyndynjyxa/aio-coding-hub/commit/fd1eb7933410510e9ee46e469234d91658a49fcf))
* **provider-chain:** 优化供应商链路视图及日志详情显示 ([72d4bc4](https://github.com/dyndynjyxa/aio-coding-hub/commit/72d4bc49dfd1380ce503ec23a98eb81b55a1d488))
* **provider-chain:** 优化展示故障切换尝试详情和错误结构化信息 ([e2d530f](https://github.com/dyndynjyxa/aio-coding-hub/commit/e2d530fceb3fcf52e9a7745847b13963e854994c))
* **provider-editor:** enhance ProviderEditorDialog with limit configuration cards ([cc14a00](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc14a009303129793ceec840933155dcfad775d8))
* **providers:** 在供应商列表页增加名称搜索功能 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **providers:** 增加供应商备注字段并支持读取API Key ([fec7057](https://github.com/dyndynjyxa/aio-coding-hub/commit/fec7057d137a7c0b38b46b5591a9e19e624b679a))
* **providers:** 实现供应商编辑器和多功能配置组件 ([e9f2a45](https://github.com/dyndynjyxa/aio-coding-hub/commit/e9f2a45f64b30c12b66ddad13cf378cccfd4d3a7))
* **providers:** 支持复制供应商配置并预填创建表单 ([c8fb707](https://github.com/dyndynjyxa/aio-coding-hub/commit/c8fb7078131ad67e4d2895f81467a9358b897d2c))
* **providers:** 添加 OAuth 限制重置倒计时 & 修复托盘退出重启问题 ([#168](https://github.com/dyndynjyxa/aio-coding-hub/issues/168)) ([4ca1248](https://github.com/dyndynjyxa/aio-coding-hub/commit/4ca124860c48fd3764ecf1da861adef072267ba7))
* **providers:** 添加供应商标签功能，支持分类筛选 ([7671732](https://github.com/dyndynjyxa/aio-coding-hub/commit/76717329a119f9a65dc2f3a9dffe0a887063b0bd))
* **proxy:** 丰富错误详情上下文并优化请求记录错误展示 ([1ac8eef](https://github.com/dyndynjyxa/aio-coding-hub/commit/1ac8eefbf6107de383ee349abb5ee0e89c774a6f))
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
* **skills:** support repo market and local CLI workflows ([3a0a24f](https://github.com/dyndynjyxa/aio-coding-hub/commit/3a0a24f13405d493237402dace175807b42de462))
* **skills:** 支持从本地源恢复缺失的ssot技能目录 ([4414f7d](https://github.com/dyndynjyxa/aio-coding-hub/commit/4414f7d38a44e4aff7de0730fe591b93146d7468))
* **skills:** 支持推理强度新增等级xhigh ([36dbb77](https://github.com/dyndynjyxa/aio-coding-hub/commit/36dbb773c94f7acaa9d1a9e6ed68d8df02d6c950))
* **skills:** 添加 Skill 返回本机已安装功能 ([84cfd0b](https://github.com/dyndynjyxa/aio-coding-hub/commit/84cfd0be09c5c38dbd9613533062a3daef6c57df))
* **sort-modes:** add toggle functionality for enabling/disabling providers in sort modes and update related services ([e6cd992](https://github.com/dyndynjyxa/aio-coding-hub/commit/e6cd99241f5c86da3f62afebefeffdf33e62b8f0))
* **taskCompleteNotifyEvents:** 改进任务完成通知的请求追踪与静默期控制 ([e3f03d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/e3f03d4bf2276db371e170ee4739cd379b6d3fe5))
* **tauri:** update error handling and add thiserror dependency ([10d918a](https://github.com/dyndynjyxa/aio-coding-hub/commit/10d918a0a0a746b4d25c1c236f88c0110ec664c3))
* **tests:** add comprehensive unit tests for various components ([98fb022](https://github.com/dyndynjyxa/aio-coding-hub/commit/98fb022d1487e593e1fe50a1e9d02592671a4944))
* **theme:** add dark mode and native window theme sync ([588a373](https://github.com/dyndynjyxa/aio-coding-hub/commit/588a37311c456bd2182ac1676c8e9b309157353f))
* **ui:** 优化首页最近代理记录卡片展示与预览数据 ([#181](https://github.com/dyndynjyxa/aio-coding-hub/issues/181)) ([c5a0068](https://github.com/dyndynjyxa/aio-coding-hub/commit/c5a0068ac81e6d97fd9a021c346ee7f0024da42d))
* **ui:** 优化首页概览、CLI 代理状态与供应商页交互 ([#106](https://github.com/dyndynjyxa/aio-coding-hub/issues/106)) ([358e73f](https://github.com/dyndynjyxa/aio-coding-hub/commit/358e73f598fd48c7bbf131a3117e4801cec03ced))
* **ui:** 优化首页概览、供应商管理与设置页主题入口 ([#102](https://github.com/dyndynjyxa/aio-coding-hub/issues/102)) ([0adec0a](https://github.com/dyndynjyxa/aio-coding-hub/commit/0adec0af676842e9c5aee3e08e3a46d890d31c11))
* **ui:** 本次主要优化了首页概览、设置页和代理记录页，同时顺手处理了首页包体和 Tauri 构建告警。 ([#111](https://github.com/dyndynjyxa/aio-coding-hub/issues/111)) ([4e6dbad](https://github.com/dyndynjyxa/aio-coding-hub/commit/4e6dbad14cab8334d47a3a30ebfcf70485ad2e0d))
* **ui:** 统一 Loading/Empty/Error 状态反馈组件 ([b27f64a](https://github.com/dyndynjyxa/aio-coding-hub/commit/b27f64a79a1ad655a37992527794759f9f8c5370))
* **ui:** 设置与首页展示优化，并修复 Node 24 下 pre-push 单测问题 ([#169](https://github.com/dyndynjyxa/aio-coding-hub/issues/169)) ([165db18](https://github.com/dyndynjyxa/aio-coding-hub/commit/165db18dce650cee175a8df07f9bc80168575aab))
* **ui:** 首页最近代理记录与配置信息展示优化 ([#185](https://github.com/dyndynjyxa/aio-coding-hub/issues/185)) ([84d98ef](https://github.com/dyndynjyxa/aio-coding-hub/commit/84d98ef1b0489bb64e83f577c355a4780fe07227))
* **updater:** display release notes in update dialog ([b891ed7](https://github.com/dyndynjyxa/aio-coding-hub/commit/b891ed75a1b67a9df281917ffa94e684fb372664))
* **update:** 更新对话框展示更新日志 ([#171](https://github.com/dyndynjyxa/aio-coding-hub/issues/171)) ([c55ccb2](https://github.com/dyndynjyxa/aio-coding-hub/commit/c55ccb2cb1fc18b1e109f41192a4362f3a4ff8af))
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
* 支持 CX2CC 使用当前 AIO 服务 Codex 网关作为来源 ([#194](https://github.com/dyndynjyxa/aio-coding-hub/issues/194)) ([2bf7117](https://github.com/dyndynjyxa/aio-coding-hub/commit/2bf7117585f20e03971831a04bd721fb6f620d67))
* 首页新增 Token 用量面板 ([#204](https://github.com/dyndynjyxa/aio-coding-hub/issues/204)) ([4513106](https://github.com/dyndynjyxa/aio-coding-hub/commit/45131069f2d67a0c909c1558102305f1cfe41028))


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
* **ci:** 修复Rust测试线程数为单线程运行 ([87ec1fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/87ec1fe962d7b2913841e739e3ee7026f7145c36))
* **ci:** 修复正则表达式转义字符问题 ([ba805c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba805c3b2f147f153cbe39042b078547eb36da5a))
* **ci:** 修正发布流程中更新器 JSON 文件参数名称 ([9a4c0f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/9a4c0f063bd645e3286e67f1607334a84a5cb6fd))
* **claude-model-validation:** 优化界面中文提示文本 ([57651b8](https://github.com/dyndynjyxa/aio-coding-hub/commit/57651b8bafc32c782733c938b44c95a423216b79))
* **ClaudeModelValidationDialog:** adjust dialog max-width for responsive design ([11bdfd0](https://github.com/dyndynjyxa/aio-coding-hub/commit/11bdfd0df1b51923a561211f6cc6d5383eedf76c))
* **cli_manager:** 修复 run_in_login_shell 的平台兼容性处理 ([9f81433](https://github.com/dyndynjyxa/aio-coding-hub/commit/9f814337d08f36c08e37486f95b9c9da62880fc1))
* **cli_proxy:** 修复数据库初始化错误处理逻辑 ([9230475](https://github.com/dyndynjyxa/aio-coding-hub/commit/9230475e4401e93e3dc34ad285ace4a03186cd16))
* **cli_update:** 修复提取语义版本字符串的字符截取逻辑 ([782fb95](https://github.com/dyndynjyxa/aio-coding-hub/commit/782fb95d05236afbbfdf80b4bfb15921b9c34b88))
* **cli-manager:** 修正模型参数解析逻辑以避免默认值覆盖 ([ae2b7fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae2b7feecfd6e06462b8b0990204fc55d6ad3ccf))
* **cli-proxy:** 修复 Codex CLI 在 Windows 下的沙箱配置与认证模式 ([b79d072](https://github.com/dyndynjyxa/aio-coding-hub/commit/b79d072f39405c15125a8a6391bff122c9a5f315))
* **cli-sessions:** 修复SessionsPage Card容器flex布局 ([2d2a048](https://github.com/dyndynjyxa/aio-coding-hub/commit/2d2a0481f9811160223f9753d6edec5a28dcfc6c))
* **codex-tab:** sandbox_mode danger-full-access 选择不生效及高级配置不回显 ([3943401](https://github.com/dyndynjyxa/aio-coding-hub/commit/3943401524c8ef6a179e65e8ee62fd2ac4f784cb))
* **codex:** resolve oauth login failure and stale provider names ([#116](https://github.com/dyndynjyxa/aio-coding-hub/issues/116)) ([48d647e](https://github.com/dyndynjyxa/aio-coding-hub/commit/48d647ef429232562c9b56e88d8a37f57960604a))
* **codex:** 修复 Pi 本地 endpoint 与 Responses 兼容约束 ([#180](https://github.com/dyndynjyxa/aio-coding-hub/issues/180)) ([8e7085e](https://github.com/dyndynjyxa/aio-coding-hub/commit/8e7085e69d1aabca28d398190239e99bcab4fe03))
* **codex:** 修复切换 Codex Home 后代理状态与实际配置不一致 ([#125](https://github.com/dyndynjyxa/aio-coding-hub/issues/125)) ([e8f80b2](https://github.com/dyndynjyxa/aio-coding-hub/commit/e8f80b2838509b748d38ed6411219218b4077bf5))
* **commands:** 修复构建Claude启动命令时路径参数传递 ([9e714f4](https://github.com/dyndynjyxa/aio-coding-hub/commit/9e714f48bdbf7dff2d729131774518845303290a))
* **components:** 修复ClaudeModelValidationDialog点击事件阻塞问题 ([9bd5f53](https://github.com/dyndynjyxa/aio-coding-hub/commit/9bd5f53f38d01fc312b979e3620b5e0badaba585))
* **cost:** price CX2CC requests with translated model basis ([#175](https://github.com/dyndynjyxa/aio-coding-hub/issues/175)) ([342f585](https://github.com/dyndynjyxa/aio-coding-hub/commit/342f585c9b713077431e83ada0d008550fa5f708))
* **cost:** 修正成本乘数比较逻辑为包含零值 ([db69776](https://github.com/dyndynjyxa/aio-coding-hub/commit/db6977635f96fc55fa1b4d84d1d71e67dd8dd0b1))
* **db:** 修复providers表note字段迁移及数据处理问题 ([fa8ecda](https://github.com/dyndynjyxa/aio-coding-hub/commit/fa8ecda9bad231ed424c812c1c7cd3d0850b8565))
* **deps:** 升级 react-router-dom 7.11→7.13 修复 XSS 漏洞 ([c4db630](https://github.com/dyndynjyxa/aio-coding-hub/commit/c4db630b4478cab2f1180f03b324cfc0af9d42e3))
* **domain:** 修复 SSE 流错误处理逻辑 ([4446939](https://github.com/dyndynjyxa/aio-coding-hub/commit/4446939c457735727f96dce0b09c91ae6b616c1d))
* **domain:** 修正 claude_terminal_launch_context 函数中 provider_id 验证和查询逻辑 ([316d375](https://github.com/dyndynjyxa/aio-coding-hub/commit/316d3754fc628d0a1e435aabf469ca992cb63efa))
* **failover:** 区分网关过滤与上游请求失败 ([441b6f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/441b6f0d84565bdd21b8d52334aaedd60425ebef))
* **gateway:** allow failover success handler arg count ([0c559cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/0c559cba801d59332a94eb650571487b6b18a53b))
* **gateway:** claude api-key auth fallback ([ac68780](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac687808a15cc8d412714d623e4a32ef10ece310))
* **gateway:** finalize failed requests and align error codes ([871de9c](https://github.com/dyndynjyxa/aio-coding-hub/commit/871de9c9c9e06ed6cb03026ceb21dfc4b0ba18d3))
* **gateway:** 修复 CX2CC 无 SSE 响应头成功响应的分类时机 ([#160](https://github.com/dyndynjyxa/aio-coding-hub/issues/160)) ([9edcd5b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9edcd5bf8396a0a2a84e5b368d984717b4d7c81f))
* **gateway:** 修复 HALF_OPEN 状态下熔断未及时解除的前端展示问题 ([#166](https://github.com/dyndynjyxa/aio-coding-hub/issues/166)) ([4c809f9](https://github.com/dyndynjyxa/aio-coding-hub/commit/4c809f9b0b6aee4e426d8f3a6d2aa53699fd7b35))
* **gateway:** 修正 Heartbeat 事件变量可见性及用法 ([a540d52](https://github.com/dyndynjyxa/aio-coding-hub/commit/a540d52012652075deee18c58d3e4b5611f4d547))
* **gateway:** 修正 provider 重新排序时清除会话绑定逻辑 ([f03c02e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f03c02e5332fe84496a029b2e5d130b558cfad31))
* **gateway:** 调整上游连接超时与错误切换逻辑 ([a6870ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6870ae14b6b5bb8edce814ca37da2000e84f6b4))
* **heartbeat_watchdog:** reload 返回 Ok 但实际异步失败时升级到窗口重建 ([#172](https://github.com/dyndynjyxa/aio-coding-hub/issues/172)) ([e4f77b0](https://github.com/dyndynjyxa/aio-coding-hub/commit/e4f77b01c7204767b61d8bb787a374b270383736))
* **heartbeat_watchdog:** 优化窗口重载的错误处理逻辑 ([6188e6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6188e6c29135c0126490882ceb54302d112ec79c))
* **home:** update HomeCostPanel with accessibility labels and improve test assertions ([0bda6c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/0bda6c1ce8aec6920b30810882f3a804c62ab732))
* **macOS:** 修复macOS 在 CLI 管理页面读取不到工具 version 并提示 INTERNAL_ERROR ([#126](https://github.com/dyndynjyxa/aio-coding-hub/issues/126)) ([ce35a94](https://github.com/dyndynjyxa/aio-coding-hub/commit/ce35a945dd278b453a1ddebbe8e95fd5f4f12ee6))
* **mcp:** 修复 MCP Server Dialog 支持 SSE 类型和保存逻辑 ([a219783](https://github.com/dyndynjyxa/aio-coding-hub/commit/a2197836c2aa9a431be0e2c8bc8b243dad2072fd))
* **mcp:** 修复导入服务器时基于 server_key 的重复处理 ([f6e6458](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6e64582e1e4c478ec014e61258611d971f8b3b4))
* **notification:** 优化系统通知及心跳监测逻辑 ([cfeb63f](https://github.com/dyndynjyxa/aio-coding-hub/commit/cfeb63f6b331df3be3feddbf0aac258018dba824))
* **oauth:** 调整 OAuth 刷新周期并移除 CLI 管理页 Claude OAuth 卡片 ([#184](https://github.com/dyndynjyxa/aio-coding-hub/issues/184)) ([3640ec7](https://github.com/dyndynjyxa/aio-coding-hub/commit/3640ec7853d81ba9dce8d8e4a049f0319f4827af))
* **prompts:** 修改新增和编辑时 Prompt 保存按钮禁用条件 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **providers:** 敏感配置信息变更时清理运行时 session 绑定 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **proxy:** 优化未匹配客户端错误的中止逻辑 ([05eb435](https://github.com/dyndynjyxa/aio-coding-hub/commit/05eb435e499786d8308f548d5115adf378adc7f6))
* **proxy:** 允许 resolve_session_bound_provider_id 函数有多个参数 ([705457e](https://github.com/dyndynjyxa/aio-coding-hub/commit/705457eac5092ccd98a0f3523ea1986c67dd9d9c))
* **release:** keep Cargo.lock synced for release PRs ([c5bd423](https://github.com/dyndynjyxa/aio-coding-hub/commit/c5bd42355641cb8dcef13a1fa04d41795aeb5c82))
* **router:** 修复侧边栏快速点击导航卡死问题 ([8b1f862](https://github.com/dyndynjyxa/aio-coding-hub/commit/8b1f862542b8bd94770ab5f937772a53a4bd1d22))
* **scripts:** 修复生成绑定文件格式化问题 ([9ff697b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9ff697b6b8c93238f48518f27f274df33ba55a9a))
* **sessions:** 剥离替换字符并调整按钮样式 ([3586a16](https://github.com/dyndynjyxa/aio-coding-hub/commit/3586a165ceb8ba3afc42a34fdf4228c71b18cce5))
* **settings:** 增强设置读取错误处理和写入保护 ([6759e60](https://github.com/dyndynjyxa/aio-coding-hub/commit/6759e60edf1cb93e63f8c05787ed4f0a320a175b))
* **settings:** 设置默认关闭Billing Header整流器 ([9331253](https://github.com/dyndynjyxa/aio-coding-hub/commit/933125358a9574225cd1f317fcdc9d6b1679c6c4))
* **settings:** 限制上游流式空闲超时最小值为60秒 ([e7284c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/e7284c251950bcfa27db72385603756472329646))
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
* **ui:** 修复 Sidebar 组件状态文字显示异常问题 ([ff0b1ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/ff0b1ae74e8c08fa5c73044fa924e7c78e24284d))
* **update:** restore changelog and local preview flow ([#178](https://github.com/dyndynjyxa/aio-coding-hub/issues/178)) ([36a564d](https://github.com/dyndynjyxa/aio-coding-hub/commit/36a564dcaa33a6bd9c76a9bda30599a1d3f92c52))
* **usage:** 修复 Usage 统计中的 CX2CC 缓存命中率计算 ([#159](https://github.com/dyndynjyxa/aio-coding-hub/issues/159)) ([3472560](https://github.com/dyndynjyxa/aio-coding-hub/commit/3472560f62f88e4eae4b4d53e920c290869a2c7d))
* **utils:** 修正computeOutputTokensPerSecond回退逻辑 ([7c59559](https://github.com/dyndynjyxa/aio-coding-hub/commit/7c59559ad0b525e478577d74518173753530a980))
* **util:** 增加请求体大小限制至100MB并更新体积错误消息 ([b12c1ba](https://github.com/dyndynjyxa/aio-coding-hub/commit/b12c1baeccbfd29b8bcc4a055e1b6d7764d6d3dd))
* WebView2 不可恢复状态检测与分级恢复 ([#156](https://github.com/dyndynjyxa/aio-coding-hub/issues/156)) ([4869745](https://github.com/dyndynjyxa/aio-coding-hub/commit/4869745cb352279829317f18b5e1956c381b5b38))
* **wsl:** atomic write, remove dead fallback, add TOML comment ([7decfb5](https://github.com/dyndynjyxa/aio-coding-hub/commit/7decfb5803378ee3bfb5433bc88adb64562a6857))
* **wsl:** 修复 WSL 脚本错误消息编码和路径写入问题 ([bccd5c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/bccd5c93110641898e04cd1cde031df604af637b))
* **wsl:** 修复WSL适配器IPv4解析和状态脚本处理 ([06ae9b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/06ae9b1f462572550f327c3afe4034179b1da491))
* **wsl:** 修正同步边界与 sessions 状态 ([cc41840](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc41840fb88744c926544436cf885b440f240403))
* 修复clippy redundant_pattern_matching警告 ([b0274a6](https://github.com/dyndynjyxa/aio-coding-hub/commit/b0274a663396a2958e470e2167933470bf34a70a))


### Performance Improvements

* **home:** 优化实时追踪卡片动画与样式过渡效果 ([48c9966](https://github.com/dyndynjyxa/aio-coding-hub/commit/48c99669d5658884bc594dcebdb35219d0c25e23))


### Code Refactoring

* **infra:** 移除 request_attempt_logs 独立表，改用 request_logs.attempts_json 派生 ([3fb2627](https://github.com/dyndynjyxa/aio-coding-hub/commit/3fb2627407e1b9b57758a8a40d3a7d7393278e1b))

## [0.38.4](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.38.3...aio-coding-hub-v0.38.4) (2026-04-17)


### Features

* **cli-manager:** 在通用配置页新增上游代理相关设置支持 ([0b2489f](https://github.com/dyndynjyxa/aio-coding-hub/commit/0b2489f4c201af5daba697b4f272ca6de5767e54))


### Bug Fixes

* **mcp:** 修复 MCP Server Dialog 支持 SSE 类型和保存逻辑 ([a219783](https://github.com/dyndynjyxa/aio-coding-hub/commit/a2197836c2aa9a431be0e2c8bc8b243dad2072fd))
* **ui:** 修复 Sidebar 组件状态文字显示异常问题 ([ff0b1ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/ff0b1ae74e8c08fa5c73044fa924e7c78e24284d))

## [0.38.3](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.38.2...aio-coding-hub-v0.38.3) (2026-04-16)


### Features

* 首页新增 Token 用量面板 ([#204](https://github.com/dyndynjyxa/aio-coding-hub/issues/204)) ([4513106](https://github.com/dyndynjyxa/aio-coding-hub/commit/45131069f2d67a0c909c1558102305f1cfe41028))


### Bug Fixes

* **util:** 增加请求体大小限制至100MB并更新体积错误消息 ([b12c1ba](https://github.com/dyndynjyxa/aio-coding-hub/commit/b12c1baeccbfd29b8bcc4a055e1b6d7764d6d3dd))

## [0.38.2](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.38.1...aio-coding-hub-v0.38.2) (2026-04-16)


### Features

* **skills:** 支持推理强度新增等级xhigh ([36dbb77](https://github.com/dyndynjyxa/aio-coding-hub/commit/36dbb773c94f7acaa9d1a9e6ed68d8df02d6c950))

## [0.38.1](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.38.0...aio-coding-hub-v0.38.1) (2026-04-12)


### Features

* **provider-chain:** 优化展示故障切换尝试详情和错误结构化信息 ([e2d530f](https://github.com/dyndynjyxa/aio-coding-hub/commit/e2d530fceb3fcf52e9a7745847b13963e854994c))

## [0.38.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.37.7...aio-coding-hub-v0.38.0) (2026-04-12)


### ⚠ BREAKING CHANGES

* **infra:** request_attempt_logs 表不再创建或写入

### Features

* add Claude OAuth support with multi-account switching ([#149](https://github.com/dyndynjyxa/aio-coding-hub/issues/149)) ([ef787b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef787b1ddf309ffcf9ffcb923d92bf2af3f557b6))
* add CX2CC tab, config export/import, Gemini config enhance, CLI… ([#138](https://github.com/dyndynjyxa/aio-coding-hub/issues/138)) ([7aba381](https://github.com/dyndynjyxa/aio-coding-hub/commit/7aba381a462812e6c6ea505de3650397d23b48b4))
* add gemini oauth code assist proxy support ([4a892d0](https://github.com/dyndynjyxa/aio-coding-hub/commit/4a892d035fbcdd5273c909de063eede5d42598fa))
* **app:** add LogsPage route and enhance CLI manager settings ([6796715](https://github.com/dyndynjyxa/aio-coding-hub/commit/679671526be935129211978b3598a3962f7e2a8e))
* **app:** 优化页面路由懒加载和加载反馈体验 ([504256e](https://github.com/dyndynjyxa/aio-coding-hub/commit/504256e11ed517797405f8bb4ab3d353607cb1ae))
* **app:** 添加应用心跳机制与监听支持 ([df78171](https://github.com/dyndynjyxa/aio-coding-hub/commit/df781712d16a2f5ccc99e37a687d39cb4e4f56c5))
* **bindings:** 添加 HomeUsagePeriod 类型契约断言 ([a540d52](https://github.com/dyndynjyxa/aio-coding-hub/commit/a540d52012652075deee18c58d3e4b5611f4d547))
* **build:** 添加Wayland兼容的AppImage构建支持 ([95c7130](https://github.com/dyndynjyxa/aio-coding-hub/commit/95c71307038962fc0e37fbd7b6df844301d8decb))
* **build:** 添加Windows便携版ZIP构建和WebView2检查 ([fef41e1](https://github.com/dyndynjyxa/aio-coding-hub/commit/fef41e12f2a1301743b40de4c7402f4b02fa80f4))
* **cache-anomaly-monitor:** implement cache anomaly monitoring feature ([59d69d1](https://github.com/dyndynjyxa/aio-coding-hub/commit/59d69d1528d888a84d71038873321f3273d4ded1))
* **charts:** migrate from ECharts to Recharts for improved charting capabilities ([b747b61](https://github.com/dyndynjyxa/aio-coding-hub/commit/b747b61a8dd587043a7e8de01a5f9b3d64ead7ae))
* **circuit-breaker:** 支持运行时动态更新断路器配置 ([3cb8722](https://github.com/dyndynjyxa/aio-coding-hub/commit/3cb87220345020ea8916f192ac6c84281d2736f2))
* **claude-validation:** add signature and caching roundtrip probes ([15badee](https://github.com/dyndynjyxa/aio-coding-hub/commit/15badee08b0c14f71695e6e71f0b165e4844371c))
* **ClaudeModelValidationDialog:** 添加综合协议验证功能与UI展示 ([b6ede30](https://github.com/dyndynjyxa/aio-coding-hub/commit/b6ede30e61e19f9af69e8d7153cbc711b3e3a180))
* **ClaudeModelValidation:** enhance cross-provider validation and UI feedback ([bf83c7e](https://github.com/dyndynjyxa/aio-coding-hub/commit/bf83c7e03c7edf78795cd51a943c01a88e0b17d7))
* **ClaudeModelValidation:** enhance output token validation and error handling ([d245288](https://github.com/dyndynjyxa/aio-coding-hub/commit/d245288d7a4937ca7b0213ebd79d9c0d5e3c76b4))
* **ClaudeModelValidation:** implement cross-provider signature validation and enhance request handling ([2e102d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/2e102d4f3fd2745e4480a5884272baeafe66b6d0))
* **claudeModelValidation:** 优化模型验证对话框和多轮验证流程支持 ([3f6c031](https://github.com/dyndynjyxa/aio-coding-hub/commit/3f6c03102db7aaeaa2d8399ebdac1cc4787c2a84))
* **cli_proxy:** 添加 merge-restore 功能以保留用户更改 ([a7d05c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/a7d05c97fe1228d83ccf97be5ed9b6ec26c7bde0))
* **cli_proxy:** 添加自动同步以修复代理配置漂移问题 ([0a7b856](https://github.com/dyndynjyxa/aio-coding-hub/commit/0a7b856e87d62fd3be84f6314a89f3f1a6706e92))
* **cli_sessions:** 优化 Claude-CLI 项目路径解析和工作目录提取 ([a0024ac](https://github.com/dyndynjyxa/aio-coding-hub/commit/a0024acf6c2e3bc1615d304d6ae681fbb4550371))
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
* **cli-manager:** 支持 Claude Code 环境变量配置并修复网关重置缓存 ([#192](https://github.com/dyndynjyxa/aio-coding-hub/issues/192)) ([b41a2cf](https://github.com/dyndynjyxa/aio-coding-hub/commit/b41a2cfad6a964fe3f7dd9bd97a3db523bcc59b4))
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
* **deps:** 新增 react-markdown 和 tailwindcss 排版插件依赖 ([a58f747](https://github.com/dyndynjyxa/aio-coding-hub/commit/a58f7477ba4c35f4f0c5b4812d985c8294cf8cf3))
* **deps:** 添加 @mdxeditor/editor 依赖 ([8da488a](https://github.com/dyndynjyxa/aio-coding-hub/commit/8da488a5c0296b793eb184d0a081f09eda14af2e))
* **domain:** 优化按小时统计总 token 计算方式 ([8f3df70](https://github.com/dyndynjyxa/aio-coding-hub/commit/8f3df7039eb5b66d1780359e0ecbface88d3bec9))
* **domain:** 支持 cx2cc 提供者及缓存令牌追踪 ([3662c26](https://github.com/dyndynjyxa/aio-coding-hub/commit/3662c26c2ea72c25ef7e330a52a414a708164cd1))
* **failover-loop:** refactor context management for improved clarity and usability ([b96df01](https://github.com/dyndynjyxa/aio-coding-hub/commit/b96df013317c77ca610bb640a43b2d60058e6847))
* **failover-loop:** refactor error handling and logging in failover loop ([ef2662c](https://github.com/dyndynjyxa/aio-coding-hub/commit/ef2662c335ae4057c591b531b0784c86cd73951e))
* **gateway,providers,oauth:** codex-oauth-with-proxy ([#76](https://github.com/dyndynjyxa/aio-coding-hub/issues/76)) ([f37e59a](https://github.com/dyndynjyxa/aio-coding-hub/commit/f37e59a3dbfdc8a95d18e592f1931c775f19c2ab))
* **gateway:** Circuit Breaker 引入 5 分钟滑动窗口衰减与 HalfOpen 渐进恢复 ([f90e59e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f90e59e3e72b5ff889ce37c1e8e9a225f15e7de3))
* **gateway:** Session TTL 改为滑动窗口，每次使用自动续期 ([3a6cb5c](https://github.com/dyndynjyxa/aio-coding-hub/commit/3a6cb5c631c733133c96aa172be7e445d85444dd))
* **gateway:** 优化协议兼容性处理与请求头管理 ([6be9161](https://github.com/dyndynjyxa/aio-coding-hub/commit/6be91613fbc0bfcd9b87fe419a8f9ad08178a1f7))
* **gateway:** 增强上游错误处理与界面错误详情展示 ([85f67c4](https://github.com/dyndynjyxa/aio-coding-hub/commit/85f67c4c1ffe08d102883458a2d9d29e0b2b98cf))
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
* integrate PageHeader component across multiple pages for consistent UI ([330da27](https://github.com/dyndynjyxa/aio-coding-hub/commit/330da276f9ef8e91744a9534d59590a3a6fec5ff))
* mac最小化优化 ([1f11a5d](https://github.com/dyndynjyxa/aio-coding-hub/commit/1f11a5df9623f8cab90bb40f5c499c5d3569f01d))
* **mcp:** implement batch import for local skills and add global error reporting ([51e053c](https://github.com/dyndynjyxa/aio-coding-hub/commit/51e053c81f1db00696f0cb968860033deee12cce))
* **mcp:** 优化服务器键生成与导入逻辑，支持保留大小写 ([f61a5c6](https://github.com/dyndynjyxa/aio-coding-hub/commit/f61a5c6be09795efaf9c4aafe7fee5f86329c0ea))
* **mcp:** 添加 env 和 header 键值对格式校验 ([7d698ca](https://github.com/dyndynjyxa/aio-coding-hub/commit/7d698ca04f1cc9f9c0d420f6d7fec2a53f2d643a))
* **Oauth:** adding gemini Oauth support ([fd1eb79](https://github.com/dyndynjyxa/aio-coding-hub/commit/fd1eb7933410510e9ee46e469234d91658a49fcf))
* **provider-chain:** 优化供应商链路视图及日志详情显示 ([72d4bc4](https://github.com/dyndynjyxa/aio-coding-hub/commit/72d4bc49dfd1380ce503ec23a98eb81b55a1d488))
* **provider-editor:** enhance ProviderEditorDialog with limit configuration cards ([cc14a00](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc14a009303129793ceec840933155dcfad775d8))
* **providers:** 在供应商列表页增加名称搜索功能 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **providers:** 增加供应商备注字段并支持读取API Key ([fec7057](https://github.com/dyndynjyxa/aio-coding-hub/commit/fec7057d137a7c0b38b46b5591a9e19e624b679a))
* **providers:** 实现供应商编辑器和多功能配置组件 ([e9f2a45](https://github.com/dyndynjyxa/aio-coding-hub/commit/e9f2a45f64b30c12b66ddad13cf378cccfd4d3a7))
* **providers:** 支持复制供应商配置并预填创建表单 ([c8fb707](https://github.com/dyndynjyxa/aio-coding-hub/commit/c8fb7078131ad67e4d2895f81467a9358b897d2c))
* **providers:** 添加 OAuth 限制重置倒计时 & 修复托盘退出重启问题 ([#168](https://github.com/dyndynjyxa/aio-coding-hub/issues/168)) ([4ca1248](https://github.com/dyndynjyxa/aio-coding-hub/commit/4ca124860c48fd3764ecf1da861adef072267ba7))
* **providers:** 添加供应商标签功能，支持分类筛选 ([7671732](https://github.com/dyndynjyxa/aio-coding-hub/commit/76717329a119f9a65dc2f3a9dffe0a887063b0bd))
* **proxy:** 丰富错误详情上下文并优化请求记录错误展示 ([1ac8eef](https://github.com/dyndynjyxa/aio-coding-hub/commit/1ac8eefbf6107de383ee349abb5ee0e89c774a6f))
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
* **tauri:** update error handling and add thiserror dependency ([10d918a](https://github.com/dyndynjyxa/aio-coding-hub/commit/10d918a0a0a746b4d25c1c236f88c0110ec664c3))
* **tests:** add comprehensive unit tests for various components ([98fb022](https://github.com/dyndynjyxa/aio-coding-hub/commit/98fb022d1487e593e1fe50a1e9d02592671a4944))
* **theme:** add dark mode and native window theme sync ([588a373](https://github.com/dyndynjyxa/aio-coding-hub/commit/588a37311c456bd2182ac1676c8e9b309157353f))
* **ui:** 优化首页最近代理记录卡片展示与预览数据 ([#181](https://github.com/dyndynjyxa/aio-coding-hub/issues/181)) ([c5a0068](https://github.com/dyndynjyxa/aio-coding-hub/commit/c5a0068ac81e6d97fd9a021c346ee7f0024da42d))
* **ui:** 优化首页概览、CLI 代理状态与供应商页交互 ([#106](https://github.com/dyndynjyxa/aio-coding-hub/issues/106)) ([358e73f](https://github.com/dyndynjyxa/aio-coding-hub/commit/358e73f598fd48c7bbf131a3117e4801cec03ced))
* **ui:** 优化首页概览、供应商管理与设置页主题入口 ([#102](https://github.com/dyndynjyxa/aio-coding-hub/issues/102)) ([0adec0a](https://github.com/dyndynjyxa/aio-coding-hub/commit/0adec0af676842e9c5aee3e08e3a46d890d31c11))
* **ui:** 本次主要优化了首页概览、设置页和代理记录页，同时顺手处理了首页包体和 Tauri 构建告警。 ([#111](https://github.com/dyndynjyxa/aio-coding-hub/issues/111)) ([4e6dbad](https://github.com/dyndynjyxa/aio-coding-hub/commit/4e6dbad14cab8334d47a3a30ebfcf70485ad2e0d))
* **ui:** 统一 Loading/Empty/Error 状态反馈组件 ([b27f64a](https://github.com/dyndynjyxa/aio-coding-hub/commit/b27f64a79a1ad655a37992527794759f9f8c5370))
* **ui:** 设置与首页展示优化，并修复 Node 24 下 pre-push 单测问题 ([#169](https://github.com/dyndynjyxa/aio-coding-hub/issues/169)) ([165db18](https://github.com/dyndynjyxa/aio-coding-hub/commit/165db18dce650cee175a8df07f9bc80168575aab))
* **ui:** 首页最近代理记录与配置信息展示优化 ([#185](https://github.com/dyndynjyxa/aio-coding-hub/issues/185)) ([84d98ef](https://github.com/dyndynjyxa/aio-coding-hub/commit/84d98ef1b0489bb64e83f577c355a4780fe07227))
* **updater:** display release notes in update dialog ([b891ed7](https://github.com/dyndynjyxa/aio-coding-hub/commit/b891ed75a1b67a9df281917ffa94e684fb372664))
* **update:** 更新对话框展示更新日志 ([#171](https://github.com/dyndynjyxa/aio-coding-hub/issues/171)) ([c55ccb2](https://github.com/dyndynjyxa/aio-coding-hub/commit/c55ccb2cb1fc18b1e109f41192a4362f3a4ff8af))
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
* 支持 CX2CC 使用当前 AIO 服务 Codex 网关作为来源 ([#194](https://github.com/dyndynjyxa/aio-coding-hub/issues/194)) ([2bf7117](https://github.com/dyndynjyxa/aio-coding-hub/commit/2bf7117585f20e03971831a04bd721fb6f620d67))


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
* **ci:** 修复Rust测试线程数为单线程运行 ([87ec1fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/87ec1fe962d7b2913841e739e3ee7026f7145c36))
* **ci:** 修复正则表达式转义字符问题 ([ba805c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba805c3b2f147f153cbe39042b078547eb36da5a))
* **ci:** 修正发布流程中更新器 JSON 文件参数名称 ([9a4c0f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/9a4c0f063bd645e3286e67f1607334a84a5cb6fd))
* **claude-model-validation:** 优化界面中文提示文本 ([57651b8](https://github.com/dyndynjyxa/aio-coding-hub/commit/57651b8bafc32c782733c938b44c95a423216b79))
* **ClaudeModelValidationDialog:** adjust dialog max-width for responsive design ([11bdfd0](https://github.com/dyndynjyxa/aio-coding-hub/commit/11bdfd0df1b51923a561211f6cc6d5383eedf76c))
* **cli_manager:** 修复 run_in_login_shell 的平台兼容性处理 ([9f81433](https://github.com/dyndynjyxa/aio-coding-hub/commit/9f814337d08f36c08e37486f95b9c9da62880fc1))
* **cli_proxy:** 修复数据库初始化错误处理逻辑 ([9230475](https://github.com/dyndynjyxa/aio-coding-hub/commit/9230475e4401e93e3dc34ad285ace4a03186cd16))
* **cli_update:** 修复提取语义版本字符串的字符截取逻辑 ([782fb95](https://github.com/dyndynjyxa/aio-coding-hub/commit/782fb95d05236afbbfdf80b4bfb15921b9c34b88))
* **cli-manager:** 修正模型参数解析逻辑以避免默认值覆盖 ([ae2b7fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae2b7feecfd6e06462b8b0990204fc55d6ad3ccf))
* **cli-proxy:** 修复 Codex CLI 在 Windows 下的沙箱配置与认证模式 ([b79d072](https://github.com/dyndynjyxa/aio-coding-hub/commit/b79d072f39405c15125a8a6391bff122c9a5f315))
* **cli-sessions:** 修复SessionsPage Card容器flex布局 ([2d2a048](https://github.com/dyndynjyxa/aio-coding-hub/commit/2d2a0481f9811160223f9753d6edec5a28dcfc6c))
* **codex-tab:** sandbox_mode danger-full-access 选择不生效及高级配置不回显 ([3943401](https://github.com/dyndynjyxa/aio-coding-hub/commit/3943401524c8ef6a179e65e8ee62fd2ac4f784cb))
* **codex:** resolve oauth login failure and stale provider names ([#116](https://github.com/dyndynjyxa/aio-coding-hub/issues/116)) ([48d647e](https://github.com/dyndynjyxa/aio-coding-hub/commit/48d647ef429232562c9b56e88d8a37f57960604a))
* **codex:** 修复 Pi 本地 endpoint 与 Responses 兼容约束 ([#180](https://github.com/dyndynjyxa/aio-coding-hub/issues/180)) ([8e7085e](https://github.com/dyndynjyxa/aio-coding-hub/commit/8e7085e69d1aabca28d398190239e99bcab4fe03))
* **codex:** 修复切换 Codex Home 后代理状态与实际配置不一致 ([#125](https://github.com/dyndynjyxa/aio-coding-hub/issues/125)) ([e8f80b2](https://github.com/dyndynjyxa/aio-coding-hub/commit/e8f80b2838509b748d38ed6411219218b4077bf5))
* **commands:** 修复构建Claude启动命令时路径参数传递 ([9e714f4](https://github.com/dyndynjyxa/aio-coding-hub/commit/9e714f48bdbf7dff2d729131774518845303290a))
* **components:** 修复ClaudeModelValidationDialog点击事件阻塞问题 ([9bd5f53](https://github.com/dyndynjyxa/aio-coding-hub/commit/9bd5f53f38d01fc312b979e3620b5e0badaba585))
* **cost:** price CX2CC requests with translated model basis ([#175](https://github.com/dyndynjyxa/aio-coding-hub/issues/175)) ([342f585](https://github.com/dyndynjyxa/aio-coding-hub/commit/342f585c9b713077431e83ada0d008550fa5f708))
* **cost:** 修正成本乘数比较逻辑为包含零值 ([db69776](https://github.com/dyndynjyxa/aio-coding-hub/commit/db6977635f96fc55fa1b4d84d1d71e67dd8dd0b1))
* **db:** 修复providers表note字段迁移及数据处理问题 ([fa8ecda](https://github.com/dyndynjyxa/aio-coding-hub/commit/fa8ecda9bad231ed424c812c1c7cd3d0850b8565))
* **deps:** 升级 react-router-dom 7.11→7.13 修复 XSS 漏洞 ([c4db630](https://github.com/dyndynjyxa/aio-coding-hub/commit/c4db630b4478cab2f1180f03b324cfc0af9d42e3))
* **domain:** 修复 SSE 流错误处理逻辑 ([4446939](https://github.com/dyndynjyxa/aio-coding-hub/commit/4446939c457735727f96dce0b09c91ae6b616c1d))
* **domain:** 修正 claude_terminal_launch_context 函数中 provider_id 验证和查询逻辑 ([316d375](https://github.com/dyndynjyxa/aio-coding-hub/commit/316d3754fc628d0a1e435aabf469ca992cb63efa))
* **failover:** 区分网关过滤与上游请求失败 ([441b6f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/441b6f0d84565bdd21b8d52334aaedd60425ebef))
* **gateway:** allow failover success handler arg count ([0c559cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/0c559cba801d59332a94eb650571487b6b18a53b))
* **gateway:** claude api-key auth fallback ([ac68780](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac687808a15cc8d412714d623e4a32ef10ece310))
* **gateway:** finalize failed requests and align error codes ([871de9c](https://github.com/dyndynjyxa/aio-coding-hub/commit/871de9c9c9e06ed6cb03026ceb21dfc4b0ba18d3))
* **gateway:** 修复 CX2CC 无 SSE 响应头成功响应的分类时机 ([#160](https://github.com/dyndynjyxa/aio-coding-hub/issues/160)) ([9edcd5b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9edcd5bf8396a0a2a84e5b368d984717b4d7c81f))
* **gateway:** 修复 HALF_OPEN 状态下熔断未及时解除的前端展示问题 ([#166](https://github.com/dyndynjyxa/aio-coding-hub/issues/166)) ([4c809f9](https://github.com/dyndynjyxa/aio-coding-hub/commit/4c809f9b0b6aee4e426d8f3a6d2aa53699fd7b35))
* **gateway:** 修正 Heartbeat 事件变量可见性及用法 ([a540d52](https://github.com/dyndynjyxa/aio-coding-hub/commit/a540d52012652075deee18c58d3e4b5611f4d547))
* **gateway:** 修正 provider 重新排序时清除会话绑定逻辑 ([f03c02e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f03c02e5332fe84496a029b2e5d130b558cfad31))
* **gateway:** 调整上游连接超时与错误切换逻辑 ([a6870ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6870ae14b6b5bb8edce814ca37da2000e84f6b4))
* **heartbeat_watchdog:** reload 返回 Ok 但实际异步失败时升级到窗口重建 ([#172](https://github.com/dyndynjyxa/aio-coding-hub/issues/172)) ([e4f77b0](https://github.com/dyndynjyxa/aio-coding-hub/commit/e4f77b01c7204767b61d8bb787a374b270383736))
* **heartbeat_watchdog:** 优化窗口重载的错误处理逻辑 ([6188e6c](https://github.com/dyndynjyxa/aio-coding-hub/commit/6188e6c29135c0126490882ceb54302d112ec79c))
* **home:** update HomeCostPanel with accessibility labels and improve test assertions ([0bda6c1](https://github.com/dyndynjyxa/aio-coding-hub/commit/0bda6c1ce8aec6920b30810882f3a804c62ab732))
* **macOS:** 修复macOS 在 CLI 管理页面读取不到工具 version 并提示 INTERNAL_ERROR ([#126](https://github.com/dyndynjyxa/aio-coding-hub/issues/126)) ([ce35a94](https://github.com/dyndynjyxa/aio-coding-hub/commit/ce35a945dd278b453a1ddebbe8e95fd5f4f12ee6))
* **mcp:** 修复导入服务器时基于 server_key 的重复处理 ([f6e6458](https://github.com/dyndynjyxa/aio-coding-hub/commit/f6e64582e1e4c478ec014e61258611d971f8b3b4))
* **notification:** 优化系统通知及心跳监测逻辑 ([cfeb63f](https://github.com/dyndynjyxa/aio-coding-hub/commit/cfeb63f6b331df3be3feddbf0aac258018dba824))
* **oauth:** 调整 OAuth 刷新周期并移除 CLI 管理页 Claude OAuth 卡片 ([#184](https://github.com/dyndynjyxa/aio-coding-hub/issues/184)) ([3640ec7](https://github.com/dyndynjyxa/aio-coding-hub/commit/3640ec7853d81ba9dce8d8e4a049f0319f4827af))
* **prompts:** 修改新增和编辑时 Prompt 保存按钮禁用条件 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **providers:** 敏感配置信息变更时清理运行时 session 绑定 ([a8df6f1](https://github.com/dyndynjyxa/aio-coding-hub/commit/a8df6f1e9bfef48ded10f0175fb95ee028e933a5))
* **proxy:** 优化未匹配客户端错误的中止逻辑 ([05eb435](https://github.com/dyndynjyxa/aio-coding-hub/commit/05eb435e499786d8308f548d5115adf378adc7f6))
* **proxy:** 允许 resolve_session_bound_provider_id 函数有多个参数 ([705457e](https://github.com/dyndynjyxa/aio-coding-hub/commit/705457eac5092ccd98a0f3523ea1986c67dd9d9c))
* **release:** keep Cargo.lock synced for release PRs ([c5bd423](https://github.com/dyndynjyxa/aio-coding-hub/commit/c5bd42355641cb8dcef13a1fa04d41795aeb5c82))
* **router:** 修复侧边栏快速点击导航卡死问题 ([8b1f862](https://github.com/dyndynjyxa/aio-coding-hub/commit/8b1f862542b8bd94770ab5f937772a53a4bd1d22))
* **scripts:** 修复生成绑定文件格式化问题 ([9ff697b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9ff697b6b8c93238f48518f27f274df33ba55a9a))
* **sessions:** 剥离替换字符并调整按钮样式 ([3586a16](https://github.com/dyndynjyxa/aio-coding-hub/commit/3586a165ceb8ba3afc42a34fdf4228c71b18cce5))
* **settings:** 增强设置读取错误处理和写入保护 ([6759e60](https://github.com/dyndynjyxa/aio-coding-hub/commit/6759e60edf1cb93e63f8c05787ed4f0a320a175b))
* **settings:** 设置默认关闭Billing Header整流器 ([9331253](https://github.com/dyndynjyxa/aio-coding-hub/commit/933125358a9574225cd1f317fcdc9d6b1679c6c4))
* **settings:** 限制上游流式空闲超时最小值为60秒 ([e7284c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/e7284c251950bcfa27db72385603756472329646))
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
* **update:** restore changelog and local preview flow ([#178](https://github.com/dyndynjyxa/aio-coding-hub/issues/178)) ([36a564d](https://github.com/dyndynjyxa/aio-coding-hub/commit/36a564dcaa33a6bd9c76a9bda30599a1d3f92c52))
* **usage:** align cache creation ttl to 5m only ([8d28bcd](https://github.com/dyndynjyxa/aio-coding-hub/commit/8d28bcd2f5d7f8d6bac1a7f65f974c04c5fce337))
* **usage:** 修复 Usage 统计中的 CX2CC 缓存命中率计算 ([#159](https://github.com/dyndynjyxa/aio-coding-hub/issues/159)) ([3472560](https://github.com/dyndynjyxa/aio-coding-hub/commit/3472560f62f88e4eae4b4d53e920c290869a2c7d))
* **utils:** 修正computeOutputTokensPerSecond回退逻辑 ([7c59559](https://github.com/dyndynjyxa/aio-coding-hub/commit/7c59559ad0b525e478577d74518173753530a980))
* WebView2 不可恢复状态检测与分级恢复 ([#156](https://github.com/dyndynjyxa/aio-coding-hub/issues/156)) ([4869745](https://github.com/dyndynjyxa/aio-coding-hub/commit/4869745cb352279829317f18b5e1956c381b5b38))
* **wsl:** atomic write, remove dead fallback, add TOML comment ([7decfb5](https://github.com/dyndynjyxa/aio-coding-hub/commit/7decfb5803378ee3bfb5433bc88adb64562a6857))
* **wsl:** 修复 WSL 脚本错误消息编码和路径写入问题 ([bccd5c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/bccd5c93110641898e04cd1cde031df604af637b))
* **wsl:** 修复WSL适配器IPv4解析和状态脚本处理 ([06ae9b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/06ae9b1f462572550f327c3afe4034179b1da491))
* **wsl:** 修正同步边界与 sessions 状态 ([cc41840](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc41840fb88744c926544436cf885b440f240403))
* 修复clippy redundant_pattern_matching警告 ([b0274a6](https://github.com/dyndynjyxa/aio-coding-hub/commit/b0274a663396a2958e470e2167933470bf34a70a))


### Performance Improvements

* **home:** 优化实时追踪卡片动画与样式过渡效果 ([48c9966](https://github.com/dyndynjyxa/aio-coding-hub/commit/48c99669d5658884bc594dcebdb35219d0c25e23))


### Code Refactoring

* **infra:** 移除 request_attempt_logs 独立表，改用 request_logs.attempts_json 派生 ([3fb2627](https://github.com/dyndynjyxa/aio-coding-hub/commit/3fb2627407e1b9b57758a8a40d3a7d7393278e1b))

## [0.37.7](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.37.6...aio-coding-hub-v0.37.7) (2026-04-11)


### Features

* **deps:** 添加 @mdxeditor/editor 依赖 ([8da488a](https://github.com/dyndynjyxa/aio-coding-hub/commit/8da488a5c0296b793eb184d0a081f09eda14af2e))

## [0.37.6](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.37.5...aio-coding-hub-v0.37.6) (2026-04-11)


### Features

* **providers:** 实现供应商编辑器和多功能配置组件 ([e9f2a45](https://github.com/dyndynjyxa/aio-coding-hub/commit/e9f2a45f64b30c12b66ddad13cf378cccfd4d3a7))
* 支持 CX2CC 使用当前 AIO 服务 Codex 网关作为来源 ([#194](https://github.com/dyndynjyxa/aio-coding-hub/issues/194)) ([2bf7117](https://github.com/dyndynjyxa/aio-coding-hub/commit/2bf7117585f20e03971831a04bd721fb6f620d67))

## [0.37.5](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.37.4...aio-coding-hub-v0.37.5) (2026-04-10)


### Features

* **cli-manager:** 支持 Claude Code 环境变量配置并修复网关重置缓存 ([#192](https://github.com/dyndynjyxa/aio-coding-hub/issues/192)) ([b41a2cf](https://github.com/dyndynjyxa/aio-coding-hub/commit/b41a2cfad6a964fe3f7dd9bd97a3db523bcc59b4))
* **gateway:** Circuit Breaker 引入 5 分钟滑动窗口衰减与 HalfOpen 渐进恢复 ([f90e59e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f90e59e3e72b5ff889ce37c1e8e9a225f15e7de3))
* **gateway:** Session TTL 改为滑动窗口，每次使用自动续期 ([3a6cb5c](https://github.com/dyndynjyxa/aio-coding-hub/commit/3a6cb5c631c733133c96aa172be7e445d85444dd))

## [0.37.4](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.37.3...aio-coding-hub-v0.37.4) (2026-04-08)


### Bug Fixes

* **proxy:** 优化未匹配客户端错误的中止逻辑 ([05eb435](https://github.com/dyndynjyxa/aio-coding-hub/commit/05eb435e499786d8308f548d5115adf378adc7f6))

## [0.37.3](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.37.2...aio-coding-hub-v0.37.3) (2026-04-08)


### Features

* **cli_sessions:** 优化 Claude-CLI 项目路径解析和工作目录提取 ([a0024ac](https://github.com/dyndynjyxa/aio-coding-hub/commit/a0024acf6c2e3bc1615d304d6ae681fbb4550371))


### Bug Fixes

* **failover:** 区分网关过滤与上游请求失败 ([441b6f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/441b6f0d84565bdd21b8d52334aaedd60425ebef))
* **settings:** 限制上游流式空闲超时最小值为60秒 ([e7284c2](https://github.com/dyndynjyxa/aio-coding-hub/commit/e7284c251950bcfa27db72385603756472329646))

## [0.37.2](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.37.1...aio-coding-hub-v0.37.2) (2026-04-07)


### Features

* **deps:** 新增 react-markdown 和 tailwindcss 排版插件依赖 ([a58f747](https://github.com/dyndynjyxa/aio-coding-hub/commit/a58f7477ba4c35f4f0c5b4812d985c8294cf8cf3))
* **domain:** 优化按小时统计总 token 计算方式 ([8f3df70](https://github.com/dyndynjyxa/aio-coding-hub/commit/8f3df7039eb5b66d1780359e0ecbface88d3bec9))
* **proxy:** 丰富错误详情上下文并优化请求记录错误展示 ([1ac8eef](https://github.com/dyndynjyxa/aio-coding-hub/commit/1ac8eefbf6107de383ee349abb5ee0e89c774a6f))
* **ui:** 首页最近代理记录与配置信息展示优化 ([#185](https://github.com/dyndynjyxa/aio-coding-hub/issues/185)) ([84d98ef](https://github.com/dyndynjyxa/aio-coding-hub/commit/84d98ef1b0489bb64e83f577c355a4780fe07227))


### Bug Fixes

* **claude-model-validation:** 优化界面中文提示文本 ([57651b8](https://github.com/dyndynjyxa/aio-coding-hub/commit/57651b8bafc32c782733c938b44c95a423216b79))
* **oauth:** 调整 OAuth 刷新周期并移除 CLI 管理页 Claude OAuth 卡片 ([#184](https://github.com/dyndynjyxa/aio-coding-hub/issues/184)) ([3640ec7](https://github.com/dyndynjyxa/aio-coding-hub/commit/3640ec7853d81ba9dce8d8e4a049f0319f4827af))

## [0.37.1](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.37.0...aio-coding-hub-v0.37.1) (2026-04-06)


### Features

* **ui:** 优化首页最近代理记录卡片展示与预览数据 ([#181](https://github.com/dyndynjyxa/aio-coding-hub/issues/181)) ([c5a0068](https://github.com/dyndynjyxa/aio-coding-hub/commit/c5a0068ac81e6d97fd9a021c346ee7f0024da42d))


### Bug Fixes

* **codex:** 修复 Pi 本地 endpoint 与 Responses 兼容约束 ([#180](https://github.com/dyndynjyxa/aio-coding-hub/issues/180)) ([8e7085e](https://github.com/dyndynjyxa/aio-coding-hub/commit/8e7085e69d1aabca28d398190239e99bcab4fe03))
* **update:** restore changelog and local preview flow ([#178](https://github.com/dyndynjyxa/aio-coding-hub/issues/178)) ([36a564d](https://github.com/dyndynjyxa/aio-coding-hub/commit/36a564dcaa33a6bd9c76a9bda30599a1d3f92c52))

## [0.37.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.36.0...aio-coding-hub-v0.37.0) (2026-04-05)


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
* **circuit-breaker:** 支持运行时动态更新断路器配置 ([3cb8722](https://github.com/dyndynjyxa/aio-coding-hub/commit/3cb87220345020ea8916f192ac6c84281d2736f2))
* **claude-validation:** add signature and caching roundtrip probes ([15badee](https://github.com/dyndynjyxa/aio-coding-hub/commit/15badee08b0c14f71695e6e71f0b165e4844371c))
* **ClaudeModelValidationDialog:** 添加综合协议验证功能与UI展示 ([b6ede30](https://github.com/dyndynjyxa/aio-coding-hub/commit/b6ede30e61e19f9af69e8d7153cbc711b3e3a180))
* **ClaudeModelValidation:** enhance cross-provider validation and UI feedback ([bf83c7e](https://github.com/dyndynjyxa/aio-coding-hub/commit/bf83c7e03c7edf78795cd51a943c01a88e0b17d7))
* **ClaudeModelValidation:** enhance output token validation and error handling ([d245288](https://github.com/dyndynjyxa/aio-coding-hub/commit/d245288d7a4937ca7b0213ebd79d9c0d5e3c76b4))
* **ClaudeModelValidation:** implement cross-provider signature validation and enhance request handling ([2e102d4](https://github.com/dyndynjyxa/aio-coding-hub/commit/2e102d4f3fd2745e4480a5884272baeafe66b6d0))
* **claudeModelValidation:** 优化模型验证对话框和多轮验证流程支持 ([3f6c031](https://github.com/dyndynjyxa/aio-coding-hub/commit/3f6c03102db7aaeaa2d8399ebdac1cc4787c2a84))
* **cli_proxy:** 添加 merge-restore 功能以保留用户更改 ([a7d05c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/a7d05c97fe1228d83ccf97be5ed9b6ec26c7bde0))
* **cli_proxy:** 添加自动同步以修复代理配置漂移问题 ([0a7b856](https://github.com/dyndynjyxa/aio-coding-hub/commit/0a7b856e87d62fd3be84f6314a89f3f1a6706e92))
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
* **gateway:** 增强上游错误处理与界面错误详情展示 ([85f67c4](https://github.com/dyndynjyxa/aio-coding-hub/commit/85f67c4c1ffe08d102883458a2d9d29e0b2b98cf))
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
* **providers:** 添加 OAuth 限制重置倒计时 & 修复托盘退出重启问题 ([#168](https://github.com/dyndynjyxa/aio-coding-hub/issues/168)) ([4ca1248](https://github.com/dyndynjyxa/aio-coding-hub/commit/4ca124860c48fd3764ecf1da861adef072267ba7))
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
* **ui:** 设置与首页展示优化，并修复 Node 24 下 pre-push 单测问题 ([#169](https://github.com/dyndynjyxa/aio-coding-hub/issues/169)) ([165db18](https://github.com/dyndynjyxa/aio-coding-hub/commit/165db18dce650cee175a8df07f9bc80168575aab))
* **updater:** display release notes in update dialog ([b891ed7](https://github.com/dyndynjyxa/aio-coding-hub/commit/b891ed75a1b67a9df281917ffa94e684fb372664))
* **update:** 更新对话框展示更新日志 ([#171](https://github.com/dyndynjyxa/aio-coding-hub/issues/171)) ([c55ccb2](https://github.com/dyndynjyxa/aio-coding-hub/commit/c55ccb2cb1fc18b1e109f41192a4362f3a4ff8af))
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
* **ci:** 修复Rust测试线程数为单线程运行 ([87ec1fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/87ec1fe962d7b2913841e739e3ee7026f7145c36))
* **ci:** 修复正则表达式转义字符问题 ([ba805c3](https://github.com/dyndynjyxa/aio-coding-hub/commit/ba805c3b2f147f153cbe39042b078547eb36da5a))
* **ci:** 修正发布流程中更新器 JSON 文件参数名称 ([9a4c0f0](https://github.com/dyndynjyxa/aio-coding-hub/commit/9a4c0f063bd645e3286e67f1607334a84a5cb6fd))
* **ClaudeModelValidationDialog:** adjust dialog max-width for responsive design ([11bdfd0](https://github.com/dyndynjyxa/aio-coding-hub/commit/11bdfd0df1b51923a561211f6cc6d5383eedf76c))
* **cli_manager:** 修复 run_in_login_shell 的平台兼容性处理 ([9f81433](https://github.com/dyndynjyxa/aio-coding-hub/commit/9f814337d08f36c08e37486f95b9c9da62880fc1))
* **cli_proxy:** 修复数据库初始化错误处理逻辑 ([9230475](https://github.com/dyndynjyxa/aio-coding-hub/commit/9230475e4401e93e3dc34ad285ace4a03186cd16))
* **cli_update:** 修复提取语义版本字符串的字符截取逻辑 ([782fb95](https://github.com/dyndynjyxa/aio-coding-hub/commit/782fb95d05236afbbfdf80b4bfb15921b9c34b88))
* **cli-manager:** 修正模型参数解析逻辑以避免默认值覆盖 ([ae2b7fe](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae2b7feecfd6e06462b8b0990204fc55d6ad3ccf))
* **cli-proxy:** 修复 Codex CLI 在 Windows 下的沙箱配置与认证模式 ([b79d072](https://github.com/dyndynjyxa/aio-coding-hub/commit/b79d072f39405c15125a8a6391bff122c9a5f315))
* **cli-sessions:** 修复SessionsPage Card容器flex布局 ([2d2a048](https://github.com/dyndynjyxa/aio-coding-hub/commit/2d2a0481f9811160223f9753d6edec5a28dcfc6c))
* **codex-tab:** sandbox_mode danger-full-access 选择不生效及高级配置不回显 ([3943401](https://github.com/dyndynjyxa/aio-coding-hub/commit/3943401524c8ef6a179e65e8ee62fd2ac4f784cb))
* **codex:** resolve oauth login failure and stale provider names ([#116](https://github.com/dyndynjyxa/aio-coding-hub/issues/116)) ([48d647e](https://github.com/dyndynjyxa/aio-coding-hub/commit/48d647ef429232562c9b56e88d8a37f57960604a))
* **codex:** 修复切换 Codex Home 后代理状态与实际配置不一致 ([#125](https://github.com/dyndynjyxa/aio-coding-hub/issues/125)) ([e8f80b2](https://github.com/dyndynjyxa/aio-coding-hub/commit/e8f80b2838509b748d38ed6411219218b4077bf5))
* **commands:** 修复构建Claude启动命令时路径参数传递 ([9e714f4](https://github.com/dyndynjyxa/aio-coding-hub/commit/9e714f48bdbf7dff2d729131774518845303290a))
* **components:** 修复ClaudeModelValidationDialog点击事件阻塞问题 ([9bd5f53](https://github.com/dyndynjyxa/aio-coding-hub/commit/9bd5f53f38d01fc312b979e3620b5e0badaba585))
* **cost:** price CX2CC requests with translated model basis ([#175](https://github.com/dyndynjyxa/aio-coding-hub/issues/175)) ([342f585](https://github.com/dyndynjyxa/aio-coding-hub/commit/342f585c9b713077431e83ada0d008550fa5f708))
* **cost:** 修正成本乘数比较逻辑为包含零值 ([db69776](https://github.com/dyndynjyxa/aio-coding-hub/commit/db6977635f96fc55fa1b4d84d1d71e67dd8dd0b1))
* **db:** 修复providers表note字段迁移及数据处理问题 ([fa8ecda](https://github.com/dyndynjyxa/aio-coding-hub/commit/fa8ecda9bad231ed424c812c1c7cd3d0850b8565))
* **deps:** 升级 react-router-dom 7.11→7.13 修复 XSS 漏洞 ([c4db630](https://github.com/dyndynjyxa/aio-coding-hub/commit/c4db630b4478cab2f1180f03b324cfc0af9d42e3))
* **domain:** 修复 SSE 流错误处理逻辑 ([4446939](https://github.com/dyndynjyxa/aio-coding-hub/commit/4446939c457735727f96dce0b09c91ae6b616c1d))
* **domain:** 修正 claude_terminal_launch_context 函数中 provider_id 验证和查询逻辑 ([316d375](https://github.com/dyndynjyxa/aio-coding-hub/commit/316d3754fc628d0a1e435aabf469ca992cb63efa))
* **gateway:** allow failover success handler arg count ([0c559cb](https://github.com/dyndynjyxa/aio-coding-hub/commit/0c559cba801d59332a94eb650571487b6b18a53b))
* **gateway:** claude api-key auth fallback ([ac68780](https://github.com/dyndynjyxa/aio-coding-hub/commit/ac687808a15cc8d412714d623e4a32ef10ece310))
* **gateway:** finalize failed requests and align error codes ([871de9c](https://github.com/dyndynjyxa/aio-coding-hub/commit/871de9c9c9e06ed6cb03026ceb21dfc4b0ba18d3))
* **gateway:** reject forwarding when CLI proxy disabled ([c9edd10](https://github.com/dyndynjyxa/aio-coding-hub/commit/c9edd10cd2f41ef86c8c4c8a3ca2262c8bcb09ef))
* **gateway:** 修复 CX2CC 无 SSE 响应头成功响应的分类时机 ([#160](https://github.com/dyndynjyxa/aio-coding-hub/issues/160)) ([9edcd5b](https://github.com/dyndynjyxa/aio-coding-hub/commit/9edcd5bf8396a0a2a84e5b368d984717b4d7c81f))
* **gateway:** 修复 HALF_OPEN 状态下熔断未及时解除的前端展示问题 ([#166](https://github.com/dyndynjyxa/aio-coding-hub/issues/166)) ([4c809f9](https://github.com/dyndynjyxa/aio-coding-hub/commit/4c809f9b0b6aee4e426d8f3a6d2aa53699fd7b35))
* **gateway:** 修正 Heartbeat 事件变量可见性及用法 ([a540d52](https://github.com/dyndynjyxa/aio-coding-hub/commit/a540d52012652075deee18c58d3e4b5611f4d547))
* **gateway:** 修正 provider 重新排序时清除会话绑定逻辑 ([f03c02e](https://github.com/dyndynjyxa/aio-coding-hub/commit/f03c02e5332fe84496a029b2e5d130b558cfad31))
* **gateway:** 调整上游连接超时与错误切换逻辑 ([a6870ae](https://github.com/dyndynjyxa/aio-coding-hub/commit/a6870ae14b6b5bb8edce814ca37da2000e84f6b4))
* **heartbeat_watchdog:** reload 返回 Ok 但实际异步失败时升级到窗口重建 ([#172](https://github.com/dyndynjyxa/aio-coding-hub/issues/172)) ([e4f77b0](https://github.com/dyndynjyxa/aio-coding-hub/commit/e4f77b01c7204767b61d8bb787a374b270383736))
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
* **settings:** 设置默认关闭Billing Header整流器 ([9331253](https://github.com/dyndynjyxa/aio-coding-hub/commit/933125358a9574225cd1f317fcdc9d6b1679c6c4))
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
* **usage:** 修复 Usage 统计中的 CX2CC 缓存命中率计算 ([#159](https://github.com/dyndynjyxa/aio-coding-hub/issues/159)) ([3472560](https://github.com/dyndynjyxa/aio-coding-hub/commit/3472560f62f88e4eae4b4d53e920c290869a2c7d))
* **utils:** 修正computeOutputTokensPerSecond回退逻辑 ([7c59559](https://github.com/dyndynjyxa/aio-coding-hub/commit/7c59559ad0b525e478577d74518173753530a980))
* WebView2 不可恢复状态检测与分级恢复 ([#156](https://github.com/dyndynjyxa/aio-coding-hub/issues/156)) ([4869745](https://github.com/dyndynjyxa/aio-coding-hub/commit/4869745cb352279829317f18b5e1956c381b5b38))
* **wsl:** atomic write, remove dead fallback, add TOML comment ([7decfb5](https://github.com/dyndynjyxa/aio-coding-hub/commit/7decfb5803378ee3bfb5433bc88adb64562a6857))
* **wsl:** 修复 WSL 脚本错误消息编码和路径写入问题 ([bccd5c9](https://github.com/dyndynjyxa/aio-coding-hub/commit/bccd5c93110641898e04cd1cde031df604af637b))
* **wsl:** 修复WSL适配器IPv4解析和状态脚本处理 ([06ae9b1](https://github.com/dyndynjyxa/aio-coding-hub/commit/06ae9b1f462572550f327c3afe4034179b1da491))
* **wsl:** 修正同步边界与 sessions 状态 ([cc41840](https://github.com/dyndynjyxa/aio-coding-hub/commit/cc41840fb88744c926544436cf885b440f240403))
* 修复clippy redundant_pattern_matching警告 ([b0274a6](https://github.com/dyndynjyxa/aio-coding-hub/commit/b0274a663396a2958e470e2167933470bf34a70a))


### Performance Improvements

* **home:** 优化实时追踪卡片动画与样式过渡效果 ([48c9966](https://github.com/dyndynjyxa/aio-coding-hub/commit/48c99669d5658884bc594dcebdb35219d0c25e23))


### Code Refactoring

* **infra:** 移除 request_attempt_logs 独立表，改用 request_logs.attempts_json 派生 ([3fb2627](https://github.com/dyndynjyxa/aio-coding-hub/commit/3fb2627407e1b9b57758a8a40d3a7d7393278e1b))

## [0.35.3](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.35.2...aio-coding-hub-v0.35.3) (2026-04-04)


### Features

* **update:** 更新对话框展示更新日志 ([#171](https://github.com/dyndynjyxa/aio-coding-hub/issues/171)) ([c55ccb2](https://github.com/dyndynjyxa/aio-coding-hub/commit/c55ccb2cb1fc18b1e109f41192a4362f3a4ff8af))


### Bug Fixes

* **heartbeat_watchdog:** reload 返回 Ok 但实际异步失败时升级到窗口重建 ([#172](https://github.com/dyndynjyxa/aio-coding-hub/issues/172)) ([e4f77b0](https://github.com/dyndynjyxa/aio-coding-hub/commit/e4f77b01c7204767b61d8bb787a374b270383736))

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
