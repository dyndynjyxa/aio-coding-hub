# Changelog

## [1.0.0](https://github.com/dyndynjyxa/aio-coding-hub/compare/aio-coding-hub-v0.11.0...aio-coding-hub-v1.0.0) (2026-01-20)


### ⚠ BREAKING CHANGES

* **gateway:** gateway now rejects forwarding for claude/codex/gemini unless the corresponding CLI proxy toggle is enabled in AIO.

### Features

* add TextEvidenceSection component for improved output display in ClaudeModelValidationResultPanel ([47be119](https://github.com/dyndynjyxa/aio-coding-hub/commit/47be119a83c365b3e7b41f22308be7550ecaede5))
* **claude-validation:** add signature and caching roundtrip probes ([f0919fc](https://github.com/dyndynjyxa/aio-coding-hub/commit/f0919fc3919e751e2668c24d471ebf2f8c9a9b46))
* enhance provider model configuration with support for model whitelisting and mapping ([4f44510](https://github.com/dyndynjyxa/aio-coding-hub/commit/4f445106fefa10badae230de52c9fee09bd2486f))
* **home:** implement window foreground detection for usage heatmap refresh ([7d9c6a6](https://github.com/dyndynjyxa/aio-coding-hub/commit/7d9c6a60df8877d8a3ec36beaca9bf6192c36b3a))
* **model-prices:** add model price alias rules ([ffda7a2](https://github.com/dyndynjyxa/aio-coding-hub/commit/ffda7a221134dbc7b0a97475d117ee7c1ced20f2))
* **providers:** collapse model mapping editors ([f7d19b2](https://github.com/dyndynjyxa/aio-coding-hub/commit/f7d19b2480e4c657632b4f594a655cc91317cc9a))
* **tauri:** add WSL support and listen modes ([a357007](https://github.com/dyndynjyxa/aio-coding-hub/commit/a35700753e9633493f6e939d1700ce979d635c93))
* **ui:** align CLI manager with network and WSL settings ([ae5b5fc](https://github.com/dyndynjyxa/aio-coding-hub/commit/ae5b5fc99330b55872e1c30da6e653d7433b7d48))


### Bug Fixes

* **gateway:** reject forwarding when CLI proxy disabled ([df734b6](https://github.com/dyndynjyxa/aio-coding-hub/commit/df734b67a22a7b827fcc0d3001f40faaa495d500))
* **usage:** align cache creation ttl to 5m only ([1ba7bc8](https://github.com/dyndynjyxa/aio-coding-hub/commit/1ba7bc805428d39604d856ee567fbab03c2f09af))

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
