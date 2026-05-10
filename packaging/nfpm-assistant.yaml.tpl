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
# nats-server is the NATS JetStream server that the assistant connects to
# as a client via the async-nats library compiled into the binary.
suggests:
  - nats-server

scripts:
  postinstall: packaging/scripts/postinstall.sh
  preremove: packaging/scripts/preremove.sh

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

  # systemd user unit files. Two long-lived processes per host:
  #
  #   1. assistant-orchestrator — single orchestrator (scheduler + bus +
  #      interface adapters). Configure which adapters to load via
  #      ~/.config/assistant/orchestrator.env (ASSISTANT_INTERFACES=slack,matrix).
  #      The single-process design avoids the scheduler race that occurs when
  #      multiple orchestrators each tick the same scheduled_tasks table.
  #
  #   2. assistant-web-ui — HTTP API server (no scheduler).
  #
  # Enable per-user with:
  #   systemctl --user enable --now assistant-orchestrator
  #   systemctl --user enable --now assistant-web-ui
  #
  # Upgrades from package versions that shipped per-interface units
  # (assistant-slack.service, assistant-matrix.service, …) are migrated
  # automatically by packaging/scripts/postinstall.sh.
  - src: packaging/systemd/user/assistant-orchestrator.service
    dst: /usr/lib/systemd/user/assistant-orchestrator.service
    file_info:
      mode: 0644

  - src: packaging/systemd/user/assistant-web-ui.service
    dst: /usr/lib/systemd/user/assistant-web-ui.service
    file_info:
      mode: 0644
