# Changelog

## [0.1.85](https://github.com/cedricziel/assistant/compare/v0.1.84...v0.1.85) (2026-04-07)


### Features

* **personas:** add search/filter to personas list ([122b44e](https://github.com/cedricziel/assistant/commit/122b44e3237bf721ff0817b018b9e63679ea75b2))


### Bug Fixes

* **agents:** guard card parsing with try/catch and fall back to agent ID ([7e01569](https://github.com/cedricziel/assistant/commit/7e01569b8cd767491ed51796b456b0e8cfc48b51))
* **agents:** show skills list from agent card in detail screen ([ffde799](https://github.com/cedricziel/assistant/commit/ffde7993f56a75d0d4e447f0f7bf75adac627423))
* **analytics:** wrap DataTables in horizontal scroll to prevent mobile overflow ([63021fa](https://github.com/cedricziel/assistant/commit/63021fa1cbc5beb9c48cbfb612f559c44e90a6dd))
* **chat:** reduce spacing between consecutive messages from the same sender ([01d4158](https://github.com/cedricziel/assistant/commit/01d4158a043a4ee1d06c48ca0a350dd4e66ffcd5))
* **connection:** add token visibility toggle to authentication field ([eba3d8c](https://github.com/cedricziel/assistant/commit/eba3d8cdad344585401087264cbfc1987d43fc23))
* **logs:** show spinner in search field during debounce window ([4391dfa](https://github.com/cedricziel/assistant/commit/4391dfa53361f611458da40558a4de761b0155f2))
* **logs:** truncate long messages to 2 lines, tap to expand ([c53fcfc](https://github.com/cedricziel/assistant/commit/c53fcfc06ec4897691ce414da80337484278ca57))
* **personas:** improve file slot tile with displayName, amber missing badge, and tooltip ([348d068](https://github.com/cedricziel/assistant/commit/348d0683e58c5859502b4eed59f44c3f4054c461))
* **personas:** show success snackbar after creating a persona ([f0f6434](https://github.com/cedricziel/assistant/commit/f0f6434dfadfe3f5cd2e1097e235e14c40b20ac6))
* **personas:** use monospace font in file editor for readability ([4fafed2](https://github.com/cedricziel/assistant/commit/4fafed278bebe4bdf2c9c3444fe58b678197ae65))
* **skills:** allow description expand-on-tap in list with detail navigation icon ([92cddb6](https://github.com/cedricziel/assistant/commit/92cddb6917905be1689becc32612ceff6afbe861))
* **skills:** show checkmark icon for 2s after copying skill body ([cafa45a](https://github.com/cedricziel/assistant/commit/cafa45a4aff2bc2a112e8b4b626d5c8bd4009ce4))
* **traces:** preserve expanded row state across scroll by lifting to screen level ([88355b6](https://github.com/cedricziel/assistant/commit/88355b63853958666189b5d3805f4e37368d73c3))
* **traces:** reserve fixed height for span detail loading to prevent layout jank ([3bb40db](https://github.com/cedricziel/assistant/commit/3bb40db957b72d378397d57d5acf0668a7bcd9b5))
* **webhooks:** add tooltip explaining what webhook verification does ([f81fa3a](https://github.com/cedricziel/assistant/commit/f81fa3abafa32b3fcb92a91924495f07e14f36b3))
* **webhooks:** require confirmation dialog before rotating webhook secret ([cb96495](https://github.com/cedricziel/assistant/commit/cb9649524f855bf5fb89a430083596594ee0a16f))
* **workflows:** cap displayed runs at 20 with overflow count label ([1999062](https://github.com/cedricziel/assistant/commit/1999062b9056bd1fd42fd8725647cc0c7366603b))
* **workflows:** move active/inactive badge into title row to prevent clipping on narrow screens ([49d941e](https://github.com/cedricziel/assistant/commit/49d941e182a58d67576732be9c16f3a3b379e831))

## [0.1.84](https://github.com/cedricziel/assistant/compare/v0.1.83...v0.1.84) (2026-04-07)


### Features

* **chat:** add stop generation button to cancel in-progress streaming ([a01ef1e](https://github.com/cedricziel/assistant/commit/a01ef1ebac15913f0a6cd095810497121ad41346))
* **ios:** add iOS platform support with Neural Void icon ([#394](https://github.com/cedricziel/assistant/issues/394)) ([05fbf50](https://github.com/cedricziel/assistant/commit/05fbf50abc99d0851f0ccbd83f2f79035e70bdd7))


### Bug Fixes

* **agents:** add services import for Clipboard in agent detail ([1f2b763](https://github.com/cedricziel/assistant/commit/1f2b763a07c691cf1bc5657888e9edc46a332b45))
* **agents:** make agent URL copyable in detail screen ([18a6669](https://github.com/cedricziel/assistant/commit/18a6669f3263795892160109554eebf4e996c788))
* **app:** fix SSE stream type error on iOS (Utf8Decoder → utf8.decode) ([#392](https://github.com/cedricziel/assistant/issues/392)) ([2dff458](https://github.com/cedricziel/assistant/commit/2dff458e5d1f82fffae18bff307519b45e829ca6))
* **chat:** dismiss error banner without clearing conversation messages ([e7cd93d](https://github.com/cedricziel/assistant/commit/e7cd93d16d1d82de0d7e5cad45f2f1e9c84dbc4e))
* **chat:** dismiss keyboard when tapping outside the input field ([969cc59](https://github.com/cedricziel/assistant/commit/969cc59badeb1bb9bc407e661380ae2c0899447e))
* **chat:** require confirmation before deleting a conversation ([33d6e16](https://github.com/cedricziel/assistant/commit/33d6e16c59d83e3d8dcee206798917a6aa53c984))
* **chat:** respect iOS home indicator safe area in input row ([39b0ef9](https://github.com/cedricziel/assistant/commit/39b0ef9410949951f90e4b8ab05c2ff5f2e6ee19))
* **chat:** use fixed max-width for message bubbles instead of screen-width fraction ([e1770b6](https://github.com/cedricziel/assistant/commit/e1770b6e999a78aeaef71d25d4e9723d7d3079ff))
* **connection:** wrap setup screen body in SafeArea ([4ce4440](https://github.com/cedricziel/assistant/commit/4ce444071d495f6ed61336eec6bbf004d5bd6052))
* **logs:** dismiss keyboard when tapping outside the search field ([bd97ba1](https://github.com/cedricziel/assistant/commit/bd97ba169b749736133626def4f10765dbfeb5a0))
* **nav:** wrap wide navigation rail layout in SafeArea ([585aeb5](https://github.com/cedricziel/assistant/commit/585aeb5a364cffca6fe04cdef0a48cbbfa123189))
* **personas:** add SafeArea to file editor body ([7e34b93](https://github.com/cedricziel/assistant/commit/7e34b93ea7fc895e0e2f036e1e93ef5d37040338))
* **personas:** add SafeArea to persona create screen ([fa86b49](https://github.com/cedricziel/assistant/commit/fa86b49789a4aa2232530d044b38ffe157a4421e))
* **personas:** auto-focus ID field and add Cancel button on create screen ([e6a8799](https://github.com/cedricziel/assistant/commit/e6a8799ddb8e5cd852c20cc4b762d6c3c659a559))
* **personas:** warn before discarding unsaved file editor changes ([70b894b](https://github.com/cedricziel/assistant/commit/70b894b60e6d9c94bee226b6311fca0957022f87))
* **skills:** add copy button for skill body content ([d79125e](https://github.com/cedricziel/assistant/commit/d79125e2cfc3b4b3bfd55108747a493d701dd87f))
* **skills:** add missing services import for Clipboard ([44b54bb](https://github.com/cedricziel/assistant/commit/44b54bbbeae6332bb8033ab5d996977e2fc25f14))

## [0.1.83](https://github.com/cedricziel/assistant/compare/v0.1.82...v0.1.83) (2026-04-07)


### Bug Fixes

* **ci:** correct macOS app bundle name in zip command ([960c692](https://github.com/cedricziel/assistant/commit/960c692b994a695aa5654382926a1e616e15a1c9))
* commit podfile.lock ([0e56ce5](https://github.com/cedricziel/assistant/commit/0e56ce539f52225cc3f33d62a3d7b804c5d15b9e))
* **deps:** optimize Rust workspace dependencies ([#391](https://github.com/cedricziel/assistant/issues/391)) ([e0ddb34](https://github.com/cedricziel/assistant/commit/e0ddb3460fa8037ebd0b4fa130adce91d7274789))
* **embedded-server:** guard Platform.isMacOS with kIsWeb to prevent crash on web ([2790094](https://github.com/cedricziel/assistant/commit/27900940be394b509028174cfa605870f232e437))
* **embedded-server:** replace drain&lt;List&lt;int&gt;&gt;() with drain&lt;void&gt;() ([2f42645](https://github.com/cedricziel/assistant/commit/2f42645933af0e93944f46e5879b5db4dcb25d8b))
* **web:** add crossorigin=use-credentials to manifest link ([d9db26a](https://github.com/cedricziel/assistant/commit/d9db26a734e5e98b076082c4819786b83c2aa177))

## [0.1.82](https://github.com/cedricziel/assistant/compare/v0.1.81...v0.1.82) (2026-04-07)


### Bug Fixes

* **app:** sync Flutter app version with release-please and Rust workspace ([#388](https://github.com/cedricziel/assistant/issues/388)) ([ad1f91d](https://github.com/cedricziel/assistant/commit/ad1f91d16bc993bfdc34de2c9408e446b2bff15d))

## [0.1.81](https://github.com/cedricziel/assistant/compare/v0.1.80...v0.1.81) (2026-04-07)


### Features

* **app:** add macOS menu bar tray icon ([#385](https://github.com/cedricziel/assistant/issues/385)) ([5d3ea49](https://github.com/cedricziel/assistant/commit/5d3ea493d2f8d252cd925671562903a67dfc6c6b))
* **app:** Flutter desktop self-update via GitHub Releases ([#386](https://github.com/cedricziel/assistant/issues/386)) ([c740430](https://github.com/cedricziel/assistant/commit/c740430e97a1649c48a39cf79dbb159167ad40a9))
* Flutter cross-platform frontend (web + macOS) ([#374](https://github.com/cedricziel/assistant/issues/374)) ([ef7f09c](https://github.com/cedricziel/assistant/commit/ef7f09c29617364cef5cf5784703615aa48c559a))
* **flutter:** embed assistant binary in macOS app bundle ([#383](https://github.com/cedricziel/assistant/issues/383)) ([4a7aff8](https://github.com/cedricziel/assistant/commit/4a7aff8d45e1ea094fe823377a12cdf782a4bbad))
* migrate to Flutter-only UX with pure REST API backend ([#382](https://github.com/cedricziel/assistant/issues/382)) ([c2bc352](https://github.com/cedricziel/assistant/commit/c2bc352a8dd969fa189f62c24d480df02d8518a5))
* replace heavy SDK deps with thin reqwest/WS messenger clients ([#384](https://github.com/cedricziel/assistant/issues/384)) ([25f3ebf](https://github.com/cedricziel/assistant/commit/25f3ebfdeebe0bcbf9e1b4380cd7b30f0287e0b2))
* **web-ui:** replace HTMX/HTML layer with Flutter SPA frontend ([55449da](https://github.com/cedricziel/assistant/commit/55449da728000ec4b2b95cda4426ce5a8a71e340))


### Bug Fixes

* **ci:** correct release packaging for web UI embedding and macOS bundle ([#387](https://github.com/cedricziel/assistant/issues/387)) ([2912b81](https://github.com/cedricziel/assistant/commit/2912b81d6483a8675cfe5ff85b1e8ab344e02d92))

## [0.1.80](https://github.com/cedricziel/assistant/compare/v0.1.79...v0.1.80) (2026-04-04)


### Bug Fixes

* **interface-slack:** unblock WebSocket reader by spawning heavy work ([#372](https://github.com/cedricziel/assistant/issues/372)) ([9517197](https://github.com/cedricziel/assistant/commit/9517197454708ffa5e89435cbf0c50a3ad64cfab))

## [0.1.79](https://github.com/cedricziel/assistant/compare/v0.1.78...v0.1.79) (2026-04-04)


### Bug Fixes

* **iceberg-exporter:** pass partition key to fix partition validation error ([#369](https://github.com/cedricziel/assistant/issues/369)) ([ee55a04](https://github.com/cedricziel/assistant/commit/ee55a04cb3cd2ed87fbf85abf61f114094b70e86))

## [0.1.78](https://github.com/cedricziel/assistant/compare/v0.1.77...v0.1.78) (2026-04-03)


### Bug Fixes

* **ci:** pass --allow-untrusted to apk index to accept nfpm-built packages ([#367](https://github.com/cedricziel/assistant/issues/367)) ([a72bcf2](https://github.com/cedricziel/assistant/commit/a72bcf254d0189c1901061df9126ed687bc6a0f3))

## [0.1.77](https://github.com/cedricziel/assistant/compare/v0.1.76...v0.1.77) (2026-04-03)


### Features

* **skills:** add rust-dependencies and ci-organization skills ([#365](https://github.com/cedricziel/assistant/issues/365)) ([b48b998](https://github.com/cedricziel/assistant/commit/b48b99866344d9bdcab7420cb5f034e8e6134d99))


### Bug Fixes

* **interface-matrix:** vendor OpenSSL for rusqlite sqlcipher musl cross-build ([#363](https://github.com/cedricziel/assistant/issues/363)) ([0482b80](https://github.com/cedricziel/assistant/commit/0482b805f28d40b4a7582f1abcdc095b1dda4327))

## [0.1.76](https://github.com/cedricziel/assistant/compare/v0.1.75...v0.1.76) (2026-04-03)


### Features

* **interface-slack:** true incremental streaming via token sink ([#359](https://github.com/cedricziel/assistant/issues/359)) ([bd56fe2](https://github.com/cedricziel/assistant/commit/bd56fe25ab2ae124e83268a8c4784f5d71c60dcb))


### Bug Fixes

* **cross:** vendor OpenSSL + fix musl cross-compilation environment ([#362](https://github.com/cedricziel/assistant/issues/362)) ([b69e801](https://github.com/cedricziel/assistant/commit/b69e80179c5ceb7247a936204ccab9772af18fd0))

## [0.1.75](https://github.com/cedricziel/assistant/compare/v0.1.74...v0.1.75) (2026-04-03)


### Features

* **runtime:** cancel in-flight worker turn when submit_turn times out ([#358](https://github.com/cedricziel/assistant/issues/358)) ([f1d6d96](https://github.com/cedricziel/assistant/commit/f1d6d96e75af5fdbb55c01990e7be89f6a3f076f))


### Bug Fixes

* **interface-mattermost:** vendor OpenSSL to fix musl cross-compilation ([#357](https://github.com/cedricziel/assistant/issues/357)) ([dae7df4](https://github.com/cedricziel/assistant/commit/dae7df436a829b19e435dd1fe992496e8a7b794d))

## [0.1.74](https://github.com/cedricziel/assistant/compare/v0.1.73...v0.1.74) (2026-04-03)


### Features

* **runtime:** per-persona turn timeout (default 3 h) ([#355](https://github.com/cedricziel/assistant/issues/355)) ([adf8d4c](https://github.com/cedricziel/assistant/commit/adf8d4c64f18d38dbfa6f8787f98a4afbd41e97f))

## [0.1.73](https://github.com/cedricziel/assistant/compare/v0.1.72...v0.1.73) (2026-04-03)


### Features

* **release:** Alpine APK packages and lean Alpine Docker image ([#352](https://github.com/cedricziel/assistant/issues/352)) ([1b850b4](https://github.com/cedricziel/assistant/commit/1b850b4c2ac17cb7f8504ecba77aef2d40744595))


### Bug Fixes

* **iceberg-exporter:** use '+00:00' timezone to match Iceberg Timestamptz Arrow mapping ([#353](https://github.com/cedricziel/assistant/issues/353)) ([1a76570](https://github.com/cedricziel/assistant/commit/1a7657005c99fdcb813cc75ec5d77985c402d8a4))

## [0.1.72](https://github.com/cedricziel/assistant/compare/v0.1.71...v0.1.72) (2026-03-31)


### Features

* **interface-slack:** stream replies via chat.update placeholder ([#349](https://github.com/cedricziel/assistant/issues/349)) ([b5503db](https://github.com/cedricziel/assistant/commit/b5503db0ff07aa4fde8ba12552db37cd2e23284f))
* **web-ui:** JSON conversation API + Stimulus chat UI migration ([#348](https://github.com/cedricziel/assistant/issues/348)) ([9602bd8](https://github.com/cedricziel/assistant/commit/9602bd8808322b65e2324e7f23022b1a0b69a944))


### Bug Fixes

* **claude:** repair WorktreeCreate hook to correctly create worktrees ([#350](https://github.com/cedricziel/assistant/issues/350)) ([0d32d5c](https://github.com/cedricziel/assistant/commit/0d32d5c8c5f3f7333d695b70805b12e46ce62d38))

## [0.1.71](https://github.com/cedricziel/assistant/compare/v0.1.70...v0.1.71) (2026-03-30)


### Features

* **claude:** add WorktreeCreate hook for web-ui vendoring and shared Cargo target ([#346](https://github.com/cedricziel/assistant/issues/346)) ([683d757](https://github.com/cedricziel/assistant/commit/683d757fb85f4253b188e26247bf310d2c3845ae))
* **web-ui:** query Iceberg Parquet files when exporter = "iceberg" ([#345](https://github.com/cedricziel/assistant/issues/345)) ([f41dd56](https://github.com/cedricziel/assistant/commit/f41dd56c098318a55bddb0bc1bdb0174d027542b))

## [0.1.70](https://github.com/cedricziel/assistant/compare/v0.1.69...v0.1.70) (2026-03-30)


### Features

* **skill:** upgrade claude-code-agent to use native process tool (Phase 3 of [#74](https://github.com/cedricziel/assistant/issues/74)) ([cbec05a](https://github.com/cedricziel/assistant/commit/cbec05a60b75b8b4ef6aa2e5ce57a51c4699a6c1))


### Bug Fixes

* **iceberg-exporter:** derive Arrow schemas from Iceberg schema to embed field IDs ([#343](https://github.com/cedricziel/assistant/issues/343)) ([f7a31e9](https://github.com/cedricziel/assistant/commit/f7a31e99684dd0b488d2cc3332651eb64aed00a9))

## [0.1.69](https://github.com/cedricziel/assistant/compare/v0.1.68...v0.1.69) (2026-03-30)


### Features

* **web-ui:** expose OpenAPI spec and Swagger UI ([#339](https://github.com/cedricziel/assistant/issues/339)) ([9b98d42](https://github.com/cedricziel/assistant/commit/9b98d42cf7c26be6bdc575980fd132b2fdc404e2))

## [0.1.68](https://github.com/cedricziel/assistant/compare/v0.1.67...v0.1.68) (2026-03-30)


### Features

* **observability:** add Apache Iceberg exporter for OTel spans, logs, and metrics ([#338](https://github.com/cedricziel/assistant/issues/338)) ([cc15760](https://github.com/cedricziel/assistant/commit/cc157603db3650365fae8e29682555fb73e5bd4f))


### Bug Fixes

* correct opentelemetry-exporter-iceberg version ([8a1d483](https://github.com/cedricziel/assistant/commit/8a1d4832c6800e540f9a77047dedbf9ac97e34c8))

## [0.1.67](https://github.com/cedricziel/assistant/compare/v0.1.66...v0.1.67) (2026-03-29)


### Features

* **backup:** add backup and restore CLI subcommands ([#330](https://github.com/cedricziel/assistant/issues/330)) ([dec47de](https://github.com/cedricziel/assistant/commit/dec47de1a1289c34b8bf10b747a088e35a87a2fc))

## [0.1.66](https://github.com/cedricziel/assistant/compare/v0.1.65...v0.1.66) (2026-03-29)


### Bug Fixes

* **runtime:** raise OTel log bridge filter from DEBUG to INFO, suppress async_nats/h2/hyper_util ([f47e810](https://github.com/cedricziel/assistant/commit/f47e8102dc2db32ed75bab9a991f751153a68b23))

## [0.1.65](https://github.com/cedricziel/assistant/compare/v0.1.64...v0.1.65) (2026-03-29)


### Features

* **003-skill-management:** skill CRUD, persona access control, and AI generation ([#326](https://github.com/cedricziel/assistant/issues/326)) ([fa9d40a](https://github.com/cedricziel/assistant/commit/fa9d40af4b24c816b22f332704b0f50e9ccce447))

## [0.1.64](https://github.com/cedricziel/assistant/compare/v0.1.63...v0.1.64) (2026-03-29)


### Features

* **observability:** UI improvements for diagnosing silent failures ([#327](https://github.com/cedricziel/assistant/issues/327)) ([7428621](https://github.com/cedricziel/assistant/commit/74286216e7e9dc07431b6a8a5946ffb0f7d8269c))

## [0.1.63](https://github.com/cedricziel/assistant/compare/v0.1.62...v0.1.63) (2026-03-28)


### Features

* **interface-matrix:** add Matrix messaging interface ([#325](https://github.com/cedricziel/assistant/issues/325)) ([57198ca](https://github.com/cedricziel/assistant/commit/57198ca24041b8957fafc43532116d3c57326c7a))
* **web-ui:** persona file editor UI ([#323](https://github.com/cedricziel/assistant/issues/323)) ([f718966](https://github.com/cedricziel/assistant/commit/f718966b1fe3c27a960a43f6f420904ace0e963c))

## [0.1.62](https://github.com/cedricziel/assistant/compare/v0.1.61...v0.1.62) (2026-03-27)


### Bug Fixes

* **cli/runtime:** scheduled tasks silent in orchestrator interface-filtered mode ([#319](https://github.com/cedricziel/assistant/issues/319)) ([d3dc567](https://github.com/cedricziel/assistant/commit/d3dc567c64271d4a815a52516e74b6308c099329))

## [0.1.61](https://github.com/cedricziel/assistant/compare/v0.1.60...v0.1.61) (2026-03-27)


### Bug Fixes

* **cli:** spawn scheduler worker in single-interface modes ([3972b17](https://github.com/cedricziel/assistant/commit/3972b172ef323a52e8bcd8a55e3feb247837a2ca))

## [0.1.60](https://github.com/cedricziel/assistant/compare/v0.1.59...v0.1.60) (2026-03-27)


### Bug Fixes

* **slack:** persist active threads to SQLite so bot responds after restart ([#316](https://github.com/cedricziel/assistant/issues/316)) ([99ba80c](https://github.com/cedricziel/assistant/commit/99ba80cdc7ec80b94a785eb789b12475697a4e9d))

## [0.1.59](https://github.com/cedricziel/assistant/compare/v0.1.58...v0.1.59) (2026-03-26)


### Bug Fixes

* **runtime:** address compaction correctness issues from PR [#313](https://github.com/cedricziel/assistant/issues/313) review ([#314](https://github.com/cedricziel/assistant/issues/314)) ([eacf1c7](https://github.com/cedricziel/assistant/commit/eacf1c710188f90c97c74c1c40782b17a5655d04))

## [0.1.58](https://github.com/cedricziel/assistant/compare/v0.1.57...v0.1.58) (2026-03-26)


### Features

* **runtime:** add context compaction with token-threshold history summarization ([#312](https://github.com/cedricziel/assistant/issues/312)) ([312790f](https://github.com/cedricziel/assistant/commit/312790f3afee13239313b7456590671842ecdbb1))
* **runtime:** add context compaction with token-threshold history summarization ([#313](https://github.com/cedricziel/assistant/issues/313)) ([8dfde7e](https://github.com/cedricziel/assistant/commit/8dfde7eb4592edc18eba9c8aeb76f55c6ed35092))
* **runtime:** index conversation history into memory_chunks after each turn ([#311](https://github.com/cedricziel/assistant/issues/311)) ([ee69a1d](https://github.com/cedricziel/assistant/commit/ee69a1da4c31161a3e698d45a103a76e01a2fcbd))
* **tool-executor:** improve memory-search with query expansion, temporal decay, MMR, and multilingual stop words ([#309](https://github.com/cedricziel/assistant/issues/309)) ([57ceb80](https://github.com/cedricziel/assistant/commit/57ceb80132b7bfbfadb479b0906d7b3f709963bb))

## [0.1.57](https://github.com/cedricziel/assistant/compare/v0.1.56...v0.1.57) (2026-03-26)


### Bug Fixes

* **cli:** spawn interface workers in orchestrator run mode ([#307](https://github.com/cedricziel/assistant/issues/307)) ([1fe6301](https://github.com/cedricziel/assistant/commit/1fe6301ffd2a176a8967dc4f9038debdace08ffd))

## [0.1.56](https://github.com/cedricziel/assistant/compare/v0.1.55...v0.1.56) (2026-03-25)


### Features

* **runtime:** split persona-bound and anonymous subagent execution scope ([#300](https://github.com/cedricziel/assistant/issues/300)) ([8edf21f](https://github.com/cedricziel/assistant/commit/8edf21fa7c4205649d74fb0cb319c77ff4966a3c))


### Bug Fixes

* **runtime:** attribute subagent telemetry to parent persona ([#299](https://github.com/cedricziel/assistant/issues/299)) ([fd7ce44](https://github.com/cedricziel/assistant/commit/fd7ce44c8f5f0a2101121f81394d782cd314539e))
* **runtime:** persist subagent parent lineage from caller context ([#298](https://github.com/cedricziel/assistant/issues/298)) ([ad1ecf5](https://github.com/cedricziel/assistant/commit/ad1ecf57b2a8eb0fb9d259a5e551a7f803b46499))
* **runtime:** scope thinking-step history load to anonymous subagent persona ([#306](https://github.com/cedricziel/assistant/issues/306)) ([9fc036e](https://github.com/cedricziel/assistant/commit/9fc036e385c70975b2edbd2f269a5e47cfb23d8c))
* **storage:** decouple telemetry traces from conversations ([#281](https://github.com/cedricziel/assistant/issues/281)) ([c5b7e14](https://github.com/cedricziel/assistant/commit/c5b7e1418249dbdeb3f6ba71975b51559f32e820))

## [0.1.55](https://github.com/cedricziel/assistant/compare/v0.1.54...v0.1.55) (2026-03-24)


### Bug Fixes

* **release:** trigger release-please ([23725b9](https://github.com/cedricziel/assistant/commit/23725b9928ac188a7027b470be8eae85bdf4cf76))

## [0.1.54](https://github.com/cedricziel/assistant/compare/v0.1.53...v0.1.54) (2026-03-24)


### Bug Fixes

* **release:** install modern protoc in cross aarch64 builds ([#276](https://github.com/cedricziel/assistant/issues/276)) ([fa4ac34](https://github.com/cedricziel/assistant/commit/fa4ac346357370c13c4bdb2360bf5ea4986b7a87))

## [0.1.53](https://github.com/cedricziel/assistant/compare/v0.1.52...v0.1.53) (2026-03-24)


### Features

* **interface-cli:** unify runtime entrypoints and service migration ([#264](https://github.com/cedricziel/assistant/issues/264)) ([f9fe94c](https://github.com/cedricziel/assistant/commit/f9fe94c347f46952f46e93be2ef81be39ed34dbb))

## [0.1.52](https://github.com/cedricziel/assistant/compare/v0.1.51...v0.1.52) (2026-03-24)


### Bug Fixes

* **interface-cli:** prevent duplicate turn consumption in interface modes ([#262](https://github.com/cedricziel/assistant/issues/262)) ([6b2f9f6](https://github.com/cedricziel/assistant/commit/6b2f9f61624ed5effaef900d7c3525cfcb350383))

## [0.1.51](https://github.com/cedricziel/assistant/compare/v0.1.50...v0.1.51) (2026-03-24)


### Features

* **observability:** add submit_turn lifecycle and correlation telemetry ([#259](https://github.com/cedricziel/assistant/issues/259)) ([ab0a005](https://github.com/cedricziel/assistant/commit/ab0a00580b63618be6910e41819207f31bfde3e4))
* **web-ui:** redesign workflow detail and editor navigation ([#260](https://github.com/cedricziel/assistant/issues/260)) ([7825d59](https://github.com/cedricziel/assistant/commit/7825d59ac68d9190ad7f64bfeeb88607c9bde169))

## [0.1.50](https://github.com/cedricziel/assistant/compare/v0.1.49...v0.1.50) (2026-03-23)


### Features

* **web-ui:** make workflow editor mobile-first and Stimulus-driven ([#256](https://github.com/cedricziel/assistant/issues/256)) ([e23450a](https://github.com/cedricziel/assistant/commit/e23450ab6251eb91cc597808cdc1d6af9b47a373))


### Bug Fixes

* **binaries:** add clap version flags to all clap programs ([#254](https://github.com/cedricziel/assistant/issues/254)) ([924aab7](https://github.com/cedricziel/assistant/commit/924aab75ac3698f8f9c6ceebf888ee2ef3e752b3))
* **interface-slack:** align websocket TLS dependency versions ([#257](https://github.com/cedricziel/assistant/issues/257)) ([0e88526](https://github.com/cedricziel/assistant/commit/0e8852642fe083f43791dbd272c76d766f5ddd9f))

## [0.1.49](https://github.com/cedricziel/assistant/compare/v0.1.48...v0.1.49) (2026-03-23)


### Features

* **web-ui:** introduce Stimulus for workflow secret reveal ([#252](https://github.com/cedricziel/assistant/issues/252)) ([754de60](https://github.com/cedricziel/assistant/commit/754de6077297dae0f08304c44c0ade1c8ce00a6c))


### Bug Fixes

* **observability:** propagate root spans across bus and webhook flows ([#250](https://github.com/cedricziel/assistant/issues/250)) ([fff345d](https://github.com/cedricziel/assistant/commit/fff345d9012db93c8359c957c4db9c23f001a665))
* **web-ui:** harden workflow detail UX and mobile navigation ([#253](https://github.com/cedricziel/assistant/issues/253)) ([6b1cf26](https://github.com/cedricziel/assistant/commit/6b1cf2688b657c2a566bc0dc6267575a27f26064))

## [0.1.48](https://github.com/cedricziel/assistant/compare/v0.1.47...v0.1.48) (2026-03-23)


### Bug Fixes

* **observability:** separate service and span names across telemetry ([#247](https://github.com/cedricziel/assistant/issues/247)) ([cde3b18](https://github.com/cedricziel/assistant/commit/cde3b18d6deeafa7d13bbba9abed70821d77af59))
* **web-ui:** follow up PR247 review feedback ([#249](https://github.com/cedricziel/assistant/issues/249)) ([f170099](https://github.com/cedricziel/assistant/commit/f17009974eded36c21b45734ebee34e26ed589d8))
* **web-ui:** restore scrolling on workflow pages ([#245](https://github.com/cedricziel/assistant/issues/245)) ([eb657f4](https://github.com/cedricziel/assistant/commit/eb657f4ccd55f5ba971bb2cd08b701dac58371b4))

## [0.1.47](https://github.com/cedricziel/assistant/compare/v0.1.46...v0.1.47) (2026-03-23)


### Bug Fixes

* **ci:** run smoke tests against GitHub Actions Ollama service ([#243](https://github.com/cedricziel/assistant/issues/243)) ([60e4abd](https://github.com/cedricziel/assistant/commit/60e4abd073187c3c37efbb1351a1a11651d03b9c))
* **web-ui:** harden responsive navigation and accessibility ([#241](https://github.com/cedricziel/assistant/issues/241)) ([e79bd5b](https://github.com/cedricziel/assistant/commit/e79bd5b11661b09041e5f2603705095e62af5a1e))

## [0.1.46](https://github.com/cedricziel/assistant/compare/v0.1.45...v0.1.46) (2026-03-22)


### Features

* **web-ui:** redesign workflow form for desktop split layout ([#235](https://github.com/cedricziel/assistant/issues/235)) ([b2d1c21](https://github.com/cedricziel/assistant/commit/b2d1c214f2c6846a5c62f670d9b021455bf24591))


### Bug Fixes

* **release:** install protoc in aarch64 cross pre-build ([#232](https://github.com/cedricziel/assistant/issues/232)) ([1596a47](https://github.com/cedricziel/assistant/commit/1596a4767f5053f08880ae92e3d6f262b12c7ed6))
* **storage:** make agent-scope migrations sqlite-safe ([#233](https://github.com/cedricziel/assistant/issues/233)) ([2a7b5f4](https://github.com/cedricziel/assistant/commit/2a7b5f4d43244c27336192f87a21786e9866d1c1))

## [0.1.45](https://github.com/cedricziel/assistant/compare/v0.1.44...v0.1.45) (2026-03-22)


### Bug Fixes

* **cli:** enable clap env attribute support ([0949a4e](https://github.com/cedricziel/assistant/commit/0949a4e01fa761e2d187fc9c87ff1874fdbefdd7))

## [0.1.44](https://github.com/cedricziel/assistant/compare/v0.1.43...v0.1.44) (2026-03-22)


### Features

* **workflows:** add loop-enabled workflow graph management ([#228](https://github.com/cedricziel/assistant/issues/228)) ([fe883e5](https://github.com/cedricziel/assistant/commit/fe883e517e096c46701a4c60077bbfee563ce195))

## [0.1.43](https://github.com/cedricziel/assistant/compare/v0.1.42...v0.1.43) (2026-03-22)


### Features

* **runtime:** introduce multi-agent contexts with scoped runtime and web UI ([#227](https://github.com/cedricziel/assistant/issues/227)) ([f3226b1](https://github.com/cedricziel/assistant/commit/f3226b14aa8aa0b5d189f8582499811928d6da8a))
* **web-ui:** add health and readiness endpoints ([#225](https://github.com/cedricziel/assistant/issues/225)) ([786ac5c](https://github.com/cedricziel/assistant/commit/786ac5c33273321af4ee0ca2020eb12e4dc395d7))

## [0.1.42](https://github.com/cedricziel/assistant/compare/v0.1.41...v0.1.42) (2026-03-20)


### Features

* add NATS JetStream message bus backend ([#223](https://github.com/cedricziel/assistant/issues/223)) ([9cd2c31](https://github.com/cedricziel/assistant/commit/9cd2c317e720c6d30a5e948233e392bed4e7b61c))

## [0.1.41](https://github.com/cedricziel/assistant/compare/v0.1.40...v0.1.41) (2026-03-16)


### Miscellaneous Chores

* release 0.1.41 ([e0853e1](https://github.com/cedricziel/assistant/commit/e0853e114970732ffe918b7c1ff67fc0ec16622b))

## [0.1.40](https://github.com/cedricziel/assistant/compare/v0.1.39...v0.1.40) (2026-03-13)


### Features

* integrate external MCP servers as tool sources ([#195](https://github.com/cedricziel/assistant/issues/195)) ([8eab50e](https://github.com/cedricziel/assistant/commit/8eab50e1acc339994646ce2592cd6dbd0e2eb2a4))
* **interface-nextcloud:** add Nextcloud Talk webhook-based bot interface ([#196](https://github.com/cedricziel/assistant/issues/196)) ([ec98096](https://github.com/cedricziel/assistant/commit/ec9809672937e7b9eafe110f0ad59b21625b7e1d))


### Bug Fixes

* **cli:** spawn turn worker in standalone interface modes ([#204](https://github.com/cedricziel/assistant/issues/204)) ([d2104d3](https://github.com/cedricziel/assistant/commit/d2104d35f5a06dcf699ebb78d5ed3daea0f8904c))
* **llm:** make retry config injectable and lower default from 20 to 3 ([#207](https://github.com/cedricziel/assistant/issues/207)) ([4dca418](https://github.com/cedricziel/assistant/commit/4dca418f3398577797d561a3a59a6a053a3b6cab))
* **mcp-client:** address CodeRabbit review round 2 ([#205](https://github.com/cedricziel/assistant/issues/205)) ([a036bf0](https://github.com/cedricziel/assistant/commit/a036bf0a49e7f40750664ee4685b71201054fb28))
* patch nextcloud version ([#208](https://github.com/cedricziel/assistant/issues/208)) ([daa8dd9](https://github.com/cedricziel/assistant/commit/daa8dd97368ccfef12f0fb0e8f076f8c4edbe6ae))
* **slack:** only respond in threads where the bot was @-mentioned ([#206](https://github.com/cedricziel/assistant/issues/206)) ([7b174e9](https://github.com/cedricziel/assistant/commit/7b174e92f3da6ce9e011ed22885fcc8793aab52b))

## [0.1.39](https://github.com/cedricziel/assistant/compare/v0.1.38...v0.1.39) (2026-03-05)


### Features

* add current timestamp to system prompt ([#193](https://github.com/cedricziel/assistant/issues/193)) ([e9c9db8](https://github.com/cedricziel/assistant/commit/e9c9db8a4a6c92ef86255573de72c26e8c94d329))

## [0.1.38](https://github.com/cedricziel/assistant/compare/v0.1.37...v0.1.38) (2026-03-05)


### Features

* **llm:** increase default retry to 20 attempts with 60s max delay ([de7c625](https://github.com/cedricziel/assistant/commit/de7c625114a99c33be7f20ea1e20d72d134471fc))

## [0.1.37](https://github.com/cedricziel/assistant/compare/v0.1.36...v0.1.37) (2026-03-05)


### Bug Fixes

* **ci:** include both amd64 and arm64 debs in APT Packages index ([d375a1b](https://github.com/cedricziel/assistant/commit/d375a1b2431981a396af050bc6a32293a285a963))

## [0.1.36](https://github.com/cedricziel/assistant/compare/v0.1.35...v0.1.36) (2026-03-05)


### Features

* **runtime:** re-implement memory indexer ([#188](https://github.com/cedricziel/assistant/issues/188)) ([97d9016](https://github.com/cedricziel/assistant/commit/97d90163ba1c90424aa35c487148000c0f4da8b8))

## [0.1.35](https://github.com/cedricziel/assistant/compare/v0.1.34...v0.1.35) (2026-03-04)


### Features

* **llm:** add retry with exponential backoff for transient API errors ([#186](https://github.com/cedricziel/assistant/issues/186)) ([088a0c1](https://github.com/cedricziel/assistant/commit/088a0c17f7c31dd6e6f220d00700a21f6385693b)), closes [#183](https://github.com/cedricziel/assistant/issues/183)


### Bug Fixes

* **otel-exporter-sqlite:** provide Tokio context for batch processor threads ([#184](https://github.com/cedricziel/assistant/issues/184)) ([f635b3c](https://github.com/cedricziel/assistant/commit/f635b3c082f3afad7fbb9e1a6e2539b0f22923bd))

## [0.1.34](https://github.com/cedricziel/assistant/compare/v0.1.33...v0.1.34) (2026-03-02)


### Bug Fixes

* **ci:** use force-with-lease for gh-pages deploy safety ([#181](https://github.com/cedricziel/assistant/issues/181)) ([26f63cf](https://github.com/cedricziel/assistant/commit/26f63cfe0fbefcc925676cdb0c2f2bbe872a3b06))

## [0.1.33](https://github.com/cedricziel/assistant/compare/v0.1.32...v0.1.33) (2026-03-02)


### Bug Fixes

* **transcription:** log file magic bytes and preserve input on failure ([#178](https://github.com/cedricziel/assistant/issues/178)) ([8258766](https://github.com/cedricziel/assistant/commit/825876694f0d5d8588a4248d227a7c6decb7e776))

## [0.1.32](https://github.com/cedricziel/assistant/compare/v0.1.31...v0.1.32) (2026-03-02)


### Bug Fixes

* **transcription:** improve error logging for audio conversion failures ([#176](https://github.com/cedricziel/assistant/issues/176)) ([23b34c6](https://github.com/cedricziel/assistant/commit/23b34c601c768614abd404299be4144a3f92d143))

## [0.1.31](https://github.com/cedricziel/assistant/compare/v0.1.30...v0.1.31) (2026-03-02)


### Bug Fixes

* **slack:** set presence on every reconnect so bot appears online ([#172](https://github.com/cedricziel/assistant/issues/172)) ([21fc06d](https://github.com/cedricziel/assistant/commit/21fc06d1e1aa27fb8f35d4bad6cf7b068a73ec3d))
* **transcription:** use atomic nonce for unique temp file names ([#175](https://github.com/cedricziel/assistant/issues/175)) ([4e21274](https://github.com/cedricziel/assistant/commit/4e21274b2904b8b6827a6e49e099e61f37c1a60f))

## [0.1.30](https://github.com/cedricziel/assistant/compare/v0.1.29...v0.1.30) (2026-03-02)


### Features

* **runtime:** extract shared helpers and decompose ReAct loops ([#164](https://github.com/cedricziel/assistant/issues/164)) ([080b427](https://github.com/cedricziel/assistant/commit/080b427c304b5a9ffa87fc1f5be7e2508ac55db6))


### Bug Fixes

* **runtime:** persist subagent thinking steps to DB ([#167](https://github.com/cedricziel/assistant/issues/167)) ([92e1ab0](https://github.com/cedricziel/assistant/commit/92e1ab0d51ab862ee61ebdae100c989489545f74))
* **runtime:** record max_iterations error in metrics ([#169](https://github.com/cedricziel/assistant/issues/169)) ([f8cdd46](https://github.com/cedricziel/assistant/commit/f8cdd4677baefa2bfe5b8ea5178facb065ce1ff5))
* **runtime:** use tokio::fs for BOOT.md and HEARTBEAT.md reads ([#170](https://github.com/cedricziel/assistant/issues/170)) ([9b39de5](https://github.com/cedricziel/assistant/commit/9b39de5501eff5be94e86a406c23e7cabc438235))

## [0.1.29](https://github.com/cedricziel/assistant/compare/v0.1.28...v0.1.29) (2026-03-02)


### Features

* **deps:** upgrade opentelemetry 0.24 to 0.31 and related crates ([#145](https://github.com/cedricziel/assistant/issues/145)) ([c08f192](https://github.com/cedricziel/assistant/commit/c08f1926be0b8c18a0e8e47b660ad78a33efadfd))
* **skills:** sync embedded builtin skills to disk on startup ([#140](https://github.com/cedricziel/assistant/issues/140)) ([ca5c9cf](https://github.com/cedricziel/assistant/commit/ca5c9cfd7a997e8ad831e396154278ea6bcc5b5c)), closes [#81](https://github.com/cedricziel/assistant/issues/81)


### Bug Fixes

* **storage:** set global PRAGMA busy_timeout to avoid SQLITE_BUSY ([#152](https://github.com/cedricziel/assistant/issues/152)) ([09e9cb3](https://github.com/cedricziel/assistant/commit/09e9cb315220b3b708460b392a062ebc1b02b6c8))


### Performance Improvements

* **storage:** wrap OTel SQLite exporters in batch transactions ([#151](https://github.com/cedricziel/assistant/issues/151)) ([8ec7a28](https://github.com/cedricziel/assistant/commit/8ec7a283ceac3d3211eee593d0577bf67bd2ef87))

## [0.1.28](https://github.com/cedricziel/assistant/compare/v0.1.27...v0.1.28) (2026-03-01)


### Features

* **skills:** expose compatibility field in list-skills and declare requirements ([#138](https://github.com/cedricziel/assistant/issues/138)) ([f1b62ff](https://github.com/cedricziel/assistant/commit/f1b62ff9aa696ce8c6ff51b151966d1bd9758679))
* **transcription:** Add audio format conversion for Deepgram ([#139](https://github.com/cedricziel/assistant/issues/139)) ([b151736](https://github.com/cedricziel/assistant/commit/b1517367029c161d3d66da9182bcd19855dbf661))


### Bug Fixes

* **runtime:** replace tracing .enter() guards with .instrument() in async code ([#132](https://github.com/cedricziel/assistant/issues/132)) ([21206a2](https://github.com/cedricziel/assistant/commit/21206a2164c7462c58042aefd66a00ad4444742b)), closes [#116](https://github.com/cedricziel/assistant/issues/116)

## [0.1.27](https://github.com/cedricziel/assistant/compare/v0.1.26...v0.1.27) (2026-03-01)


### Features

* add voice message transcription (Whisper, Ollama, Deepgram) ([#131](https://github.com/cedricziel/assistant/issues/131)) ([9610c0d](https://github.com/cedricziel/assistant/commit/9610c0d7fe0f335875d9576a346aeee9f65b6ebb))
* **provider-openai,provider-moonshot:** add hosted web search support ([#126](https://github.com/cedricziel/assistant/issues/126)) ([a90de0a](https://github.com/cedricziel/assistant/commit/a90de0a26fe8af1cdcf210b83e365b5b1a6490f2))
* **provider-openai:** migrate from Chat Completions to Responses API ([#128](https://github.com/cedricziel/assistant/issues/128)) ([89eeffb](https://github.com/cedricziel/assistant/commit/89eeffbe4772565d83f28a8ec7786e89ff10a59f))


### Bug Fixes

* **runtime:** scope turn workers by interface to prevent cross-service theft ([#130](https://github.com/cedricziel/assistant/issues/130)) ([9b672b2](https://github.com/cedricziel/assistant/commit/9b672b2b6d405dee7264fe49dd293c6d91acdfeb))

## [0.1.26](https://github.com/cedricziel/assistant/compare/v0.1.25...v0.1.26) (2026-03-01)


### Features

* **provider-moonshot:** add Moonshot AI (Kimi) LLM provider ([#124](https://github.com/cedricziel/assistant/issues/124)) ([0c438ed](https://github.com/cedricziel/assistant/commit/0c438ed4b84ece11fe1114f82f871f74500ff718))


### Bug Fixes

* **storage:** cast metric aggregations to REAL for sqlx f64 decoding ([3194520](https://github.com/cedricziel/assistant/commit/31945205b355a0c0d78f6ca0df95ab51f472d789))

## [0.1.25](https://github.com/cedricziel/assistant/compare/v0.1.24...v0.1.25) (2026-03-01)


### Bug Fixes

* **ci:** add make vendor to release build workflow ([38aba3d](https://github.com/cedricziel/assistant/commit/38aba3d1aa08659c2e68e31a41d606922012d3ec))

## [0.1.24](https://github.com/cedricziel/assistant/compare/v0.1.23...v0.1.24) (2026-03-01)


### Features

* **web-ui:** route chat through Orchestrator for full assistant capabilities ([#121](https://github.com/cedricziel/assistant/issues/121)) ([7aabc85](https://github.com/cedricziel/assistant/commit/7aabc850b84ca38574fd96d1bcd59a7a3198d656))


### Bug Fixes

* **ci:** never cancel in-progress CI runs on main ([ac76ebf](https://github.com/cedricziel/assistant/commit/ac76ebfe6f4bfcefff49e58774ad22f23aa39e6d))
* **web-ui:** increase screenshot diff tolerance to 5% for cross-platform fonts ([969bf4a](https://github.com/cedricziel/assistant/commit/969bf4a122b498a24dfc75127034ac2b9a759a82))

## [0.1.23](https://github.com/cedricziel/assistant/compare/v0.1.22...v0.1.23) (2026-02-28)


### Bug Fixes

* **upload:** replace content_base64 with path param to prevent binary data in LLM context ([#118](https://github.com/cedricziel/assistant/issues/118)) ([84f08c6](https://github.com/cedricziel/assistant/commit/84f08c6b87786126514a5eb5beea7ccaac2427a0))

## [0.1.22](https://github.com/cedricziel/assistant/compare/v0.1.21...v0.1.22) (2026-02-28)


### Features

* **llm:** add dedicated embedding provider with Voyage AI support ([#109](https://github.com/cedricziel/assistant/issues/109)) ([b71ff13](https://github.com/cedricziel/assistant/commit/b71ff137b8d929159b3cd7af23e2e5dd19d9a9c7))
* **provider-openai:** add OpenAI LLM provider with API key and OAuth PKCE auth ([#105](https://github.com/cedricziel/assistant/issues/105)) ([af44f8a](https://github.com/cedricziel/assistant/commit/af44f8a7609c9bc9cbdec3042cc9ca97f633ba3c))
* **web-ui:** add PWA support for installable offline-capable app ([#112](https://github.com/cedricziel/assistant/issues/112)) ([395db47](https://github.com/cedricziel/assistant/commit/395db47b4647ed27c5ed63fc5809dfcb90ef1efb))
* **web-ui:** chat interface with LLM streaming and Askama template migration ([#107](https://github.com/cedricziel/assistant/issues/107)) ([e0de5b9](https://github.com/cedricziel/assistant/commit/e0de5b94866b399577dabd12a49438e0a192c399))


### Bug Fixes

* **interface-slack:** strip cite tags and convert markdown tables in mrkdwn output ([#108](https://github.com/cedricziel/assistant/issues/108)) ([b88a93b](https://github.com/cedricziel/assistant/commit/b88a93b2596073791d16e1494d8349719cdd04a6))
* truncate bash output and increase turn timeout ([#110](https://github.com/cedricziel/assistant/issues/110)) ([67a1327](https://github.com/cedricziel/assistant/commit/67a132729b971f070d2cac62c082001da10539e3))

## [0.1.21](https://github.com/cedricziel/assistant/compare/v0.1.20...v0.1.21) (2026-02-28)


### Features

* **web-ui:** add --no-secure-cookie flag for plain HTTP on non-loopback ([#103](https://github.com/cedricziel/assistant/issues/103)) ([901db66](https://github.com/cedricziel/assistant/commit/901db660b79d7baa46972402639b5996c0b4b622))

## [0.1.20](https://github.com/cedricziel/assistant/compare/v0.1.19...v0.1.20) (2026-02-28)


### Bug Fixes

* **ci:** build and ship assistant-web-ui binary in release ([#101](https://github.com/cedricziel/assistant/issues/101)) ([06c55cc](https://github.com/cedricziel/assistant/commit/06c55cc9eeca9e860b4275900671fc54903e3889))

## [0.1.19](https://github.com/cedricziel/assistant/compare/v0.1.18...v0.1.19) (2026-02-28)


### Bug Fixes

* **packaging:** correct web-ui binary name and add EnvironmentFile ([#99](https://github.com/cedricziel/assistant/issues/99)) ([5848347](https://github.com/cedricziel/assistant/commit/58483474330008b66d8aade75abf1ad43ec13bdb))

## [0.1.18](https://github.com/cedricziel/assistant/compare/v0.1.17...v0.1.18) (2026-02-27)


### Features

* **web-ui:** add token-based authentication ([#98](https://github.com/cedricziel/assistant/issues/98)) ([bf479d4](https://github.com/cedricziel/assistant/commit/bf479d4fcd34c25f4059455a4c26018454ebe718))
* **web-ui:** add webhook management with HMAC-SHA256 verification ([#95](https://github.com/cedricziel/assistant/issues/95)) ([0d3af9b](https://github.com/cedricziel/assistant/commit/0d3af9b2fb27e9ecaa8c457d86953d3b6bbd413c))

## [0.1.17](https://github.com/cedricziel/assistant/compare/v0.1.16...v0.1.17) (2026-02-27)


### Features

* **skill:** add coding-agent skill for multi-agent background orchestration ([#92](https://github.com/cedricziel/assistant/issues/92)) ([60d73c8](https://github.com/cedricziel/assistant/commit/60d73c847f53b2e766f5909dbaf0c53fedfd08a7))
* **skills:** add playwright-cli skill ([#70](https://github.com/cedricziel/assistant/issues/70)) ([4c48d9c](https://github.com/cedricziel/assistant/commit/4c48d9c947130b2269b32737d72fe493038be212))
* **slack:** add listen mode to control which messages the bot reacts to ([#93](https://github.com/cedricziel/assistant/issues/93)) ([3cea363](https://github.com/cedricziel/assistant/commit/3cea363cd85972dbd3e677b5fc685d18a4de8b0a))
* **tool-executor:** add native process tool for background process management ([#88](https://github.com/cedricziel/assistant/issues/88)) ([0ca39c6](https://github.com/cedricziel/assistant/commit/0ca39c62df3387b01baa8ca974ea532b37907167))


### Bug Fixes

* **skill:** use stdin for prompt delivery to avoid shell escaping issues ([#86](https://github.com/cedricziel/assistant/issues/86)) ([c3757dd](https://github.com/cedricziel/assistant/commit/c3757dd8ae9fb7d318ad1a806af72a243b1b327b))

## [0.1.16](https://github.com/cedricziel/assistant/compare/v0.1.15...v0.1.16) (2026-02-27)


### Features

* add A2A protocol support with agent management UI ([#69](https://github.com/cedricziel/assistant/issues/69)) ([58c68e5](https://github.com/cedricziel/assistant/commit/58c68e556f263ee9359f890959d4d27da2abf8b3))
* add OpenTelemetry metrics with SQLite persistence and analytics dashboard ([#76](https://github.com/cedricziel/assistant/issues/76)) ([6d1364c](https://github.com/cedricziel/assistant/commit/6d1364c5bbd206ff73c3da273b0ece8362d77b3a))
* **skill:** add claude-code-agent skill with async tmux support ([#75](https://github.com/cedricziel/assistant/issues/75)) ([4a052a1](https://github.com/cedricziel/assistant/commit/4a052a147b68a572c62581a5fdb466039703b9f3))


### Bug Fixes

* sync Cargo.lock versions for a2a crates after release bump ([d0dd77e](https://github.com/cedricziel/assistant/commit/d0dd77e50ac60451503944655022643c7cfffaf8))
* use literal version strings in a2a crates for release-please compatibility ([#84](https://github.com/cedricziel/assistant/issues/84)) ([7231c50](https://github.com/cedricziel/assistant/commit/7231c5022081761ba5d555f7aee3d3248f5c4331))

## [0.1.15](https://github.com/cedricziel/assistant/compare/v0.1.14...v0.1.15) (2026-02-26)


### Features

* add subagent support with tool filtering, lifecycle tracking, and OTel observability ([#67](https://github.com/cedricziel/assistant/issues/67)) ([dba4255](https://github.com/cedricziel/assistant/commit/dba4255c7d57638fa30714bff8d99f01f3058e4c))

## [0.1.14](https://github.com/cedricziel/assistant/compare/v0.1.13...v0.1.14) (2026-02-26)


### Bug Fixes

* **cli:** spawn scheduler before interface branches so all modes get scheduled tasks ([#65](https://github.com/cedricziel/assistant/issues/65)) ([be929fb](https://github.com/cedricziel/assistant/commit/be929fb62cdf821d8df4c57e668092d6d27b2c8a))

## [0.1.13](https://github.com/cedricziel/assistant/compare/v0.1.12...v0.1.13) (2026-02-26)


### Features

* add durable topic-based message bus for inter-component communication ([#63](https://github.com/cedricziel/assistant/issues/63)) ([dd6a520](https://github.com/cedricziel/assistant/commit/dd6a52004206ab08843817089e4326f1a55af772))

## [0.1.12](https://github.com/cedricziel/assistant/compare/v0.1.11...v0.1.12) (2026-02-26)


### Features

* **core:** extract memory templates to .md files, add TOOLS/BOOTSTRAP/HEARTBEAT/BOOT ([#60](https://github.com/cedricziel/assistant/issues/60)) ([0837a23](https://github.com/cedricziel/assistant/commit/0837a23515dc60842ca7503a1e1d47c70c24c2d1))


### Bug Fixes

* **packaging:** use /run/systemd/users instead of loginctl for service restart ([#58](https://github.com/cedricziel/assistant/issues/58)) ([d4ff032](https://github.com/cedricziel/assistant/commit/d4ff032460acc6eb04b9ff72d9878a6dc9dc2db3))
* **slack,mattermost:** handle whitespace, no-pad, and data-URI in base64 uploads ([#62](https://github.com/cedricziel/assistant/issues/62)) ([7f753ff](https://github.com/cedricziel/assistant/commit/7f753ff297ae8d9828d90c980a6173256884f680))

## [0.1.11](https://github.com/cedricziel/assistant/compare/v0.1.10...v0.1.11) (2026-02-26)


### Features

* **tool-executor:** add memory-append builtin tool ([#55](https://github.com/cedricziel/assistant/issues/55)) ([4bf16ed](https://github.com/cedricziel/assistant/commit/4bf16eda5f63f6b30086b3d50a5c357f5d99c312))

## [0.1.10](https://github.com/cedricziel/assistant/compare/v0.1.9...v0.1.10) (2026-02-26)


### Features

* add attachment/file sending support across all interfaces ([#56](https://github.com/cedricziel/assistant/issues/56)) ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **cli:** deliver file attachments from tool outputs to disk ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **core:** add Attachment type and attachment support to ToolOutput ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **mattermost:** add file upload tool with multipart support ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **runtime:** collect attachments from tool outputs and add error recovery ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **slack:** add binary/base64 upload support to upload tool ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))


### Bug Fixes

* **runtime:** prevent empty FinalAnswer from poisoning conversation history ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **runtime:** prevent empty FinalAnswer from poisoning conversation history ([#53](https://github.com/cedricziel/assistant/issues/53)) ([e11cf60](https://github.com/cedricziel/assistant/commit/e11cf60dc3851f06ff44e9f4afd68d3f8950e082))

## [0.1.9](https://github.com/cedricziel/assistant/compare/v0.1.8...v0.1.9) (2026-02-25)


### Features

* **runtime:** add sysiphos.heartbeat root span to heartbeat traces ([#50](https://github.com/cedricziel/assistant/issues/50)) ([b468fb9](https://github.com/cedricziel/assistant/commit/b468fb9c90fff007f9784bf98baf105e0647e85a))


### Bug Fixes

* **packaging:** restart services on upgrade instead of leaving them dead ([#52](https://github.com/cedricziel/assistant/issues/52)) ([d61c1c1](https://github.com/cedricziel/assistant/commit/d61c1c171d255f601e0bea31b676cd20af0d18b4))

## [0.1.8](https://github.com/cedricziel/assistant/compare/v0.1.7...v0.1.8) (2026-02-25)


### Features

* **scheduler:** add cancel-task, list-tasks tools and one-shot scheduling ([#49](https://github.com/cedricziel/assistant/issues/49)) ([a9f84aa](https://github.com/cedricziel/assistant/commit/a9f84aa0ba878b705d6604ace771bf12b1118d8a))
* **slack:** receive file attachments with vision support ([#48](https://github.com/cedricziel/assistant/issues/48)) ([fc3fe5b](https://github.com/cedricziel/assistant/commit/fc3fe5bf8ed4fdb6ac2aa2b35d8cc3c3b28f825b))


### Bug Fixes

* **packaging:** use Restart=always so services recover after self-update ([#46](https://github.com/cedricziel/assistant/issues/46)) ([df4c964](https://github.com/cedricziel/assistant/commit/df4c9645937d807429f7b11d487865e574d1d541))

## [0.1.7](https://github.com/cedricziel/assistant/compare/v0.1.6...v0.1.7) (2026-02-25)


### Features

* add OTel log ingestion pipeline with web UI ([#44](https://github.com/cedricziel/assistant/issues/44)) ([0315aa3](https://github.com/cedricziel/assistant/commit/0315aa3d9496e1e9a5a03d00d19c5f91da3ad369))
* **interface-slack:** queue indicator and message stacking for Slack threads ([#42](https://github.com/cedricziel/assistant/issues/42)) ([8bd59a0](https://github.com/cedricziel/assistant/commit/8bd59a0d928070ecaac94ef09b7ece30f5e00331))


### Bug Fixes

* **runtime:** reject end_turn when LLM skips reply in messaging interfaces ([#45](https://github.com/cedricziel/assistant/issues/45)) ([c077b30](https://github.com/cedricziel/assistant/commit/c077b30dc296805b99870b1802691294ba2ba33d))

## [0.1.6](https://github.com/cedricziel/assistant/compare/v0.1.5...v0.1.6) (2026-02-25)


### Features

* add otel tracng ([9972d42](https://github.com/cedricziel/assistant/commit/9972d4205578370667a47fdfd4e0d361152c2027))
* **anthropic:** expose hosted web fetch tool ([40eb8ef](https://github.com/cedricziel/assistant/commit/40eb8ef8f4c8a673f25c1079093d70bef4b34581))
* **anthropic:** wire hosted web search tool ([e21bbf0](https://github.com/cedricziel/assistant/commit/e21bbf02afb0306dc73b2e14370b68ecbef3632e))
* **core:** add AGENTS.md — session startup ritual and memory discipline ([5e15f01](https://github.com/cedricziel/assistant/commit/5e15f0195069af983fb688d20e9019eb7393acef))
* **interface-slack:** add 7 ambient Slack tools and fix thinking responses ([e533c83](https://github.com/cedricziel/assistant/commit/e533c830909683aab7904472ff2aeb9c0d329f66))
* **llm:** add provider metadata to LlmProvider trait ([552c42c](https://github.com/cedricziel/assistant/commit/552c42c58c4d4ad69038fe896f2f5f9d0bc43618))
* **llm:** add response metadata to LlmResponse ([c6f1550](https://github.com/cedricziel/assistant/commit/c6f15504559eae1d598375bc83bc612d551510b7))
* **observability:** add otel spans and trace UI ([141294b](https://github.com/cedricziel/assistant/commit/141294b10e032866c26e2f4055de2700c8160a78))
* **provider-anthropic:** add Anthropic Claude provider ([1d264c2](https://github.com/cedricziel/assistant/commit/1d264c2aed0a0fdbf8ba8613f006f9411ebde5af))
* redesign trace analytics ui ([c523298](https://github.com/cedricziel/assistant/commit/c52329889059b03f696e1a443d12a176e1227db8))
* **refactor:** separate Skills (knowledge) from Tools (executables) ([#36](https://github.com/cedricziel/assistant/issues/36)) ([fc81988](https://github.com/cedricziel/assistant/commit/fc81988d57f1f3a41a22f3d42fea67da72ec2cc8))
* **runtime:** add opt-in GenAI content capture on spans ([50eca82](https://github.com/cedricziel/assistant/commit/50eca8258c7d06d9422acfeb11b4d733fbfd8335))
* **runtime:** align spans with OTel GenAI semantic conventions ([77ddcdc](https://github.com/cedricziel/assistant/commit/77ddcdc66455fb268e2631c5561f1c071fc069a7))
* **runtime:** enrich self-analyze with token usage data ([27eea5a](https://github.com/cedricziel/assistant/commit/27eea5a8d3deae006e3f3a081fb3d1055f799b48))
* **runtime:** propagate OTel trace context across conversation turns ([b2bfaa4](https://github.com/cedricziel/assistant/commit/b2bfaa4c7c1975de789d75ae8bee785c66791ca1))
* **runtime:** raise default max_iterations to 80 ([d8ee35d](https://github.com/cedricziel/assistant/commit/d8ee35d5caefb4f1e478ecd8d665523f50f0a375))
* **signal:** propagate OTel trace context across conversation turns ([e668253](https://github.com/cedricziel/assistant/commit/e6682538b6863bb56ac230b2fab4ff5cc05e52a1))
* **skills:** auto-discover external skill folders ([5e15f01](https://github.com/cedricziel/assistant/commit/5e15f0195069af983fb688d20e9019eb7393acef))
* **skills:** enrich metadata parsing and surface available skills ([e5aa782](https://github.com/cedricziel/assistant/commit/e5aa7825f33191680569366f236893505539cb0c))
* **slack:** treat reactions as turns ([a4148b9](https://github.com/cedricziel/assistant/commit/a4148b98f5ec9c9583b9b530b887449dc9b7cc4c))
* **storage:** add token usage columns to distributed_traces ([a6e763a](https://github.com/cedricziel/assistant/commit/a6e763aba9bae6dcce7d2e89202639e8159ccbaf))
* **ui:** add web trace viewer ([eb41f9c](https://github.com/cedricziel/assistant/commit/eb41f9cbff151dca2ff769e42edbbe65db680b8e))
* **web-ui:** redesign trace analytics UI ([4857a9f](https://github.com/cedricziel/assistant/commit/4857a9f210769064df8b7f44be3fde62521d1e4a))


### Bug Fixes

* **ci:** use inline version strings for release-please compatibility ([#39](https://github.com/cedricziel/assistant/issues/39)) ([f3e3ed9](https://github.com/cedricziel/assistant/commit/f3e3ed95e2229b347679fb530d600e2988c50218))
* **core:** fix SOUL.md memory instructions — remove phantom memory-save tool ([5e15f01](https://github.com/cedricziel/assistant/commit/5e15f0195069af983fb688d20e9019eb7393acef))
* **core:** fix SOUL.md memory instructions — remove phantom memory-save tool ([#37](https://github.com/cedricziel/assistant/issues/37)) ([5e15f01](https://github.com/cedricziel/assistant/commit/5e15f0195069af983fb688d20e9019eb7393acef))
* **interface-slack:** use generic reply/react/upload extension tool names and hide slack-post during threaded turns ([b43164b](https://github.com/cedricziel/assistant/commit/b43164b60b722066a5b6434ea1e6bf41d1631b58))
* **llm:** handle empty content from thinking models (qwen3) ([46be85a](https://github.com/cedricziel/assistant/commit/46be85ab25e78e8d3a9fc82ccbe7251ae095e06e))
* **runtime:** record end_turn tool result ([5d10217](https://github.com/cedricziel/assistant/commit/5d10217f3ef55c4eb8b582895f4c79e075f8eb61))
* **signal:** add missing trace_cx parameter to run_turn_streaming ([e668253](https://github.com/cedricziel/assistant/commit/e6682538b6863bb56ac230b2fab4ff5cc05e52a1))

## [0.1.5](https://github.com/cedricziel/assistant/compare/v0.1.4...v0.1.5) (2026-02-23)


### Features

* **ci:** publish APT/YUM package repo to GitHub Pages ([#34](https://github.com/cedricziel/assistant/issues/34)) ([b166763](https://github.com/cedricziel/assistant/commit/b16676325d6374bcd0479e24a7bea83d5349986d))

## [0.1.4](https://github.com/cedricziel/assistant/compare/v0.1.3...v0.1.4) (2026-02-23)


### Features

* **cli:** unified binary with ambient skill plugin architecture ([#32](https://github.com/cedricziel/assistant/issues/32)) ([90364f2](https://github.com/cedricziel/assistant/commit/90364f25a4cddf0bb11aff1440f0c1326cbaa890))
* **packaging:** systemd user services for Slack and Mattermost bots ([90364f2](https://github.com/cedricziel/assistant/commit/90364f25a4cddf0bb11aff1440f0c1326cbaa890))

## [0.1.3](https://github.com/cedricziel/assistant/compare/v0.1.2...v0.1.3) (2026-02-23)


### Features

* **memory:** memory-get + memory-search with FTS5/vector indexing; remove SafetyGate ([#27](https://github.com/cedricziel/assistant/issues/27)) ([9ddbace](https://github.com/cedricziel/assistant/commit/9ddbaceb2ed8207693dcfbb2175741513eb31129))

## [0.1.2](https://github.com/cedricziel/assistant/compare/v0.1.1...v0.1.2) (2026-02-22)


### Features

* add bash skill ([90d5330](https://github.com/cedricziel/assistant/commit/90d53306e11aae8f888cf3ea4dc80d9d54c9c889))
* **cli:** add reset subcommand to wipe all assistant data ([5670653](https://github.com/cedricziel/assistant/commit/5670653cdcd39674160fac092b973f59454b28f0))
* **core:** add LlmProviderKind to LlmConfig for future provider selection ([ed7abcb](https://github.com/cedricziel/assistant/commit/ed7abcb6e70b5e779a898f5693bf417b749961b9))
* **core:** embed bundled skills into the binary at compile time ([ae0969e](https://github.com/cedricziel/assistant/commit/ae0969e76e4dd081bc738da8518c4e383d3d182f))
* **core:** improve default memory file templates ([6c0c959](https://github.com/cedricziel/assistant/commit/6c0c959c24b919360fbebfd1d6990cc330f7f6dc))
* **core:** instruct LLM when and how to write daily notes ([e8f6cd1](https://github.com/cedricziel/assistant/commit/e8f6cd1ddc9b3ed85e805b560846d03fc9a1458a))
* **interface-slack:** presence and typing status indicators ([3b6ecf5](https://github.com/cedricziel/assistant/commit/3b6ecf5dfdf2cd49eb8b8ad20e5789da16026781))
* **llm:** add LlmProvider trait with Capabilities and ToolSupport ([920d3d3](https://github.com/cedricziel/assistant/commit/920d3d3efcff01f309add3b6f3007a7cd85172e6))
* **llm:** ChatHistoryMessage enum with structured tool-call variants ([96f17d7](https://github.com/cedricziel/assistant/commit/96f17d7c8812075ccaf07a105606dc28d5251ff3))
* **llm:** support multiple simultaneous tool calls ([7ec9d0d](https://github.com/cedricziel/assistant/commit/7ec9d0d43fff6b1f82d2792d01c8b1db88c0f53b))
* **provider-ollama:** new crate with OllamaProvider implementing LlmProvider ([ce6911d](https://github.com/cedricziel/assistant/commit/ce6911d9795d9884ce712163b32ed65461cf22b4))
* **runtime:** add end_turn tool and soften messaging-interface prompt ([8a9f832](https://github.com/cedricziel/assistant/commit/8a9f832f6c60798c1619e75e763ee8c01aa2cbb4))
* **scheduler:** wire scheduler and add heartbeat loop ([2043ee4](https://github.com/cedricziel/assistant/commit/2043ee43386fa4c2a15c3064016ffbfc303cc5d5))
* **skills-executor:** add file-read, file-write, file-edit, file-glob, web-search builtins ([a577eec](https://github.com/cedricziel/assistant/commit/a577eecd2514d76dd57ab8f761abed49dcd72f11))
* **skills-executor:** add memory-patch builtin skill ([1889fe3](https://github.com/cedricziel/assistant/commit/1889fe3f55e94ebf8368d6a7c4eb4ecbe4bc9e45))
* **storage:** persist tool-call and tool-result messages to DB ([e449b24](https://github.com/cedricziel/assistant/commit/e449b2459a832f7c3875668f2e8e2928db67acc2))
* **tools:** JSON Schema param validation and output_schema for ToolHandler ([25345f5](https://github.com/cedricziel/assistant/commit/25345f5319a5144835214cefffbd141772581b20))
* **tools:** proper JSON Schema for all ToolHandler param schemas ([7b383ec](https://github.com/cedricziel/assistant/commit/7b383ec8c0bac302d7ec9b037938ea8a1610e261))
* **tools:** wire output_schema and structured data into observations ([bf02232](https://github.com/cedricziel/assistant/commit/bf02232c2845f8b6c0e0c1c767f04f642ab21168))


### Bug Fixes

* **interface-slack,runtime:** prevent double replies and concurrent turns ([94d6bac](https://github.com/cedricziel/assistant/commit/94d6bac48b5ff35b673f478651a6baac4322fabc))
* **interface-slack:** convert Markdown to Slack mrkdwn before posting ([6e5767c](https://github.com/cedricziel/assistant/commit/6e5767c6c45ecc8c88602d36d8907bbef54c84bf))
* **runtime:** persist tool results for all early-exit paths in orchestrator ([5c96352](https://github.com/cedricziel/assistant/commit/5c9635251a1f9b57c083bb7ec5f4683172d2005f))
* **runtime:** prevent double-posting and wrong tool in Slack auto-post fallback ([1f4fc6b](https://github.com/cedricziel/assistant/commit/1f4fc6bb77e7bbec1b728b886631259920478071))
* **runtime:** require ack before end_turn in messaging interfaces ([9af5fbc](https://github.com/cedricziel/assistant/commit/9af5fbc1d02043312eb6d7d8321b1394c7b423c0))
* **skills:** correct memory-patch SKILL.md frontmatter format ([8cd1dee](https://github.com/cedricziel/assistant/commit/8cd1deedc3e6513c02d2fd2103a4ca5f2b481206))
* **storage:** make migration 005 idempotent with IF NOT EXISTS ([2deb103](https://github.com/cedricziel/assistant/commit/2deb1038184da2783584b87f831a3d4a2c300732))
* **storage:** revert IF NOT EXISTS — macOS system SQLite &lt; 3.37 unsupported ([5d8f730](https://github.com/cedricziel/assistant/commit/5d8f73035bc1c445b9dd41eb3e7cb2cd78b594d3))
* **storage:** track applied migrations to prevent re-running on each launch ([6d8d29b](https://github.com/cedricziel/assistant/commit/6d8d29b992573b63ad66232642fab3ea02e55d9c))
