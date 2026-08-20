# Deploying on TrueNAS SCALE

Runs the assistant as two Docker containers on TrueNAS SCALE
(Electric Eel 24.10+ / Fangtooth 25.04+, i.e. the Docker-based app engine
that replaced k3s). Files live in [`deploy/truenas/`](../../deploy/truenas/).

## Why two containers

`assistant webui serve` is intentionally a thin HTTP/SSE/UI server: it
publishes chat turns to the message bus but starts **no** turn-processing
worker pool and **no** title-generator worker. Those live in the
orchestrator. Running workers in both processes makes them race for every
bus claim, which shows up as per-second `claim failed` warnings.

| Container | Command | Role |
| --- | --- | --- |
| `assistant-webui` | `webui serve --listen 0.0.0.0:8080` | HTTP, SSE, Flutter SPA, OAuth |
| `assistant-orchestrator` | `orchestrator run --no-repl` | scheduler, bus, worker pool, messenger adapters |

Both bind-mount the same host dataset at `/home/assistant/.assistant`, so
they share `server.toml`, `orgs/` and the SQLite databases.

## 1. Create the dataset

The image runs as the non-root user `assistant` — **uid 100, gid 101**, home
`/home/assistant`. The dataset must be owned by that pair or every write
fails with `Permission denied`.

```sh
# On the TrueNAS box (Datasets UI, or zfs directly):
zfs create tank/apps/assistant

chown -R 100:101 /mnt/tank/apps/assistant
chmod 750 /mnt/tank/apps/assistant
```

Adjust `tank` to your pool name.

## 2. Deploy

### Option A — SSH + docker compose

```sh
mkdir -p /mnt/tank/apps/assistant-compose && cd $_
# copy docker-compose.yml and .env.example from deploy/truenas/
cp .env.example .env
vi .env                      # set ASSISTANT_DATA, ASSISTANT_WEB_TOKEN
docker compose up -d
```

### Option B — TrueNAS "Custom App"

Apps → Discover Apps → Custom App → *Install via YAML*, then paste
`docker-compose.yml`. The UI has no `.env` file, so `${VAR:-default}`
placeholders resolve to their defaults — replace them inline before saving:

- `${ASSISTANT_DATA:-/mnt/tank/apps/assistant}` → your dataset path
- `${ASSISTANT_WEB_TOKEN:-}` → your first-login token
- `${ASSISTANT_PORT:-8080}` → a free host port

## 3. First login

A fresh install has no `orgs/<slug>/org.db`, so there are no users yet and
the server falls back to legacy single-token auth. Open
`http://<hive>:8080`, enter the server URL and the `ASSISTANT_WEB_TOKEN`
value. Multi-user mode (OAuth2, password or OIDC) activates automatically
as soon as `org.db` exists — see [authentication.md](../authentication.md)
and [multi-user.md](../multi-user.md).

## 4. Configure the LLM provider

Drop a `config.toml` into the dataset root; the container reads it as
`~/.assistant/config.toml`:

```sh
# The image ships a fully commented example at /etc/assistant/config.toml.example
docker exec assistant-webui cat /etc/assistant/config.toml.example \
  > /mnt/tank/apps/assistant/config.toml
chown 100:101 /mnt/tank/apps/assistant/config.toml
```

The default provider is Ollama at `http://localhost:11434` — which inside
the container means *the container itself*. Point `base_url` at the real
host, e.g. `http://10.0.0.5:11434`, or switch to a cloud provider.

## Reverse proxy / TLS

Binding to a non-loopback address auto-sets the `Secure` cookie attribute,
which browsers reject over plain `http://` — login fails with no visible
error. Hence `--no-secure-cookie` in the shipped `.env.example`.

Behind a TLS-terminating proxy, do the opposite: clear
`ASSISTANT_WEBUI_ARGS` and set `ASSISTANT_PUBLIC_URL` to the external URL,
otherwise the OAuth issuer and the A2A agent card advertise
`http://0.0.0.0:8080`.

## Updating

```sh
docker compose pull && docker compose up -d
```

Pin `ASSISTANT_TAG` to a release tag so a restart can't silently pull a new
`latest`. Take a backup first — `assistant backup` archives the whole
installation into `~/.assistant/backups/` (excluded from its own walk, so
archives don't nest), which is inside the mounted dataset:

```sh
docker exec assistant-orchestrator assistant backup
# -> /mnt/tank/apps/assistant/backups/assistant-backup-YYYYMMDD-HHMMSS.tar.gz
```

## Troubleshooting

| Symptom | Cause |
| --- | --- |
| `Permission denied` on startup | dataset not owned by `100:101` |
| Login form reloads, no error | `Secure` cookie over plain HTTP — add `--no-secure-cookie` |
| `claim failed` every second | a second process runs workers; only the orchestrator may |
| OAuth redirects to `0.0.0.0:8080` | `ASSISTANT_PUBLIC_URL` unset behind a proxy |
| Data written to `/` instead of the volume | `HOME` unset; the compose file pins it |

`assistant doctor` diagnoses config, database and provider health:

```sh
docker exec assistant-webui assistant doctor
```
