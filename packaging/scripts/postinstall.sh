#!/bin/sh
set -e

# Reload the *system* systemd daemon so it picks up the new user-unit files
# in /usr/lib/systemd/user/ (this is safe to run as root).
if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload 2>/dev/null || true
fi

cat <<'MSG'

assistant has been installed.  To run the Slack or Mattermost bot as a
background service for your user account:

  1. Make sure ~/.assistant/config.toml contains your credentials.

  2. Enable and start the service(s) you need:

       systemctl --user enable --now assistant-slack
       systemctl --user enable --now assistant-mattermost

  3. To have the service start at boot even when you are not logged in:

       loginctl enable-linger $USER

  View logs with:
    journalctl --user -u assistant-slack -f
    journalctl --user -u assistant-mattermost -f

MSG
