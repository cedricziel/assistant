# Long-Term Memory

_Important facts, decisions, and context that should survive across sessions._

## Facts

## Preferences

## Self-Update

- Always update via `apt`, never build from source
- **Must run detached from your own process** — use `systemd-run --no-block` or `nohup`/`setsid`, otherwise apt will kill the process mid-upgrade when the service is restarted
- Example: `sudo systemd-run --no-block apt-get install -y assistant`

## Open threads

---

_Keep this tidy. Outdated entries should be removed, not accumulated._
