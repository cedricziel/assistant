name: "assistant"
arch: "ARCH_PLACEHOLDER"
platform: "linux"
version: "VERSION_PLACEHOLDER"
maintainer: "Cedric Ziel <cedric@cedric-ziel.com>"
description: "Local self-improving AI assistant (unified: REPL + MCP + Slack + Mattermost)"
homepage: "https://github.com/cedricziel/assistant"
license: "MIT"

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

  # systemd user unit files — enable per-user with:
  #   systemctl --user enable --now assistant-slack
  #   systemctl --user enable --now assistant-mattermost
  #   systemctl --user enable --now assistant-web-ui
  - src: packaging/systemd/user/assistant-slack.service
    dst: /usr/lib/systemd/user/assistant-slack.service
    file_info:
      mode: 0644

  - src: packaging/systemd/user/assistant-mattermost.service
    dst: /usr/lib/systemd/user/assistant-mattermost.service
    file_info:
      mode: 0644

  - src: packaging/systemd/user/assistant-web-ui.service
    dst: /usr/lib/systemd/user/assistant-web-ui.service
    file_info:
      mode: 0644
