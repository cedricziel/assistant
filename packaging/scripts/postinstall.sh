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
    #
    # loginctl list-sessions is unreliable in non-interactive dpkg contexts
    # (e.g. during a self-triggered apt upgrade). Instead, iterate over
    # /run/systemd/users/ which lists UIDs of all active user manager instances.
    for uid_path in /run/systemd/users/*; do
        user_id=$(basename "$uid_path")
        # Skip non-numeric entries (e.g. root = 0 is fine, but skip any stray files)
        case "$user_id" in
            *[!0-9]*) continue ;;
        esac
        # Skip root and system users (UID < 1000)
        if [ "$user_id" -lt 1000 ] 2>/dev/null; then
            continue
        fi
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
