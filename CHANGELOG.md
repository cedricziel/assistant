# Changelog

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
