# Changelog

## [0.1.1](https://github.com/cedricziel/assistant/compare/assistant-mcp-server-v0.1.0...assistant-mcp-server-v0.1.1) (2026-02-22)


### Features

* initial implementation of minimalist self-improving personal AI assistant ([62d7f46](https://github.com/cedricziel/assistant/commit/62d7f4647bbe9a14ab0eedf3aed42e797616756a))
* **install:** add /install CLI command and install_skill MCP tool ([9e7b97a](https://github.com/cedricziel/assistant/commit/9e7b97a0a62a86209c3b093a4777d57f4bc7ee8d))
* **mcp-server:** expose each skill as its own MCP tool ([#19](https://github.com/cedricziel/assistant/issues/19)) ([a27ef82](https://github.com/cedricziel/assistant/commit/a27ef824028318ae1fb69ba87ab244cdf180a0cc))
* **release:** add release-please, binary packaging, and Docker publishing ([#20](https://github.com/cedricziel/assistant/issues/20)) ([64157cf](https://github.com/cedricziel/assistant/commit/64157cfb90579ea06c42645794cd5ecccf8699c9))
* **skills:** LLM-powered self-analyze generates real SKILL.md refinement proposals ([fb738db](https://github.com/cedricziel/assistant/commit/fb738dbc75a4466de821551a0f5e1861c2f108a4))
* Slack and Mattermost messenger interfaces with full conversation history ([#18](https://github.com/cedricziel/assistant/issues/18)) ([7158b58](https://github.com/cedricziel/assistant/commit/7158b587e323359d627fda650ceab32a1c074b7a))


### Bug Fixes

* **lint:** derive Default for config structs, remove redundant closure ([0f19e59](https://github.com/cedricziel/assistant/commit/0f19e594a56b1c4f17e4e7d0fc20030eea4529e0))
* **release:** explicit versions in crates and update release-please config ([28bf899](https://github.com/cedricziel/assistant/commit/28bf899e4934fa13594c48f929762eeb0ffb7faa))
