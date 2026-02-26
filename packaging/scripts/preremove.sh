#!/bin/sh
set -e

# Stop user services before the binary is removed/upgraded.
# systemctl --user cannot be run as root, so we use loginctl to find
# active sessions and stop the units on behalf of each user.
#
# On upgrade ($1 = "upgrade") we only stop — the postinst will restart.
# On full remove ($1 = "remove") we also disable.
action="${1:-remove}"

for uid_path in /run/systemd/users/*; do
    user_id=$(basename "$uid_path")
    # Skip non-numeric entries
    case "$user_id" in
        *[!0-9]*) continue ;;
    esac
    # Skip root and system users (UID < 1000)
    if [ "$user_id" -lt 1000 ] 2>/dev/null; then
        continue
    fi
    for svc in assistant-slack assistant-mattermost assistant-web-ui; do
        systemctl --user -M "${user_id}@.host" stop "$svc" 2>/dev/null || true
        if [ "$action" = "remove" ]; then
            systemctl --user -M "${user_id}@.host" disable "$svc" 2>/dev/null || true
        fi
    done
done

# Fallback: reload system daemon to drop stale unit references.
if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload 2>/dev/null || true
fi
