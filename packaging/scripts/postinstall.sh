#!/bin/sh
set -e

# Reload the *system* systemd daemon so it picks up the new user-unit files
# in /usr/lib/systemd/user/ (this is safe to run as root).
if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload 2>/dev/null || true
fi

# On upgrade: re-enable and restart services that were previously enabled.
# On fresh install: just print instructions.
action="${1:-configure}"

if [ "$action" = "configure" ] && [ -n "$2" ]; then
    # $2 is set to the old version on upgrade — restart services.
    for user_id in $(loginctl list-sessions --no-legend 2>/dev/null | awk '{print $3}' | sort -u); do
        for svc in assistant-slack assistant-mattermost assistant-web-ui; do
            # Only restart if the unit file exists and was previously enabled.
            if systemctl --user -M "${user_id}@.host" is-enabled "$svc" 2>/dev/null | grep -q enabled; then
                systemctl --user -M "${user_id}@.host" daemon-reload 2>/dev/null || true
                systemctl --user -M "${user_id}@.host" restart "$svc" 2>/dev/null || true
            fi
        done
    done
else
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
fi
