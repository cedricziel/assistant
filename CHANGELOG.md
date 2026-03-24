# Changelog

## [0.1.41](https://github.com/cedricziel/assistant/compare/v0.1.54...v0.1.41) (2026-03-24)


### Features

* add A2A protocol support with agent management UI ([#69](https://github.com/cedricziel/assistant/issues/69)) ([58c68e5](https://github.com/cedricziel/assistant/commit/58c68e556f263ee9359f890959d4d27da2abf8b3))
* add attachment/file sending support across all interfaces ([#56](https://github.com/cedricziel/assistant/issues/56)) ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* add bash skill ([90d5330](https://github.com/cedricziel/assistant/commit/90d53306e11aae8f888cf3ea4dc80d9d54c9c889))
* add current timestamp to system prompt ([#193](https://github.com/cedricziel/assistant/issues/193)) ([e9c9db8](https://github.com/cedricziel/assistant/commit/e9c9db8a4a6c92ef86255573de72c26e8c94d329))
* add durable topic-based message bus for inter-component communication ([#63](https://github.com/cedricziel/assistant/issues/63)) ([dd6a520](https://github.com/cedricziel/assistant/commit/dd6a52004206ab08843817089e4326f1a55af772))
* add NATS JetStream message bus backend ([#223](https://github.com/cedricziel/assistant/issues/223)) ([9cd2c31](https://github.com/cedricziel/assistant/commit/9cd2c317e720c6d30a5e948233e392bed4e7b61c))
* add OpenTelemetry metrics with SQLite persistence and analytics dashboard ([#76](https://github.com/cedricziel/assistant/issues/76)) ([6d1364c](https://github.com/cedricziel/assistant/commit/6d1364c5bbd206ff73c3da273b0ece8362d77b3a))
* add OTel log ingestion pipeline with web UI ([#44](https://github.com/cedricziel/assistant/issues/44)) ([0315aa3](https://github.com/cedricziel/assistant/commit/0315aa3d9496e1e9a5a03d00d19c5f91da3ad369))
* add otel tracng ([9972d42](https://github.com/cedricziel/assistant/commit/9972d4205578370667a47fdfd4e0d361152c2027))
* add subagent support with tool filtering, lifecycle tracking, and OTel observability ([#67](https://github.com/cedricziel/assistant/issues/67)) ([dba4255](https://github.com/cedricziel/assistant/commit/dba4255c7d57638fa30714bff8d99f01f3058e4c))
* add voice message transcription (Whisper, Ollama, Deepgram) ([#131](https://github.com/cedricziel/assistant/issues/131)) ([9610c0d](https://github.com/cedricziel/assistant/commit/9610c0d7fe0f335875d9576a346aeee9f65b6ebb))
* **anthropic:** expose hosted web fetch tool ([40eb8ef](https://github.com/cedricziel/assistant/commit/40eb8ef8f4c8a673f25c1079093d70bef4b34581))
* **anthropic:** wire hosted web search tool ([e21bbf0](https://github.com/cedricziel/assistant/commit/e21bbf02afb0306dc73b2e14370b68ecbef3632e))
* **ci:** publish APT/YUM package repo to GitHub Pages ([#34](https://github.com/cedricziel/assistant/issues/34)) ([b166763](https://github.com/cedricziel/assistant/commit/b16676325d6374bcd0479e24a7bea83d5349986d))
* **cli:** add reset subcommand to wipe all assistant data ([5670653](https://github.com/cedricziel/assistant/commit/5670653cdcd39674160fac092b973f59454b28f0))
* **cli:** add spinner progress indicator while orchestrator runs ([97d0dfd](https://github.com/cedricziel/assistant/commit/97d0dfdd61dd7d3011b69e1923cbe71d3ee8d476))
* **cli:** deliver file attachments from tool outputs to disk ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **cli:** unified binary with ambient skill plugin architecture ([#32](https://github.com/cedricziel/assistant/issues/32)) ([90364f2](https://github.com/cedricziel/assistant/commit/90364f25a4cddf0bb11aff1440f0c1326cbaa890))
* **core:** add AGENTS.md — session startup ritual and memory discipline ([5e15f01](https://github.com/cedricziel/assistant/commit/5e15f0195069af983fb688d20e9019eb7393acef))
* **core:** add Attachment type and attachment support to ToolOutput ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **core:** add LlmProviderKind to LlmConfig for future provider selection ([ed7abcb](https://github.com/cedricziel/assistant/commit/ed7abcb6e70b5e779a898f5693bf417b749961b9))
* **core:** embed bundled skills into the binary at compile time ([ae0969e](https://github.com/cedricziel/assistant/commit/ae0969e76e4dd081bc738da8518c4e383d3d182f))
* **core:** extract memory templates to .md files, add TOOLS/BOOTSTRAP/HEARTBEAT/BOOT ([#60](https://github.com/cedricziel/assistant/issues/60)) ([0837a23](https://github.com/cedricziel/assistant/commit/0837a23515dc60842ca7503a1e1d47c70c24c2d1))
* **core:** improve default memory file templates ([6c0c959](https://github.com/cedricziel/assistant/commit/6c0c959c24b919360fbebfd1d6990cc330f7f6dc))
* **core:** instruct LLM when and how to write daily notes ([e8f6cd1](https://github.com/cedricziel/assistant/commit/e8f6cd1ddc9b3ed85e805b560846d03fc9a1458a))
* **deps:** upgrade opentelemetry 0.24 to 0.31 and related crates ([#145](https://github.com/cedricziel/assistant/issues/145)) ([c08f192](https://github.com/cedricziel/assistant/commit/c08f1926be0b8c18a0e8e47b660ad78a33efadfd))
* initial implementation of minimalist self-improving personal AI assistant ([62d7f46](https://github.com/cedricziel/assistant/commit/62d7f4647bbe9a14ab0eedf3aed42e797616756a))
* **install:** add /install CLI command and install_skill MCP tool ([9e7b97a](https://github.com/cedricziel/assistant/commit/9e7b97a0a62a86209c3b093a4777d57f4bc7ee8d))
* integrate external MCP servers as tool sources ([#195](https://github.com/cedricziel/assistant/issues/195)) ([8eab50e](https://github.com/cedricziel/assistant/commit/8eab50e1acc339994646ce2592cd6dbd0e2eb2a4))
* **interface-cli:** unify runtime entrypoints and service migration ([#264](https://github.com/cedricziel/assistant/issues/264)) ([f9fe94c](https://github.com/cedricziel/assistant/commit/f9fe94c347f46952f46e93be2ef81be39ed34dbb))
* **interface-nextcloud:** add Nextcloud Talk webhook-based bot interface ([#196](https://github.com/cedricziel/assistant/issues/196)) ([ec98096](https://github.com/cedricziel/assistant/commit/ec9809672937e7b9eafe110f0ad59b21625b7e1d))
* **interface-signal:** implement Signal messenger interface ([#16](https://github.com/cedricziel/assistant/issues/16)) ([5e8d11d](https://github.com/cedricziel/assistant/commit/5e8d11d07b8b54a286c97b0937a77f51faf8eb37))
* **interface-slack:** add 7 ambient Slack tools and fix thinking responses ([e533c83](https://github.com/cedricziel/assistant/commit/e533c830909683aab7904472ff2aeb9c0d329f66))
* **interface-slack:** presence and typing status indicators ([3b6ecf5](https://github.com/cedricziel/assistant/commit/3b6ecf5dfdf2cd49eb8b8ad20e5789da16026781))
* **interface-slack:** queue indicator and message stacking for Slack threads ([#42](https://github.com/cedricziel/assistant/issues/42)) ([8bd59a0](https://github.com/cedricziel/assistant/commit/8bd59a0d928070ecaac94ef09b7ece30f5e00331))
* **llm,runtime,cli:** stream LLM output token-by-token in the CLI ([#11](https://github.com/cedricziel/assistant/issues/11)) ([866789a](https://github.com/cedricziel/assistant/commit/866789a11ecdeacbda240c07d56a1eb0f45876a7))
* **llm:** add dedicated embedding provider with Voyage AI support ([#109](https://github.com/cedricziel/assistant/issues/109)) ([b71ff13](https://github.com/cedricziel/assistant/commit/b71ff137b8d929159b3cd7af23e2e5dd19d9a9c7))
* **llm:** add LlmProvider trait with Capabilities and ToolSupport ([920d3d3](https://github.com/cedricziel/assistant/commit/920d3d3efcff01f309add3b6f3007a7cd85172e6))
* **llm:** add provider metadata to LlmProvider trait ([552c42c](https://github.com/cedricziel/assistant/commit/552c42c58c4d4ad69038fe896f2f5f9d0bc43618))
* **llm:** add response metadata to LlmResponse ([c6f1550](https://github.com/cedricziel/assistant/commit/c6f15504559eae1d598375bc83bc612d551510b7))
* **llm:** add retry with exponential backoff for transient API errors ([#186](https://github.com/cedricziel/assistant/issues/186)) ([088a0c1](https://github.com/cedricziel/assistant/commit/088a0c17f7c31dd6e6f220d00700a21f6385693b)), closes [#183](https://github.com/cedricziel/assistant/issues/183)
* **llm:** ChatHistoryMessage enum with structured tool-call variants ([96f17d7](https://github.com/cedricziel/assistant/commit/96f17d7c8812075ccaf07a105606dc28d5251ff3))
* **llm:** increase default retry to 20 attempts with 60s max delay ([de7c625](https://github.com/cedricziel/assistant/commit/de7c625114a99c33be7f20ea1e20d72d134471fc))
* **llm:** support multiple simultaneous tool calls ([7ec9d0d](https://github.com/cedricziel/assistant/commit/7ec9d0d43fff6b1f82d2792d01c8b1db88c0f53b))
* **mattermost:** add file upload tool with multipart support ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **mcp-server:** expose each skill as its own MCP tool ([#19](https://github.com/cedricziel/assistant/issues/19)) ([a27ef82](https://github.com/cedricziel/assistant/commit/a27ef824028318ae1fb69ba87ab244cdf180a0cc))
* **memory:** memory-get + memory-search with FTS5/vector indexing; remove SafetyGate ([#27](https://github.com/cedricziel/assistant/issues/27)) ([9ddbace](https://github.com/cedricziel/assistant/commit/9ddbaceb2ed8207693dcfbb2175741513eb31129))
* **observability:** add otel spans and trace UI ([141294b](https://github.com/cedricziel/assistant/commit/141294b10e032866c26e2f4055de2700c8160a78))
* **observability:** add submit_turn lifecycle and correlation telemetry ([#259](https://github.com/cedricziel/assistant/issues/259)) ([ab0a005](https://github.com/cedricziel/assistant/commit/ab0a00580b63618be6910e41819207f31bfde3e4))
* **packaging:** systemd user services for Slack and Mattermost bots ([90364f2](https://github.com/cedricziel/assistant/commit/90364f25a4cddf0bb11aff1440f0c1326cbaa890))
* **provider-anthropic:** add Anthropic Claude provider ([1d264c2](https://github.com/cedricziel/assistant/commit/1d264c2aed0a0fdbf8ba8613f006f9411ebde5af))
* **provider-moonshot:** add Moonshot AI (Kimi) LLM provider ([#124](https://github.com/cedricziel/assistant/issues/124)) ([0c438ed](https://github.com/cedricziel/assistant/commit/0c438ed4b84ece11fe1114f82f871f74500ff718))
* **provider-ollama:** new crate with OllamaProvider implementing LlmProvider ([ce6911d](https://github.com/cedricziel/assistant/commit/ce6911d9795d9884ce712163b32ed65461cf22b4))
* **provider-openai,provider-moonshot:** add hosted web search support ([#126](https://github.com/cedricziel/assistant/issues/126)) ([a90de0a](https://github.com/cedricziel/assistant/commit/a90de0a26fe8af1cdcf210b83e365b5b1a6490f2))
* **provider-openai:** add OpenAI LLM provider with API key and OAuth PKCE auth ([#105](https://github.com/cedricziel/assistant/issues/105)) ([af44f8a](https://github.com/cedricziel/assistant/commit/af44f8a7609c9bc9cbdec3042cc9ca97f633ba3c))
* **provider-openai:** migrate from Chat Completions to Responses API ([#128](https://github.com/cedricziel/assistant/issues/128)) ([89eeffb](https://github.com/cedricziel/assistant/commit/89eeffbe4772565d83f28a8ec7786e89ff10a59f))
* redesign trace analytics ui ([c523298](https://github.com/cedricziel/assistant/commit/c52329889059b03f696e1a443d12a176e1227db8))
* **refactor:** separate Skills (knowledge) from Tools (executables) ([#36](https://github.com/cedricziel/assistant/issues/36)) ([fc81988](https://github.com/cedricziel/assistant/commit/fc81988d57f1f3a41a22f3d42fea67da72ec2cc8))
* **release:** add release-please, binary packaging, and Docker publishing ([#20](https://github.com/cedricziel/assistant/issues/20)) ([64157cf](https://github.com/cedricziel/assistant/commit/64157cfb90579ea06c42645794cd5ecccf8699c9))
* **release:** use RELEASE_PLEASE_TOKEN and release workspace as a whole ([46edf52](https://github.com/cedricziel/assistant/commit/46edf52308bc41f20dc9b68a5956ca6ea77793a1))
* **runtime:** add end_turn tool and soften messaging-interface prompt ([8a9f832](https://github.com/cedricziel/assistant/commit/8a9f832f6c60798c1619e75e763ee8c01aa2cbb4))
* **runtime:** add opt-in GenAI content capture on spans ([50eca82](https://github.com/cedricziel/assistant/commit/50eca8258c7d06d9422acfeb11b4d733fbfd8335))
* **runtime:** add sysiphos.heartbeat root span to heartbeat traces ([#50](https://github.com/cedricziel/assistant/issues/50)) ([b468fb9](https://github.com/cedricziel/assistant/commit/b468fb9c90fff007f9784bf98baf105e0647e85a))
* **runtime:** align spans with OTel GenAI semantic conventions ([77ddcdc](https://github.com/cedricziel/assistant/commit/77ddcdc66455fb268e2631c5561f1c071fc069a7))
* **runtime:** collect attachments from tool outputs and add error recovery ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **runtime:** enrich self-analyze with token usage data ([27eea5a](https://github.com/cedricziel/assistant/commit/27eea5a8d3deae006e3f3a081fb3d1055f799b48))
* **runtime:** extract shared helpers and decompose ReAct loops ([#164](https://github.com/cedricziel/assistant/issues/164)) ([080b427](https://github.com/cedricziel/assistant/commit/080b427c304b5a9ffa87fc1f5be7e2508ac55db6))
* **runtime:** inject SKILL.md body as sub-prompt for prompt-tier skills ([9a28eec](https://github.com/cedricziel/assistant/commit/9a28eec8958ee71e9a2f54cc064e2733871a5496))
* **runtime:** introduce multi-agent contexts with scoped runtime and web UI ([#227](https://github.com/cedricziel/assistant/issues/227)) ([f3226b1](https://github.com/cedricziel/assistant/commit/f3226b14aa8aa0b5d189f8582499811928d6da8a))
* **runtime:** propagate OTel trace context across conversation turns ([b2bfaa4](https://github.com/cedricziel/assistant/commit/b2bfaa4c7c1975de789d75ae8bee785c66791ca1))
* **runtime:** raise default max_iterations to 80 ([d8ee35d](https://github.com/cedricziel/assistant/commit/d8ee35d5caefb4f1e478ecd8d665523f50f0a375))
* **runtime:** re-implement memory indexer ([#188](https://github.com/cedricziel/assistant/issues/188)) ([97d9016](https://github.com/cedricziel/assistant/commit/97d90163ba1c90424aa35c487148000c0f4da8b8))
* **scheduler:** add cancel-task, list-tasks tools and one-shot scheduling ([#49](https://github.com/cedricziel/assistant/issues/49)) ([a9f84aa](https://github.com/cedricziel/assistant/commit/a9f84aa0ba878b705d6604ace771bf12b1118d8a))
* **scheduler:** wire scheduler and add heartbeat loop ([2043ee4](https://github.com/cedricziel/assistant/commit/2043ee43386fa4c2a15c3064016ffbfc303cc5d5))
* **signal:** propagate OTel trace context across conversation turns ([e668253](https://github.com/cedricziel/assistant/commit/e6682538b6863bb56ac230b2fab4ff5cc05e52a1))
* **skill:** add claude-code-agent skill with async tmux support ([#75](https://github.com/cedricziel/assistant/issues/75)) ([4a052a1](https://github.com/cedricziel/assistant/commit/4a052a147b68a572c62581a5fdb466039703b9f3))
* **skill:** add coding-agent skill for multi-agent background orchestration ([#92](https://github.com/cedricziel/assistant/issues/92)) ([60d73c8](https://github.com/cedricziel/assistant/commit/60d73c847f53b2e766f5909dbaf0c53fedfd08a7))
* **skills-executor:** add file-read, file-write, file-edit, file-glob, web-search builtins ([a577eec](https://github.com/cedricziel/assistant/commit/a577eecd2514d76dd57ab8f761abed49dcd72f11))
* **skills-executor:** add memory-patch builtin skill ([1889fe3](https://github.com/cedricziel/assistant/commit/1889fe3f55e94ebf8368d6a7c4eb4ecbe4bc9e45))
* **skills-executor:** wire up WASM execution tier via extism ([#12](https://github.com/cedricziel/assistant/issues/12)) ([22dfd6c](https://github.com/cedricziel/assistant/commit/22dfd6c014018cae49f70fee2a99c4cd449091a6))
* **skills:** add playwright-cli skill ([#70](https://github.com/cedricziel/assistant/issues/70)) ([4c48d9c](https://github.com/cedricziel/assistant/commit/4c48d9c947130b2269b32737d72fe493038be212))
* **skills:** auto-discover external skill folders ([5e15f01](https://github.com/cedricziel/assistant/commit/5e15f0195069af983fb688d20e9019eb7393acef))
* **skills:** enrich metadata parsing and surface available skills ([e5aa782](https://github.com/cedricziel/assistant/commit/e5aa7825f33191680569366f236893505539cb0c))
* **skills:** expose compatibility field in list-skills and declare requirements ([#138](https://github.com/cedricziel/assistant/issues/138)) ([f1b62ff](https://github.com/cedricziel/assistant/commit/f1b62ff9aa696ce8c6ff51b151966d1bd9758679))
* **skills:** LLM-powered self-analyze generates real SKILL.md refinement proposals ([fb738db](https://github.com/cedricziel/assistant/commit/fb738dbc75a4466de821551a0f5e1861c2f108a4))
* **skills:** sync embedded builtin skills to disk on startup ([#140](https://github.com/cedricziel/assistant/issues/140)) ([ca5c9cf](https://github.com/cedricziel/assistant/commit/ca5c9cfd7a997e8ad831e396154278ea6bcc5b5c)), closes [#81](https://github.com/cedricziel/assistant/issues/81)
* Slack and Mattermost messenger interfaces with full conversation history ([#18](https://github.com/cedricziel/assistant/issues/18)) ([7158b58](https://github.com/cedricziel/assistant/commit/7158b587e323359d627fda650ceab32a1c074b7a))
* **slack:** add binary/base64 upload support to upload tool ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **slack:** add listen mode to control which messages the bot reacts to ([#93](https://github.com/cedricziel/assistant/issues/93)) ([3cea363](https://github.com/cedricziel/assistant/commit/3cea363cd85972dbd3e677b5fc685d18a4de8b0a))
* **slack:** receive file attachments with vision support ([#48](https://github.com/cedricziel/assistant/issues/48)) ([fc3fe5b](https://github.com/cedricziel/assistant/commit/fc3fe5bf8ed4fdb6ac2aa2b35d8cc3c3b28f825b))
* **slack:** treat reactions as turns ([a4148b9](https://github.com/cedricziel/assistant/commit/a4148b98f5ec9c9583b9b530b887449dc9b7cc4c))
* **storage:** add token usage columns to distributed_traces ([a6e763a](https://github.com/cedricziel/assistant/commit/a6e763aba9bae6dcce7d2e89202639e8159ccbaf))
* **storage:** persist tool-call and tool-result messages to DB ([e449b24](https://github.com/cedricziel/assistant/commit/e449b2459a832f7c3875668f2e8e2928db67acc2))
* **tool-executor:** add memory-append builtin tool ([#55](https://github.com/cedricziel/assistant/issues/55)) ([4bf16ed](https://github.com/cedricziel/assistant/commit/4bf16eda5f63f6b30086b3d50a5c357f5d99c312))
* **tool-executor:** add native process tool for background process management ([#88](https://github.com/cedricziel/assistant/issues/88)) ([0ca39c6](https://github.com/cedricziel/assistant/commit/0ca39c62df3387b01baa8ca974ea532b37907167))
* **tools:** JSON Schema param validation and output_schema for ToolHandler ([25345f5](https://github.com/cedricziel/assistant/commit/25345f5319a5144835214cefffbd141772581b20))
* **tools:** proper JSON Schema for all ToolHandler param schemas ([7b383ec](https://github.com/cedricziel/assistant/commit/7b383ec8c0bac302d7ec9b037938ea8a1610e261))
* **tools:** wire output_schema and structured data into observations ([bf02232](https://github.com/cedricziel/assistant/commit/bf02232c2845f8b6c0e0c1c767f04f642ab21168))
* **transcription:** Add audio format conversion for Deepgram ([#139](https://github.com/cedricziel/assistant/issues/139)) ([b151736](https://github.com/cedricziel/assistant/commit/b1517367029c161d3d66da9182bcd19855dbf661))
* **ui:** add web trace viewer ([eb41f9c](https://github.com/cedricziel/assistant/commit/eb41f9cbff151dca2ff769e42edbbe65db680b8e))
* **web-ui:** add --no-secure-cookie flag for plain HTTP on non-loopback ([#103](https://github.com/cedricziel/assistant/issues/103)) ([901db66](https://github.com/cedricziel/assistant/commit/901db660b79d7baa46972402639b5996c0b4b622))
* **web-ui:** add health and readiness endpoints ([#225](https://github.com/cedricziel/assistant/issues/225)) ([786ac5c](https://github.com/cedricziel/assistant/commit/786ac5c33273321af4ee0ca2020eb12e4dc395d7))
* **web-ui:** add PWA support for installable offline-capable app ([#112](https://github.com/cedricziel/assistant/issues/112)) ([395db47](https://github.com/cedricziel/assistant/commit/395db47b4647ed27c5ed63fc5809dfcb90ef1efb))
* **web-ui:** add token-based authentication ([#98](https://github.com/cedricziel/assistant/issues/98)) ([bf479d4](https://github.com/cedricziel/assistant/commit/bf479d4fcd34c25f4059455a4c26018454ebe718))
* **web-ui:** add webhook management with HMAC-SHA256 verification ([#95](https://github.com/cedricziel/assistant/issues/95)) ([0d3af9b](https://github.com/cedricziel/assistant/commit/0d3af9b2fb27e9ecaa8c457d86953d3b6bbd413c))
* **web-ui:** chat interface with LLM streaming and Askama template migration ([#107](https://github.com/cedricziel/assistant/issues/107)) ([e0de5b9](https://github.com/cedricziel/assistant/commit/e0de5b94866b399577dabd12a49438e0a192c399))
* **web-ui:** introduce Stimulus for workflow secret reveal ([#252](https://github.com/cedricziel/assistant/issues/252)) ([754de60](https://github.com/cedricziel/assistant/commit/754de6077297dae0f08304c44c0ade1c8ce00a6c))
* **web-ui:** make workflow editor mobile-first and Stimulus-driven ([#256](https://github.com/cedricziel/assistant/issues/256)) ([e23450a](https://github.com/cedricziel/assistant/commit/e23450ab6251eb91cc597808cdc1d6af9b47a373))
* **web-ui:** redesign trace analytics UI ([4857a9f](https://github.com/cedricziel/assistant/commit/4857a9f210769064df8b7f44be3fde62521d1e4a))
* **web-ui:** redesign workflow detail and editor navigation ([#260](https://github.com/cedricziel/assistant/issues/260)) ([7825d59](https://github.com/cedricziel/assistant/commit/7825d59ac68d9190ad7f64bfeeb88607c9bde169))
* **web-ui:** redesign workflow form for desktop split layout ([#235](https://github.com/cedricziel/assistant/issues/235)) ([b2d1c21](https://github.com/cedricziel/assistant/commit/b2d1c214f2c6846a5c62f670d9b021455bf24591))
* **web-ui:** route chat through Orchestrator for full assistant capabilities ([#121](https://github.com/cedricziel/assistant/issues/121)) ([7aabc85](https://github.com/cedricziel/assistant/commit/7aabc850b84ca38574fd96d1bcd59a7a3198d656))
* **workflows:** add loop-enabled workflow graph management ([#228](https://github.com/cedricziel/assistant/issues/228)) ([fe883e5](https://github.com/cedricziel/assistant/commit/fe883e517e096c46701a4c60077bbfee563ce195))


### Bug Fixes

* **binaries:** add clap version flags to all clap programs ([#254](https://github.com/cedricziel/assistant/issues/254)) ([924aab7](https://github.com/cedricziel/assistant/commit/924aab75ac3698f8f9c6ceebf888ee2ef3e752b3))
* **ci:** add make vendor to release build workflow ([38aba3d](https://github.com/cedricziel/assistant/commit/38aba3d1aa08659c2e68e31a41d606922012d3ec))
* **ci:** build and ship assistant-web-ui binary in release ([#101](https://github.com/cedricziel/assistant/issues/101)) ([06c55cc](https://github.com/cedricziel/assistant/commit/06c55cc9eeca9e860b4275900671fc54903e3889))
* **ci:** include both amd64 and arm64 debs in APT Packages index ([d375a1b](https://github.com/cedricziel/assistant/commit/d375a1b2431981a396af050bc6a32293a285a963))
* **ci:** never cancel in-progress CI runs on main ([ac76ebf](https://github.com/cedricziel/assistant/commit/ac76ebfe6f4bfcefff49e58774ad22f23aa39e6d))
* **ci:** run smoke tests against GitHub Actions Ollama service ([#243](https://github.com/cedricziel/assistant/issues/243)) ([60e4abd](https://github.com/cedricziel/assistant/commit/60e4abd073187c3c37efbb1351a1a11651d03b9c))
* **ci:** use force-with-lease for gh-pages deploy safety ([#181](https://github.com/cedricziel/assistant/issues/181)) ([26f63cf](https://github.com/cedricziel/assistant/commit/26f63cfe0fbefcc925676cdb0c2f2bbe872a3b06))
* **ci:** use inline version strings for release-please compatibility ([#39](https://github.com/cedricziel/assistant/issues/39)) ([f3e3ed9](https://github.com/cedricziel/assistant/commit/f3e3ed95e2229b347679fb530d600e2988c50218))
* **cli:** enable clap env attribute support ([0949a4e](https://github.com/cedricziel/assistant/commit/0949a4e01fa761e2d187fc9c87ff1874fdbefdd7))
* **cli:** spawn scheduler before interface branches so all modes get scheduled tasks ([#65](https://github.com/cedricziel/assistant/issues/65)) ([be929fb](https://github.com/cedricziel/assistant/commit/be929fb62cdf821d8df4c57e668092d6d27b2c8a))
* **cli:** spawn turn worker in standalone interface modes ([#204](https://github.com/cedricziel/assistant/issues/204)) ([d2104d3](https://github.com/cedricziel/assistant/commit/d2104d35f5a06dcf699ebb78d5ed3daea0f8904c))
* **core:** fix SOUL.md memory instructions — remove phantom memory-save tool ([5e15f01](https://github.com/cedricziel/assistant/commit/5e15f0195069af983fb688d20e9019eb7393acef))
* **core:** fix SOUL.md memory instructions — remove phantom memory-save tool ([#37](https://github.com/cedricziel/assistant/issues/37)) ([5e15f01](https://github.com/cedricziel/assistant/commit/5e15f0195069af983fb688d20e9019eb7393acef))
* **interface-cli:** prevent duplicate turn consumption in interface modes ([#262](https://github.com/cedricziel/assistant/issues/262)) ([6b2f9f6](https://github.com/cedricziel/assistant/commit/6b2f9f61624ed5effaef900d7c3525cfcb350383))
* **interface-slack,runtime:** prevent double replies and concurrent turns ([94d6bac](https://github.com/cedricziel/assistant/commit/94d6bac48b5ff35b673f478651a6baac4322fabc))
* **interface-slack:** align websocket TLS dependency versions ([#257](https://github.com/cedricziel/assistant/issues/257)) ([0e88526](https://github.com/cedricziel/assistant/commit/0e8852642fe083f43791dbd272c76d766f5ddd9f))
* **interface-slack:** convert Markdown to Slack mrkdwn before posting ([6e5767c](https://github.com/cedricziel/assistant/commit/6e5767c6c45ecc8c88602d36d8907bbef54c84bf))
* **interface-slack:** strip cite tags and convert markdown tables in mrkdwn output ([#108](https://github.com/cedricziel/assistant/issues/108)) ([b88a93b](https://github.com/cedricziel/assistant/commit/b88a93b2596073791d16e1494d8349719cdd04a6))
* **interface-slack:** use generic reply/react/upload extension tool names and hide slack-post during threaded turns ([b43164b](https://github.com/cedricziel/assistant/commit/b43164b60b722066a5b6434ea1e6bf41d1631b58))
* **lint:** derive Default for config structs, remove redundant closure ([0f19e59](https://github.com/cedricziel/assistant/commit/0f19e594a56b1c4f17e4e7d0fc20030eea4529e0))
* **llm:** handle empty content from thinking models (qwen3) ([46be85a](https://github.com/cedricziel/assistant/commit/46be85ab25e78e8d3a9fc82ccbe7251ae095e06e))
* **llm:** make retry config injectable and lower default from 20 to 3 ([#207](https://github.com/cedricziel/assistant/issues/207)) ([4dca418](https://github.com/cedricziel/assistant/commit/4dca418f3398577797d561a3a59a6a053a3b6cab))
* **mcp-client:** address CodeRabbit review round 2 ([#205](https://github.com/cedricziel/assistant/issues/205)) ([a036bf0](https://github.com/cedricziel/assistant/commit/a036bf0a49e7f40750664ee4685b71201054fb28))
* **observability:** propagate root spans across bus and webhook flows ([#250](https://github.com/cedricziel/assistant/issues/250)) ([fff345d](https://github.com/cedricziel/assistant/commit/fff345d9012db93c8359c957c4db9c23f001a665))
* **observability:** separate service and span names across telemetry ([#247](https://github.com/cedricziel/assistant/issues/247)) ([cde3b18](https://github.com/cedricziel/assistant/commit/cde3b18d6deeafa7d13bbba9abed70821d77af59))
* **otel-exporter-sqlite:** provide Tokio context for batch processor threads ([#184](https://github.com/cedricziel/assistant/issues/184)) ([f635b3c](https://github.com/cedricziel/assistant/commit/f635b3c082f3afad7fbb9e1a6e2539b0f22923bd))
* **packaging:** correct web-ui binary name and add EnvironmentFile ([#99](https://github.com/cedricziel/assistant/issues/99)) ([5848347](https://github.com/cedricziel/assistant/commit/58483474330008b66d8aade75abf1ad43ec13bdb))
* **packaging:** restart services on upgrade instead of leaving them dead ([#52](https://github.com/cedricziel/assistant/issues/52)) ([d61c1c1](https://github.com/cedricziel/assistant/commit/d61c1c171d255f601e0bea31b676cd20af0d18b4))
* **packaging:** use /run/systemd/users instead of loginctl for service restart ([#58](https://github.com/cedricziel/assistant/issues/58)) ([d4ff032](https://github.com/cedricziel/assistant/commit/d4ff032460acc6eb04b9ff72d9878a6dc9dc2db3))
* **packaging:** use Restart=always so services recover after self-update ([#46](https://github.com/cedricziel/assistant/issues/46)) ([df4c964](https://github.com/cedricziel/assistant/commit/df4c9645937d807429f7b11d487865e574d1d541))
* patch nextcloud version ([#208](https://github.com/cedricziel/assistant/issues/208)) ([daa8dd9](https://github.com/cedricziel/assistant/commit/daa8dd97368ccfef12f0fb0e8f076f8c4edbe6ae))
* **release:** explicit versions in crates and update release-please config ([28bf899](https://github.com/cedricziel/assistant/commit/28bf899e4934fa13594c48f929762eeb0ffb7faa))
* **release:** install modern protoc in cross aarch64 builds ([#276](https://github.com/cedricziel/assistant/issues/276)) ([fa4ac34](https://github.com/cedricziel/assistant/commit/fa4ac346357370c13c4bdb2360bf5ea4986b7a87))
* **release:** install protoc in aarch64 cross pre-build ([#232](https://github.com/cedricziel/assistant/issues/232)) ([1596a47](https://github.com/cedricziel/assistant/commit/1596a4767f5053f08880ae92e3d6f262b12c7ed6))
* **release:** prefix extra-files paths with / for repo-root resolution ([fbedb56](https://github.com/cedricziel/assistant/commit/fbedb561d1972adb1d29d24462190eeb259c1b95))
* **release:** remove illegal ../ path traversal from changelog-path ([668c8ac](https://github.com/cedricziel/assistant/commit/668c8ac529658b95d9f4d1d7d2d5aebcbabf9ea1))
* **runtime:** persist subagent thinking steps to DB ([#167](https://github.com/cedricziel/assistant/issues/167)) ([92e1ab0](https://github.com/cedricziel/assistant/commit/92e1ab0d51ab862ee61ebdae100c989489545f74))
* **runtime:** persist tool results for all early-exit paths in orchestrator ([5c96352](https://github.com/cedricziel/assistant/commit/5c9635251a1f9b57c083bb7ec5f4683172d2005f))
* **runtime:** prevent double-posting and wrong tool in Slack auto-post fallback ([1f4fc6b](https://github.com/cedricziel/assistant/commit/1f4fc6bb77e7bbec1b728b886631259920478071))
* **runtime:** prevent empty FinalAnswer from poisoning conversation history ([e5ae6f1](https://github.com/cedricziel/assistant/commit/e5ae6f14a42fc5b60bc176c82ee2238a41a54ea2))
* **runtime:** prevent empty FinalAnswer from poisoning conversation history ([#53](https://github.com/cedricziel/assistant/issues/53)) ([e11cf60](https://github.com/cedricziel/assistant/commit/e11cf60dc3851f06ff44e9f4afd68d3f8950e082))
* **runtime:** record end_turn tool result ([5d10217](https://github.com/cedricziel/assistant/commit/5d10217f3ef55c4eb8b582895f4c79e075f8eb61))
* **runtime:** record max_iterations error in metrics ([#169](https://github.com/cedricziel/assistant/issues/169)) ([f8cdd46](https://github.com/cedricziel/assistant/commit/f8cdd4677baefa2bfe5b8ea5178facb065ce1ff5))
* **runtime:** reject end_turn when LLM skips reply in messaging interfaces ([#45](https://github.com/cedricziel/assistant/issues/45)) ([c077b30](https://github.com/cedricziel/assistant/commit/c077b30dc296805b99870b1802691294ba2ba33d))
* **runtime:** replace tracing .enter() guards with .instrument() in async code ([#132](https://github.com/cedricziel/assistant/issues/132)) ([21206a2](https://github.com/cedricziel/assistant/commit/21206a2164c7462c58042aefd66a00ad4444742b)), closes [#116](https://github.com/cedricziel/assistant/issues/116)
* **runtime:** require ack before end_turn in messaging interfaces ([9af5fbc](https://github.com/cedricziel/assistant/commit/9af5fbc1d02043312eb6d7d8321b1394c7b423c0))
* **runtime:** scope turn workers by interface to prevent cross-service theft ([#130](https://github.com/cedricziel/assistant/issues/130)) ([9b672b2](https://github.com/cedricziel/assistant/commit/9b672b2b6d405dee7264fe49dd293c6d91acdfeb))
* **runtime:** use tokio::fs for BOOT.md and HEARTBEAT.md reads ([#170](https://github.com/cedricziel/assistant/issues/170)) ([9b39de5](https://github.com/cedricziel/assistant/commit/9b39de5501eff5be94e86a406c23e7cabc438235))
* **signal:** add missing trace_cx parameter to run_turn_streaming ([e668253](https://github.com/cedricziel/assistant/commit/e6682538b6863bb56ac230b2fab4ff5cc05e52a1))
* **skills:** correct memory-patch SKILL.md frontmatter format ([8cd1dee](https://github.com/cedricziel/assistant/commit/8cd1deedc3e6513c02d2fd2103a4ca5f2b481206))
* **skill:** use stdin for prompt delivery to avoid shell escaping issues ([#86](https://github.com/cedricziel/assistant/issues/86)) ([c3757dd](https://github.com/cedricziel/assistant/commit/c3757dd8ae9fb7d318ad1a806af72a243b1b327b))
* **slack,mattermost:** handle whitespace, no-pad, and data-URI in base64 uploads ([#62](https://github.com/cedricziel/assistant/issues/62)) ([7f753ff](https://github.com/cedricziel/assistant/commit/7f753ff297ae8d9828d90c980a6173256884f680))
* **slack:** only respond in threads where the bot was @-mentioned ([#206](https://github.com/cedricziel/assistant/issues/206)) ([7b174e9](https://github.com/cedricziel/assistant/commit/7b174e92f3da6ce9e011ed22885fcc8793aab52b))
* **slack:** set presence on every reconnect so bot appears online ([#172](https://github.com/cedricziel/assistant/issues/172)) ([21fc06d](https://github.com/cedricziel/assistant/commit/21fc06d1e1aa27fb8f35d4bad6cf7b068a73ec3d))
* **storage:** cast metric aggregations to REAL for sqlx f64 decoding ([3194520](https://github.com/cedricziel/assistant/commit/31945205b355a0c0d78f6ca0df95ab51f472d789))
* **storage:** make agent-scope migrations sqlite-safe ([#233](https://github.com/cedricziel/assistant/issues/233)) ([2a7b5f4](https://github.com/cedricziel/assistant/commit/2a7b5f4d43244c27336192f87a21786e9866d1c1))
* **storage:** make migration 005 idempotent with IF NOT EXISTS ([2deb103](https://github.com/cedricziel/assistant/commit/2deb1038184da2783584b87f831a3d4a2c300732))
* **storage:** revert IF NOT EXISTS — macOS system SQLite &lt; 3.37 unsupported ([5d8f730](https://github.com/cedricziel/assistant/commit/5d8f73035bc1c445b9dd41eb3e7cb2cd78b594d3))
* **storage:** set global PRAGMA busy_timeout to avoid SQLITE_BUSY ([#152](https://github.com/cedricziel/assistant/issues/152)) ([09e9cb3](https://github.com/cedricziel/assistant/commit/09e9cb315220b3b708460b392a062ebc1b02b6c8))
* **storage:** track applied migrations to prevent re-running on each launch ([6d8d29b](https://github.com/cedricziel/assistant/commit/6d8d29b992573b63ad66232642fab3ea02e55d9c))
* sync Cargo.lock versions for a2a crates after release bump ([d0dd77e](https://github.com/cedricziel/assistant/commit/d0dd77e50ac60451503944655022643c7cfffaf8))
* **transcription:** improve error logging for audio conversion failures ([#176](https://github.com/cedricziel/assistant/issues/176)) ([23b34c6](https://github.com/cedricziel/assistant/commit/23b34c601c768614abd404299be4144a3f92d143))
* **transcription:** log file magic bytes and preserve input on failure ([#178](https://github.com/cedricziel/assistant/issues/178)) ([8258766](https://github.com/cedricziel/assistant/commit/825876694f0d5d8588a4248d227a7c6decb7e776))
* **transcription:** use atomic nonce for unique temp file names ([#175](https://github.com/cedricziel/assistant/issues/175)) ([4e21274](https://github.com/cedricziel/assistant/commit/4e21274b2904b8b6827a6e49e099e61f37c1a60f))
* truncate bash output and increase turn timeout ([#110](https://github.com/cedricziel/assistant/issues/110)) ([67a1327](https://github.com/cedricziel/assistant/commit/67a132729b971f070d2cac62c082001da10539e3))
* **upload:** replace content_base64 with path param to prevent binary data in LLM context ([#118](https://github.com/cedricziel/assistant/issues/118)) ([84f08c6](https://github.com/cedricziel/assistant/commit/84f08c6b87786126514a5eb5beea7ccaac2427a0))
* use literal version strings in a2a crates for release-please compatibility ([#84](https://github.com/cedricziel/assistant/issues/84)) ([7231c50](https://github.com/cedricziel/assistant/commit/7231c5022081761ba5d555f7aee3d3248f5c4331))
* **web-ui:** follow up PR247 review feedback ([#249](https://github.com/cedricziel/assistant/issues/249)) ([f170099](https://github.com/cedricziel/assistant/commit/f17009974eded36c21b45734ebee34e26ed589d8))
* **web-ui:** harden responsive navigation and accessibility ([#241](https://github.com/cedricziel/assistant/issues/241)) ([e79bd5b](https://github.com/cedricziel/assistant/commit/e79bd5b11661b09041e5f2603705095e62af5a1e))
* **web-ui:** harden workflow detail UX and mobile navigation ([#253](https://github.com/cedricziel/assistant/issues/253)) ([6b1cf26](https://github.com/cedricziel/assistant/commit/6b1cf2688b657c2a566bc0dc6267575a27f26064))
* **web-ui:** increase screenshot diff tolerance to 5% for cross-platform fonts ([969bf4a](https://github.com/cedricziel/assistant/commit/969bf4a122b498a24dfc75127034ac2b9a759a82))
* **web-ui:** restore scrolling on workflow pages ([#245](https://github.com/cedricziel/assistant/issues/245)) ([eb657f4](https://github.com/cedricziel/assistant/commit/eb657f4ccd55f5ba971bb2cd08b701dac58371b4))


### Performance Improvements

* **storage:** wrap OTel SQLite exporters in batch transactions ([#151](https://github.com/cedricziel/assistant/issues/151)) ([8ec7a28](https://github.com/cedricziel/assistant/commit/8ec7a283ceac3d3211eee593d0577bf67bd2ef87))


### Miscellaneous Chores

* release 0.1.41 ([e0853e1](https://github.com/cedricziel/assistant/commit/e0853e114970732ffe918b7c1ff67fc0ec16622b))

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
