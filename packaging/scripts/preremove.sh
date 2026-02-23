#!/bin/sh
set -e

# Stop and disable user services before the binary is removed.
# systemctl --user cannot be run as root, so we use loginctl to find
# active sessions and stop the units on behalf of each user.
for user_id in $(loginctl list-sessions --no-legend 2>/dev/null | awk '{print $3}' | sort -u); do
    for svc in assistant-slack assistant-mattermost; do
        systemctl --user -M "${user_id}@.host" stop    "$svc" 2>/dev/null || true
        systemctl --user -M "${user_id}@.host" disable "$svc" 2>/dev/null || true
    done
done

# Fallback: reload system daemon to drop stale unit references.
if command -v systemctl >/dev/null 2>&1; then
    systemctl daemon-reload 2>/dev/null || true
fi
