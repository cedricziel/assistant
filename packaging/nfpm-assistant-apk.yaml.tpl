name: "assistant"
arch: "ARCH_PLACEHOLDER"
platform: "linux"
version: "VERSION_PLACEHOLDER"
maintainer: "Cedric Ziel <cedric@cedric-ziel.com>"
description: "Local self-improving AI assistant (unified: orchestrator + worker + webui + MCP + interfaces)"
homepage: "https://github.com/cedricziel/assistant"
license: "MIT"

depends:
  - ffmpeg

# Optional: only needed when [bus] kind = "nats" is configured.
suggests:
  - nats-server

scripts:
  postinstall: packaging/scripts/postinstall-alpine.sh
  preremove: packaging/scripts/preremove-alpine.sh

contents:
  # Main binary
  - src: BIN_DIR_PLACEHOLDER/assistant
    dst: /usr/local/bin/assistant
    file_info:
      mode: 0755

  # Default config template — never overwritten on upgrade
  - src: BIN_DIR_PLACEHOLDER/config.toml.example
    dst: /etc/assistant/config.toml.example
    type: config|noreplace
